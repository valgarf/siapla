mod datastructures;
mod db_layer;
mod ga;
mod interval;
mod weak_hash_set;

use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{broadcast::error::RecvError, mpsc::UnboundedReceiver};

pub use datastructures::*;
pub use db_layer::query_problem;
pub use interval::{Bound, EndBound, Interval, Intervals, StartBound};
pub use weak_hash_set::WeakHashSet;

use crate::{
    gql::context::Context,
    scheduling::{
        db_layer::store_plan,
        ga::{GASettings, milestone_cost, plan_individual, run_ga},
    },
};

pub async fn recalculate_loop(
    app_state: Arc<crate::app_state::AppState>,
    mut manual_rx: UnboundedReceiver<()>,
) {
    // Startup: perform a calculation immediately
    let mut modify_rx = app_state.modify_tx.subscribe();
    let debounce = tokio::time::sleep(Duration::from_secs(0));
    tokio::pin!(debounce);

    // after first run, set finished
    // Note: perform_recalculation sets Finished itself when successful

    loop {
        // wait for either: modification event, manual trigger, or debounce timeout
        tokio::select! {
            biased;
            // manual recalculation takes precedence
            maybe_manual = manual_rx.recv() => {
                if maybe_manual.is_some() {
                    debounce.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(24*3600*7));
                    if let Err(e) = perform_recalculation(&app_state).await {
                        println!("Error recalculating (manual): {}", e);
                    }
                } else {
                    // channel closed, break loop
                    break;
                }
            }
            // modification event: reset debounce timer
            mod_res = modify_rx.recv() => {
                match mod_res {
                    Ok(_sender) => {
                        // mark modified and reset timer
                        app_state.set_state(crate::app_state::CalculationState::Modified);
                        debounce.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(300));
                    }
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => {
                        // continue; treat as modification
                        app_state.set_state(crate::app_state::CalculationState::Modified);
                        debounce.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(300));
                    }
                }
            }
            _ = &mut debounce => {
                // timer fired -> start calculation
                println!("debounce fired");
                debounce.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(24*3600*7));
                if let Err(e) = perform_recalculation(&app_state).await {
                    println!("Error recalculating (debounce): {}", e);
                }
            }
        }
    }
}

async fn perform_recalculation(app_state: &Arc<crate::app_state::AppState>) -> anyhow::Result<()> {
    // build a Context for this calculation
    app_state.set_state(crate::app_state::CalculationState::Calculating);
    let ctx = Context::new(Arc::clone(app_state));
    let settings = GASettings::default();
    match query_problem(&ctx).await {
        Err(err) => {
            println!("Error querying problem: {}", err);
            return Err(err);
        }
        Ok(mut problem) => {
            let individual = run_ga(&mut problem, &settings);
            let task_order =
                individual.tasks.iter().map(|t| t.task.borrow().title.clone()).collect::<Vec<_>>();
            println!("Problem recalculated successfully. Task order: {:?}", &task_order);
            let plan = plan_individual(&problem, &individual);
            let tasks = problem
                .objs
                .tasks
                .iter()
                .map(|t| (t.borrow().db_id, t))
                .collect::<HashMap<i32, _>>();
            println!("Plan:");
            for (tid, assignments) in &plan.assignments {
                let resources: Vec<i32> = assignments.keys().cloned().collect();
                let task = tasks[&tid].borrow();
                println!(" {} ({}): {:?}", task.title, tid, resources);
                println!("    {}", assignments.values().last().unwrap().range);
            }
            println!(" -> Costs:");
            for ms in &problem.objs.milestones {
                let m = ms.borrow();
                let cost = milestone_cost(&problem, &settings, &plan, &m);
                if let Some(fulfilled_milestone) = plan.fulfilled_milestones.get(&m.db_id) {
                    println!(
                        " {} ({}): {} (target:{} finished:{})",
                        m.title, m.db_id, cost, m.schedule_target, fulfilled_milestone.date
                    );
                } else {
                    // milestone not fulfilled: penalize as if finished at calculation_end + (end - start)
                    println!(
                        " {} ({}): {} (target:{} unfinished!)",
                        m.title, m.db_id, cost, m.schedule_target
                    );
                }
            }
            match store_plan(&ctx, &problem, &plan).await {
                Ok(_) => {
                    println!("Stored new plan successfully.");
                }
                Err(err) => {
                    println!("Error storing plan: {}", err);
                }
            }
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
    app_state.set_state(crate::app_state::CalculationState::Finished);
    Ok(())
}
