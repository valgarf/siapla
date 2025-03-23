use std::env;

use sea_orm::{Database, DatabaseConnection};
use tokio::sync::OnceCell;

#[derive(Default, Clone)]
pub struct Context {
    db: OnceCell<DatabaseConnection>,
    // x: u64,
}

impl juniper::Context for Context {}

impl Context {
    pub fn new() -> Self {
        Default::default()
    }

    pub async fn db(&self) -> anyhow::Result<&DatabaseConnection> {
        self.db
            .get_or_try_init::<anyhow::Error, _, _>(|| async {
                Ok(Database::connect(env::var("DATABASE_URL")?).await?)
            })
            .await
    }
}
