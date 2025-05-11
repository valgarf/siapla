use std::{collections::HashSet, str::FromStr};

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use itertools::{Either, Itertools as _};
use juniper::{FieldResult, GraphQLEnum, Nullable, graphql_object};
use sea_orm::{ActiveModelTrait as _, ActiveValue, EntityTrait as _, prelude::*};
use strum::{EnumString, IntoStaticStr};
use tracing::trace;

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
    fn designation(&self) -> anyhow::Result<TaskDesignation> {
        Ok(TaskDesignation::from_str(&self.designation)?)
    }
    pub async fn predecessors(&self, ctx: &Context) -> anyhow::Result<Vec<Self>> {
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
    pub async fn successors(&self, ctx: &Context) -> anyhow::Result<Vec<Self>> {
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
    pub async fn children(&self, ctx: &Context) -> anyhow::Result<Vec<Self>> {
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

async fn update_predecessors(
    ctx: &Context,
    model: &task::Model,
    mut predecessors: Vec<i32>,
) -> anyhow::Result<()> {
    let txn = ctx.txn().await?;
    let existing: HashSet<i32> = model
        .predecessors(ctx)
        .await?
        .iter()
        .map(|el| el.id)
        .collect();
    let target: HashSet<i32> = HashSet::from_iter(predecessors.drain(..));
    let remove: HashSet<i32> = existing.difference(&target).cloned().collect();
    let add: HashSet<i32> = target.difference(&existing).cloned().collect();
    trace!(
        "predecessors: existing={:?}, target={:?}, remove={:?}, add={:?}",
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
    Ok(())
}

async fn update_successors(
    ctx: &Context,
    model: &task::Model,
    mut successors: Vec<i32>,
) -> anyhow::Result<()> {
    let txn = ctx.txn().await?;
    let existing: HashSet<i32> = model
        .successors(ctx)
        .await?
        .iter()
        .map(|el| el.id)
        .collect();
    let target: HashSet<i32> = HashSet::from_iter(successors.drain(..));
    let remove: HashSet<i32> = existing.difference(&target).cloned().collect();
    let add: HashSet<i32> = target.difference(&existing).cloned().collect();
    trace!(
        "successors: existing={:?}, target={:?}, remove={:?}, add={:?}",
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
    Ok(())
}

async fn update_children(
    ctx: &Context,
    model: &task::Model,
    mut children: Vec<i32>,
) -> anyhow::Result<()> {
    let txn = ctx.txn().await?;
    let existing: HashSet<i32> = model.children(ctx).await?.iter().map(|el| el.id).collect();
    let target: HashSet<i32> = HashSet::from_iter(children.drain(..));
    let remove: HashSet<i32> = existing.difference(&target).cloned().collect();
    let add: HashSet<i32> = target.difference(&existing).cloned().collect();
    trace!(
        "children: existing={:?}, target={:?}, remove={:?}, add={:?}",
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
    Ok(())
}

pub async fn task_save(ctx: &Context, mut task: TaskSaveInput) -> anyhow::Result<task::Model> {
    let predecessors = task.predecessors.take();
    let successors = task.successors.take();
    let children = task.children.take();
    let am = task::ActiveModel::from(task);
    let txn = ctx.txn().await?;
    let model = if am.id.is_set() {
        am.update(txn).await?
    } else {
        am.insert(txn).await?
    };

    if let Some(predecessors) = predecessors {
        update_predecessors(ctx, &model, predecessors).await?;
    }

    if let Some(successors) = successors {
        update_successors(ctx, &model, successors).await?;
    }

    if let Some(mut children) = children {
        update_children(ctx, &model, children).await?;
    }

    // TODO: before committing, check if we now have predecessor or parent cycles!
    Ok(model)
}
