use std::{collections::HashSet, str::FromStr};

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use itertools::Itertools as _;
use juniper::{GraphQLEnum, Nullable, graphql_object};
use sea_orm::{ActiveValue, QueryOrder as _, prelude::*};
use strum::{EnumString, IntoStaticStr};
use tracing::trace;

use crate::{
    entity::{
        allocation, dependency, resource, resource_constraint, resource_constraint_entry, task,
    },
    gql::{
        common::{nullable_to_av, opt_to_av, resolve_many_to_many},
        context::Context,
    },
};

#[derive(GraphQLEnum, IntoStaticStr, EnumString, PartialEq, Eq)]
pub enum TaskDesignation {
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
        resolve_many_to_many!(
            ctx,
            dependency::Entity,
            dependency::Column::SuccessorId,
            self.id,
            |l: dependency::Model| l.predecessor_id,
            task::Entity,
            task::Column::Id
        )
    }
    pub async fn successors(&self, ctx: &Context) -> anyhow::Result<Vec<Self>> {
        resolve_many_to_many!(
            ctx,
            dependency::Entity,
            dependency::Column::PredecessorId,
            self.id,
            |l: dependency::Model| l.successor_id,
            task::Entity,
            task::Column::Id
        )
    }
    pub async fn children(&self, ctx: &Context) -> anyhow::Result<Vec<Self>> {
        const CIDX: usize = task::Column::ParentId as usize;
        ctx.load_by_col::<task::Entity, CIDX>(self.id).await
    }

    pub async fn issues(&self, ctx: &Context) -> anyhow::Result<Vec<crate::entity::issue::Model>> {
        const CIDX: usize = crate::entity::issue::Column::TaskId as usize;
        ctx.load_by_col::<crate::entity::issue::Entity, CIDX>(self.id).await
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
    async fn resource_constraints(
        &self,
        ctx: &Context,
    ) -> anyhow::Result<Vec<resource_constraint::Model>> {
        const CIDX: usize = resource_constraint::Column::TaskId as usize;
        ctx.load_by_col::<resource_constraint::Entity, CIDX>(self.id).await
    }

    async fn allocations(&self, ctx: &Context) -> anyhow::Result<Vec<allocation::Model>> {
        const CIDX: usize = allocation::Column::TaskId as usize;
        ctx.load_by_col::<allocation::Entity, CIDX>(self.id).await
    }
}

#[graphql_object]
#[graphql(name = "ResourceConstraint")]
impl resource_constraint::Model {
    fn id(&self) -> i32 {
        self.id
    }
    fn optional(&self) -> bool {
        self.optional
    }
    fn speed(&self) -> f64 {
        self.speed as f64
    }
    async fn entries(
        &self,
        ctx: &Context,
    ) -> anyhow::Result<Vec<resource_constraint_entry::Model>> {
        let entries = resource_constraint_entry::Entity::find()
            .filter(resource_constraint_entry::Column::ResourceConstraintId.eq(self.id))
            .all(ctx.txn().await?)
            .await?;
        Ok(entries)
    }
}

#[graphql_object]
#[graphql(name = "ResourceConstraintEntry")]
impl resource_constraint_entry::Model {
    fn id(&self) -> i32 {
        self.id
    }
    async fn resource(&self, ctx: &Context) -> anyhow::Result<resource::Model> {
        const CIDX: usize = resource::Column::Id as usize;
        let result = ctx.load_one_by_col::<resource::Entity, CIDX>(self.resource_id).await?;
        result.ok_or(anyhow!("Resource not found"))
    }
}

#[derive(juniper::GraphQLInputObject, Clone)]
pub struct ResourceConstraintEntryInput {
    pub resource_id: i32,
}

#[derive(juniper::GraphQLInputObject, Clone)]
pub struct ResourceConstraintInput {
    pub optional: bool,
    pub speed: f64,
    pub entries: Vec<ResourceConstraintEntryInput>,
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
    pub resource_constraints: Option<Vec<ResourceConstraintInput>>,
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
    let existing: HashSet<i32> = model.predecessors(ctx).await?.iter().map(|el| el.id).collect();
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
    let existing: HashSet<i32> = model.successors(ctx).await?.iter().map(|el| el.id).collect();
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
            .col_expr(task::Column::ParentId, Expr::value(Value::Int(Some(model.id))))
            .filter(task::Column::Id.is_in(add))
            .exec(txn)
            .await?;
    }
    Ok(())
}

