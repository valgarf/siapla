use chrono::{DateTime, Utc};

use juniper::{Nullable, graphql_object};
use sea_orm::{ActiveModelTrait as _, ActiveValue};

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
    async fn holiday(&self, ctx: &Context) -> anyhow::Result<Option<GQLHoliday>> {
        const CIDX: usize = holiday::Column::Id as usize;
        let holiday = ctx
            .load_one_by_col::<holiday::Entity, CIDX>(self.holiday_id)
            .await?;
        Ok(holiday.map(GQLHoliday::from_model))
    }
    pub async fn availability(&self, ctx: &Context) -> anyhow::Result<Vec<availability::Model>> {
        const CIDX: usize = availability::Column::ResourceId as usize;
        let availability = ctx
            .load_by_col::<availability::Entity, CIDX>(self.id)
            .await?;
        Ok(availability)
    }
    pub async fn vacation(&self, ctx: &Context) -> anyhow::Result<Vec<vacation::Model>> {
        const CIDX: usize = vacation::Column::ResourceId as usize;
        let vacation = ctx.load_by_col::<vacation::Entity, CIDX>(self.id).await?;
        Ok(vacation)
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
    let _added_vacations = resource.added_vacations.take();
    let _removed_vacations = resource.removed_vacations.take();
    let am = resource::ActiveModel::from(resource);
    let txn = ctx.txn().await?;
    let model = if am.id.is_set() {
        am.update(txn).await?
    } else {
        am.insert(txn).await?
    };

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
