use std::collections::{BTreeSet, HashMap, HashSet};

use chrono::{DateTime, Utc};
use juniper::{GraphQLEnum, graphql_interface, graphql_object};
use sea_orm::*;

use crate::{
    entity::{
        booking, dependency, resource_constraint, resource_constraint_entry, resource_iteration,
        revision, task_iteration as task,
    },
    gql::{
        context::Context,
        scalars::{Int64, MyScalarValue},
    },
};

use super::{booking::GQLBooking, task::GQLTask};

#[derive(GraphQLEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeType {
    Created,
    Updated,
    Deleted,
}

#[derive(GraphQLEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchDirection {
    Backward,
    Forward,
}

#[graphql_interface]
#[graphql(
    for = [TaskIterationChange, BookingChange, DependencyChange, ResourceConstraintChange],
    context = Context,
    scalar = MyScalarValue,
)]
#[allow(dead_code)]
pub trait IChange {
    fn revision_id(&self) -> Int64;
    fn timestamp(&self) -> DateTime<Utc>;
    fn change_type(&self) -> ChangeType;
}

pub struct TaskIterationChange {
    rev_id: i64,
    ts: DateTime<Utc>,
    ct: ChangeType,
    task_model: task::Model,
    revision: Option<i64>,
}

#[graphql_object]
#[graphql(impl = IChangeValue, context = Context, scalar = MyScalarValue)]
impl TaskIterationChange {
    fn revision_id(&self) -> Int64 {
        Int64(self.rev_id)
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.ts
    }

    fn change_type(&self) -> ChangeType {
        self.ct
    }

    fn task_iteration(&self) -> GQLTask {
        GQLTask::at_revision(self.task_model.clone(), self.revision)
    }
}

pub struct BookingChange {
    rev_id: i64,
    ts: DateTime<Utc>,
    ct: ChangeType,
    booking_model: Option<booking::Model>,
    revision: Option<i64>,
}

#[graphql_object]
#[graphql(impl = IChangeValue, context = Context, scalar = MyScalarValue)]
impl BookingChange {
    fn revision_id(&self) -> Int64 {
        Int64(self.rev_id)
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.ts
    }

    fn change_type(&self) -> ChangeType {
        self.ct
    }

    fn booking(&self) -> Option<GQLBooking> {
        self.booking_model.as_ref().map(|m| GQLBooking::at_revision(m.clone(), self.revision))
    }
}

pub struct DependencyChange {
    rev_id: i64,
    ts: DateTime<Utc>,
    ct: ChangeType,
    predecessor_id_val: i32,
    successor_id_val: i32,
    predecessor_title_val: String,
    successor_title_val: String,
}

#[graphql_object]
#[graphql(impl = IChangeValue, context = Context, scalar = MyScalarValue)]
impl DependencyChange {
    fn revision_id(&self) -> Int64 {
        Int64(self.rev_id)
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.ts
    }

    fn change_type(&self) -> ChangeType {
        self.ct
    }

    fn predecessor_id(&self) -> i32 {
        self.predecessor_id_val
    }

    fn successor_id(&self) -> i32 {
        self.successor_id_val
    }

    fn predecessor_title(&self) -> &str {
        &self.predecessor_title_val
    }

    fn successor_title(&self) -> &str {
        &self.successor_title_val
    }
}

pub struct ResourceConstraintChange {
    rev_id: i64,
    ts: DateTime<Utc>,
    ct: ChangeType,
    constraint_id_val: i32,
    optional_val: bool,
    speed_val: f64,
    resource_ids_val: Vec<i32>,
    resource_names_val: Vec<String>,
}

#[graphql_object]
#[graphql(impl = IChangeValue, context = Context, scalar = MyScalarValue)]
impl ResourceConstraintChange {
    fn revision_id(&self) -> Int64 {
        Int64(self.rev_id)
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.ts
    }

    fn change_type(&self) -> ChangeType {
        self.ct
    }

    fn constraint_id(&self) -> i32 {
        self.constraint_id_val
    }

    fn optional(&self) -> bool {
        self.optional_val
    }

    fn speed(&self) -> f64 {
        self.speed_val
    }

    fn resource_ids(&self) -> &[i32] {
        &self.resource_ids_val
    }

    fn resource_names(&self) -> &[String] {
        &self.resource_names_val
    }
}

