use std::{
    collections::HashMap,
    env,
    sync::{Arc, Weak},
};

use super::dataloader::{BatcherLoaderKey, BatcherToKey, LoaderWrapper};

use sea_orm::{Database, DatabaseTransaction, TransactionTrait as _};
use tokio::sync::{OnceCell, RwLock};

static GLOBAL_DATABASE_URL: OnceCell<String> = OnceCell::const_new();

pub fn set_global_database_url(url: impl Into<String>) {
    let _ = GLOBAL_DATABASE_URL.set(url.into());
}

pub struct DbContext {
    txn: OnceCell<DatabaseTransaction>,
    generic_batch_loaders:
        Arc<RwLock<HashMap<BatcherLoaderKey, Arc<dyn std::any::Any + Send + Sync>>>>,
    me: Weak<Self>,
}

impl std::fmt::Debug for DbContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbContext").field("txn", &self.txn).finish()
    }
}

impl DbContext {
    pub fn new() -> Arc<Self> {
        Arc::new_cyclic(|me| Self {
            txn: Default::default(),
            generic_batch_loaders: Arc::new(RwLock::new(HashMap::new())),
            me: me.clone(),
        })
    }

    pub async fn txn(&self) -> anyhow::Result<&DatabaseTransaction> {
        self.txn
            .get_or_try_init::<anyhow::Error, _, _>(|| async {
                let url = GLOBAL_DATABASE_URL
                    .get()
                    .cloned()
                    .or_else(|| env::var("DATABASE_URL").ok())
                    .ok_or_else(|| anyhow::anyhow!("DATABASE_URL not set"))?;
                Ok(Database::connect(url).await?.begin().await?)
            })
            .await
    }

    pub async fn commit(&mut self) -> anyhow::Result<()> {
        if let Some(txn) = self.txn.take() {
            txn.commit().await?;
        }
        Ok(())
    }

    pub async fn rollback(&mut self) -> anyhow::Result<()> {
        if let Some(txn) = self.txn.take() {
            txn.rollback().await?
        }
        Ok(())
    }

    pub async fn loader<B: BatcherToKey>(&self, batcher: B) -> LoaderWrapper<B> {
        let key = BatcherLoaderKey::new::<B>(batcher.loader_map_key());
        loop {
            let read_loaders = self.generic_batch_loaders.read().await;
            if let Some(loader) = read_loaders
                .get(&key)
                .and_then(|loader| Arc::clone(loader).downcast::<LoaderWrapper<B>>().ok())
            {
                return (*loader).clone();
            }
            drop(read_loaders);

            let mut write_loaders = self.generic_batch_loaders.write().await;
            write_loaders.entry(key.clone()).or_insert_with(|| {
                Arc::new(if B::cached() {
                    LoaderWrapper::new_cached(self.me.clone(), batcher.clone())
                } else {
                    LoaderWrapper::new_non_cached(self.me.clone(), batcher.clone())
                })
            });
        }
    }
}
