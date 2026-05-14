use crate::gql::wrapper::*;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use std::{
    collections::{BTreeSet, HashSet},
    str::FromStr,
};

use juniper::{GraphQLEnum, Nullable, graphql_object};
use sea_orm::{ActiveValue, prelude::*};
use strum::{EnumString, IntoStaticStr};

use crate::{
    db::{
        r#impl::resource_constraint::ResourceConstraintUpserter,
        r#impl::{
            dependency::TaskDependencyUpserter,
            task_iteration::{TaskIterationParentUpdater, TaskIterationUpserter},
        },
        revisioning::LazyRevision,
        update::update_rev_many,
        upsert::{upsert_rev_many, upsert_rev_one},
    },
    entity::{dependency, resource_constraint, resource_constraint_entry, task_iteration},
    gql::{
        common::nullable_to_av,
        context::Context,
        scalars::{ExtendedScalarValue, Int64},
    },
};

use super::{allocation::GQLAllocation, issue::GQLIssue, resource::GQLResource};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(GraphQLEnum, IntoStaticStr, EnumString, PartialEq, Eq, Clone, Debug)]
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

pub type GQLTask = ModelWrapper<task_iteration::Entity>;

#[graphql_object]
#[graphql(name = "Task", context = Context, scalar= ExtendedScalarValue)]
impl GQLTask {
    /// Stable identity — always the `task_header.id`.
    fn db_id(&self) -> i32 {
        self.model.header_id
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
    fn rev_created(&self) -> Int64 {
        self.model.rev_created.into()
    }
    fn rev_deleted(&self) -> Option<Int64> {
        self.model.rev_deleted.map(|v| v.into())
    }
    async fn header_rev_created(&self, ctx: &Context) -> anyhow::Result<Int64> {
        let hm = self.model.dataloader_header(ctx.db()).await?;
        Ok(hm.rev_created.into())
    }
    async fn header_rev_deleted(&self, ctx: &Context) -> anyhow::Result<Option<Int64>> {
        let hm = self.model.dataloader_header(ctx.db()).await?;
        Ok(hm.rev_deleted.map(|v| v.into()))
    }

    // -- Predecessors (revision-aware via link dataloader) -------------------
    pub async fn predecessors(&self, ctx: &Context) -> anyhow::Result<Vec<GQLTask>> {
        self.model
            .dataloader_predecessors(ctx.db(), self.revision)
            .await
            .into_wrapper(self.revision)
    }

    // -- Successors (revision-aware via link dataloader) ---------------------
    pub async fn successors(&self, ctx: &Context) -> anyhow::Result<Vec<GQLTask>> {
        self.model.dataloader_successors(ctx.db(), self.revision).await.into_wrapper(self.revision)
    }

    pub async fn children(&self, ctx: &Context) -> anyhow::Result<Vec<GQLTask>> {
        self.model.dataloader_children(ctx.db(), self.revision).await.into_wrapper(self.revision)
    }

    pub async fn issues(&self, ctx: &Context) -> anyhow::Result<Vec<GQLIssue>> {
        self.model.dataloader_issues(ctx.db(), self.revision).await.into_wrapper(self.revision)
    }

    async fn parent(&self, ctx: &Context) -> anyhow::Result<Option<GQLTask>> {
        self.model.dataloader_parent(ctx.db(), self.revision).await.into_wrapper(self.revision)
    }

    async fn resource_constraints(
        &self,
        ctx: &Context,
    ) -> anyhow::Result<Vec<GQLResourceConstraint>> {
        let mut constraints: Vec<GQLResourceConstraint> = self
            .model
            .dataloader_resource_constraints(ctx.db(), self.revision)
            .await
            .into_wrapper(self.revision)?;
        constraints.sort_by_key(|constraint| constraint.model.position);
        Ok(constraints)
    }

    async fn allocations(&self, ctx: &Context) -> anyhow::Result<Vec<GQLAllocation>> {
        let mut res: Vec<GQLAllocation> = self
            .model
            .dataloader_allocations(ctx.db(), self.revision)
            .await
            .into_wrapper(self.revision)?;
        res.sort_by_key(|a| a.model.end);
        Ok(res)
    }
}
// ---------------------------------------------------------------------------
// GQLResourceConstraint – revision-aware wrapper
// ---------------------------------------------------------------------------

pub type GQLResourceConstraint = ModelWrapper<resource_constraint::Entity>;

#[graphql_object]
#[graphql(name = "ResourceConstraint", context = Context, scalar = ExtendedScalarValue)]
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
        self.model.dataloader_entries(ctx.db()).await.into_wrapper(self.revision)
    }
}

