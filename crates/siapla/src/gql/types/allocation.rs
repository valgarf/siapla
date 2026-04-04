use super::resource::GQLResource;
use super::task::GQLTask;
use crate::gql::common::resolve_many_to_many;
use crate::gql::dataloader::{ByColBatcher, ByColRevBatcher};
use crate::revisioning::resolve_revision;
use crate::{
    entity::{
        allocated_resource, allocation, resource_iteration as resource, task_iteration as task,
    },
    gql::context::Context,
};
use chrono::{DateTime, Utc};
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

pub struct GQLAllocation {
    pub model: allocation::Model,
    pub revision: Option<i64>,
}

impl GQLAllocation {
    pub fn at_revision(model: allocation::Model, revision: Option<i64>) -> Self {
        Self { model, revision }
    }
}

impl From<allocation::Model> for GQLAllocation {
    fn from(model: allocation::Model) -> Self {
        Self { model, revision: None }
    }
}

#[graphql_object]
#[graphql(name = "Allocation")]
impl GQLAllocation {
    fn db_id(&self) -> &i32 {
        &self.model.id
    }
    fn start(&self) -> &DateTime<Utc> {
        &self.model.start
    }
    fn end(&self) -> &DateTime<Utc> {
        &self.model.end
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
            target_revision: self.revision,
            allocated_resource::Entity,
            allocated_resource::Column::AllocationId,
            self.model.id,
            |l: allocated_resource::Model| l.resource_id,
            resource::Entity,
            resource::Column::HeaderId
        )?;
        Ok(models.into_iter().map(|m| GQLResource::at_revision(m, self.revision)).collect())
    }
    pub async fn task(&self, ctx: &Context) -> anyhow::Result<GQLTask> {
        let txn = ctx.txn().await?;
        let revision = resolve_revision(txn, self.revision)
            .await?
            .ok_or(anyhow::anyhow!("No revision found in database"))?;
        let current = ctx
            .loader(ByColRevBatcher::<task::Entity> { revision, col: task::Column::HeaderId })
            .await
            .load_one(self.model.task_id.into())
            .await?;
        if let Some(model) = current {
            return Ok(GQLTask::at_revision(model, self.revision));
        }
        let model = ctx
            .loader(ByColBatcher::<task::Entity> { col: task::Column::Id })
            .await
            .load_one(self.model.task_id.into())
            .await?
            .expect("Task must exist.");
        Ok(GQLTask::at_revision(model, self.revision))
    }
}
