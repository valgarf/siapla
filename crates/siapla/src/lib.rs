use std::fmt::Display;

use thiserror::Error;

pub mod app_state;
pub mod db;
pub mod gql;
pub mod scheduling;

// Re-exports for backward compatibility
pub use db::entity;
pub use db::{ColumnIntoUsize, Link, RangeRevColumns};
pub use db::{
    PredecessorTaskHeaders, PredecessorTaskIterations, ResourceHeadersFromAllocation,
    ResourceHeadersFromBooking, ResourceHeadersFromResourceConstraint,
    ResourceIterationsFromAllocation, ResourceIterationsFromBooking,
    ResourceIterationsFromResourceConstraint, SuccessorTaskHeaders, SuccessorTaskIterations,
};

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
