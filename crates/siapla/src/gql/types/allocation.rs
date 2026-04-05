use super::resource::GQLResource;
use super::task::GQLTask;
use crate::{
    entity::allocation,
    gql::{context::Context, scalars::ExtendedScalarValue},
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
#[graphql(name = "Allocation", context = Context, scalar = ExtendedScalarValue)]
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
        let models = self.model.dataloader_resource_iterations(ctx.db(), self.revision).await?;
        Ok(models.into_iter().map(|m| GQLResource::at_revision(m, self.revision)).collect())
    }
    pub async fn task(&self, ctx: &Context) -> anyhow::Result<GQLTask> {
        let model = self
            .model
            .dataloader_task_iteration(ctx.db(), self.revision)
            .await?
            .expect("Task must exist.");
        Ok(GQLTask::at_revision(model, self.revision))
    }
}
