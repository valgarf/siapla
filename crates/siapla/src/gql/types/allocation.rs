use crate::entity::task;
use crate::gql::common::resolve_many_to_many;
use crate::{
    entity::{allocated_resource, allocation, resource},
    gql::context::Context,
};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use juniper::{GraphQLEnum, graphql_object};
use strum::{EnumString, IntoStaticStr};

#[derive(GraphQLEnum, IntoStaticStr, EnumString, PartialEq, Eq, Clone, Copy, Debug)]
pub enum AllocationType {
    PLAN,
    BOOKING,
}

impl From<AllocationType> for String {
    fn from(v: AllocationType) -> Self {
        let s: &'static str = v.into();
        s.into()
    }
}

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
    fn allocation_type(&self) -> anyhow::Result<AllocationType> {
        Ok(self.allocation_type.as_str().try_into()?)
    }
    fn r#final(&self) -> bool {
        self.r#final
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
    pub async fn task(&self, ctx: &Context) -> anyhow::Result<task::Model> {
        const CIDX: usize = task::Column::Id as usize;
        ctx.load_one_by_col::<task::Entity, CIDX>(self.task_id)
            .await
            .map(|opt_t| opt_t.expect("Task must exist."))
    }
}
