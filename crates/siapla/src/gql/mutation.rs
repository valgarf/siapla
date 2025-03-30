use std::time::Duration;

use juniper::{FieldResult, graphql_object};
use sea_orm::ActiveModelTrait;

use crate::entity::task;

use super::{context::Context, task::TaskSaveInput};

#[derive(Default)]
pub struct Mutation {}

#[graphql_object]
#[graphql(context = Context)]
impl Mutation {
    pub fn new() -> Self {
        Default::default()
    }

    async fn task_save(ctx: &Context, task: TaskSaveInput) -> FieldResult<task::Model> {
        let am = task::ActiveModel::from(task);
        let db = ctx.db().await?;
        tokio::time::sleep(Duration::from_secs(3)).await;
        if am.id.is_set() {
            Ok(am.update(db).await?)
        } else {
            Ok(am.insert(db).await?)
        }
    }

    async fn task_delete(ctx: &Context, task_id: i32) -> FieldResult<bool> {
        let db = ctx.db().await?;
        tokio::time::sleep(Duration::from_secs(3)).await;
        let am = task::ActiveModel {
            id: sea_orm::ActiveValue::Set(task_id),
            ..Default::default()
        };
        let res = am.delete(db).await?;
        Ok(res.rows_affected > 0)
    }
}
