mod datastructures;
mod db_layer;
mod interval;
mod weak_hash_set;

use std::{env, time::Duration};

pub use datastructures::*;
pub use db_layer::query_problem;
pub use interval::{Bound, EndBound, Interval, Intervals, StartBound};
use sea_orm::Database;
pub use weak_hash_set::WeakHashSet;

pub async fn recalculate_loop() {
    loop {
        match Database::connect(env::var("DATABASE_URL").expect("DATABASE_URL not set.")).await {
            Ok(db) => match query_problem(&db).await {
                Err(err) => println!("Error querying problem: {}", err),
                Ok(problem) => {
                    println!("Problem recalculated successfully.");
                    drop(problem);
                }
            },
            Err(err) => println!("Failed to connect to database: {}", err),
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
