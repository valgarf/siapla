use futures::stream::BoxStream;
use juniper::graphql_subscription;

use super::context::Context;

#[derive(Default)]
pub struct Subscription {}

#[graphql_subscription]
#[graphql(context = Context)]
impl Subscription {
    async fn api_version() -> BoxStream<'static, &'static str> {
        Box::pin(futures::stream::once(async { "0.1" }))
    }
}

impl Subscription {
    pub fn new() -> Self {
        Default::default()
    }
}