// ---------------------------------------------------------------------------
// GQLResourceConstraintEntry – revision-aware wrapper
// ---------------------------------------------------------------------------

pub type GQLResourceConstraintEntry = ModelWrapper<resource_constraint_entry::Entity>;

#[graphql_object]
#[graphql(name = "ResourceConstraintEntry", context = Context, scalar = ExtendedScalarValue)]
impl GQLResourceConstraintEntry {
    fn id(&self) -> i32 {
        self.model.id
    }
    async fn resource(&self, ctx: &Context) -> anyhow::Result<GQLResource> {
        let model = self.model.dataloader_resource_iteration(ctx.db(), self.revision).await?;
        Ok(GQLResource::at_revision(model, self.revision))
    }
}

// ---------------------------------------------------------------------------
// Input types (unchanged)
// ---------------------------------------------------------------------------

#[derive(juniper::GraphQLInputObject, Debug, Clone)]
pub struct ResourceConstraintEntryInput {
    pub resource_id: i32,
}

#[derive(juniper::GraphQLInputObject, Debug, Clone)]
pub struct ResourceConstraintInput {
    pub optional: bool,
    pub speed: f64,
    pub entries: Vec<ResourceConstraintEntryInput>,
}

#[derive(juniper::GraphQLInputObject, Debug)]
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

fn sorted_ids(ids: &[i32]) -> Vec<i32> {
    ids.iter().copied().collect::<BTreeSet<_>>().into_iter().collect()
}

// ---------------------------------------------------------------------------
// Mutation helpers – update predecessors / successors / children / constraints
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum TaskDependencyInputScope {
    Predecessors,
    Successors,
}

fn build_task_dependency_models(
    task_header_id: i32,
    mut related_task_header_ids: Vec<i32>,
    scope: TaskDependencyInputScope,
) -> Vec<(dependency::ActiveModel, ())> {
    related_task_header_ids.sort();
    related_task_header_ids
        .into_iter()
        .map(|related_task_header_id| {
            let (predecessor_id, successor_id) = match scope {
                TaskDependencyInputScope::Predecessors => (related_task_header_id, task_header_id),
                TaskDependencyInputScope::Successors => (task_header_id, related_task_header_id),
            };
            (
                dependency::ActiveModel {
                    id: ActiveValue::NotSet,
                    predecessor_id: ActiveValue::Set(predecessor_id),
                    successor_id: ActiveValue::Set(successor_id),
                    rev_created: ActiveValue::NotSet,
                    rev_deleted: ActiveValue::NotSet,
                },
                (),
            )
        })
        .collect()
}

async fn update_predecessors(
    ctx: &Context,
    model: &task_iteration::Model,
    predecessors: Vec<i32>,
    revision: &LazyRevision,
) -> anyhow::Result<()> {
    let models = build_task_dependency_models(
        model.header_id,
        predecessors,
        TaskDependencyInputScope::Predecessors,
    );
    upsert_rev_many(
        ctx.db(),
        &revision,
        TaskDependencyUpserter::new_for_predecessors(model.header_id),
        models,
    )
    .await?;
    Ok(())
}

async fn update_successors(
    ctx: &Context,
    model: &task_iteration::Model,
    successors: Vec<i32>,
    revision: &LazyRevision,
) -> anyhow::Result<()> {
    let models = build_task_dependency_models(
        model.header_id,
        successors,
        TaskDependencyInputScope::Successors,
    );
    upsert_rev_many(
        ctx.db(),
        &revision,
        TaskDependencyUpserter::new_for_successors(model.header_id),
        models,
    )
    .await?;
    Ok(())
}