pub struct TaskHistoryResult {
    pub changes: Vec<IChangeValue>,
    pub has_more: bool,
}

#[graphql_object]
#[graphql(context = Context, scalar = MyScalarValue)]
impl TaskHistoryResult {
    fn changes(&self) -> &[IChangeValue] {
        &self.changes
    }

    fn has_more(&self) -> bool {
        self.has_more
    }
}

fn fallback_task_title(header_id: i32) -> String {
    format!("Task #{header_id}")
}

fn fallback_resource_name(header_id: i32) -> String {
    format!("Resource #{header_id}")
}

async fn resolve_task_title_at_revision(
    txn: &DatabaseTransaction,
    header_id: i32,
    rev_id: i64,
) -> anyhow::Result<String> {
    if let Some(model) = task::Entity::find()
        .filter(task::Column::HeaderId.eq(header_id))
        .filter(task::Column::RevCreated.lte(rev_id))
        .filter(
            Condition::any()
                .add(task::Column::RevDeleted.is_null())
                .add(task::Column::RevDeleted.gt(rev_id)),
        )
        .order_by_desc(task::Column::RevCreated)
        .one(txn)
        .await?
    {
        return Ok(model.title);
    }

    if let Some(model) = task::Entity::find()
        .filter(task::Column::HeaderId.eq(header_id))
        .order_by_desc(task::Column::RevCreated)
        .one(txn)
        .await?
    {
        return Ok(model.title);
    }

    Ok(fallback_task_title(header_id))
}

async fn resolve_resource_name_at_revision(
    txn: &DatabaseTransaction,
    header_id: i32,
    rev_id: i64,
) -> anyhow::Result<String> {
    if let Some(model) = resource_iteration::Entity::find()
        .filter(resource_iteration::Column::HeaderId.eq(header_id))
        .filter(resource_iteration::Column::RevCreated.lte(rev_id))
        .filter(
            Condition::any()
                .add(resource_iteration::Column::RevDeleted.is_null())
                .add(resource_iteration::Column::RevDeleted.gt(rev_id)),
        )
        .order_by_desc(resource_iteration::Column::RevCreated)
        .one(txn)
        .await?
    {
        return Ok(model.name);
    }

    if let Some(model) = resource_iteration::Entity::find()
        .filter(resource_iteration::Column::HeaderId.eq(header_id))
        .order_by_desc(resource_iteration::Column::RevCreated)
        .one(txn)
        .await?
    {
        return Ok(model.name);
    }

    Ok(fallback_resource_name(header_id))
}

