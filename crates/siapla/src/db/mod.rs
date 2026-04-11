use crate::revisioning::active_for_revision;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

pub mod context;
pub mod dataloader;
pub mod delete;
pub mod entity;
pub mod r#impl;
pub mod insert;
pub mod upsert;

pub use context::DbContext;

/// Trait for entities that have revision range columns (rev_created and rev_deleted).
pub trait RangeRevColumns: EntityTrait {
    fn rev_created_column() -> Self::Column;
    fn rev_deleted_column() -> Self::Column;

    fn condition(revision: i64) -> sea_orm::Condition {
        active_for_revision(Self::rev_created_column(), Self::rev_deleted_column(), Some(revision))
    }

    fn condition_latest() -> sea_orm::Condition {
        active_for_revision(Self::rev_created_column(), Self::rev_deleted_column(), None)
    }

    fn find_revision(revision: i64) -> sea_orm::Select<Self> {
        Self::find().filter(Self::condition(revision))
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

pub trait Link: Clone + Send + Sync + 'static {
    type LinkEntity: EntityTrait;
    type TargetEntity: EntityTrait + RangeRevColumns;

    fn filter_column() -> <Self::LinkEntity as EntityTrait>::Column;
    fn join_link_column() -> <Self::LinkEntity as EntityTrait>::Column;
    fn join_target_column() -> <Self::TargetEntity as EntityTrait>::Column;
    fn link_rev_created_column() -> Option<<Self::LinkEntity as EntityTrait>::Column> {
        None
    }
    fn link_rev_deleted_column() -> Option<<Self::LinkEntity as EntityTrait>::Column> {
        None
    }
}

macro_rules! define_link {
    (
        $name:ident,
        link: $link_entity:ty,
        filter: $filter_col:expr,
        link_join: $link_join_col:expr,
        target: $target_entity:ty,
        target_join: $target_join_col:expr
    ) => {
        #[derive(Clone)]
        pub struct $name;

        impl Link for $name {
            type LinkEntity = $link_entity;
            type TargetEntity = $target_entity;

            fn filter_column() -> <Self::LinkEntity as EntityTrait>::Column {
                $filter_col
            }

            fn join_link_column() -> <Self::LinkEntity as EntityTrait>::Column {
                $link_join_col
            }

            fn join_target_column() -> <Self::TargetEntity as EntityTrait>::Column {
                $target_join_col
            }
        }
    };
    (
        $name:ident,
        link: $link_entity:ty,
        filter: $filter_col:expr,
        link_join: $link_join_col:expr,
        target: $target_entity:ty,
        target_join: $target_join_col:expr,
        link_rev_created: $link_rev_created_col:expr,
        link_rev_deleted: $link_rev_deleted_col:expr
    ) => {
        #[derive(Clone)]
        pub struct $name;

        impl Link for $name {
            type LinkEntity = $link_entity;
            type TargetEntity = $target_entity;

            fn filter_column() -> <Self::LinkEntity as EntityTrait>::Column {
                $filter_col
            }

            fn join_link_column() -> <Self::LinkEntity as EntityTrait>::Column {
                $link_join_col
            }

            fn join_target_column() -> <Self::TargetEntity as EntityTrait>::Column {
                $target_join_col
            }

            fn link_rev_created_column() -> Option<<Self::LinkEntity as EntityTrait>::Column> {
                Some($link_rev_created_col)
            }

            fn link_rev_deleted_column() -> Option<<Self::LinkEntity as EntityTrait>::Column> {
                Some($link_rev_deleted_col)
            }
        }
    };
}

define_link!(
    PredecessorTaskIterations,
    link: entity::dependency::Entity,
    filter: entity::dependency::Column::SuccessorId,
    link_join: entity::dependency::Column::PredecessorId,
    target: entity::task_iteration::Entity,
    target_join: entity::task_iteration::Column::HeaderId,
    link_rev_created: entity::dependency::Column::RevCreated,
    link_rev_deleted: entity::dependency::Column::RevDeleted
);

define_link!(
    SuccessorTaskIterations,
    link: entity::dependency::Entity,
    filter: entity::dependency::Column::PredecessorId,
    link_join: entity::dependency::Column::SuccessorId,
    target: entity::task_iteration::Entity,
    target_join: entity::task_iteration::Column::HeaderId,
    link_rev_created: entity::dependency::Column::RevCreated,
    link_rev_deleted: entity::dependency::Column::RevDeleted
);

define_link!(
    PredecessorTaskHeaders,
    link: entity::dependency::Entity,
    filter: entity::dependency::Column::SuccessorId,
    link_join: entity::dependency::Column::PredecessorId,
    target: entity::task_header::Entity,
    target_join: entity::task_header::Column::Id,
    link_rev_created: entity::dependency::Column::RevCreated,
    link_rev_deleted: entity::dependency::Column::RevDeleted
);

define_link!(
    SuccessorTaskHeaders,
    link: entity::dependency::Entity,
    filter: entity::dependency::Column::PredecessorId,
    link_join: entity::dependency::Column::SuccessorId,
    target: entity::task_header::Entity,
    target_join: entity::task_header::Column::Id,
    link_rev_created: entity::dependency::Column::RevCreated,
    link_rev_deleted: entity::dependency::Column::RevDeleted
);

define_link!(
    ResourceIterationsFromAllocation,
    link: entity::allocated_resource::Entity,
    filter: entity::allocated_resource::Column::AllocationId,
    link_join: entity::allocated_resource::Column::ResourceId,
    target: entity::resource_iteration::Entity,
    target_join: entity::resource_iteration::Column::HeaderId
);

define_link!(
    ResourceHeadersFromAllocation,
    link: entity::allocated_resource::Entity,
    filter: entity::allocated_resource::Column::AllocationId,
    link_join: entity::allocated_resource::Column::ResourceId,
    target: entity::resource_header::Entity,
    target_join: entity::resource_header::Column::Id
);

define_link!(
    ResourceIterationsFromBooking,
    link: entity::booking_resource::Entity,
    filter: entity::booking_resource::Column::BookingId,
    link_join: entity::booking_resource::Column::ResourceId,
    target: entity::resource_iteration::Entity,
    target_join: entity::resource_iteration::Column::HeaderId
);

define_link!(
    ResourceHeadersFromBooking,
    link: entity::booking_resource::Entity,
    filter: entity::booking_resource::Column::BookingId,
    link_join: entity::booking_resource::Column::ResourceId,
    target: entity::resource_header::Entity,
    target_join: entity::resource_header::Column::Id
);

define_link!(
    ResourceIterationsFromResourceConstraint,
    link: entity::resource_constraint_entry::Entity,
    filter: entity::resource_constraint_entry::Column::ResourceConstraintId,
    link_join: entity::resource_constraint_entry::Column::ResourceId,
    target: entity::resource_iteration::Entity,
    target_join: entity::resource_iteration::Column::HeaderId
);

define_link!(
    ResourceHeadersFromResourceConstraint,
    link: entity::resource_constraint_entry::Entity,
    filter: entity::resource_constraint_entry::Column::ResourceConstraintId,
    link_join: entity::resource_constraint_entry::Column::ResourceId,
    target: entity::resource_header::Entity,
    target_join: entity::resource_header::Column::Id
);
