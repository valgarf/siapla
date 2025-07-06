use chrono::{NaiveDate, NaiveDateTime};

#[derive(Debug, Clone)]
pub struct Task<'a> {
    pub db_id: i32,
    pub title: String,
    pub effort: f64,
    pub requirements: Vec<&'a Requirement<'a>>,
    pub milestones: Vec<&'a Milestone<'a>>,
    pub predecessor: Vec<&'a Task<'a>>,
    pub successor: Vec<&'a Task<'a>>,
    pub constraints: Vec<&'a ResourceConstraint<'a>>,
}

#[derive(Debug, Clone)]
pub struct Requirement<'a> {
    pub db_id: i32,
    pub title: String,
    pub earliest_start: NaiveDateTime,
    pub tasks: Vec<&'a Task<'a>>,
}

#[derive(Debug, Clone)]
pub struct Milestone<'a> {
    pub db_id: i32,
    pub title: String,
    pub schedule_target: NaiveDateTime,
    pub tasks: Vec<&'a Task<'a>>,
}

pub struct Project<'a> {
    pub start_date: NaiveDate,
    pub tasks: Vec<Task<'a>>,
    pub requirements: Vec<Requirement<'a>>,
    pub milestones: Vec<Milestone<'a>>,
}

#[derive(Debug, Clone)]
pub struct Resource {
    pub db_id: i32,
    pub name: String,
    pub timezone: String,
    pub slots: Vec<Slot>,
}

#[derive(Debug, Clone)]
pub struct Slot {
    pub range: super::Interval<NaiveDateTime>,
    pub extensible: bool,
    pub intervals: super::Intervals<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct ResourceConstraint<'a> {
    // Note: all constraints are currently 'any' constraints
    pub db_id: i32,
    pub task: &'a Task<'a>,
    pub optional: bool,
    pub speed: f64,
    pub constraints: Vec<ResourceConstraintEntry<'a>>,
}

#[derive(Debug, Clone)]
pub struct ResourceConstraintEntry<'a> {
    pub db_id: i32,
    pub resource: &'a Resource,
}
