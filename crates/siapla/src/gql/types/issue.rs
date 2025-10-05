use crate::entity::task;
use crate::{entity::issue, gql::context::Context};
use juniper::GraphQLEnum;
use juniper::graphql_object;
use std::str::FromStr;
use strum::{EnumString, FromRepr, IntoStaticStr};

#[graphql_object]
#[graphql(name = "Issue")]
impl issue::Model {
    fn db_id(&self) -> &i32 {
        &self.id
    }
    fn code(&self) -> IssueCode {
        IssueCode::from_repr(self.code as usize).unwrap_or(IssueCode::Unknown)
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn r#type(&self) -> anyhow::Result<IssueType> {
        Ok(IssueType::from_str(&self.r#type)?)
    }
    pub async fn task(&self, ctx: &Context) -> anyhow::Result<Option<task::Model>> {
        const CIDX: usize = task::Column::Id as usize;
        let t = ctx.load_one_by_col::<task::Entity, CIDX>(self.task_id).await?;
        Ok(t)
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
