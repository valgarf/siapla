use super::resource::GQLResource;
use crate::{
    db::upserter::{LazyRevision, Upserter, upsert},
    entity::{availability, resource_iteration},
    gql::{context::Context, scalars::ExtendedScalarValue, wrapper::ModelWrapper},
};
use juniper::{GraphQLEnum, graphql_object};
use sea_orm::{ActiveValue, prelude::*};
use siapla_migration::IntoCondition;
use strum::{EnumString, IntoStaticStr};

#[derive(GraphQLEnum, IntoStaticStr, EnumString, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl From<Weekday> for String {
    fn from(value: Weekday) -> Self {
        let s: &'static str = value.into();
        s.into()
    }
}

pub type GQLAvailability = ModelWrapper<availability::Entity>;

#[graphql_object]
#[graphql(name = "Availability", context = Context, scalar = ExtendedScalarValue)]
impl GQLAvailability {
    fn db_id(&self) -> &i32 {
        &self.model.id
    }
    async fn resource(&self, ctx: &Context) -> anyhow::Result<GQLResource> {
        self.model
            .dataloader_resource(ctx.db(), self.revision)
            .await
            .map(|r| GQLResource::at_revision(r, self.revision))
    }
    fn duration(&self) -> anyhow::Result<i32> {
        let mut secs = self.model.duration * Decimal::new(3600, 0);
        secs.rescale(0); // rounding to whole seconds
        Ok(secs.try_into()?)
    }
    fn weekday(&self) -> anyhow::Result<Weekday> {
        Ok(self.model.weekday.as_str().try_into()?)
    }
}

#[derive(juniper::GraphQLInputObject, Debug)]
pub struct AvailabilityInput {
    weekday: Weekday,
    duration: i32,
}

impl From<&AvailabilityInput> for crate::entity::availability::ActiveModel {
    fn from(value: &AvailabilityInput) -> Self {
        crate::entity::availability::ActiveModel {
            id: ActiveValue::NotSet,
            resource_id: ActiveValue::NotSet,
            weekday: ActiveValue::Set(value.weekday.into()),
            duration: ActiveValue::Set(Decimal::from(value.duration) / Decimal::from(3600)),
            rev_created: ActiveValue::NotSet,
            rev_deleted: ActiveValue::NotSet,
        }
    }
}

pub struct AvailabilityUpserter {
    pub resource_header_id: i32,
}

impl AvailabilityUpserter {
    pub fn new(resource_header_id: i32) -> Self {
        Self { resource_header_id }
    }
}

impl Upserter for AvailabilityUpserter {
    type Entity = availability::Entity;
    type Key = Weekday;

    fn existing_condition(&self) -> sea_orm::Condition {
        availability::Column::ResourceId.eq(self.resource_header_id).into_condition()
    }

    fn key(&self, model: &availability::ActiveModel) -> anyhow::Result<Weekday> {
        model
            .weekday
            .try_as_ref()
            .ok_or(anyhow::anyhow!("availability model needs to have a weekday"))?
            .as_str()
            .try_into()
            .map_err(anyhow::Error::from)
    }

    fn model_equal(
        &self,
        lhs: &<Self::Entity as EntityTrait>::ActiveModel,
        rhs: &<Self::Entity as EntityTrait>::ActiveModel,
    ) -> bool {
        lhs.duration == rhs.duration
    }
}

pub async fn update_availability(
    ctx: &Context,
    model: &resource_iteration::Model,
    availability: Vec<AvailabilityInput>,
    revision_id: i64,
) -> anyhow::Result<()> {
    let new_models = availability
        .iter()
        .map(|a| {
            let mut am: availability::ActiveModel = a.into();
            am.resource_id = ActiveValue::Set(model.header_id);
            am.rev_created = ActiveValue::Set(revision_id);
            am.rev_deleted = ActiveValue::Set(None);
            am
        })
        .collect::<Vec<availability::ActiveModel>>();
    let lazy_rev = LazyRevision::from_revision(revision_id);
    upsert(ctx.db(), &lazy_rev, AvailabilityUpserter::new(model.header_id), new_models).await
}
