use std::{collections::HashSet, str::FromStr};

use anyhow::anyhow;
use chrono::{DateTime, Utc};

use juniper::{GraphQLEnum, Nullable, graphql_object};
use sea_orm::{ActiveValue, Condition, QueryOrder as _, prelude::*};
use strum::{EnumString, IntoStaticStr};
use tracing::trace;

use crate::{
    entity::{
        allocation, dependency, resource_constraint, resource_constraint_entry,
        resource_iteration as resource, task_header, task_iteration as task,
    },
    gql::{
        common::{nullable_to_av, resolve_many_to_many},
        context::Context,
    },
    revisioning::{PlanState, active_for_revision, create_revision},
};

use super::resource::GQLResource;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// GQLTask – revision-aware GraphQL wrapper for task_iteration::Model
// ---------------------------------------------------------------------------

pub struct GQLTask {
    pub model: task::Model,
    pub revision: Option<i64>,
    pub header_model: Option<task_header::Model>,
}

impl GQLTask {
    pub fn at_revision(model: task::Model, revision: Option<i64>) -> Self {
        Self { model, revision, header_model: None }
    }

    pub fn with_header(model: task::Model, header_model: task_header::Model) -> Self {
        Self { model, revision: None, header_model: Some(header_model) }
    }
}

impl From<task::Model> for GQLTask {
    fn from(model: task::Model) -> Self {
        Self { model, revision: None, header_model: None }
    }
}

#[graphql_object]
#[graphql(name = "Task")]
impl GQLTask {
    /// Stable identity — always the `task_header.id`.
    fn db_id(&self) -> i32 {
        self.model.header_id.unwrap_or(self.model.id)
    }
    /// The mutable iteration id (changes on every edit).
    fn iteration_id(&self) -> &i32 {
        &self.model.id
    }
    fn title(&self) -> &str {
        &self.model.title
    }
    fn description(&self) -> &str {
        &self.model.description
    }
    fn earliest_start(&self) -> &Option<DateTime<Utc>> {
        &self.model.earliest_start
    }
    fn schedule_target(&self) -> &Option<DateTime<Utc>> {
        &self.model.schedule_target
    }
    fn effort(&self) -> Option<f64> {
        self.model.effort.map(Into::into)
    }
    fn priority(&self) -> f64 {
        self.model.priority as f64
    }
    fn designation(&self) -> anyhow::Result<TaskDesignation> {
        Ok(TaskDesignation::from_str(&self.model.designation)?)
    }
    fn rev_created(&self) -> i32 {
        self.model.rev_created as i32
    }
    fn rev_deleted(&self) -> Option<i32> {
        self.model.rev_deleted.map(|v| v as i32)
    }
    async fn header_rev_created(&self, ctx: &Context) -> anyhow::Result<Option<i32>> {
        let hm = self.load_header(ctx).await?;
        Ok(hm.and_then(|h| h.rev_created).map(|v| v as i32))
    }
    async fn header_rev_deleted(&self, ctx: &Context) -> anyhow::Result<Option<i32>> {
        let hm = self.load_header(ctx).await?;
        Ok(hm.and_then(|h| h.rev_deleted).map(|v| v as i32))
    }

    // -- Predecessors (revision-aware via revision-aware dataloader) ---------
    pub async fn predecessors(&self, ctx: &Context) -> anyhow::Result<Vec<GQLTask>> {
        let rev = self.revision;
        let Some(header_id) = self.model.header_id else {
            return Ok(Vec::new());
        };
        let models: Vec<task::Model> = resolve_many_to_many!(
            ctx,
            rev,
            dependency::Entity,
            dependency::Column::SuccessorId,
            header_id,
            |d: dependency::Model| d.predecessor_id,
            task::Entity,
            task::Column::HeaderId
        )?;
        Ok(models.into_iter().map(|m| GQLTask::at_revision(m, rev)).collect())
    }

    // -- Successors (revision-aware via revision-aware dataloader) -----------
    pub async fn successors(&self, ctx: &Context) -> anyhow::Result<Vec<GQLTask>> {
        let rev = self.revision;
        let Some(header_id) = self.model.header_id else {
            return Ok(Vec::new());
        };
        let models: Vec<task::Model> = resolve_many_to_many!(
            ctx,
            rev,
            dependency::Entity,
            dependency::Column::PredecessorId,
            header_id,
            |d: dependency::Model| d.successor_id,
            task::Entity,
            task::Column::HeaderId
        )?;
        Ok(models.into_iter().map(|m| GQLTask::at_revision(m, rev)).collect())
    }

