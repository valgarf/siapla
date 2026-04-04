use super::task::GQLTask;
use crate::{entity::issue, gql::context::Context};
use juniper::GraphQLEnum;
use juniper::graphql_object;
use std::str::FromStr;
use strum::{EnumString, FromRepr, IntoStaticStr};

pub struct GQLIssue {
    pub model: issue::Model,
    pub revision: i64,
}

impl GQLIssue {
    pub fn at_revision(model: issue::Model, revision: i64) -> Self {
        Self { model, revision }
    }
}

impl From<issue::Model> for GQLIssue {
    fn from(model: issue::Model) -> Self {
        let revision = model.rev_created;
        Self { model, revision }
    }
}

#[graphql_object]
#[graphql(name = "Issue")]
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
        let model = self.model.query_task_iteration(ctx.db(), self.revision).await?;
        Ok(model.map(|m| GQLTask::at_revision(m, self.revision)))
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
