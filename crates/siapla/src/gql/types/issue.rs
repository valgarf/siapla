use super::task::GQLTask;
use crate::entity::task_iteration as task;
use crate::{entity::issue, gql::context::Context};
use anyhow::anyhow;
use juniper::GraphQLEnum;
use juniper::graphql_object;
use std::str::FromStr;
use strum::{EnumString, FromRepr, IntoStaticStr};

pub struct GQLIssue {
    pub model: issue::Model,
    pub revision: Option<i64>,
}

impl GQLIssue {
    pub fn at_revision(model: issue::Model, revision: Option<i64>) -> Self {
        Self { model, revision }
    }
}

impl From<issue::Model> for GQLIssue {
    fn from(model: issue::Model) -> Self {
        Self { model, revision: None }
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
        let Some(task_id) = self.model.task_id else {
            return Ok(None);
        };
        let txn = ctx.txn().await?;
        let revision = crate::revisioning::resolve_revision(txn, self.revision)
            .await?
            .ok_or(anyhow!("No revision found in database"))?;
        let revision_loader = ctx
            .loader(crate::gql::dataloader::ByColRevBatcher::<task::Entity> {
                revision,
                col: task::Column::HeaderId,
            })
            .await;
        if let Some(model) = revision_loader.load_one(task_id.into()).await? {
            return Ok(Some(GQLTask::at_revision(model, self.revision)));
        }
        let loader = ctx
            .loader(crate::gql::dataloader::ByColBatcher::<task::Entity> { col: task::Column::Id })
            .await;
        let t = loader.load_one(task_id.into()).await?;
        Ok(t.map(|m| GQLTask::at_revision(m, self.revision)))
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
