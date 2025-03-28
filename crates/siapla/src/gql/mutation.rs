use juniper::{FieldResult, graphql_object};
use sea_orm::ActiveModelTrait;

use crate::entity::task;

use super::{
    context::Context,
    task::{TaskCreateInput, TaskUpdateInput},
};

#[derive(Default)]
pub struct Mutation {}

#[graphql_object]
#[graphql(context = Context)]
impl Mutation {
    pub fn new() -> Self {
        Default::default()
    }

    async fn task_create(ctx: &Context, task: TaskCreateInput) -> FieldResult<task::Model> {
        Ok(task::ActiveModel::from(task)
            .insert(ctx.db().await?)
            .await?)
    }

    async fn task_update(ctx: &Context, task: TaskUpdateInput) -> FieldResult<task::Model> {
        Ok(task::ActiveModel::from(task)
            .update(ctx.db().await?)
            .await?)
    }
}