async fn update_children(
    ctx: &Context,
    model: &task_iteration::Model,
    children: Vec<i32>,
    revision: &LazyRevision,
) -> anyhow::Result<()> {
    update_rev_many(
        ctx.db(),
        &revision,
        TaskIterationParentUpdater::new(model.header_id, sorted_ids(&children)),
    )
    .await?;
    Ok(())
}

fn build_resource_constraint_models(
    task_header_id: i32,
    constraints: &[ResourceConstraintInput],
) -> Vec<(resource_constraint::ActiveModel, Vec<i32>)> {
    constraints
        .iter()
        .enumerate()
        .map(|(position, constraint)| {
            let mut resource_ids =
                constraint.entries.iter().map(|entry| entry.resource_id).collect::<Vec<_>>();
            resource_ids.sort_unstable();

            (
                resource_constraint::ActiveModel {
                    id: ActiveValue::NotSet,
                    task_id: ActiveValue::Set(task_header_id),
                    r#type: ActiveValue::Set("any".to_string()),
                    optional: ActiveValue::Set(constraint.optional),
                    speed: ActiveValue::Set(constraint.speed as f32),
                    position: ActiveValue::Set(position as i32),
                    rev_created: ActiveValue::NotSet,
                    rev_deleted: ActiveValue::NotSet,
                },
                resource_ids,
            )
        })
        .collect()
}

async fn update_resource_constraints(
    ctx: &Context,
    model: &task_iteration::Model,
    constraints: &[ResourceConstraintInput],
    revision: &LazyRevision,
) -> anyhow::Result<()> {
    let models_with_rel = build_resource_constraint_models(model.header_id, constraints);
    upsert_rev_many(
        ctx.db(),
        &revision,
        ResourceConstraintUpserter::new(model.header_id),
        models_with_rel,
    )
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// task_save – public mutation entry-point (returns raw model; caller wraps)
// ---------------------------------------------------------------------------

pub async fn task_save(
    ctx: &Context,
    mut task: TaskSaveInput,
) -> anyhow::Result<(task_iteration::Model, i64)> {
    let db = ctx.db();
    let txn = ctx.txn().await?;
    let predecessors = task.predecessors.take();
    let successors = task.successors.take();
    let children = task.children.take();
    let resource_constraints = task.resource_constraints.take();
    let input_header_id = task.db_id.take();
    let revision = LazyRevision::new();

    let mut am = task.into_active_model();
    let header_id = if let Some(header_id) = input_header_id {
        header_id
    } else {
        let revision_id = revision.get(db).await?;
        crate::entity::task_header::ActiveModel {
            id: ActiveValue::NotSet,
            rev_created: ActiveValue::Set(revision_id),
            rev_deleted: ActiveValue::Set(None),
        }
        .insert(txn)
        .await?
        .id
    };
    am.header_id = ActiveValue::Set(header_id);
    let (_, model) =
        upsert_rev_one(ctx.db(), &revision, TaskIterationUpserter::new(header_id), am, ()).await?;

    if let Some(predecessors) = predecessors {
        update_predecessors(ctx, &model, predecessors, &revision).await?;
    }
    if let Some(successors) = successors {
        update_successors(ctx, &model, successors, &revision).await?;
    }
    if let Some(children) = children {
        update_children(ctx, &model, children, &revision).await?;
    }
    if let Some(ref constraints) = resource_constraints {
        let all_used_resources: std::collections::HashSet<i32> =
            constraints.iter().flat_map(|c| c.entries.iter().map(|e| e.resource_id)).collect();
        let num_entries: usize = constraints.iter().map(|c| c.entries.len()).sum();
        if num_entries != all_used_resources.len() {
            return Err(anyhow::anyhow!("Each resource can only be used once!"));
        }
        update_resource_constraints(ctx, &model, constraints, &revision).await?;
    }

    let (changed, revision_id) = revision.resolve(ctx.db()).await?;
    if !changed {
        return Ok((model, revision_id));
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
            let t = task_iteration::Entity::find_by_id(cid).one(txn).await?;
            cur = match t {
                Some(tt) => tt.parent_id,
                None => None,
            };
        }
    }

    ctx.app_state().notify_modified("graphql".to_string());
    Ok((model, revision_id))
}
