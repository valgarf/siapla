use std::sync::Arc;

use crate::app_state::AppState;
use crate::db::context::DbContext;
use crate::db::dataloader::{BatcherToKey, LoaderWrapper};
use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use futures::lock::Mutex;
use sea_orm::DatabaseTransaction;

pub use crate::db::context::set_global_database_url;

pub struct Context {
    db: Arc<DbContext>,
    app_state: Arc<AppState>,
    success: Mutex<bool>,
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context").field("db", &self.db).field("app_state", &self.app_state).finish()
    }
}

impl juniper::Context for Context {}

impl Context {
    pub fn new(app_state: Arc<AppState>) -> Arc<Self> {
        Arc::new(Self { db: DbContext::new(), app_state, success: Mutex::new(true) })
    }

    pub fn db(&self) -> &DbContext {
        &self.db
    }

    pub async fn failed(&self) {
        let mut lock_guard = self.success.lock().await;
        *lock_guard = false;
    }

    pub async fn txn(&self) -> anyhow::Result<&DatabaseTransaction> {
        self.db.txn().await
    }

    pub async fn loader<B: BatcherToKey>(&self, batcher: B) -> LoaderWrapper<B> {
        self.db.loader(batcher).await
    }

    pub async fn commit(self) -> anyhow::Result<()> {
        let mut db =
            Arc::into_inner(self.db).expect("DbContext must be uniquely owned at commit time");
        db.commit().await
    }

    pub async fn rollback(self) -> anyhow::Result<()> {
        let mut db =
            Arc::into_inner(self.db).expect("DbContext must be uniquely owned at rollback time");
        db.rollback().await
    }

    pub fn app_state(&self) -> Arc<AppState> {
        Arc::clone(&self.app_state)
    }
}

pub async fn add_context(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let app_state = req
        .extensions()
        .get::<Arc<AppState>>()
        .cloned()
        .expect("AppState must be provided as an Extension");
    let ctx = Context::new(app_state);
    req.extensions_mut().insert(Arc::clone(&ctx));
    let res = next.run(req).await;
    let ctx = Arc::into_inner(ctx).expect("All other references should have been destroyed");
    let ctx_success: bool = *ctx.success.lock().await;
    if res.status().is_success() && ctx_success {
        ctx.commit().await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    } else {
        ctx.rollback().await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    }
    Ok(res)
}
