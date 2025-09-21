use chrono::{DateTime, Utc};

use juniper::{Nullable, graphql_object};
use sea_orm::ActiveValue;
use sea_orm::prelude::*;

use crate::{
    entity::{availability, holiday, resource, vacation},
    gql::{
        common::{nullable_to_av, opt_to_av},
        context::Context,
    },
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
        const CIDX: usize = availability::Column::ResourceId as usize;
        let availability = ctx.load_by_col::<availability::Entity, CIDX>(self.id).await?;
        Ok(availability)
    }
    pub async fn vacation(&self, ctx: &Context) -> anyhow::Result<Vec<vacation::Model>> {
        const CIDX: usize = vacation::Column::ResourceId as usize;
        let vacation = ctx.load_by_col::<vacation::Entity, CIDX>(self.id).await?;
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
        let ivs = ctx.load_combined_availability(self.id, s, e).await?;
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

impl From<ResourceSaveInput> for crate::entity::resource::ActiveModel {
    fn from(value: ResourceSaveInput) -> Self {
        crate::entity::resource::ActiveModel {
            id: opt_to_av!(value.db_id),
            name: ActiveValue::Set(value.name),
            timezone: ActiveValue::Set(value.timezone),
            added: ActiveValue::Set(value.added),
            removed: nullable_to_av!(value.removed),
            holiday_id: nullable_to_av!(value.holiday_id),
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
    let am = resource::ActiveModel::from(resource);
    let txn = ctx.txn().await?;
    let model = if am.id.is_set() { am.update(txn).await? } else { am.insert(txn).await? };

    // Handle adding new vacations
    for vacation_input in added_vacations {
        let mut vacation_am = crate::entity::vacation::ActiveModel::from(vacation_input);
        vacation_am.resource_id = ActiveValue::Set(model.id);
        vacation_am.insert(txn).await?;
    }

    // Handle removing vacations
    if !removed_vacations.is_empty() {
        vacation::Entity::delete_many()
            .filter(vacation::Column::Id.is_in(removed_vacations))
            .exec(txn)
            .await?;
    }

    if let Some(availability) = availability {
        update_availability(ctx, &model, availability).await?;
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
