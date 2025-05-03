pub mod common;
pub mod context;
pub mod dataloader;
pub mod mutation;
pub mod query;
pub mod subscription;
mod types;

pub use types::holiday;
pub use types::task;

use juniper::*;

pub type Schema = RootNode<'static, query::Query, mutation::Mutation, subscription::Subscription>;

pub fn schema() -> Schema {
    Schema::new(
        query::Query::new(),
        mutation::Mutation::new(),
        subscription::Subscription::new(),
    )
}
