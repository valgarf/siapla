use std::{
    env,
    sync::{Arc, Weak},
};

use super::dataloader::{AvailabilityBatcher, BatcherLoaderKey, BatcherWrapper, ByColBatcher};
use crate::{ColumnIntoUsize, gql::dataloader::BatcherToKey, scheduling::Intervals};
use crate::{RevModeEntity, revisioning::resolve_revision};
use crate::{SiaplaError, gql::dataloader::ByColRevBatcher};
use chrono::NaiveDateTime;

use crate::app_state::AppState;
use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use futures::{TryFutureExt, lock::Mutex};
use sea_orm::{
    Database, DatabaseTransaction, EntityTrait, TransactionTrait as _, strum::IntoEnumIterator,
};
use std::collections::HashMap;
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

    pub async fn loader<B: BatcherToKey>(
        &self,
        batcher: B,
    ) -> Arc<
        dataloader::cached::Loader<B::Key, Result<B::Value, Arc<anyhow::Error>>, BatcherWrapper<B>>,
    > {
        let key = BatcherLoaderKey::new::<B>(batcher.loader_map_key());
        loop {
            let read_loaders = self.generic_batch_loaders.read().await;
            if let Some(loader) = read_loaders.get(&key).and_then(|loader| {
                Arc::clone(loader)
                    .downcast::<dataloader::cached::Loader<
                        B::Key,
                        Result<B::Value, Arc<anyhow::Error>>,
                        BatcherWrapper<B>,
                    >>()
                    .ok()
            }) {
                return loader;
            }
            drop(read_loaders);

            let mut write_loaders = self.generic_batch_loaders.write().await;
            write_loaders.entry(key.clone()).or_insert_with(|| {
                Arc::new(
                    dataloader::cached::Loader::new(BatcherWrapper::new(
                        self.me.clone(),
                        batcher.clone(),
                    ))
                    .with_yield_count(100),
                )
            });
        }
    }

    /// Generic dataloader that loads values by column value
    ///
    /// Column selection is a little hackish, you have to provide the column as usize.
    /// Usage:
    /// ```ignore
    /// const CIDX: usize = task::Column::Id as usize;
    /// ctx.load_by_col::<task::Entity, CIDX>(parent_id).await
    /// ```
    pub fn load_by_col<ET: EntityTrait, const CIDX: usize>(
        &self,
        value: impl Into<sea_orm::Value>,
    ) -> impl Future<Output = anyhow::Result<Vec<ET::Model>>> + 'static
    where
        ET::Model: Send + Sync,
        ET::Column: ColumnIntoUsize,
    {
        let me = self.me.clone();
        let value: sea_orm::Value = value.into();

        async move {
            let ctx =
                me.upgrade().ok_or(SiaplaError::new("Weak ref not upgradable in dataloader."))?;
            let _txn = ctx.txn().await?;
            let col = <ET::Column as IntoEnumIterator>::iter()
                .nth(CIDX)
                .ok_or(anyhow::anyhow!("Column index does not exist!"))?;
            let loader = ctx.loader(ByColBatcher::<ET> { col }).await;
            let res = loader.try_load(value).await;
            match res {
                Err(_) => Ok(vec![]), // id cannot be found
                Ok(value) => Ok(value.map_err(|err| anyhow::anyhow!("{err}"))?),
            }
        }
    }

    pub fn load_one_by_col<ET: EntityTrait, const CIDX: usize>(
        &self,
        value: impl Into<sea_orm::Value>,
    ) -> impl Future<Output = anyhow::Result<Option<ET::Model>>> + 'static
    where
        ET::Model: Send + Sync,
        ET::Column: ColumnIntoUsize,
    {
        let fut = self.load_by_col::<ET, CIDX>(value);

        fut.and_then(|mut res: Vec<ET::Model>| async move {
            if res.is_empty() {
                Ok(None)
            } else if res.len() == 1 {
                Ok(res.drain(..).next())
            } else {
                Err(anyhow::anyhow!("More than one entry found"))
            }
        })
    }

    pub fn load_by_col_at_revision<ET: EntityTrait, const CIDX: usize>(
        &self,
        value: impl Into<sea_orm::Value>,
        revision: Option<i64>,
    ) -> impl Future<Output = anyhow::Result<Vec<ET::Model>>> + 'static
    where
        ET: RevModeEntity,
        ET::Model: Send + Sync,
        ET::Column: IntoEnumIterator + ColumnIntoUsize,
    {
        let me = self.me.clone();
        let value: sea_orm::Value = value.into();

        async move {
            let ctx =
                me.upgrade().ok_or(SiaplaError::new("Weak ref not upgradable in dataloader."))?;
            let txn = ctx.txn().await?;
            let revision = resolve_revision(txn, revision)
                .await?
                .ok_or(anyhow::anyhow!("No revision found in database"))?;
            let col = <ET::Column as IntoEnumIterator>::iter()
                .nth(CIDX)
                .ok_or(anyhow::anyhow!("Column index does not exist!"))?;
            let loader = ctx.loader(ByColRevBatcher::<ET> { revision, col }).await;
            let res = loader.try_load(value).await;
            match res {
                Err(_) => Ok(vec![]), // id cannot be found
                Ok(value) => Ok(value.map_err(|err| anyhow::anyhow!("{err}"))?),
            }
        }
    }

    pub fn load_one_by_col_at_revision<ET: EntityTrait, const CIDX: usize>(
        &self,
        value: impl Into<sea_orm::Value>,
        revision: Option<i64>,
    ) -> impl Future<Output = anyhow::Result<Option<ET::Model>>> + 'static
    where
        ET: RevModeEntity,
        ET::Model: Send + Sync,
        ET::Column: IntoEnumIterator + ColumnIntoUsize,
    {
        let fut = self.load_by_col_at_revision::<ET, CIDX>(value, revision);

        fut.and_then(|mut res: Vec<ET::Model>| async move {
            if res.is_empty() {
                Ok(None)
            } else if res.len() == 1 {
                Ok(res.drain(..).next())
            } else {
                Err(anyhow::anyhow!("More than one entry found"))
            }
        })
    }

    pub fn load_allocations_by_task_at_revision<const KEY_CIDX: usize>(
        &self,
        value: impl Into<sea_orm::Value>,
        revision: i64,
    ) -> impl Future<Output = anyhow::Result<Vec<crate::entity::allocation::Model>>> + 'static {
        self.load_by_col_at_revision::<crate::entity::allocation::Entity, KEY_CIDX>(
            value,
            Some(revision),
        )
    }

    pub fn load_issues_by_task_at_revision<const KEY_CIDX: usize>(
        &self,
        value: impl Into<sea_orm::Value>,
        revision: i64,
    ) -> impl Future<Output = anyhow::Result<Vec<crate::entity::issue::Model>>> + 'static {
        self.load_by_col_at_revision::<crate::entity::issue::Entity, KEY_CIDX>(
            value,
            Some(revision),
        )
    }

    pub fn load_combined_availability(
        &self,
        resource_id: i32,
        start: NaiveDateTime,
        end: NaiveDateTime,
        revision: Option<i64>,
    ) -> impl Future<Output = anyhow::Result<Intervals<NaiveDateTime>>> + 'static {
        // let loaders = Arc::clone(&self.availability_loaders);
        let me = self.me.clone();

        async move {
            let ctx =
                me.upgrade().ok_or(SiaplaError::new("Weak ref not upgradable in dataloader."))?;
            let txn = ctx.txn().await?;
            let revision = resolve_revision(txn, revision)
                .await?
                .ok_or(anyhow::anyhow!("No revision found in database"))?;

            let loader = ctx.loader(AvailabilityBatcher { start, end, revision }).await;
            let res = loader.try_load(resource_id).await;
            match res {
                Err(_) => Ok(Intervals::new()), // id cannot be found
                Ok(v) => v.map_err(|err| anyhow::anyhow!("{err}")),
            }
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
