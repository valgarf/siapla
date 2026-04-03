use std::fmt::Display;

use sea_orm::sea_query::IntoCondition as _;
use sea_orm::{ColumnTrait, EntityTrait};
use thiserror::Error;

use crate::revisioning::active_for_revision;

pub mod app_state;
pub mod entity;
pub mod gql;
pub mod revisioning;
pub mod scheduling;

/// Enum representing the revision mode of an entity: either a range (created and deleted columns)
/// or an exact revision column.
pub enum RevMode<C> {
    Range { created: C, deleted: C },
    Exact { revision: C },
}

impl<C: ColumnTrait> RevMode<C> {
    pub fn condition(&self, revision: i64) -> sea_orm::Condition {
        match self {
            Self::Range { created, deleted } => {
                active_for_revision(*created, *deleted, Some(revision))
            }
            Self::Exact { revision: rev_col } => rev_col.eq(revision).into_condition(),
        }
    }
}

/// Trait for every entity that has some relation to revisions.
/// Allows to access the revision mode and construct a filter condition for a given revision.
pub trait RevModeEntity: EntityTrait {
    fn rev_mode() -> RevMode<Self::Column>;
    fn rev_condition(revision: i64) -> sea_orm::Condition {
        Self::rev_mode().condition(revision)
    }
}

/// Trait for entities that have a revision range (created and deleted columns).
/// They also implement `RevModeEntity` with `RevMode::Range`.
pub trait RangeRevColumns: EntityTrait {
    fn rev_created_column() -> Self::Column;
    fn rev_deleted_column() -> Self::Column;
}

/// Trait for entities that have a single revision column.
/// They also implement `RevModeEntity` with `RevMode::Exact`.
pub trait ExactRevColumn: EntityTrait {
    fn rev_column() -> Self::Column;
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

        impl RevModeEntity for $entity {
            fn rev_mode() -> RevMode<Self::Column> {
                RevMode::Range { created: Self::Column::$created, deleted: Self::Column::$deleted }
            }
        }
    };
}

macro_rules! impl_exact_rev_column {
    ($entity:path, $revision:ident) => {
        impl ExactRevColumn for $entity {
            fn rev_column() -> Self::Column {
                Self::Column::$revision
            }
        }

        impl RevModeEntity for $entity {
            fn rev_mode() -> RevMode<Self::Column> {
                RevMode::Exact { revision: Self::Column::$revision }
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

impl_range_rev_columns!(entity::task_iteration::Entity, RevCreated, RevDeleted);
impl_range_rev_columns!(entity::resource_iteration::Entity, RevCreated, RevDeleted);
impl_range_rev_columns!(entity::dependency::Entity, RevCreated, RevDeleted);
impl_range_rev_columns!(entity::resource_constraint::Entity, RevCreated, RevDeleted);
impl_range_rev_columns!(entity::availability::Entity, RevCreated, RevDeleted);
impl_range_rev_columns!(entity::vacation::Entity, RevCreated, RevDeleted);
impl_range_rev_columns!(entity::booking::Entity, RevCreated, RevDeleted);

impl_exact_rev_column!(entity::allocation::Entity, Revision);
impl_exact_rev_column!(entity::allocated_resource::Entity, Revision);
impl_exact_rev_column!(entity::issue::Entity, Revision);

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