    pub async fn children(&self, ctx: &Context) -> anyhow::Result<Vec<GQLTask>> {
        let Some(header_id) = self.model.header_id else {
            return Ok(Vec::new());
        };
        let txn = ctx.txn().await?;
        let children = task::Entity::find()
            .filter(task::Column::ParentId.eq(header_id))
            .filter(active_for_revision(
                task::Column::RevCreated,
                task::Column::RevDeleted,
                self.revision,
            )?)
            .order_by_asc(task::Column::Title)
            .all(txn)
            .await?;
        Ok(children.into_iter().map(|m| GQLTask::at_revision(m, self.revision)).collect())
    }

    pub async fn issues(&self, ctx: &Context) -> anyhow::Result<Vec<crate::entity::issue::Model>> {
        let header_id = self.model.header_id.unwrap_or(self.model.id);
        const CIDX: usize = crate::entity::issue::Column::TaskId as usize;
        ctx.load_by_col::<crate::entity::issue::Entity, CIDX>(header_id).await
    }

    async fn parent(&self, ctx: &Context) -> anyhow::Result<Option<GQLTask>> {
        let parent_header_id = match self.model.parent_id {
            Some(parent_header_id) => parent_header_id,
            None => return Ok(None),
        };
        let txn = ctx.txn().await?;
        let active = task::Entity::find()
            .filter(task::Column::HeaderId.eq(parent_header_id))
            .filter(active_for_revision(
                task::Column::RevCreated,
                task::Column::RevDeleted,
                self.revision,
            )?)
            .one(txn)
            .await?;
        Ok(active.map(|m| GQLTask::at_revision(m, self.revision)))
    }

    // -- Resource constraints (revision-aware) ------------------------------
    async fn resource_constraints(
        &self,
        ctx: &Context,
    ) -> anyhow::Result<Vec<GQLResourceConstraint>> {
        let txn = ctx.txn().await?;
        let constraints = resource_constraint::Entity::find()
            .filter(resource_constraint::Column::TaskId.eq(self.model.id))
            .filter(active_for_revision(
                resource_constraint::Column::RevCreated,
                resource_constraint::Column::RevDeleted,
                self.revision,
            )?)
            .order_by_asc(resource_constraint::Column::Id)
            .all(txn)
            .await?;
        Ok(constraints
            .into_iter()
            .map(|m| GQLResourceConstraint { model: m, revision: self.revision })
            .collect())
    }

    async fn allocations(&self, ctx: &Context) -> anyhow::Result<Vec<allocation::Model>> {
        let header_id = self.model.header_id.unwrap_or(self.model.id);
        const CIDX: usize = allocation::Column::TaskId as usize;
        ctx.load_by_col::<allocation::Entity, CIDX>(header_id).await.map(|mut res| {
            res.sort_by_key(|a| a.end);
            res
        })
    }
}

impl GQLTask {
    async fn load_header(&self, ctx: &Context) -> anyhow::Result<Option<task_header::Model>> {
        if let Some(ref hm) = self.header_model {
            return Ok(Some(hm.clone()));
        }
        let Some(hid) = self.model.header_id else {
            return Ok(None);
        };
        let txn = ctx.txn().await?;
        Ok(task_header::Entity::find_by_id(hid).one(txn).await?)
    }
}

// ---------------------------------------------------------------------------
// GQLResourceConstraint – revision-aware wrapper
// ---------------------------------------------------------------------------

pub struct GQLResourceConstraint {
    pub model: resource_constraint::Model,
    pub revision: Option<i64>,
}

#[graphql_object]
#[graphql(name = "ResourceConstraint")]
impl GQLResourceConstraint {
    fn id(&self) -> i32 {
        self.model.id
    }
    fn optional(&self) -> bool {
        self.model.optional
    }
    fn speed(&self) -> f64 {
        self.model.speed as f64
    }
    async fn entries(&self, ctx: &Context) -> anyhow::Result<Vec<GQLResourceConstraintEntry>> {
        let entries = resource_constraint_entry::Entity::find()
            .filter(resource_constraint_entry::Column::ResourceConstraintId.eq(self.model.id))
            .all(ctx.txn().await?)
            .await?;
        Ok(entries
            .into_iter()
            .map(|m| GQLResourceConstraintEntry { model: m, revision: self.revision })
            .collect())
    }
}

