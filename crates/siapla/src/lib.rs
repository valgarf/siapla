use std::fmt::Display;

use sea_orm::{ColumnTrait, EntityTrait};
use thiserror::Error;

use crate::revisioning::active_for_revision;

pub mod app_state;
pub mod entity;
pub mod gql;
pub mod revisioning;
pub mod scheduling;

/// Trait for entities that have revision range columns (rev_created and rev_deleted).
pub trait RangeRevColumns: EntityTrait {
    fn rev_created_column() -> Self::Column;
    fn rev_deleted_column() -> Self::Column;

    fn condition(revision: i64) -> sea_orm::Condition {
        active_for_revision(Self::rev_created_column(), Self::rev_deleted_column(), Some(revision))
    }
}

macro_rules! impl_range_rev_columns {
    ($entity:path, $created:ident, $deleted:ident) => {
        impl RangeRevColumns for $entity {
            fn rev_created_column() -> Self::Column {
                Self::Column::$created
            }

            fn rev_deleted_column() -> Self::Column {
                Self::Column::$deleted
            }
        }
    };
}

pub trait ColumnIntoUsize: ColumnTrait {
    fn to_column_index(&self) -> usize;
}

macro_rules! impl_column_into_usize {
    ($column:path) => {
        impl ColumnIntoUsize for $column {
            fn to_column_index(&self) -> usize {
                (*self) as usize
            }
        }
    };
}

impl_column_into_usize!(entity::allocated_resource::Column);
impl_column_into_usize!(entity::allocation::Column);
impl_column_into_usize!(entity::availability::Column);
impl_column_into_usize!(entity::booking::Column);
impl_column_into_usize!(entity::booking_resource::Column);
impl_column_into_usize!(entity::dependency::Column);
impl_column_into_usize!(entity::holiday::Column);
impl_column_into_usize!(entity::holiday_entry::Column);
impl_column_into_usize!(entity::issue::Column);
impl_column_into_usize!(entity::resource_constraint::Column);
impl_column_into_usize!(entity::resource_constraint_entry::Column);
impl_column_into_usize!(entity::resource_header::Column);
impl_column_into_usize!(entity::resource_iteration::Column);
impl_column_into_usize!(entity::revision::Column);
impl_column_into_usize!(entity::task_header::Column);
impl_column_into_usize!(entity::task_iteration::Column);
impl_column_into_usize!(entity::vacation::Column);

impl_range_rev_columns!(entity::task_header::Entity, RevCreated, RevDeleted);
impl_range_rev_columns!(entity::resource_header::Entity, RevCreated, RevDeleted);
impl_range_rev_columns!(entity::task_iteration::Entity, RevCreated, RevDeleted);
impl_range_rev_columns!(entity::resource_iteration::Entity, RevCreated, RevDeleted);
impl_range_rev_columns!(entity::dependency::Entity, RevCreated, RevDeleted);
impl_range_rev_columns!(entity::resource_constraint::Entity, RevCreated, RevDeleted);
impl_range_rev_columns!(entity::availability::Entity, RevCreated, RevDeleted);
impl_range_rev_columns!(entity::vacation::Entity, RevCreated, RevDeleted);
impl_range_rev_columns!(entity::booking::Entity, RevCreated, RevDeleted);
impl_range_rev_columns!(entity::allocation::Entity, RevCreated, RevDeleted);
impl_range_rev_columns!(entity::issue::Entity, RevCreated, RevDeleted);

// ---------------------------------------------------------------------------
// Link – describes a many-to-many traversal through a link table
// ---------------------------------------------------------------------------

/// Describes how to traverse from a source id through a link table to a target table.
///
/// The dataloader uses this to batch-load targets:
/// 1. Query the link table filtered by `link_filter_col` ∈ source ids.
/// 2. Extract target ids via `extract_target_id`.
/// 3. Query the target table filtered by `target_match_col` ∈ target ids (and revision).
pub trait Link: Clone + Send + Sync + 'static {
    type LinkEntity: EntityTrait;
    type TargetEntity: EntityTrait;

    fn link_filter_col() -> <Self::LinkEntity as EntityTrait>::Column;
    fn extract_target_id(link: &<Self::LinkEntity as EntityTrait>::Model) -> sea_orm::Value;
    fn target_match_col() -> <Self::TargetEntity as EntityTrait>::Column;

    /// Override to add a revision filter on the **link** table.
    /// Return `None` (the default) when the link table has no revision columns.
    fn link_revision_condition(_revision: i64) -> Option<sea_orm::Condition> {
        None
    }
}

