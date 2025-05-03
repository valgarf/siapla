use std::str::FromStr;

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use itertools::{Either, Itertools as _};
use juniper::{FieldResult, GraphQLEnum, Nullable, graphql_object};
use sea_orm::ActiveValue;
use strum::{EnumString, IntoStaticStr};

use crate::{
    entity::{dependency, task},
    gql::{
        common::{nullable_to_av, opt_to_av, resolve_many_to_many},
        context::Context,
    },
};

#[derive(GraphQLEnum, IntoStaticStr, EnumString)]
enum TaskDesignation {
    Task,
    Group,
    Requirement,
    Milestone,
}

impl From<TaskDesignation> for String {
    fn from(value: TaskDesignation) -> Self {
        let s: &'static str = value.into();
        s.into()
    }
}

#[graphql_object]
#[graphql(name = "Task")]
impl task::Model {
    fn db_id(&self) -> &i32 {
        &self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn earliest_start(&self) -> &Option<DateTime<Utc>> {
        &self.earliest_start
    }
    fn schedule_target(&self) -> &Option<DateTime<Utc>> {
        &self.schedule_target
    }
    fn effort(&self) -> Option<f64> {
        self.effort.map(Into::into)
    }
    fn designation(&self) -> FieldResult<TaskDesignation> {
        Ok(TaskDesignation::from_str(&self.designation)?)
    }
    pub async fn predecessors(&self, ctx: &Context) -> FieldResult<Vec<Self>> {
        let result = resolve_many_to_many!(
            ctx,
            dependency::Entity,
            dependency::Column::SuccessorId,
            self.id,
            |l: dependency::Model| l.predecessor_id,
            task::Entity,
            task::Column::Id
        );
        Ok(result?)
    }
    pub async fn successors(&self, ctx: &Context) -> FieldResult<Vec<Self>> {
        let result = resolve_many_to_many!(
            ctx,
            dependency::Entity,
            dependency::Column::PredecessorId,
            self.id,
            |l: dependency::Model| l.successor_id,
            task::Entity,
            task::Column::Id
        );
        Ok(result?)
    }
    pub async fn children(&self, ctx: &Context) -> FieldResult<Vec<Self>> {
        const CIDX: usize = task::Column::ParentId as usize;
        Ok(ctx.load_by_col::<task::Entity, CIDX>(self.id).await?)
    }
    async fn parent(&self, ctx: &Context) -> anyhow::Result<Option<Self>> {
        match self.parent_id {
            None => Ok(None),
            Some(parent_id) => {
                const CIDX: usize = task::Column::Id as usize;
                ctx.load_one_by_col::<task::Entity, CIDX>(parent_id).await
            }
        }
    }
}

#[derive(juniper::GraphQLInputObject)]
pub struct TaskSaveInput {
    db_id: Option<i32>,
    title: String,
    description: String,
    designation: TaskDesignation,
    parent_id: Nullable<i32>,
    earliest_start: Nullable<DateTime<Utc>>,
    schedule_target: Nullable<DateTime<Utc>>,
    effort: Nullable<f64>,
    pub predecessors: Option<Vec<i32>>,
    pub successors: Option<Vec<i32>>,
    pub children: Option<Vec<i32>>,
}

impl From<TaskSaveInput> for crate::entity::task::ActiveModel {
    fn from(value: TaskSaveInput) -> Self {
        crate::entity::task::ActiveModel {
            id: opt_to_av!(value.db_id),
            title: ActiveValue::Set(value.title),
            description: ActiveValue::Set(value.description),
            designation: ActiveValue::Set(value.designation.into()),
            parent_id: nullable_to_av!(value.parent_id),
            earliest_start: nullable_to_av!(value.earliest_start),
            schedule_target: nullable_to_av!(value.schedule_target),
            effort: nullable_to_av!(value.effort.map(|v| v as f32)),
        }
    }
}
