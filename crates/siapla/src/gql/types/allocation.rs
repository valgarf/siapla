use chrono::{DateTime, Utc};

use juniper::{Nullable, graphql_object};
use sea_orm::prelude::*;
use sea_orm::{ActiveModelTrait as _, ActiveValue};

use crate::gql::common::resolve_many_to_many;
use crate::{
    entity::{allocated_resource, allocation, resource},
    gql::{
        common::{nullable_to_av, opt_to_av},
        context::Context,
    },
};
use itertools::Itertools;

#[graphql_object]
#[graphql(name = "Allocation")]
impl allocation::Model {
    fn db_id(&self) -> &i32 {
        &self.id
    }
    fn start(&self) -> &DateTime<Utc> {
        &self.start
    }
    fn end(&self) -> &DateTime<Utc> {
        &self.end
    }
    pub async fn resources(&self, ctx: &Context) -> anyhow::Result<Vec<resource::Model>> {
        resolve_many_to_many!(
            ctx,
            allocated_resource::Entity,
            allocated_resource::Column::AllocationId,
            self.id,
            |l: allocated_resource::Model| l.resource_id,
            resource::Entity,
            resource::Column::Id
        )
    }
    // pub async fn availability(&self, ctx: &Context) -> anyhow::Result<Vec<availability::Model>> {
    //     const CIDX: usize = availability::Column::ResourceId as usize;
    //     let availability = ctx.load_by_col::<availability::Entity, CIDX>(self.id).await?;
    //     Ok(availability)
    // }
    // pub async fn vacation(&self, ctx: &Context) -> anyhow::Result<Vec<vacation::Model>> {
    //     const CIDX: usize = vacation::Column::ResourceId as usize;
    //     let vacation = ctx.load_by_col::<vacation::Entity, CIDX>(self.id).await?;
    //     Ok(vacation)
    // }
}
