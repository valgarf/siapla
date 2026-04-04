use super::batcher::{Batcher, BatcherWrapper};
use crate::db::context::DbContext;
use dataloader::{cached::Loader as CachedLoader, non_cached::Loader as NonCachedLoader};
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

/// Detects if a batcher returns a `Vec` and provides helper methods to load single items.
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

/// Wraps a loader (cached or uncached) and provides a convenient interface for loading values.
#[derive(Clone)]
pub enum LoaderWrapper<B: Batcher> {
    Cached(Arc<CachedLoader<B::Key, B::Value, BatcherWrapper<B>>>),
    NonCached(Arc<NonCachedLoader<B::Key, B::Value, BatcherWrapper<B>>>),
}

impl<B: Batcher> LoaderWrapper<B> {
    pub fn new_cached(ctx: Weak<DbContext>, batcher: B) -> Self {
        Self::Cached(Arc::new(
            CachedLoader::new(BatcherWrapper::new(ctx, batcher))
                .with_yield_count(B::yield_count())
                .with_max_batch_size(B::max_batch_size()),
        ))
    }

    pub fn new_non_cached(ctx: Weak<DbContext>, batcher: B) -> Self {
        Self::NonCached(Arc::new(
            NonCachedLoader::new(BatcherWrapper::new(ctx, batcher))
                .with_yield_count(B::yield_count())
                .with_max_batch_size(B::max_batch_size()),
        ))
    }

    pub async fn load(&self, key: B::Key) -> anyhow::Result<B::Value> {
        match self {
            Self::Cached(loader) => {
                loader.try_load(key).await.map_err(|_| anyhow::anyhow!("Key not found"))
            }
            Self::NonCached(loader) => {
                loader.try_load(key).await.map_err(|_| anyhow::anyhow!("Key not found"))
            }
        }
    }

    pub async fn load_many(&self, keys: Vec<B::Key>) -> anyhow::Result<HashMap<B::Key, B::Value>> {
        match self {
            Self::Cached(loader) => {
                loader.try_load_many(keys).await.map_err(|_| anyhow::anyhow!("Key not found"))
            }
            Self::NonCached(loader) => {
                loader.try_load_many(keys).await.map_err(|_| anyhow::anyhow!("Key not found"))
            }
        }
    }
}

impl<B: VecBatcher> LoaderWrapper<B> {
    pub async fn load_one(&self, key: B::Key) -> anyhow::Result<Option<B::Item>> {
        let mut values = self.load(key).await?;
        if values.is_empty() {
            Ok(None)
        } else if values.len() == 1 {
            Ok(values.drain(..).next())
        } else {
            Err(anyhow::anyhow!("More than one entry found"))
        }
    }

    pub async fn load_many_one(
        &self,
        keys: Vec<B::Key>,
    ) -> anyhow::Result<HashMap<B::Key, Option<B::Item>>> {
        self.load_many(keys)
            .await?
            .into_iter()
            .map(|(key, mut values)| {
                if values.len() > 1 {
                    Err(anyhow::anyhow!("More than one entry found"))
                } else {
                    Ok((key, values.pop()))
                }
            })
            .collect()
    }
}
