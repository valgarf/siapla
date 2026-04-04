pub mod common;
pub mod context;
pub mod mutation;
pub mod query;
pub mod scalars;
pub mod subscription;
mod types;

pub use types::{
    allocation, availability, booking, history, holiday, issue, plan, resource, task, vacation,
};

use juniper::*;

pub type Schema =
    RootNode<query::Query, mutation::Mutation, subscription::Subscription, scalars::MyScalarValue>;

pub fn schema() -> Schema {
    Schema::new_with_scalar_value(
        query::Query::new(),
        mutation::Mutation::new(),
        subscription::Subscription::new(),
    )
}
