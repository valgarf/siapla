use super::resource::GQLResource;
use super::task::GQLTask;
use crate::{
    entity::allocation,
    gql::{
        context::Context,
        scalars::ExtendedScalarValue,
        wrapper::{ModelWrapper, ResultVecToWrapper},
    },
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

pub type GQLAllocation = ModelWrapper<allocation::Entity>;

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
        self.model
            .dataloader_resource_iterations(ctx.db(), self.revision)
            .await
            .into_wrapper(self.revision)
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