// ---------------------------------------------------------------------------
// GQLResourceConstraintEntry – revision-aware wrapper
// ---------------------------------------------------------------------------

pub struct GQLResourceConstraintEntry {
    pub model: resource_constraint_entry::Model,
    pub revision: Option<i64>,
}

#[graphql_object]
#[graphql(name = "ResourceConstraintEntry")]
impl GQLResourceConstraintEntry {
    fn id(&self) -> i32 {
        self.model.id
    }
    async fn resource(&self, ctx: &Context) -> anyhow::Result<GQLResource> {
        let txn = ctx.txn().await?;
        // Resolve via header so we return the iteration active at the target revision
        let iter = resource::Entity::find_by_id(self.model.resource_id).one(txn).await?;
        let iter = match iter {
            Some(it) => it,
            None => return Err(anyhow!("Resource not found")),
        };
        match iter.header_id {
            Some(hid) => {
                let active = resource::Entity::find()
                    .filter(resource::Column::HeaderId.eq(hid))
                    .filter(active_for_revision(
                        resource::Column::RevCreated,
                        resource::Column::RevDeleted,
                        self.revision,
                    )?)
                    .one(txn)
                    .await?;
                active
                    .map(|m| GQLResource::at_revision(m, self.revision))
                    .ok_or_else(|| anyhow!("Resource not found at revision"))
            }
            None => Ok(GQLResource::at_revision(iter, self.revision)),
        }
    }
}

// ---------------------------------------------------------------------------
// Input types (unchanged)
// ---------------------------------------------------------------------------

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
    /// When set, this is the **header id** (stable identity) of the task to update.
    db_id: Option<i32>,
    title: String,
    description: String,
    designation: TaskDesignation,
    parent_id: Nullable<i32>,
    earliest_start: Nullable<DateTime<Utc>>,
    schedule_target: Nullable<DateTime<Utc>>,
    effort: Nullable<f64>,
    priority: f64,
    pub predecessors: Option<Vec<i32>>,
    pub successors: Option<Vec<i32>>,
    pub children: Option<Vec<i32>>,
    pub resource_constraints: Option<Vec<ResourceConstraintInput>>,
}

impl TaskSaveInput {
    fn into_active_model(self) -> crate::entity::task_iteration::ActiveModel {
        crate::entity::task_iteration::ActiveModel {
            id: ActiveValue::NotSet,
            title: ActiveValue::Set(self.title),
            description: ActiveValue::Set(self.description),
            designation: ActiveValue::Set(self.designation.into()),
            parent_id: nullable_to_av!(self.parent_id),
            earliest_start: nullable_to_av!(self.earliest_start),
            schedule_target: nullable_to_av!(self.schedule_target),
            effort: nullable_to_av!(self.effort.map(|v| v as f32)),
            priority: ActiveValue::Set(self.priority as f32),
            header_id: ActiveValue::NotSet,
            rev_created: ActiveValue::NotSet,
            rev_deleted: ActiveValue::NotSet,
        }
    }
}

// ---------------------------------------------------------------------------
// Mutation helpers – update predecessors / successors / children / constraints
// ---------------------------------------------------------------------------