async fn update_resource_constraint_entries(
    ctx: &Context,
    model: &resource_constraint::Model,
    new_entries: &[ResourceConstraintEntryInput],
) -> anyhow::Result<()> {
    let txn = ctx.txn().await?;
    let existing: HashSet<i32> =
        model.entries(ctx).await?.iter().map(|el| el.resource_id).collect();
    let target: HashSet<i32> = new_entries.iter().map(|el| el.resource_id).collect();
    let remove: HashSet<i32> = existing.difference(&target).cloned().collect();
    let add: HashSet<i32> = target.difference(&existing).cloned().collect();
    // Delete entries not in new_ids
    if !remove.is_empty() {
        resource_constraint_entry::Entity::delete_many()
            .filter(resource_constraint_entry::Column::ResourceId.is_in(remove))
            .filter(resource_constraint_entry::Column::ResourceConstraintId.eq(model.id))
            .exec(txn)
            .await?;
    }
    if !add.is_empty() {
        resource_constraint_entry::Entity::insert_many(
            add.into_iter()
                .map(|rid| resource_constraint_entry::ActiveModel {
                    id: ActiveValue::NotSet,
                    resource_constraint_id: ActiveValue::Set(model.id),
                    resource_id: ActiveValue::Set(rid),
                })
                .collect::<Vec<_>>(),
        )
        .exec(txn)
        .await?;
    }
    Ok(())
}

async fn update_resource_constraints(
    ctx: &Context,
    model: &task::Model,
    constraints: &[ResourceConstraintInput],
) -> anyhow::Result<()> {
    let txn = ctx.txn().await?;
    // Fetch old constraints (assume order is preserved)
    let old = resource_constraint::Entity::find()
        .filter(resource_constraint::Column::TaskId.eq(model.id))
        .order_by(resource_constraint::Column::Id, sea_orm::Order::Asc)
        .all(txn)
        .await?;
    let old_len = old.len();
    let new_len = constraints.len();
    let min_len = old_len.min(new_len);

    // check if new reosurce constraints do not use one resource multiple times
    // TODO: this restriction could be lifted by making the scheduling algorithm more complex
    let all_used_resources: HashSet<i32> =
        constraints.iter().flat_map(|c| c.entries.iter().map(|e| e.resource_id)).collect();
    let num_entries: usize = constraints.iter().map(|c| c.entries.len()).sum();
    if num_entries != all_used_resources.len() {
        return Err(anyhow::anyhow!("Each resource can only be used once!"));
    }

    // the constraintsare sane, Update existing constraints
    for (i, c) in constraints.iter().take(min_len).enumerate() {
        let old_c = &old[i];
        // update columns, only update if changed
        let needs_update = old_c.optional != c.optional
            || old_c.speed != (c.speed as f32)
            || old_c.r#type != old_c.r#type; // type is not changed by input, but keep for completeness
        if needs_update {
            let am = resource_constraint::ActiveModel {
                id: ActiveValue::Set(old_c.id),
                task_id: ActiveValue::Set(model.id),
                r#type: ActiveValue::Set(old_c.r#type.clone()),
                optional: ActiveValue::Set(c.optional),
                speed: ActiveValue::Set(c.speed as f32),
            };
            am.update(txn).await?;
        }
        // update relationships
        update_resource_constraint_entries(ctx, old_c, &c.entries).await?;
    }
    // Add new constraints if new_len > old_len
    if new_len > old_len {
        for c in constraints.iter().skip(old_len) {
            let rc = resource_constraint::ActiveModel {
                id: ActiveValue::NotSet,
                task_id: ActiveValue::Set(model.id),
                r#type: ActiveValue::Set("any".to_string()),
                optional: ActiveValue::Set(c.optional),
                speed: ActiveValue::Set(c.speed as f32),
            };
            let rc = rc.insert(txn).await?;
            let entries: Vec<resource_constraint_entry::ActiveModel> = c
                .entries
                .iter()
                .map(|entry| resource_constraint_entry::ActiveModel {
                    id: ActiveValue::NotSet,
                    resource_constraint_id: ActiveValue::Set(rc.id),
                    resource_id: ActiveValue::Set(entry.resource_id),
                })
                .collect();
            if !entries.is_empty() {
                resource_constraint_entry::Entity::insert_many(entries).exec(txn).await?;
            }
        }
    }
    // Remove old constraints if old_len > new_len
    if old_len > new_len {
        let ids_to_remove: Vec<i32> = old.iter().skip(new_len).map(|c| c.id).collect();
        if !ids_to_remove.is_empty() {
            resource_constraint_entry::Entity::delete_many()
                .filter(
                    resource_constraint_entry::Column::ResourceConstraintId
                        .is_in(ids_to_remove.iter().cloned()),
                )
                .exec(txn)
                .await?;
            resource_constraint::Entity::delete_many()
                .filter(resource_constraint::Column::Id.is_in(ids_to_remove))
                .exec(txn)
                .await?;
        }
    }
    Ok(())
}

