use std::fmt::Display;

use sea_orm::EntityTrait;
use thiserror::Error;

pub mod app_state;
pub mod entity;
pub mod gql;
pub mod revisioning;
pub mod scheduling;

pub trait RevColumns: EntityTrait {
    fn rev_created_column_index() -> usize;
    fn rev_deleted_column_index() -> usize;
}

impl RevColumns for entity::task_iteration::Entity {
    fn rev_created_column_index() -> usize {
        entity::task_iteration::Column::RevCreated as usize
    }

    fn rev_deleted_column_index() -> usize {
        entity::task_iteration::Column::RevDeleted as usize
    }
}

impl RevColumns for entity::resource_iteration::Entity {
    fn rev_created_column_index() -> usize {
        entity::resource_iteration::Column::RevCreated as usize
    }

    fn rev_deleted_column_index() -> usize {
        entity::resource_iteration::Column::RevDeleted as usize
    }
}

impl RevColumns for entity::dependency::Entity {
    fn rev_created_column_index() -> usize {
        entity::dependency::Column::RevCreated as usize
    }

    fn rev_deleted_column_index() -> usize {
        entity::dependency::Column::RevDeleted as usize
    }
}

impl RevColumns for entity::resource_constraint::Entity {
    fn rev_created_column_index() -> usize {
        entity::resource_constraint::Column::RevCreated as usize
    }

    fn rev_deleted_column_index() -> usize {
        entity::resource_constraint::Column::RevDeleted as usize
    }
}

impl RevColumns for entity::availability::Entity {
    fn rev_created_column_index() -> usize {
        entity::availability::Column::RevCreated as usize
    }

    fn rev_deleted_column_index() -> usize {
        entity::availability::Column::RevDeleted as usize
    }
}

impl RevColumns for entity::vacation::Entity {
    fn rev_created_column_index() -> usize {
        entity::vacation::Column::RevCreated as usize
    }

    fn rev_deleted_column_index() -> usize {
        entity::vacation::Column::RevDeleted as usize
    }
}

impl RevColumns for entity::booking::Entity {
    fn rev_created_column_index() -> usize {
        entity::booking::Column::RevCreated as usize
    }

    fn rev_deleted_column_index() -> usize {
        entity::booking::Column::RevDeleted as usize
    }
}

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