pub async fn query_task_history(
    ctx: &Context,
    task_header_id: i32,
    from_revision: Option<Int64>,
    from_timestamp: Option<DateTime<Utc>>,
    direction: SearchDirection,
    limit: Option<i32>,
) -> anyhow::Result<TaskHistoryResult> {
    let txn = ctx.txn().await?;
    let limit = (limit.unwrap_or(30).min(50).max(1)) as usize;

    let from_rev: Option<i64> = match (from_revision, from_timestamp) {
        (Some(rev), _) => Some(rev.0),
        (None, Some(ts)) => {
            let before = revision::Entity::find()
                .filter(revision::Column::Timestamp.lte(ts))
                .order_by_desc(revision::Column::Timestamp)
                .one(txn)
                .await?;
            let after = revision::Entity::find()
                .filter(revision::Column::Timestamp.gt(ts))
                .order_by_asc(revision::Column::Timestamp)
                .one(txn)
                .await?;
            match (before, after) {
                (Some(b), Some(a)) => {
                    let diff_b = (ts - b.timestamp).num_seconds().unsigned_abs();
                    let diff_a = (a.timestamp - ts).num_seconds().unsigned_abs();
                    Some(if diff_b <= diff_a { b.id } else { a.id })
                }
                (Some(b), None) => Some(b.id),
                (None, Some(a)) => Some(a.id),
                (None, None) => None,
            }
        }
        (None, None) => None,
    };

    let task_iterations: Vec<task::Model> =
        task::Entity::find().filter(task::Column::HeaderId.eq(task_header_id)).all(txn).await?;

    let deps: Vec<dependency::Model> = dependency::Entity::find()
        .filter(
            Condition::any()
                .add(dependency::Column::PredecessorId.eq(task_header_id))
                .add(dependency::Column::SuccessorId.eq(task_header_id)),
        )
        .all(txn)
        .await?;

    let bookings: Vec<booking::Model> =
        booking::Entity::find().filter(booking::Column::TaskId.eq(task_header_id)).all(txn).await?;

    let constraints: Vec<resource_constraint::Model> = resource_constraint::Entity::find()
        .filter(resource_constraint::Column::TaskId.eq(task_header_id))
        .all(txn)
        .await?;

    let constraint_ids: Vec<i32> = constraints.iter().map(|c| c.id).collect();
    let entries_by_constraint: HashMap<i32, Vec<i32>> = if !constraint_ids.is_empty() {
        let all_entries: Vec<resource_constraint_entry::Model> =
            resource_constraint_entry::Entity::find()
                .filter(
                    resource_constraint_entry::Column::ResourceConstraintId.is_in(constraint_ids),
                )
                .all(txn)
                .await?;
        let mut map: HashMap<i32, Vec<i32>> = HashMap::new();
        for e in all_entries {
            map.entry(e.resource_constraint_id).or_default().push(e.resource_id);
        }
        map
    } else {
        HashMap::new()
    };

    let mut rev_ids: BTreeSet<i64> = BTreeSet::new();
    for t in &task_iterations {
        rev_ids.insert(t.rev_created);
        if let Some(rd) = t.rev_deleted {
            rev_ids.insert(rd);
        }
    }
    for d in &deps {
        rev_ids.insert(d.rev_created);
        if let Some(rd) = d.rev_deleted {
            rev_ids.insert(rd);
        }
    }
    for b in &bookings {
        rev_ids.insert(b.rev_created);
        if let Some(rd) = b.rev_deleted {
            rev_ids.insert(rd);
        }
    }
    for c in &constraints {
        rev_ids.insert(c.rev_created);
        if let Some(rd) = c.rev_deleted {
            rev_ids.insert(rd);
        }
    }

    let sorted_revs: Vec<i64> = match direction {
        SearchDirection::Backward => {
            let mut v: Vec<i64> = if let Some(from) = from_rev {
                rev_ids.into_iter().filter(|&r| r <= from).collect()
            } else {
                rev_ids.into_iter().collect()
            };
            v.sort_unstable_by(|a, b| b.cmp(a));
            v
        }
        SearchDirection::Forward => {
            let mut v: Vec<i64> = if let Some(from) = from_rev {
                rev_ids.into_iter().filter(|&r| r >= from).collect()
            } else {
                rev_ids.into_iter().collect()
            };
            v.sort_unstable();
            v
        }
    };

    let rev_timestamps: HashMap<i64, DateTime<Utc>> = if !sorted_revs.is_empty() {
        revision::Entity::find()
            .filter(revision::Column::Id.is_in(sorted_revs.clone()))
            .all(txn)
            .await?
            .into_iter()
            .map(|r| (r.id, r.timestamp))
            .collect()
    } else {
        HashMap::new()
    };

    let mut changes: Vec<IChangeValue> = Vec::new();

    for &rev_id in &sorted_revs {
        let ts = rev_timestamps.get(&rev_id).copied().unwrap_or_default();

        // --- Task iteration changes ---
        let created_iters: Vec<&task::Model> =
            task_iterations.iter().filter(|t| t.rev_created == rev_id).collect();
        let deleted_iters: Vec<&task::Model> =
            task_iterations.iter().filter(|t| t.rev_deleted == Some(rev_id)).collect();

        let created_headers: HashSet<i32> =
            created_iters.iter().filter_map(|t| t.header_id).collect();
        let deleted_headers: HashSet<i32> =
            deleted_iters.iter().filter_map(|t| t.header_id).collect();
        let updated_headers: HashSet<i32> =
            created_headers.intersection(&deleted_headers).copied().collect();

        for t in &created_iters {
            let hid = t.header_id.unwrap_or(t.id);
            let ct = if updated_headers.contains(&hid) {
                ChangeType::Updated
            } else {
                let is_first = !task_iterations.iter().any(|other| {
                    other.header_id == t.header_id && other.id != t.id && other.rev_created < rev_id
                });
                if is_first { ChangeType::Created } else { ChangeType::Updated }
            };
            changes.push(
                TaskIterationChange {
                    rev_id,
                    ts,
                    ct,
                    task_model: (*t).clone(),
                    revision: Some(rev_id),
                }
                .into(),
            );
        }

        for t in &deleted_iters {
            let hid = t.header_id.unwrap_or(t.id);
            if !updated_headers.contains(&hid) {
                changes.push(
                    TaskIterationChange {
                        rev_id,
                        ts,
                        ct: ChangeType::Deleted,
                        task_model: (*t).clone(),
                        revision: Some(rev_id),
                    }
                    .into(),
                );
            }
        }

        // --- Dependency changes ---
        for d in &deps {
            if d.rev_created == rev_id {
                changes.push(
                    DependencyChange {
                        rev_id,
                        ts,
                        ct: ChangeType::Created,
                        predecessor_id_val: d.predecessor_id,
                        successor_id_val: d.successor_id,
                        predecessor_title_val: resolve_task_title_at_revision(
                            txn,
                            d.predecessor_id,
                            rev_id,
                        )
                        .await?,
                        successor_title_val: resolve_task_title_at_revision(
                            txn,
                            d.successor_id,
                            rev_id,
                        )
                        .await?,
                    }
                    .into(),
                );
            }
            if d.rev_deleted == Some(rev_id) {
                changes.push(
                    DependencyChange {
                        rev_id,
                        ts,
                        ct: ChangeType::Deleted,
                        predecessor_id_val: d.predecessor_id,
                        successor_id_val: d.successor_id,
                        predecessor_title_val: resolve_task_title_at_revision(
                            txn,
                            d.predecessor_id,
                            rev_id,
                        )
                        .await?,
                        successor_title_val: resolve_task_title_at_revision(
                            txn,
                            d.successor_id,
                            rev_id,
                        )
                        .await?,
                    }
                    .into(),
                );
            }
        }

        // --- Booking changes ---
        for b in &bookings {
            if b.rev_created == rev_id {
                changes.push(
                    BookingChange {
                        rev_id,
                        ts,
                        ct: ChangeType::Created,
                        booking_model: Some(b.clone()),
                        revision: Some(rev_id),
                    }
                    .into(),
                );
            }
            if b.rev_deleted == Some(rev_id) {
                changes.push(
                    BookingChange {
                        rev_id,
                        ts,
                        ct: ChangeType::Deleted,
                        booking_model: None,
                        revision: Some(rev_id),
                    }
                    .into(),
                );
            }
        }

        // --- Resource constraint changes ---
        for c in &constraints {
            if c.rev_created == rev_id {
                changes.push(
                    ResourceConstraintChange {
                        rev_id,
                        ts,
                        ct: ChangeType::Created,
                        constraint_id_val: c.id,
                        optional_val: c.optional,
                        speed_val: c.speed as f64,
                        resource_ids_val: entries_by_constraint
                            .get(&c.id)
                            .cloned()
                            .unwrap_or_default(),
                        resource_names_val: {
                            let resource_ids =
                                entries_by_constraint.get(&c.id).cloned().unwrap_or_default();
                            let mut names = Vec::with_capacity(resource_ids.len());
                            for resource_id in resource_ids {
                                names.push(
                                    resolve_resource_name_at_revision(txn, resource_id, rev_id)
                                        .await?,
                                );
                            }
                            names
                        },
                    }
                    .into(),
                );
            }
            if c.rev_deleted == Some(rev_id) {
                changes.push(
                    ResourceConstraintChange {
                        rev_id,
                        ts,
                        ct: ChangeType::Deleted,
                        constraint_id_val: c.id,
                        optional_val: c.optional,
                        speed_val: c.speed as f64,
                        resource_ids_val: entries_by_constraint
                            .get(&c.id)
                            .cloned()
                            .unwrap_or_default(),
                        resource_names_val: {
                            let resource_ids =
                                entries_by_constraint.get(&c.id).cloned().unwrap_or_default();
                            let mut names = Vec::with_capacity(resource_ids.len());
                            for resource_id in resource_ids {
                                names.push(
                                    resolve_resource_name_at_revision(txn, resource_id, rev_id)
                                        .await?,
                                );
                            }
                            names
                        },
                    }
                    .into(),
                );
            }
        }

        if changes.len() > limit {
            break;
        }
    }

    let has_more = changes.len() > limit;
    changes.truncate(limit);

    Ok(TaskHistoryResult { changes, has_more })
}
