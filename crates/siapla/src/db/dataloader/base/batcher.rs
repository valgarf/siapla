use std::{collections::HashMap, hash::Hash, sync::Weak};

use crate::db::context::DbContext;
use std::fmt::Debug;

pub trait Batcher: Clone + Send + Sync + 'static {
    type Key: Eq + Hash + Clone + Debug + Send + Sync;
    type Value: Clone + Send + Sync;

    fn load(
        &self,
        db: &DbContext,
        keys: &[Self::Key],
    ) -> impl Future<Output = Result<HashMap<Self::Key, Self::Value>, anyhow::Error>> + Send;

    fn yield_count() -> usize {
        100
    }

    fn max_batch_size() -> usize {
        200
    }

    fn cached() -> bool {
        true
    }
}

pub trait BatcherToKey: Batcher {
    type MapKey: Eq + Hash + Clone + Debug + Send + Sync;
    fn loader_map_key(&self) -> Self::MapKey;
}

impl<B: Batcher> BatcherToKey for B
where
    B: Hash + Eq + Debug,
{
    type MapKey = Self;
    fn loader_map_key(&self) -> Self::MapKey {
        self.clone()
    }
}

/// The dataloader needs a context, so this combines the batcher with the context.
pub struct BatcherWrapper<B: Batcher> {
    ctx: Weak<DbContext>,
    batcher: B,
}

impl<B: Batcher> BatcherWrapper<B> {
    pub(crate) fn new(ctx: Weak<DbContext>, batcher: B) -> Self {
        Self { ctx, batcher }
    }
}

/// Actual dataloder implementation that is picked up by the dataloader crate
/// This basically tries to get the context and then delegates to the batcher.
impl<B: Batcher> dataloader::BatchFn<B::Key, B::Value> for BatcherWrapper<B> {
    async fn load(&mut self, values: &[B::Key]) -> HashMap<B::Key, B::Value> {
        let ctx = self.ctx.upgrade();
        match ctx {
            None => Default::default(),
            Some(ctx) => match self.batcher.load(&ctx, values).await {
                Ok(data) => data,
                Err(_) => Default::default(),
            },
        }
    }
}
