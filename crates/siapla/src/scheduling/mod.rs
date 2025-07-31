mod datastructures;
mod db_layer;
mod ga;
mod interval;
mod weak_hash_set;

use std::{env, sync::Arc, time::Duration};

pub use datastructures::*;
pub use db_layer::query_problem;
pub use interval::{Bound, EndBound, Interval, Intervals, StartBound};
use sea_orm::Database;
pub use weak_hash_set::WeakHashSet;

use crate::{gql::context::Context, scheduling::ga::generate_random_individual};

pub async fn recalculate_loop() {
    loop {
        let ctx = Context::new();
        match query_problem(&ctx).await {
            Err(err) => println!("Error querying problem: {}", err),
            Ok(mut problem) => {
                let individual = generate_random_individual(&mut problem);
                let task_order = individual
                    .tasks
                    .iter()
                    .map(|t| t.task.borrow().title.clone())
                    .collect::<Vec<_>>();
                println!("Problem recalculated successfully. Random order: {:?}", &task_order);
                drop(problem);
            }
        }
        match Arc::into_inner(ctx)
            .expect("This function is the only one with a strong reference.")
            .commit()
            .await
        {
            Err(err) => println!("Error committing: {}", err),
            Ok(_) => {}
        }

        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
