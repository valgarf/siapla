use crate::scheduling::datastructures::*;
use crate::{entity::*, gql::task::TaskDesignation};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

pub async fn query_problem(db: &DatabaseConnection) -> Result<Project<'static>, sea_orm::DbErr> {
    // Query all tasks (including requirements and milestones)
    // TODO: only query tasks not marked as 'done'
    let db_task_models_vec = task::Entity::find().all(db).await?;
    let db_task_models: std::collections::HashMap<i32, _> =
        db_task_models_vec.into_iter().map(|t| (t.id, t)).collect();
    // Query all resources
    let resource_models = resource::Entity::find().all(db).await?;
    // Query all slots
    // let slot_models = slot::Entity::find().all(db).await?;
    // Query all resource constraints
    let rc_models = resource_constraint::Entity::find().all(db).await?;
    // Query all resource constraint entries
    let rce_models = resource_constraint_entry::Entity::find().all(db).await?;

    // Build all Task, Requirement, Milestone objects (with empty Vecs for references)
    let mut all_tasks: Vec<Task> = Vec::new();
    let mut all_requirements: Vec<Requirement> = Vec::new();
    let mut all_milestones: Vec<Milestone> = Vec::new();

    // Map task models to Task/Requirement/Milestone objects
    for t in db_task_models.values() {
        match t.designation.as_str() {
            val if val == <&'static str>::from(TaskDesignation::Requirement) => {
                all_requirements.push(Requirement {
                    db_id: t.id,
                    title: t.title.clone(),
                    earliest_start: t.earliest_start.map(|dt| dt.naive_utc()).unwrap_or_default(),
                    tasks: Vec::new(),
                });
            }
            val if val == <&'static str>::from(TaskDesignation::Milestone) => {
                all_milestones.push(Milestone {
                    db_id: t.id,
                    title: t.title.clone(),
                    schedule_target: t.schedule_target.map(|dt| dt.naive_utc()).unwrap_or_default(),
                    tasks: Vec::new(),
                });
            }
            val if val == <&'static str>::from(TaskDesignation::Task) => {
                all_tasks.push(Task {
                    db_id: t.id,
                    title: t.title.clone(),
                    effort: t.effort.unwrap_or(0.0) as f64,
                    requirements: Vec::new(),
                    milestones: Vec::new(),
                    predecessor: Vec::new(),
                    successor: Vec::new(),
                    constraints: Vec::new(),
                });
            }
            val => panic!("Unknown task designation: {}", val),
        }
    }

    // Build lookup maps for id -> &T
    use std::collections::HashMap;
    let mut task_map: HashMap<i32, &Task> = HashMap::new();
    let mut req_map: HashMap<i32, &Requirement> = HashMap::new();
    let mut ms_map: HashMap<i32, &Milestone> = HashMap::new();
    for t in &all_tasks {
        task_map.insert(t.db_id, t);
    }
    for r in &all_requirements {
        req_map.insert(r.db_id, r);
    }
    for m in &all_milestones {
        ms_map.insert(m.db_id, m);
    }

    // Now fill references

    Ok(Project {
        start_date: chrono::NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
        tasks: all_tasks,
        requirements: all_requirements,
        milestones: all_milestones,
    })
}