pub async fn task_save(ctx: &Context, mut task: TaskSaveInput) -> anyhow::Result<task::Model> {
    let predecessors = task.predecessors.take();
    let successors = task.successors.take();
    let children = task.children.take();
    let resource_constraints = task.resource_constraints.take();
    // keep a copy for issue detection after mutations (not used for now)
    let am = task::ActiveModel::from(task);
    let txn = ctx.txn().await?;
    let model = if am.id.is_set() { am.update(txn).await? } else { am.insert(txn).await? };

    if let Some(predecessors) = predecessors {
        update_predecessors(ctx, &model, predecessors).await?;
    }
    if let Some(successors) = successors {
        update_successors(ctx, &model, successors).await?;
    }
    if let Some(children) = children {
        update_children(ctx, &model, children).await?;
    }
    // Check if the provided resource constraints reuse the same resource across
    // multiple constraints. If so, abort early with an error.
    if let Some(ref constraints) = resource_constraints {
        let all_used_resources: std::collections::HashSet<i32> =
            constraints.iter().flat_map(|c| c.entries.iter().map(|e| e.resource_id)).collect();
        let num_entries: usize = constraints.iter().map(|c| c.entries.len()).sum();
        if num_entries != all_used_resources.len() {
            return Err(anyhow::anyhow!("Each resource can only be used once!"));
        }
        update_resource_constraints(&ctx, &model, &constraints).await?;
    }
    // Check for dependency cycles (abort save on loop)
    // Fetch dependencies and run a simple DFS cycle detection
    let deps = dependency::Entity::find().all(txn).await?;
    use std::collections::HashMap as _HM;
    let mut adj: _HM<i32, Vec<i32>> = _HM::new();
    for d in deps.iter() {
        adj.entry(d.predecessor_id).or_default().push(d.successor_id);
    }
    // DFS
    fn has_cycle(adj: &std::collections::HashMap<i32, Vec<i32>>) -> bool {
        fn visit(
            n: i32,
            adj: &std::collections::HashMap<i32, Vec<i32>>,
            visiting: &mut HashSet<i32>,
            visited: &mut HashSet<i32>,
        ) -> bool {
            if visited.contains(&n) {
                return false;
            }
            if visiting.contains(&n) {
                return true;
            }
            visiting.insert(n);
            if let Some(neis) = adj.get(&n) {
                for &m in neis {
                    if visit(m, adj, visiting, visited) {
                        return true;
                    }
                }
            }
            visiting.remove(&n);
            visited.insert(n);
            false
        }
        let mut visiting = std::collections::HashSet::new();
        let mut visited = std::collections::HashSet::new();
        for &n in adj.keys() {
            if visit(n, adj, &mut visiting, &mut visited) {
                return true;
            }
        }
        false
    }
    if has_cycle(&adj) {
        return Err(anyhow!("Dependency loop detected"));
    }

    // Check parent (hierarchy) loops for this task
    {
        use std::collections::HashSet;
        let mut seen: HashSet<i32> = HashSet::new();
        let mut cur = Some(model.id);
        while let Some(cid) = cur {
            if seen.contains(&cid) {
                return Err(anyhow!("Hierarchy loop detected"));
            }
            seen.insert(cid);
            let t = task::Entity::find_by_id(cid).one(txn).await?;
            cur = match t {
                Some(tt) => tt.parent_id,
                None => None,
            };
        }
    }

    // resource-constraint checks are handled as planning issues in detect_project_issues

    Ok(model)
}
