use std::collections::HashSet;

use juniper::{FieldResult, graphql_object};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use tracing::trace;

use sea_orm::prelude::*;

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
        let children = task.children.take();
        let am = task::ActiveModel::from(task);
        let txn = ctx.txn().await?;

        // Note: must be done outside of a transaction, otherwise it will block for sqlite
        let mut existing_predecessors: HashSet<i32> = Default::default();
        let mut existing_successors: HashSet<i32> = Default::default();
        let mut existing_children: HashSet<i32> = Default::default();
        const CIDX: usize = task::Column::Id as usize;
        if (predecessors.is_some() || successors.is_some() || children.is_some()) && am.id.is_set()
        {
            let model = ctx
                .load_one_by_col::<task::Entity, CIDX>(am.id.clone().into_value().unwrap())
                .await?;
            if let Some(model) = model {
                if predecessors.is_some() {
                    existing_predecessors = model
                        .predecessors(ctx)
                        .await?
                        .iter()
                        .map(|el| el.id)
                        .collect();
                }
                if predecessors.is_some() {
                    existing_successors = model
                        .successors(ctx)
                        .await?
                        .iter()
                        .map(|el| el.id)
                        .collect();
                }
                if children.is_some() {
                    existing_children = model.children(ctx).await?.iter().map(|el| el.id).collect();
                }
            }
        }

        let model = if am.id.is_set() {
            am.update(txn).await?
        } else {
            am.insert(txn).await?
        };

        if let Some(mut predecessors) = predecessors {
            let existing = existing_predecessors;
            let target: HashSet<i32> = HashSet::from_iter(predecessors.drain(..));
            let remove: HashSet<i32> = existing.difference(&target).cloned().collect();
            let add: HashSet<i32> = target.difference(&existing).cloned().collect();
            trace!(
                "PREDECESSORS: existing={:?}, target={:?}, remove={:?}, add={:?}",
                existing, target, remove, add
            );
            if !remove.is_empty() {
                dependency::Entity::delete_many()
                    .filter(
                        dependency::Column::SuccessorId
                            .eq(model.id)
                            .and(dependency::Column::PredecessorId.is_in(remove)),
                    )
                    .exec(txn)
                    .await?;
            }
            if !add.is_empty() {
                dependency::Entity::insert_many(add.into_iter().map(|i| dependency::ActiveModel {
                    predecessor_id: sea_orm::ActiveValue::Set(i),
                    successor_id: sea_orm::ActiveValue::Set(model.id),
                    ..Default::default()
                }))
                .exec(txn)
                .await?;
            }
        }

        if let Some(mut successors) = successors {
            let existing = existing_successors;
            let target: HashSet<i32> = HashSet::from_iter(successors.drain(..));
            let remove: HashSet<i32> = existing.difference(&target).cloned().collect();
            let add: HashSet<i32> = target.difference(&existing).cloned().collect();
            trace!(
                "SUCCESSORS: existing={:?}, target={:?}, remove={:?}, add={:?}",
                existing, target, remove, add
            );
            if !remove.is_empty() {
                dependency::Entity::delete_many()
                    .filter(
                        dependency::Column::PredecessorId
                            .eq(model.id)
                            .and(dependency::Column::SuccessorId.is_in(remove)),
                    )
                    .exec(txn)
                    .await?;
            }
            if !add.is_empty() {
                dependency::Entity::insert_many(add.into_iter().map(|i| dependency::ActiveModel {
                    successor_id: sea_orm::ActiveValue::Set(i),
                    predecessor_id: sea_orm::ActiveValue::Set(model.id),
                    ..Default::default()
                }))
                .exec(txn)
                .await?;
            }
        }

        if let Some(mut children) = children {
            let existing = existing_children;
            let target: HashSet<i32> = HashSet::from_iter(children.drain(..));
            let remove: HashSet<i32> = existing.difference(&target).cloned().collect();
            let add: HashSet<i32> = target.difference(&existing).cloned().collect();
            trace!(
                "CHILDREN: existing={:?}, target={:?}, remove={:?}, add={:?}",
                existing, target, remove, add
            );
            if !remove.is_empty() {
                task::Entity::update_many()
                    .col_expr(task::Column::ParentId, Expr::value(Value::Int(None)))
                    .filter(task::Column::Id.is_in(remove))
                    .exec(txn)
                    .await?;
            }
            if !add.is_empty() {
                task::Entity::update_many()
                    .col_expr(
                        task::Column::ParentId,
                        Expr::value(Value::Int(Some(model.id))),
                    )
                    .filter(task::Column::Id.is_in(add))
                    .exec(txn)
                    .await?;
            }
        }

        // TODO: before committing, check if we now have predecessor or parent cycles!
        Ok(model)
    }

    async fn task_delete(ctx: &Context, task_id: i32) -> FieldResult<bool> {
        let txn = ctx.txn().await?;
        let am = task::ActiveModel {
            id: sea_orm::ActiveValue::Set(task_id),
            ..Default::default()
        };
        let res = am.delete(txn).await?;
        Ok(res.rows_affected > 0)
    }
}
