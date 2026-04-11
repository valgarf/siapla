use chrono::{DateTime, Utc};

use juniper::{Nullable, graphql_object};
use sea_orm::ActiveValue;

use tracing::error;

use crate::db::DbContext;
use crate::db::delete::delete_rev_by_pk;
use crate::db::r#impl::resource_iteration::ResourceIterationUpserter;
use crate::db::insert::insert_rev;
use crate::db::upsert::LazyRevision;
use crate::db::upsert::upsert_rev;
use crate::entity::resource_header;
use crate::gql::scalars::ExtendedScalarValue;
use crate::gql::scalars::Int64;
use crate::gql::wrapper::ModelWrapper;
use crate::gql::wrapper::ResultVecToWrapper;
use crate::{
    entity::{resource_iteration, vacation},
    gql::{
        availability::GQLAvailability, common::nullable_to_av, context::Context,
        vacation::GQLVacation,
    },
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
#[graphql(name = "Interval", context = Context, scalar = ExtendedScalarValue)]
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

pub type GQLResource = ModelWrapper<resource_iteration::Entity>;

#[graphql_object]
#[graphql(name = "Resource", context = Context, scalar = ExtendedScalarValue)]
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
    fn rev_created(&self) -> Int64 {
        self.model.rev_created.into()
    }
    fn rev_deleted(&self) -> Option<Int64> {
        self.model.rev_deleted.map(|v| v.into())
    }
    async fn header_rev_created(&self, ctx: &Context) -> anyhow::Result<Int64> {
        let hm = self.model.dataloader_header(ctx.db()).await?;
        Ok(hm.rev_created.into())
    }
    async fn header_rev_deleted(&self, ctx: &Context) -> anyhow::Result<Option<Int64>> {
        let hm = self.model.dataloader_header(ctx.db()).await?;
        Ok(hm.rev_deleted.map(|v| v.into()))
    }

    pub async fn holiday(&self, ctx: &Context) -> anyhow::Result<Option<GQLHoliday>> {
        Ok(self.model.dataloader_holiday(ctx.db()).await?.map(GQLHoliday::from_model))
    }

    pub async fn availability(&self, ctx: &Context) -> anyhow::Result<Vec<GQLAvailability>> {
        self.model
            .dataloader_availability(ctx.db(), self.revision)
            .await
            .into_wrapper(self.revision)
    }

    pub async fn vacation(&self, ctx: &Context) -> anyhow::Result<Vec<GQLVacation>> {
        self.model.dataloader_vacation(ctx.db(), self.revision).await.into_wrapper(self.revision)
    }

    pub async fn combined_availability(
        &self,
        ctx: &Context,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> anyhow::Result<Vec<GQLInterval>> {
        let intervals = self
            .model
            .dataloader_combined_availability(
                ctx.db(),
                start.naive_utc(),
                end.naive_utc(),
                self.revision,
            )
            .await
            .inspect_err(|e| {
                error!("Failed to load combined availability from dataloader: {:?}", e)
            })?;

        Ok(intervals
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
    db: &DbContext,
    revision: &LazyRevision,
    mut resource: ResourceSaveInput,
) -> anyhow::Result<i32> {
    // ensure a header exists
    let header_id =
        resource_header::Model::ensure_header_id(db, revision, resource.db_id.take()).await?;

    // adding new vacations
    let added_vacations = resource.added_vacations.take().unwrap_or_default();
    let added_vacations_ams: Vec<_> = added_vacations
        .into_iter()
        .map(|v| {
            let mut vacation_am = crate::entity::vacation::ActiveModel::from(v);
            vacation_am.resource_id = ActiveValue::Set(header_id);
            vacation_am
        })
        .collect();
    insert_rev::<vacation::Entity>(db, revision, added_vacations_ams).await?;

    // removing vacations
    let removed_vacations = resource.removed_vacations.take().unwrap_or_default();
    delete_rev_by_pk::<vacation::Entity>(db, revision, removed_vacations).await?;

    // availability
    if let Some(availability) = resource.availability.take() {
        update_availability(db, header_id, availability, revision).await?;
    }

    // update resource iteration itself
    let upserter = ResourceIterationUpserter::new(header_id);
    let mut am = resource.into_active_model();
    am.header_id = ActiveValue::Set(header_id);
    upsert_rev(db, revision, upserter, vec![am]).await?;

    Ok(header_id)
}
