use super::task::GQLTask;
use crate::gql::wrapper::ResultOptionToWrapper;
use crate::gql::{scalars::ExtendedScalarValue, wrapper::ModelWrapper};
use crate::{entity::issue, gql::context::Context};
use juniper::GraphQLEnum;
use juniper::graphql_object;
use std::str::FromStr;
use strum::{EnumString, FromRepr, IntoStaticStr};

pub type GQLIssue = ModelWrapper<issue::Entity>;

#[graphql_object]
#[graphql(name = "Issue", context = Context, scalar = ExtendedScalarValue)]
impl GQLIssue {
    fn db_id(&self) -> &i32 {
        &self.model.id
    }
    fn code(&self) -> IssueCode {
        IssueCode::from_repr(self.model.code as usize).unwrap_or(IssueCode::Unknown)
    }
    fn description(&self) -> &str {
        &self.model.description
    }
    fn r#type(&self) -> anyhow::Result<IssueType> {
        Ok(IssueType::from_str(&self.model.r#type)?)
    }
    pub async fn task(&self, ctx: &Context) -> anyhow::Result<Option<GQLTask>> {
        self.model
            .dataloader_task_iteration(ctx.db(), self.revision)
            .await
            .into_wrapper(self.revision)
    }
}

#[derive(GraphQLEnum, IntoStaticStr, EnumString)]
pub enum IssueType {
    #[graphql(name = "TASK")]
    Task,
    #[graphql(name = "PLANNING_TASK")]
    PlanningTask,
    #[graphql(name = "PLANNING_GENERAL")]
    PlanningGeneral,
    #[graphql(name = "GENERAL")]
    General,
}

#[derive(GraphQLEnum, FromRepr, Debug, PartialEq, Clone, Copy)]
#[repr(usize)]
pub enum IssueCode {
    PredIssue = 101,
    RequirementMissing = 201,
    MilestoneMissing = 202,
    ResourceMissing = 203,
    NoEffort = 204,
    NoSlotFound = 301,
    DependencyLoop = 302,
    HierarchyLoop = 303,
    Unknown = 999,
}
