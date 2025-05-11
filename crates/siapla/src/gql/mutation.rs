use juniper::graphql_object;
use sea_orm::ActiveModelTrait;

use crate::entity::{resource, task};

use super::{
    context::Context,
    resource::{ResourceSaveInput, resource_save},
    task::{TaskSaveInput, task_save},
};

#[derive(Default)]
pub struct Mutation {}

#[graphql_object]
#[graphql(context = Context)]
impl Mutation {
    pub fn new() -> Self {
        Default::default()
    }

    async fn task_save(ctx: &Context, task: TaskSaveInput) -> anyhow::Result<task::Model> {
        task_save(ctx, task).await
    }

    async fn task_delete(ctx: &Context, task_id: i32) -> anyhow::Result<bool> {
        let txn = ctx.txn().await?;
        let am = task::ActiveModel {
            id: sea_orm::ActiveValue::Set(task_id),
            ..Default::default()
        };
        let res = am.delete(txn).await?;
        Ok(res.rows_affected > 0)
    }

    async fn resource_save(
        ctx: &Context,
        resource: ResourceSaveInput,
    ) -> anyhow::Result<resource::Model> {
        resource_save(ctx, resource).await
    }

    async fn resource_delete(ctx: &Context, resource_id: i32) -> anyhow::Result<bool> {
        let txn = ctx.txn().await?;
        let am = resource::ActiveModel {
            id: sea_orm::ActiveValue::Set(resource_id),
            ..Default::default()
        };
        let res = am.delete(txn).await?;
        Ok(res.rows_affected > 0)
    }
}
