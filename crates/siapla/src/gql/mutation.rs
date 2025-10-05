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
        let res = match task_save(ctx, task).await {
            Ok(res) => res,
            Err(err) => {
                ctx.failed().await;
                Err(err)?
            }
        };
        // notify modification channel
        ctx.app_state().notify_modified("graphql".to_string());
        Ok(res)
    }

    async fn task_delete(ctx: &Context, task_id: i32) -> anyhow::Result<bool> {
        let txn = ctx.txn().await?;
        let am = task::ActiveModel { id: sea_orm::ActiveValue::Set(task_id), ..Default::default() };
        let res = am.delete(txn).await?;
        let ok = res.rows_affected > 0;
        if ok {
            ctx.app_state().notify_modified("graphql".to_string());
        }
        Ok(ok)
    }

    async fn resource_save(
        ctx: &Context,
        resource: ResourceSaveInput,
    ) -> anyhow::Result<resource::Model> {
        let res = match resource_save(ctx, resource).await {
            Ok(res) => res,
            Err(err) => {
                ctx.failed().await;
                Err(err)?
            }
        };
        ctx.app_state().notify_modified("graphql".to_string());
        Ok(res)
    }

    async fn resource_delete(ctx: &Context, resource_id: i32) -> anyhow::Result<bool> {
        let txn = ctx.txn().await?;
        let am = resource::ActiveModel {
            id: sea_orm::ActiveValue::Set(resource_id),
            ..Default::default()
        };
        let res = am.delete(txn).await?;
        let ok = res.rows_affected > 0;
        if ok {
            ctx.app_state().notify_modified("graphql".to_string());
        }
        Ok(ok)
    }

    /// Trigger a manual recalculation now
    async fn recalculate_now(ctx: &Context) -> anyhow::Result<bool> {
        ctx.app_state().trigger_manual();
        Ok(true)
    }
}
