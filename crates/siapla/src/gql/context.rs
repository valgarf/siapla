use std::{env, sync::Arc};

use super::dataloader::{IdBatcher, TaskLoader};
use sea_orm::{Database, DatabaseConnection};
use tokio::sync::OnceCell;
#[derive(Clone)]
pub struct Context {
    db: OnceCell<DatabaseConnection>,
    task_loader: OnceCell<TaskLoader>,
    batcher: IdBatcher,
}

impl juniper::Context for Context {}

impl Context {
    pub fn new() -> Arc<Self> {
        Arc::new_cyclic(|ctx| Self {
            db: Default::default(),
            task_loader: Default::default(),
            batcher: IdBatcher { ctx: ctx.clone() },
        })
    }

    pub async fn db(&self) -> anyhow::Result<&DatabaseConnection> {
        self.db
            .get_or_try_init::<anyhow::Error, _, _>(|| async {
                Ok(Database::connect(env::var("DATABASE_URL")?).await?)
            })
            .await
    }

    pub async fn task_loader(&self) -> &TaskLoader {
        self.task_loader
            .get_or_init(|| async { TaskLoader::new(self.batcher.clone()).with_yield_count(100) })
            .await
    }
}
