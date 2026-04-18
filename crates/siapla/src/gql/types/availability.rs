use super::resource::GQLResource;
use crate::{
    db::{
        DbContext, r#impl::availability::AvailabilityUpserter, revisioning::LazyRevision,
        upsert::upsert_rev_many,
    },
    entity::availability,
    gql::{context::Context, scalars::ExtendedScalarValue, wrapper::ModelWrapper},
};
use juniper::{GraphQLEnum, graphql_object};
use sea_orm::{ActiveValue, prelude::*};
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

pub async fn update_availability(
    db: &DbContext,
    resource_header_id: i32,
    availability: Vec<AvailabilityInput>,
    revision: &LazyRevision,
) -> anyhow::Result<()> {
    let new_models = availability
        .iter()
        .map(|a| {
            let mut am: availability::ActiveModel = a.into();
            am.resource_id = ActiveValue::Set(resource_header_id);
            am
        })
        .collect::<Vec<availability::ActiveModel>>();
    upsert_rev_many(db, &revision, AvailabilityUpserter::new(resource_header_id), new_models).await
}
