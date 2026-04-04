use super::batcher::BatcherToKey;
use std::{any::Any, collections::HashMap, sync::Arc};
use std::{
    any::TypeId,
    hash::{Hash, Hasher},
};
use tokio::sync::RwLock;

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

pub type GenericBatchLoaderMap = RwLock<HashMap<BatcherLoaderKey, Arc<dyn Any + Send + Sync>>>;
