use std::{
    any::{Any, TypeId},
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

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

pub struct BatcherLoaderKey {
    type_id: TypeId,
    batcher_key: Arc<dyn Any + Send + Sync>,
    eq_fn: fn(&(dyn Any + Send + Sync), &(dyn Any + Send + Sync)) -> bool,
    hash_fn: fn(&(dyn Any + Send + Sync), &mut dyn Hasher),
}

impl BatcherLoaderKey {
    pub(crate) fn new<B: BatcherToKey>(batcher_key: B::MapKey) -> Self {
        Self {
            type_id: TypeId::of::<B>(),
            batcher_key: Arc::new(batcher_key),
            eq_fn: |left, right| {
                right
                    .downcast_ref::<B::MapKey>()
                    .is_some_and(|right| left.downcast_ref::<B::MapKey>() == Some(right))
            },
            hash_fn: |batcher_key, state| {
                if let Some(batcher_key) = batcher_key.downcast_ref::<B::MapKey>() {
                    let mut inner = std::collections::hash_map::DefaultHasher::new();
                    batcher_key.hash(&mut inner);
                    state.write_u64(inner.finish());
                }
            },
        }
    }
}

impl Clone for BatcherLoaderKey {
    fn clone(&self) -> Self {
        Self {
            type_id: self.type_id,
            batcher_key: Arc::clone(&self.batcher_key),
            eq_fn: self.eq_fn,
            hash_fn: self.hash_fn,
        }
    }
}

impl PartialEq for BatcherLoaderKey {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
            && (self.eq_fn)(self.batcher_key.as_ref(), other.batcher_key.as_ref())
    }
}

impl Eq for BatcherLoaderKey {}

impl std::hash::Hash for BatcherLoaderKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
        (self.hash_fn)(self.batcher_key.as_ref(), state);
    }
}

pub struct BatcherWrapper<B: Batcher> {
    ctx: Weak<DbContext>,
    batcher: B,
}

impl<B: Batcher> BatcherWrapper<B> {
    pub(crate) fn new(ctx: Weak<DbContext>, batcher: B) -> Self {
        Self { ctx, batcher }
    }
}

impl<B: Batcher> dataloader::BatchFn<B::Key, B::Value> for BatcherWrapper<B> {
    async fn load(&mut self, values: &[B::Key]) -> HashMap<B::Key, B::Value> {
        let ctx = self.ctx.upgrade();
        match ctx {
            None => {
                return Default::default();
            }
            Some(ctx) => match self.batcher.load(&ctx, values).await {
                Ok(data) => data,
                Err(_) => {
                    return Default::default();
                }
            },
        }
    }
}

pub trait VecBatcher: Batcher<Value = Vec<Self::Item>> {
    type Item: Clone + Send + Sync;
}

impl<B, T> VecBatcher for B
where
    B: Batcher<Value = Vec<T>>,
    T: Clone + Send + Sync,
{
    type Item = T;
}

