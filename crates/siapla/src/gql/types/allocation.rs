use super::resource::GQLResource;
use super::task::GQLTask;
use crate::gql::dataloader::ByColRevBatcher;
use crate::{
    entity::{allocation, resource_iteration as resource, task_iteration as task},
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
    pub revision: i64,
}

impl GQLAllocation {
    pub fn at_revision(model: allocation::Model, revision: i64) -> Self {
        Self { model, revision }
    }
}

impl From<allocation::Model> for GQLAllocation {
    fn from(model: allocation::Model) -> Self {
        let revision = model.rev_created;
        Self { model, revision }
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
        let models: Vec<resource::Model> = ctx
            .loader(
                crate::gql::dataloader::LinkBatcher::<
                    crate::ResourceIterationsFromAllocation,
                >::new(self.revision),
            )
            .await
            .load(self.model.id.into())
            .await?;
        Ok(models
            .into_iter()
            .map(|m| GQLResource::at_revision(m, self.revision))
            .collect())
    }
    pub async fn task(&self, ctx: &Context) -> anyhow::Result<GQLTask> {
        let revision = self.revision;
        let current = ctx
            .loader(ByColRevBatcher::<task::Entity> {
                revision,
                col: task::Column::HeaderId
            })
            .await
            .load_one(self.model.task_id.into())
            .await?;
        if let Some(model) = current {
            return Ok(GQLTask::at_revision(model, revision));
        }
        let model = ctx
            .loader(crate::gql::dataloader::ByColBatcher::<task::Entity> {
                col: task::Column::Id,
            })
            .await
            .load_one(self.model.task_id.into())
            .await?
            .expect("Task must exist.");
        Ok(GQLTask::at_revision(model, revision))
    }
}
