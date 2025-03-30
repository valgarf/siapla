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
        if am.id.is_set() {
            Ok(am.update(db).await?)
        } else {
            Ok(am.insert(db).await?)
        }
    }
}
