use std::env;

use sea_orm_migration::prelude::*;
use sqlx::migrate::MigrateDatabase as _;

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    match env::var("DATABASE_URL").map(|url| {
        if url.starts_with("sqlite:") {
            Some(url)
        } else {
            None
        }
    }) {
        Ok(Some(url)) => {
            if !sqlx::Sqlite::database_exists(&url).await? {
                sqlx::Sqlite::create_database(&url).await?;
            }
        }
        Ok(None) => {}
        Err(err) => println!("Error trying to check for existence of database: {}", err),
    }

    cli::run_cli(siapla_migration::Migrator).await;
    Ok(())
}
