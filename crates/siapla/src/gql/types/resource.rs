use chrono::{DateTime, Utc};

use juniper::{Nullable, graphql_object};
use sea_orm::ActiveValue;
use sea_orm::prelude::*;
use tracing::error;

use crate::db::dataloader::AvailabilityBatcher;
use crate::{
    entity::{resource_header, resource_iteration as resource, vacation},
    gql::{
        availability::GQLAvailability, common::nullable_to_av, context::Context,
        vacation::GQLVacation,
    },
    revisioning::{PlanState, create_revision},
};

use super::{
    availability::{AvailabilityInput, update_availability},
    holiday::GQLHoliday,
    vacation::VacationInput,
};

use crate::scheduling::Interval;

// ---------------------------------------------------------------------------
// GQLInterval (unchanged)
// ---------------------------------------------------------------------------

pub struct GQLInterval {
    iv: Interval<DateTime<Utc>>,
}

#[graphql_object]
#[graphql(name = "Interval")]
impl GQLInterval {
    pub fn start(&self) -> DateTime<Utc> {
        self.iv.start().value().expect("Must be bounded")
    }
    pub fn end(&self) -> DateTime<Utc> {
        self.iv.end().value().expect("Must be bounded")
    }
}

// ---------------------------------------------------------------------------
// GQLResource – revision-aware GraphQL wrapper for resource_iteration::Model
// ---------------------------------------------------------------------------

pub struct GQLResource {
    pub model: resource::Model,
    pub revision: i64,
    pub header_model: Option<resource_header::Model>,
}

impl GQLResource {
    pub fn at_revision(model: resource::Model, revision: i64) -> Self {
        Self { model, revision, header_model: None }
    }

    pub fn with_header(model: resource::Model, header_model: resource_header::Model) -> Self {
        let revision = model.rev_created;
        Self { model, revision, header_model: Some(header_model) }
    }
}

impl From<resource::Model> for GQLResource {
    fn from(model: resource::Model) -> Self {
        let revision = model.rev_created;
        Self { model, revision, header_model: None }
    }
}

#[graphql_object]
#[graphql(name = "Resource")]
impl GQLResource {
    fn db_id(&self) -> i32 {
        self.model.header_id
    }
    fn iteration_id(&self) -> &i32 {
        &self.model.id
    }
    fn name(&self) -> &str {
        &self.model.name
    }
    fn timezone(&self) -> &str {
        &self.model.timezone
    }
    fn added(&self) -> &DateTime<Utc> {
        &self.model.added
    }
    fn removed(&self) -> &Option<DateTime<Utc>> {
        &self.model.removed
    }
    fn rev_created(&self) -> i32 {
        self.model.rev_created as i32
    }
    fn rev_deleted(&self) -> Option<i32> {
        self.model.rev_deleted.map(|v| v as i32)
    }
    async fn header_rev_created(&self, ctx: &Context) -> anyhow::Result<Option<i32>> {
        let hm = self.load_header(ctx).await?;
        Ok(hm.map(|h| h.rev_created as i32))
    }
    async fn header_rev_deleted(&self, ctx: &Context) -> anyhow::Result<Option<i32>> {
        let hm = self.load_header(ctx).await?;
        Ok(hm.and_then(|h| h.rev_deleted).map(|v| v as i32))
    }

    pub async fn holiday(&self, ctx: &Context) -> anyhow::Result<Option<GQLHoliday>> {
        Ok(self.model.query_holiday(ctx.db()).await?.map(GQLHoliday::from_model))
    }

    pub async fn availability(&self, ctx: &Context) -> anyhow::Result<Vec<GQLAvailability>> {
        let availability = self.model.query_availability(ctx.db(), self.revision).await?;
        Ok(availability
            .into_iter()
            .map(|m| GQLAvailability::at_revision(m, self.revision))
            .collect())
    }

    pub async fn vacation(&self, ctx: &Context) -> anyhow::Result<Vec<GQLVacation>> {
        let vacation = self.model.query_vacation(ctx.db(), self.revision).await?;
        Ok(vacation.into_iter().map(|m| GQLVacation::at_revision(m, self.revision)).collect())
    }

