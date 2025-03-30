use std::{collections::HashSet, time::Duration};

use juniper::{FieldResult, graphql_object};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};

use crate::entity::{dependency, task};

use super::{context::Context, task::TaskSaveInput};

#[derive(Default)]
pub struct Mutation {}

#[graphql_object]
#[graphql(context = Context)]
impl Mutation {
    pub fn new() -> Self {
        Default::default()
    }

    async fn task_save(ctx: &Context, mut task: TaskSaveInput) -> FieldResult<task::Model> {
        let predecessors = task.predecessors.take();
        let successors = task.successors.take();
        let am = task::ActiveModel::from(task);
        let txn = ctx.db().await?.begin().await?;
        tokio::time::sleep(Duration::from_secs(3)).await;
        let model = if am.id.is_set() {
            am.update(&txn).await?
        } else {
            am.insert(&txn).await?
        };

        if let Some(mut predecessors) = predecessors {
            let existing: HashSet<i32> = model
                .predecessors(ctx)
                .await?
                .iter()
                .map(|el| el.id)
                .collect();
            let target: HashSet<i32> = HashSet::from_iter(predecessors.drain(..));
            let remove = existing.difference(&target);
            let add = target.difference(&existing);
            dependency::Entity::delete_many()
                .filter(
                    dependency::Column::SuccessorId
                        .eq(model.id)
                        .and(dependency::Column::PredecessorId.is_in(remove.cloned())),
                )
                .exec(&txn)
                .await?;
            dependency::Entity::insert_many(add.map(|i| dependency::ActiveModel {
                predecessor_id: sea_orm::ActiveValue::Set(*i),
                successor_id: sea_orm::ActiveValue::Set(model.id),
                ..Default::default()
            }))
            .exec(&txn)
            .await?;
        }

        if let Some(mut successors) = successors {
            let existing: HashSet<i32> = model
                .successors(ctx)
                .await?
                .iter()
                .map(|el| el.id)
                .collect();
            let target: HashSet<i32> = HashSet::from_iter(successors.drain(..));
            let remove = existing.difference(&target);
            let add = target.difference(&existing);
            dependency::Entity::delete_many()
                .filter(
                    dependency::Column::PredecessorId
                        .eq(model.id)
                        .and(dependency::Column::SuccessorId.is_in(remove.cloned())),
                )
                .exec(&txn)
                .await?;
            dependency::Entity::insert_many(add.map(|i| dependency::ActiveModel {
                successor_id: sea_orm::ActiveValue::Set(*i),
                predecessor_id: sea_orm::ActiveValue::Set(model.id),
                ..Default::default()
            }))
            .exec(&txn)
            .await?;
        }
        txn.commit().await?;
        Ok(model)
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