// Helper macro – two arms: without and with `revisioned_link`.
macro_rules! define_link {
    // Non-revisioned link table
    (
        $name:ident,
        link: $link_entity:ty, filter: $filter_col:expr,
        extract: |$lp:ident| $extract:expr,
        target: $target_entity:ty, match_col: $match_col:expr
    ) => {
        #[derive(Clone)]
        pub struct $name;

        impl Link for $name {
            type LinkEntity = $link_entity;
            type TargetEntity = $target_entity;

            fn link_filter_col() -> <Self::LinkEntity as EntityTrait>::Column {
                $filter_col
            }
            fn extract_target_id(
                $lp: &<Self::LinkEntity as EntityTrait>::Model,
            ) -> sea_orm::Value {
                $extract
            }
            fn target_match_col() -> <Self::TargetEntity as EntityTrait>::Column {
                $match_col
            }
        }
    };
    // Revisioned link table
    (
        $name:ident,
        link: $link_entity:ty, filter: $filter_col:expr,
        extract: |$lp:ident| $extract:expr,
        target: $target_entity:ty, match_col: $match_col:expr,
        revisioned_link: $rev_entity:ty
    ) => {
        #[derive(Clone)]
        pub struct $name;

        impl Link for $name {
            type LinkEntity = $link_entity;
            type TargetEntity = $target_entity;

            fn link_filter_col() -> <Self::LinkEntity as EntityTrait>::Column {
                $filter_col
            }
            fn extract_target_id(
                $lp: &<Self::LinkEntity as EntityTrait>::Model,
            ) -> sea_orm::Value {
                $extract
            }
            fn target_match_col() -> <Self::TargetEntity as EntityTrait>::Column {
                $match_col
            }
            fn link_revision_condition(revision: i64) -> Option<sea_orm::Condition> {
                Some(<$rev_entity as RangeRevColumns>::condition(revision))
            }
        }
    };
}

// -- dependency-based links (revisioned link table) -------------------------

define_link!(PredecessorTaskIterations,
    link: entity::dependency::Entity, filter: entity::dependency::Column::SuccessorId,
    extract: |d| d.predecessor_id.into(),
    target: entity::task_iteration::Entity, match_col: entity::task_iteration::Column::HeaderId,
    revisioned_link: entity::dependency::Entity
);

define_link!(SuccessorTaskIterations,
    link: entity::dependency::Entity, filter: entity::dependency::Column::PredecessorId,
    extract: |d| d.successor_id.into(),
    target: entity::task_iteration::Entity, match_col: entity::task_iteration::Column::HeaderId,
    revisioned_link: entity::dependency::Entity
);

define_link!(PredecessorTaskHeaders,
    link: entity::dependency::Entity, filter: entity::dependency::Column::SuccessorId,
    extract: |d| d.predecessor_id.into(),
    target: entity::task_header::Entity, match_col: entity::task_header::Column::Id,
    revisioned_link: entity::dependency::Entity
);

define_link!(SuccessorTaskHeaders,
    link: entity::dependency::Entity, filter: entity::dependency::Column::PredecessorId,
    extract: |d| d.successor_id.into(),
    target: entity::task_header::Entity, match_col: entity::task_header::Column::Id,
    revisioned_link: entity::dependency::Entity
);

// -- allocated_resource-based links (non-revisioned link table) -------------

define_link!(ResourceIterationsFromAllocation,
    link: entity::allocated_resource::Entity, filter: entity::allocated_resource::Column::AllocationId,
    extract: |a| a.resource_id.into(),
    target: entity::resource_iteration::Entity, match_col: entity::resource_iteration::Column::HeaderId
);

define_link!(ResourceHeadersFromAllocation,
    link: entity::allocated_resource::Entity, filter: entity::allocated_resource::Column::AllocationId,
    extract: |a| a.resource_id.into(),
    target: entity::resource_header::Entity, match_col: entity::resource_header::Column::Id
);

// -- booking_resource-based links (non-revisioned link table) ---------------

define_link!(ResourceIterationsFromBooking,
    link: entity::booking_resource::Entity, filter: entity::booking_resource::Column::BookingId,
    extract: |b| b.resource_id.into(),
    target: entity::resource_iteration::Entity, match_col: entity::resource_iteration::Column::HeaderId
);

define_link!(ResourceHeadersFromBooking,
    link: entity::booking_resource::Entity, filter: entity::booking_resource::Column::BookingId,
    extract: |b| b.resource_id.into(),
    target: entity::resource_header::Entity, match_col: entity::resource_header::Column::Id
);

// -- resource_constraint_entry-based links (non-revisioned link table) ------

define_link!(ResourceIterationsFromResourceConstraint,
    link: entity::resource_constraint_entry::Entity,
    filter: entity::resource_constraint_entry::Column::ResourceConstraintId,
    extract: |e| e.resource_id.into(),
    target: entity::resource_iteration::Entity, match_col: entity::resource_iteration::Column::HeaderId
);

define_link!(ResourceHeadersFromResourceConstraint,
    link: entity::resource_constraint_entry::Entity,
    filter: entity::resource_constraint_entry::Column::ResourceConstraintId,
    extract: |e| e.resource_id.into(),
    target: entity::resource_header::Entity, match_col: entity::resource_header::Column::Id
);

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Error, Debug)]
pub struct SiaplaError {
    msg: String,
}

impl SiaplaError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { msg: msg.into() }
    }
}

impl Display for SiaplaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.msg.fmt(f)
    }
}