    pub async fn combined_availability(
        &self,
        ctx: &Context,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> anyhow::Result<Vec<GQLInterval>> {
        let start = start.naive_utc();
        let end = end.naive_utc();
        let revision = self.revision;
        let loader = ctx.loader(AvailabilityBatcher { start, end, revision }).await;

        let ivs = loader.load(self.model.id).await.inspect_err(|e| {
            error!("Failed to load combined availability from dataloader: {:?}", e)
        })?;
        Ok(ivs
            .into_iter()
            .map(|iv| GQLInterval {
                iv: Interval::new_closed(
                    iv.start().value().expect("Must be bounded").and_utc(),
                    iv.end().value().expect("Must be bounded").and_utc(),
                ),
            })
            .collect())
    }
}

impl GQLResource {
    async fn load_header(&self, ctx: &Context) -> anyhow::Result<Option<resource_header::Model>> {
        if let Some(ref hm) = self.header_model {
            return Ok(Some(hm.clone()));
        }
        let hid = self.model.header_id;
        let txn = ctx.txn().await?;
        Ok(resource_header::Entity::find_by_id(hid).one(txn).await?)
    }
}

// Input / save logic
// ---------------------------------------------------------------------------

#[derive(juniper::GraphQLInputObject)]
pub struct ResourceSaveInput {
    /// When set, this is the **header id** (stable identity) of the resource to update.
    db_id: Option<i32>,
    name: String,
    timezone: String,
    added: DateTime<Utc>,
    removed: Nullable<DateTime<Utc>>,
    holiday_id: Nullable<i32>,
    pub availability: Option<Vec<AvailabilityInput>>,
    pub added_vacations: Option<Vec<VacationInput>>,
    pub removed_vacations: Option<Vec<i32>>,
}

impl ResourceSaveInput {
    fn into_active_model(self) -> crate::entity::resource_iteration::ActiveModel {
        crate::entity::resource_iteration::ActiveModel {
            id: ActiveValue::NotSet,
            name: ActiveValue::Set(self.name),
            timezone: ActiveValue::Set(self.timezone),
            added: ActiveValue::Set(self.added),
            removed: nullable_to_av!(self.removed),
            holiday_id: nullable_to_av!(self.holiday_id),
            header_id: ActiveValue::NotSet,
            rev_created: ActiveValue::NotSet,
            rev_deleted: ActiveValue::NotSet,
        }
    }
}

/// Save or update a resource. Returns the raw model; the caller wraps into GQLResource.
pub async fn resource_save(
    ctx: &Context,
    mut resource: ResourceSaveInput,
) -> anyhow::Result<resource::Model> {
    let availability = resource.availability.take();
    let added_vacations = resource.added_vacations.take().unwrap_or_default();
    let removed_vacations = resource.removed_vacations.take().unwrap_or_default();
    let input_header_id = resource.db_id.take();
    let txn = ctx.txn().await?;
    let revision_id = create_revision(txn, PlanState::NotCalculated).await?;
    let mut am = resource.into_active_model();

    let model = if let Some(header_id) = input_header_id {
        let existing = resource::Entity::find()
            .filter(resource::Column::HeaderId.eq(header_id))
            .filter(resource::Column::RevDeleted.is_null())
            .one(txn)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("No active resource iteration found for header {}", header_id)
            })?;
        let old_id = existing.id;
        // Soft-delete the old iteration
        resource::Entity::update_many()
            .col_expr(resource::Column::RevDeleted, Expr::value(Value::BigInt(Some(revision_id))))
            .filter(resource::Column::Id.eq(old_id))
            .filter(resource::Column::RevDeleted.is_null())
            .exec(txn)
            .await?;
        // Create new iteration
        am.header_id = ActiveValue::Set(existing.header_id);
        am.rev_created = ActiveValue::Set(revision_id);
        am.rev_deleted = ActiveValue::Set(None);
        am.insert(txn).await?
    } else {
        let header = crate::entity::resource_header::ActiveModel {
            id: ActiveValue::NotSet,
            rev_created: ActiveValue::Set(revision_id),
            rev_deleted: ActiveValue::Set(None),
        }
        .insert(txn)
        .await?;
        am.header_id = ActiveValue::Set(header.id);
        am.rev_created = ActiveValue::Set(revision_id);
        am.rev_deleted = ActiveValue::Set(None);
        am.insert(txn).await?
    };

    // Handle adding new vacations
    for vacation_input in added_vacations {
        let mut vacation_am = crate::entity::vacation::ActiveModel::from(vacation_input);
        vacation_am.resource_id = ActiveValue::Set(model.header_id);
        vacation_am.rev_created = ActiveValue::Set(revision_id);
        vacation_am.rev_deleted = ActiveValue::Set(None);
        vacation_am.insert(txn).await?;
    }

    // Handle removing vacations
    if !removed_vacations.is_empty() {
        vacation::Entity::update_many()
            .col_expr(vacation::Column::RevDeleted, Expr::value(Value::BigInt(Some(revision_id))))
            .filter(vacation::Column::Id.is_in(removed_vacations))
            .exec(txn)
            .await?;
    }

    if let Some(availability) = availability {
        update_availability(ctx, &model, availability, revision_id).await?;
    }

    Ok(model)
}
