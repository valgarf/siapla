use futures::StreamExt;
use futures::stream::BoxStream;
use juniper::GraphQLEnum;
use juniper::graphql_object;
use juniper::graphql_subscription;
use strum::EnumString;
use strum::IntoStaticStr;
use tokio_stream::wrappers::WatchStream;

use super::context::Context;
use crate::app_state::CalculationState;

#[derive(Clone, Copy, Default)]
pub struct Subscription {}

pub struct GQLCalculationUpdate {
    inner: CalculationState,
}

#[derive(GraphQLEnum, IntoStaticStr, EnumString)]
#[graphql(name = "CalculationState")]
pub enum GQLCalculationState {
    Modified,
    Calculating,
    Finished,
}

#[graphql_object(name = "CalculationUpdate")]
impl GQLCalculationUpdate {
    pub fn state(&self) -> GQLCalculationState {
        match &self.inner {
            CalculationState::Modified => GQLCalculationState::Modified,
            CalculationState::Calculating => GQLCalculationState::Calculating,
            CalculationState::Finished => GQLCalculationState::Finished,
        }
    }

    pub async fn plan(&self, _ctx: &Context) -> Option<crate::gql::types::plan::Plan> {
        match &self.inner {
            CalculationState::Finished => Some(crate::gql::types::plan::Plan {}),
            _ => None,
        }
    }
}

#[graphql_subscription]
#[graphql(context = Context)]
impl Subscription {
    async fn api_version() -> BoxStream<'static, &'static str> {
        Box::pin(futures::stream::once(async { "0.1" }))
    }

    async fn calculation_update(
        ctx: &Context,
    ) -> BoxStream<'static, Result<GQLCalculationUpdate, juniper::FieldError>> {
        println!("START SUBSCRIPTION");
        println!("START SUBSCRIPTION");
        println!("START SUBSCRIPTION");
        let app_state = ctx.app_state();
        let rx = app_state.state_tx.subscribe();
        let stream = WatchStream::new(rx).map(|s| Ok(GQLCalculationUpdate { inner: s }));
        Box::pin(stream)
    }
}

impl Subscription {
    pub fn new() -> Self {
        Default::default()
    }
}
