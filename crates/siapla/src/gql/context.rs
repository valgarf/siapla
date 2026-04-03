use std::{
    collections::HashMap,
    env,
    sync::{Arc, Weak},
};

use super::dataloader::{BatcherLoaderKey, BatcherToKey, LoaderWrapper};

use crate::app_state::AppState;
use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use futures::lock::Mutex;
use sea_orm::{Database, DatabaseTransaction, TransactionTrait as _};
use tokio::sync::{OnceCell, RwLock};

// global database url that can be set from the server's command line
static GLOBAL_DATABASE_URL: OnceCell<String> = OnceCell::const_new();

/// Set the global database url used by Context. Call early in program startup.
pub fn set_global_database_url(url: impl Into<String>) {
    let _ = GLOBAL_DATABASE_URL.set(url.into());
}

pub struct Context {
    txn: OnceCell<DatabaseTransaction>,
    generic_batch_loaders: Arc<
        RwLock<std::collections::HashMap<BatcherLoaderKey, Arc<dyn std::any::Any + Send + Sync>>>,
    >,
    me: Weak<Self>,
    app_state: Arc<AppState>,
    success: Mutex<bool>,
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field("txn", &self.txn)
            .field("app_state", &self.app_state)
            .finish()
    }
}

impl juniper::Context for Context {}

impl Context {
    pub fn new(app_state: Arc<AppState>) -> Arc<Self> {
        Arc::new_cyclic(|me| Self {
            txn: Default::default(),
            generic_batch_loaders: Arc::new(RwLock::new(HashMap::new())),
            me: me.clone(),
            success: Mutex::new(true),
            app_state,
        })
    }

    pub async fn failed(&self) {
        let mut lock_guard = self.success.lock().await;
        *lock_guard = false;
    }

    pub async fn txn(&self) -> anyhow::Result<&DatabaseTransaction> {
        self.txn
            .get_or_try_init::<anyhow::Error, _, _>(|| async {
                // prefer explicitly set global url, fall back to env var for compatibility
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

pub async fn add_context(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    // app_state should be provided as an Extension earlier in the stack
    let app_state = req
        .extensions()
        .get::<Arc<AppState>>()
        .cloned()
        .expect("AppState must be provided as an Extension");
    let ctx = Context::new(app_state);
    req.extensions_mut().insert(Arc::clone(&ctx));
    let res = next.run(req).await;
    let mut ctx = Arc::into_inner(ctx).expect("All other references should have been destroyed");
    let ctx_success: bool = *ctx.success.lock().await;
    if res.status().is_success() && ctx_success {
        ctx.commit().await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    } else {
        ctx.rollback().await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    }
    Ok(res)
}

impl Context {
    pub fn app_state(&self) -> Arc<AppState> {
        Arc::clone(&self.app_state)
    }
}
