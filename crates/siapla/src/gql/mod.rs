pub mod common;
pub mod context;
pub mod dataloader;
pub mod mutation;
pub mod query;
pub mod subscription;
mod types;

pub use types::{allocation, availability, holiday, plan, resource, task, vacation};

use juniper::*;

pub type Schema = RootNode<'static, query::Query, mutation::Mutation, subscription::Subscription>;

pub fn schema() -> Schema {
    Schema::new(query::Query::new(), mutation::Mutation::new(), subscription::Subscription::new())
}
