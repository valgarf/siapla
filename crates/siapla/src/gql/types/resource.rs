use chrono::{DateTime, Utc};

use juniper::{Nullable, graphql_object};
use sea_orm::ActiveValue;
use sea_orm::prelude::*;
use tracing::error;

use crate::{
    entity::{availability, holiday, resource_iteration as resource, vacation},
    gql::{
        common::{nullable_to_av, opt_to_av},
        context::Context,
    },
    revisioning::{create_revision, PlanState},
};

use super::{
    availability::{AvailabilityInput, update_availability},
    holiday::GQLHoliday,
    vacation::VacationInput,
};

use crate::scheduling::Interval;

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
#[graphql_object]
#[graphql(name = "Resource")]
impl resource::Model {
    fn db_id(&self) -> &i32 {
        &self.id
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn timezone(&self) -> &str {
        &self.timezone
    }
    fn added(&self) -> &DateTime<Utc> {
        &self.added
    }
    fn removed(&self) -> &Option<DateTime<Utc>> {
        &self.removed
    }
    pub async fn holiday(&self, ctx: &Context) -> anyhow::Result<Option<GQLHoliday>> {
        const CIDX: usize = holiday::Column::Id as usize;
        let holiday = ctx.load_one_by_col::<holiday::Entity, CIDX>(self.holiday_id).await?;
        Ok(holiday.map(GQLHoliday::from_model))
    }
    pub async fn availability(&self, ctx: &Context) -> anyhow::Result<Vec<availability::Model>> {
        let availability = availability::Entity::find()
            .filter(availability::Column::ResourceId.eq(self.id))
            .filter(availability::Column::RevDeleted.is_null())
            .all(ctx.txn().await?)
            .await?;
        Ok(availability)
    }
    pub async fn vacation(&self, ctx: &Context) -> anyhow::Result<Vec<vacation::Model>> {
        let vacation = vacation::Entity::find()
            .filter(vacation::Column::ResourceId.eq(self.id))
            .filter(vacation::Column::RevDeleted.is_null())
            .all(ctx.txn().await?)
            .await?;
        Ok(vacation)
    }
    pub async fn combined_availability(
        &self,
        ctx: &Context,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> anyhow::Result<Vec<GQLInterval>> {
        // Context::load_combined_availability expects NaiveDateTime start/end in UTC
        let s = start.naive_utc();
        let e = end.naive_utc();
        let ivs = ctx.load_combined_availability(self.id, s, e, None).await.inspect_err(|e| {
            error!("Faild to load combined availability from dataloader:: {:?}", e)
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

#[derive(juniper::GraphQLInputObject)]
pub struct ResourceSaveInput {
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

impl From<ResourceSaveInput> for crate::entity::resource_iteration::ActiveModel {
    fn from(value: ResourceSaveInput) -> Self {
        crate::entity::resource_iteration::ActiveModel {
            id: opt_to_av!(value.db_id),
            name: ActiveValue::Set(value.name),
            timezone: ActiveValue::Set(value.timezone),
            added: ActiveValue::Set(value.added),
            removed: nullable_to_av!(value.removed),
            holiday_id: nullable_to_av!(value.holiday_id),
            header_id: ActiveValue::NotSet,
            rev_created: ActiveValue::NotSet,
            rev_deleted: ActiveValue::NotSet,
        }
    }
}

pub async fn resource_save(
    ctx: &Context,
    mut resource: ResourceSaveInput,
) -> anyhow::Result<resource::Model> {
    let availability = resource.availability.take();
    let added_vacations = resource.added_vacations.take().unwrap_or_default();
    let removed_vacations = resource.removed_vacations.take().unwrap_or_default();
    let txn = ctx.txn().await?;
    let revision_id = create_revision(txn, PlanState::NotCalculated).await?;
    let mut am = resource::ActiveModel::from(resource);

    let model = if am.id.is_set() {
        am.rev_deleted = ActiveValue::Set(Some(revision_id));
        let old = am.update(txn).await?;
        resource::ActiveModel {
            id: ActiveValue::NotSet,
            name: ActiveValue::Set(old.name),
            timezone: ActiveValue::Set(old.timezone),
            added: ActiveValue::Set(old.added),
            removed: ActiveValue::Set(old.removed),
            holiday_id: ActiveValue::Set(old.holiday_id),
            header_id: ActiveValue::Set(old.header_id),
            rev_created: ActiveValue::Set(revision_id),
            rev_deleted: ActiveValue::Set(None),
        }
        .insert(txn)
        .await?
    } else {
        let header = crate::entity::resource_header::ActiveModel {
            id: ActiveValue::NotSet,
            rev_created: ActiveValue::Set(revision_id),
            rev_deleted: ActiveValue::Set(None),
        }
        .insert(txn)
        .await?;
        am.header_id = ActiveValue::Set(Some(header.id));
        am.rev_created = ActiveValue::Set(revision_id);
        am.rev_deleted = ActiveValue::Set(None);
        am.insert(txn).await?
    };

    // Handle adding new vacations
    for vacation_input in added_vacations {
        let mut vacation_am = crate::entity::vacation::ActiveModel::from(vacation_input);
        vacation_am.resource_id = ActiveValue::Set(model.id);
        vacation_am.rev_created = ActiveValue::Set(revision_id);
        vacation_am.rev_deleted = ActiveValue::Set(None);
        vacation_am.insert(txn).await?;
    }

    // Handle removing vacations
    if !removed_vacations.is_empty() {
        vacation::Entity::update_many()
            .col_expr(
                vacation::Column::RevDeleted,
                Expr::value(Value::BigUnsigned(Some(revision_id))),
            )
            .filter(vacation::Column::Id.is_in(removed_vacations))
            .exec(txn)
            .await?;
    }

    if let Some(availability) = availability {
        update_availability(ctx, &model, availability, revision_id).await?;
    }

    // if let Some(successors) = successors {
    //     update_successors(ctx, &model, successors).await?;
    // }

    // if let Some(mut children) = children {
    //     update_children(ctx, &model, children).await?;
    // }

    // TODO: before committing, check if we now have predecessor or parent cycles!
    Ok(model)
}
