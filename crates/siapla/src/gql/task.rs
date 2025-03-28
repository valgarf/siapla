use chrono::NaiveDateTime;
use juniper::{Nullable, graphql_object};
use sea_orm::ActiveValue;

use super::{
    common::{nullable_to_av, opt_to_av},
    context::Context,
};
use crate::entity::task;

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
    fn earliest_start(&self) -> &Option<NaiveDateTime> {
        &self.earliest_start
    }
    fn schedule_target(&self) -> &Option<NaiveDateTime> {
        &self.schedule_target
    }
    fn effort(&self) -> Option<f64> {
        self.effort.map(Into::into)
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
pub struct TaskCreateInput {
    title: String,
    description: String,
    parent_id: Option<i32>,
    earlies_start: Option<NaiveDateTime>,
    schedule_target: Option<NaiveDateTime>,
    effort: Option<f64>,
}

#[derive(juniper::GraphQLInputObject)]
pub struct TaskUpdateInput {
    db_id: i32,
    title: Option<String>,
    description: Option<String>,
    parent_id: Nullable<i32>,
    earlies_start: Nullable<NaiveDateTime>,
    schedule_target: Nullable<NaiveDateTime>,
    effort: Nullable<f64>,
}

impl From<TaskCreateInput> for crate::entity::task::ActiveModel {
    fn from(value: TaskCreateInput) -> Self {
        crate::entity::task::ActiveModel {
            id: Default::default(),
            title: ActiveValue::Set(value.title),
            description: ActiveValue::Set(value.description),
            parent_id: opt_to_av!(value.parent_id).into(),
            earliest_start: opt_to_av!(value.earlies_start).into(),
            schedule_target: opt_to_av!(value.schedule_target).into(),
            effort: opt_to_av!(value.effort.map(|v| v as f32)).into(),
        }
    }
}

impl From<TaskUpdateInput> for crate::entity::task::ActiveModel {
    fn from(value: TaskUpdateInput) -> Self {
        crate::entity::task::ActiveModel {
            id: ActiveValue::Set(value.db_id),
            title: opt_to_av!(value.title),
            description: opt_to_av!(value.description),
            parent_id: nullable_to_av!(value.parent_id),
            earliest_start: nullable_to_av!(value.earlies_start),
            schedule_target: nullable_to_av!(value.schedule_target),
            effort: nullable_to_av!(value.effort.map(|v| v as f32)),
        }
    }
}
