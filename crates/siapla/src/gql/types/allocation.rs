use super::resource::GQLResource;
use super::task::GQLTask;
use crate::gql::common::resolve_many_to_many;
use crate::{
    entity::{
        allocated_resource, allocation, resource_iteration as resource, task_iteration as task,
    },
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
        Ok(AllocationType::PLAN)
    }
    fn r#final(&self) -> bool {
        false
    }
    pub async fn resources(&self, ctx: &Context) -> anyhow::Result<Vec<GQLResource>> {
        let models: Vec<resource::Model> = resolve_many_to_many!(
            ctx,
            allocated_resource::Entity,
            allocated_resource::Column::AllocationId,
            self.id,
            |l: allocated_resource::Model| l.resource_id,
            resource::Entity,
            resource::Column::Id
        )?;
        Ok(models.into_iter().map(GQLResource::from).collect())
    }
    pub async fn task(&self, ctx: &Context) -> anyhow::Result<GQLTask> {
        let txn = ctx.txn().await?;
        use sea_orm::{ColumnTrait as _, EntityTrait as _, QueryFilter as _};
        // task_id references task_header.id; find the current active iteration
        let current = task::Entity::find()
            .filter(task::Column::HeaderId.eq(self.task_id))
            .filter(task::Column::RevDeleted.is_null())
            .one(txn)
            .await?;
        if let Some(model) = current {
            return Ok(GQLTask::from(model));
        }
        // Fallback: try direct id lookup for backward compatibility
        const CIDX: usize = task::Column::Id as usize;
        let model = ctx
            .load_one_by_col::<task::Entity, CIDX>(self.task_id)
            .await?
            .expect("Task must exist.");
        Ok(GQLTask::from(model))
    }
}