async fn update_predecessors(
    ctx: &Context,
    model: &task::Model,
    mut predecessors: Vec<i32>,
    revision_id: i64,
) -> anyhow::Result<()> {
    let txn = ctx.txn().await?;
    let Some(successor_header_id) = model.header_id else {
        return Ok(());
    };
    let existing_deps = dependency::Entity::find()
        .filter(dependency::Column::SuccessorId.eq(successor_header_id))
        .filter(dependency::Column::RevDeleted.is_null())
        .all(txn)
        .await?;
    let existing: HashSet<i32> = existing_deps.into_iter().map(|d| d.predecessor_id).collect();

    let predecessor_ids: Vec<i32> = predecessors.drain(..).collect();
    let current_targets = task::Entity::find()
        .filter(task::Column::Id.is_in(predecessor_ids.clone()))
        .filter(task::Column::RevDeleted.is_null())
        .all(txn)
        .await?;
    let mut target: HashSet<i32> =
        current_targets.into_iter().filter_map(|task| task.header_id).collect();

    if target.len() < predecessor_ids.len() {
        let historical_targets = task::Entity::find()
            .filter(task::Column::HeaderId.is_in(predecessor_ids))
            .all(txn)
            .await?;
        target.extend(historical_targets.into_iter().filter_map(|task| task.header_id));
    }

    let remove: HashSet<i32> = existing.difference(&target).cloned().collect();
    let add: HashSet<i32> = target.difference(&existing).cloned().collect();
    trace!(
        "predecessors: existing={:?}, target={:?}, remove={:?}, add={:?}",
        existing, target, remove, add
    );
    if !remove.is_empty() {
        dependency::Entity::update_many()
            .col_expr(dependency::Column::RevDeleted, Expr::value(Value::BigInt(Some(revision_id))))
            .filter(
                dependency::Column::SuccessorId
                    .eq(successor_header_id)
                    .and(dependency::Column::PredecessorId.is_in(remove)),
            )
            .filter(dependency::Column::RevDeleted.is_null())
            .exec(txn)
            .await?;
    }
    if !add.is_empty() {
        dependency::Entity::insert_many(add.into_iter().map(|i| dependency::ActiveModel {
            predecessor_id: sea_orm::ActiveValue::Set(i),
            successor_id: sea_orm::ActiveValue::Set(successor_header_id),
            rev_created: sea_orm::ActiveValue::Set(revision_id),
            rev_deleted: sea_orm::ActiveValue::Set(None),
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
    revision_id: i64,
) -> anyhow::Result<()> {
    let txn = ctx.txn().await?;
    let Some(predecessor_header_id) = model.header_id else {
        return Ok(());
    };
    let existing_deps = dependency::Entity::find()
        .filter(dependency::Column::PredecessorId.eq(predecessor_header_id))
        .filter(dependency::Column::RevDeleted.is_null())
        .all(txn)
        .await?;
    let existing: HashSet<i32> = existing_deps.into_iter().map(|d| d.successor_id).collect();

    let successor_ids: Vec<i32> = successors.drain(..).collect();
    let current_targets = task::Entity::find()
        .filter(task::Column::Id.is_in(successor_ids.clone()))
        .filter(task::Column::RevDeleted.is_null())
        .all(txn)
        .await?;
    let mut target: HashSet<i32> =
        current_targets.into_iter().filter_map(|task| task.header_id).collect();

    if target.len() < successor_ids.len() {
        let historical_targets = task::Entity::find()
            .filter(task::Column::HeaderId.is_in(successor_ids))
            .all(txn)
            .await?;
        target.extend(historical_targets.into_iter().filter_map(|task| task.header_id));
    }

    let remove: HashSet<i32> = existing.difference(&target).cloned().collect();
    let add: HashSet<i32> = target.difference(&existing).cloned().collect();
    trace!(
        "successors: existing={:?}, target={:?}, remove={:?}, add={:?}",
        existing, target, remove, add
    );
    if !remove.is_empty() {
        dependency::Entity::update_many()
            .col_expr(dependency::Column::RevDeleted, Expr::value(Value::BigInt(Some(revision_id))))
            .filter(
                dependency::Column::PredecessorId
                    .eq(predecessor_header_id)
                    .and(dependency::Column::SuccessorId.is_in(remove)),
            )
            .filter(dependency::Column::RevDeleted.is_null())
            .exec(txn)
            .await?;
    }
    if !add.is_empty() {
        dependency::Entity::insert_many(add.into_iter().map(|i| dependency::ActiveModel {
            successor_id: sea_orm::ActiveValue::Set(i),
            predecessor_id: sea_orm::ActiveValue::Set(predecessor_header_id),
            rev_created: sea_orm::ActiveValue::Set(revision_id),
            rev_deleted: sea_orm::ActiveValue::Set(None),
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
    revision_id: i64,
) -> anyhow::Result<()> {
    let Some(header_id) = model.header_id else {
        return Ok(());
    };
    let txn = ctx.txn().await?;
    let existing_children = task::Entity::find()
        .filter(task::Column::ParentId.eq(header_id))
        .filter(task::Column::RevDeleted.is_null())
        .all(txn)
        .await?;
    let existing: HashSet<i32> =
        existing_children.into_iter().filter_map(|child| child.header_id).collect();

    let child_ids: Vec<i32> = children.drain(..).collect();
    let target_children = task::Entity::find()
        .filter(task::Column::HeaderId.is_in(child_ids.clone()))
        .filter(active_for_revision(
            task::Column::RevCreated,
            task::Column::RevDeleted,
            Some(revision_id),
        )?)
        .all(txn)
        .await?;
    let mut target: HashSet<i32> =
        target_children.into_iter().filter_map(|child| child.header_id).collect();

    if target.len() < child_ids.len() {
        let unresolved_ids: Vec<i32> =
            child_ids.into_iter().filter(|child_id| !target.contains(child_id)).collect();
        if !unresolved_ids.is_empty() {
            let fallback_children = task::Entity::find()
                .filter(task::Column::Id.is_in(unresolved_ids))
                .all(txn)
                .await?;
            target.extend(fallback_children.into_iter().filter_map(|child| child.header_id));
        }
    }

    let remove: HashSet<i32> = existing.difference(&target).cloned().collect();
    let add: HashSet<i32> = target.difference(&existing).cloned().collect();
    trace!(
        "children: existing={:?}, target={:?}, remove={:?}, add={:?}",
        existing, target, remove, add
    );
    if !remove.is_empty() {
        task::Entity::update_many()
            .col_expr(task::Column::ParentId, Expr::value(Value::Int(None)))
            .filter(task::Column::HeaderId.is_in(remove))
            .filter(task::Column::RevDeleted.is_null())
            .exec(txn)
            .await?;
    }
    if !add.is_empty() {
        task::Entity::update_many()
            .col_expr(task::Column::ParentId, Expr::value(Value::Int(Some(header_id))))
            .filter(task::Column::HeaderId.is_in(add))
            .filter(task::Column::RevDeleted.is_null())
            .exec(txn)
            .await?;
    }
    Ok(())
}

async fn update_resource_constraints(
    ctx: &Context,
    model: &task::Model,
    constraints: &[ResourceConstraintInput],
    revision_id: i64,
) -> anyhow::Result<()> {
    let txn = ctx.txn().await?;
    // Fetch active old constraints (assume order is preserved)
    let old = resource_constraint::Entity::find()
        .filter(resource_constraint::Column::TaskId.eq(model.id))
        .filter(resource_constraint::Column::RevDeleted.is_null())
        .order_by(resource_constraint::Column::Id, sea_orm::Order::Asc)
        .all(txn)
        .await?;
    let old_len = old.len();
    let new_len = constraints.len();
    let min_len = old_len.min(new_len);

    // check if new resource constraints do not use one resource multiple times
    let all_used_resources: HashSet<i32> =
        constraints.iter().flat_map(|c| c.entries.iter().map(|e| e.resource_id)).collect();
    let num_entries: usize = constraints.iter().map(|c| c.entries.len()).sum();
    if num_entries != all_used_resources.len() {
        return Err(anyhow::anyhow!("Each resource can only be used once!"));
    }

    // Compare pairwise and version changed constraints.
    for (i, c) in constraints.iter().take(min_len).enumerate() {
        let old_c = &old[i];
        let old_entries: HashSet<i32> = resource_constraint_entry::Entity::find()
            .filter(resource_constraint_entry::Column::ResourceConstraintId.eq(old_c.id))
            .all(txn)
            .await?
            .into_iter()
            .map(|entry| entry.resource_id)
            .collect();
        let new_entries: HashSet<i32> = c.entries.iter().map(|entry| entry.resource_id).collect();

        let needs_update = old_c.optional != c.optional
            || old_c.speed != (c.speed as f32)
            || old_entries != new_entries;

        if needs_update {
            resource_constraint::Entity::update_many()
                .col_expr(
                    resource_constraint::Column::RevDeleted,
                    Expr::value(Value::BigInt(Some(revision_id))),
                )
                .filter(resource_constraint::Column::Id.eq(old_c.id))
                .filter(resource_constraint::Column::RevDeleted.is_null())
                .exec(txn)
                .await?;

            let new_constraint = resource_constraint::ActiveModel {
                id: ActiveValue::NotSet,
                task_id: ActiveValue::Set(model.id),
                r#type: ActiveValue::Set(old_c.r#type.clone()),
                optional: ActiveValue::Set(c.optional),
                speed: ActiveValue::Set(c.speed as f32),
                rev_created: ActiveValue::Set(revision_id),
                rev_deleted: ActiveValue::Set(None),
            }
            .insert(txn)
            .await?;

            if !c.entries.is_empty() {
                resource_constraint_entry::Entity::insert_many(c.entries.iter().map(|entry| {
                    resource_constraint_entry::ActiveModel {
                        id: ActiveValue::NotSet,
                        resource_constraint_id: ActiveValue::Set(new_constraint.id),
                        resource_id: ActiveValue::Set(entry.resource_id),
                    }
                }))
                .exec(txn)
                .await?;
            }
        }
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
                rev_created: ActiveValue::Set(revision_id),
                rev_deleted: ActiveValue::Set(None),
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
        let ids_to_remove: Vec<i32> =
            old.iter().skip(new_len).map(|constraint| constraint.id).collect();
        if !ids_to_remove.is_empty() {
            resource_constraint::Entity::update_many()
                .col_expr(
                    resource_constraint::Column::RevDeleted,
                    Expr::value(Value::BigInt(Some(revision_id))),
                )
                .filter(resource_constraint::Column::Id.is_in(ids_to_remove))
                .filter(resource_constraint::Column::RevDeleted.is_null())
                .exec(txn)
                .await?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// task_save – public mutation entry-point (returns raw model; caller wraps)
// ---------------------------------------------------------------------------

pub async fn task_save(ctx: &Context, mut task: TaskSaveInput) -> anyhow::Result<task::Model> {
    let predecessors = task.predecessors.take();
    let successors = task.successors.take();
    let children = task.children.take();
    let resource_constraints = task.resource_constraints.take();
    let input_header_id = task.db_id.take();
    let txn = ctx.txn().await?;
    let revision_id = create_revision(txn, PlanState::NotCalculated).await?;
    let mut am = task.into_active_model();
    let model = if let Some(header_id) = input_header_id {
        // Resolve header_id → current active iteration
        let existing = task::Entity::find()
            .filter(task::Column::HeaderId.eq(header_id))
            .filter(task::Column::RevDeleted.is_null())
            .one(txn)
            .await?
            .ok_or_else(|| {
                anyhow!("No active task iteration found for header {}", header_id)
            })?;
        let old_id = existing.id;
        // Soft-delete the old iteration
        task::Entity::update_many()
            .col_expr(task::Column::RevDeleted, Expr::value(Value::BigInt(Some(revision_id))))
            .filter(task::Column::Id.eq(old_id))
            .filter(task::Column::RevDeleted.is_null())
            .exec(txn)
            .await?;
        // Create new iteration
        am.header_id = ActiveValue::Set(existing.header_id);
        am.rev_created = ActiveValue::Set(revision_id);
        am.rev_deleted = ActiveValue::Set(None);
        let new_model = am.insert(txn).await?;

        // ── Migrate relationships from old iteration to new iteration ──

        // 1. Dependencies
        let old_header_id = existing.header_id.unwrap_or(old_id);
        let new_header_id = new_model.header_id.unwrap_or(new_model.id);
        let old_deps = dependency::Entity::find()
            .filter(
                Condition::any()
                    .add(dependency::Column::PredecessorId.eq(old_header_id))
                    .add(dependency::Column::SuccessorId.eq(old_header_id)),
            )
            .filter(dependency::Column::RevDeleted.is_null())
            .all(txn)
            .await?;
        for dep in &old_deps {
            dependency::Entity::update_many()
                .col_expr(
                    dependency::Column::RevDeleted,
                    Expr::value(Value::BigInt(Some(revision_id))),
                )
                .filter(dependency::Column::Id.eq(dep.id))
                .filter(dependency::Column::RevDeleted.is_null())
                .exec(txn)
                .await?;

            let predecessor_header_id = if dep.predecessor_id == old_header_id {
                new_header_id
            } else {
                dep.predecessor_id
            };
            let successor_header_id =
                if dep.successor_id == old_header_id { new_header_id } else { dep.successor_id };

            let replacement_exists = dependency::Entity::find()
                .filter(dependency::Column::PredecessorId.eq(predecessor_header_id))
                .filter(dependency::Column::SuccessorId.eq(successor_header_id))
                .filter(dependency::Column::RevDeleted.is_null())
                .one(txn)
                .await?
                .is_some();

            if !replacement_exists {
                dependency::ActiveModel {
                    predecessor_id: sea_orm::ActiveValue::Set(predecessor_header_id),
                    successor_id: sea_orm::ActiveValue::Set(successor_header_id),
                    rev_created: sea_orm::ActiveValue::Set(revision_id),
                    rev_deleted: sea_orm::ActiveValue::Set(None),
                    ..Default::default()
                }
                .insert(txn)
                .await?;
            }
        }

        // 2. Resource constraints
        let old_constraints = resource_constraint::Entity::find()
            .filter(resource_constraint::Column::TaskId.eq(old_id))
            .filter(resource_constraint::Column::RevDeleted.is_null())
            .all(txn)
            .await?;
        for old_c in &old_constraints {
            resource_constraint::Entity::update_many()
                .col_expr(
                    resource_constraint::Column::RevDeleted,
                    Expr::value(Value::BigInt(Some(revision_id))),
                )
                .filter(resource_constraint::Column::Id.eq(old_c.id))
                .filter(resource_constraint::Column::RevDeleted.is_null())
                .exec(txn)
                .await?;
            let old_entries = resource_constraint_entry::Entity::find()
                .filter(resource_constraint_entry::Column::ResourceConstraintId.eq(old_c.id))
                .all(txn)
                .await?;
            let new_c = resource_constraint::ActiveModel {
                id: ActiveValue::NotSet,
                task_id: ActiveValue::Set(new_model.id),
                r#type: ActiveValue::Set(old_c.r#type.clone()),
                optional: ActiveValue::Set(old_c.optional),
                speed: ActiveValue::Set(old_c.speed),
                rev_created: ActiveValue::Set(revision_id),
                rev_deleted: ActiveValue::Set(None),
            }
            .insert(txn)
            .await?;
            for entry in &old_entries {
                resource_constraint_entry::ActiveModel {
                    id: ActiveValue::NotSet,
                    resource_constraint_id: ActiveValue::Set(new_c.id),
                    resource_id: ActiveValue::Set(entry.resource_id),
                }
                .insert(txn)
                .await?;
            }
        }

        new_model
    } else {
        let header = crate::entity::task_header::ActiveModel {
            id: ActiveValue::NotSet,
            rev_created: ActiveValue::Set(Some(revision_id)),
            rev_deleted: ActiveValue::Set(None),
        }
        .insert(txn)
        .await?;
        am.header_id = ActiveValue::Set(Some(header.id));
        am.rev_created = ActiveValue::Set(revision_id);
        am.rev_deleted = ActiveValue::Set(None);
        am.insert(txn).await?
    };

    if let Some(predecessors) = predecessors {
        update_predecessors(ctx, &model, predecessors, revision_id).await?;
    }
    if let Some(successors) = successors {
        update_successors(ctx, &model, successors, revision_id).await?;
    }
    if let Some(children) = children {
        update_children(ctx, &model, children, revision_id).await?;
    }
    if let Some(ref constraints) = resource_constraints {
        let all_used_resources: std::collections::HashSet<i32> =
            constraints.iter().flat_map(|c| c.entries.iter().map(|e| e.resource_id)).collect();
        let num_entries: usize = constraints.iter().map(|c| c.entries.len()).sum();
        if num_entries != all_used_resources.len() {
            return Err(anyhow::anyhow!("Each resource can only be used once!"));
        }
        update_resource_constraints(ctx, &model, constraints, revision_id).await?;
    }

    // Dependency cycle detection
    let deps = dependency::Entity::find().all(txn).await?;
    use std::collections::HashMap as _HM;
    let mut adj: _HM<i32, Vec<i32>> = _HM::new();
    for d in deps.iter() {
        adj.entry(d.predecessor_id).or_default().push(d.successor_id);
    }
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

    // Hierarchy loop detection
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

    Ok(model)
}
