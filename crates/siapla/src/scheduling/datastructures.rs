use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use chrono::{NaiveDateTime, TimeDelta};
use petgraph::Graph;

// Project base information

pub struct Project {
    pub start: NaiveDateTime,
    pub calculation_end: NaiveDateTime,
    pub objs: ProjectObjects,
    pub g: Graph<Node, ()>,
    // collected issues discovered at project/query time (code, description, optional task_id)
    pub issues: Vec<PlanningIssue>,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectObjects {
    pub tasks: Vec<Rc<RefCell<Task>>>,
    pub requirements: Vec<Rc<RefCell<Requirement>>>,
    pub milestones: Vec<Rc<RefCell<Milestone>>>,
    pub resources: Vec<Rc<RefCell<Resource>>>,
    pub groups: Vec<Rc<RefCell<Group>>>,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub parent: Option<Weak<RefCell<Group>>>,
    pub db_id: i32,
    pub title: String,
    pub effort: f64,
    pub constraints: Vec<ResourceConstraint>,
    // booking-related metadata (filled during query_problem)
    pub booked_until: Option<NaiveDateTime>,
    pub booked_resources: Vec<i32>,
    pub booked_remaining_effort: f64,
    pub booked_final: bool,
}

#[derive(Debug, Clone)]
pub struct Requirement {
    pub db_id: i32,
    pub title: String,
    pub earliest_start: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct Milestone {
    pub db_id: i32,
    pub title: String,
    pub schedule_target: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct FulfilledMilestone {
    pub task_id: i32,
    pub date: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub parent: Option<Weak<RefCell<Group>>>,
    pub db_id: i32,
    pub constraints: Vec<ResourceConstraint>,
}

#[derive(Debug, Clone)]
pub enum Node {
    Task(Rc<RefCell<Task>>),
    Requirement(Rc<RefCell<Requirement>>),
    Milestone(Rc<RefCell<Milestone>>),
    Group(Rc<RefCell<Group>>),
}

#[derive(Debug, Clone)]
pub struct Resource {
    pub db_id: i32,
    pub name: String,
    pub timezone: String,
    pub slots: Vec<Slot>,
    // last booking end time (if any)
    pub last_booking_end: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct Slot {
    pub range: super::Interval<NaiveDateTime>,
    pub extensible: bool,
    pub duration: TimeDelta,
    pub intervals: super::Intervals<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct ResourceConstraint {
    // Note: all constraints are currently 'any' constraints
    pub db_id: i32,
    pub optional: bool,
    pub speed: f64,
    pub constraints: Vec<ResourceConstraintEntry>,
}

#[derive(Debug, Clone)]
pub struct ResourceConstraintEntry {
    pub db_id: i32,
    pub resource: Weak<RefCell<Resource>>,
}

// ### resulting plan

#[derive(Debug, Default, Clone)]
pub struct Plan {
    pub assignments: HashMap<i32, HashMap<i32, Slot>>, // task_id -> (resource_id -> Slot)
    pub fulfilled_milestones: HashMap<i32, FulfilledMilestone>,
    // collected issues during planning: (code, description, optional task_id)
    pub issues: Vec<PlanningIssue>,
}

#[derive(Debug, Clone)]
pub struct PlanningIssue {
    pub code: crate::gql::issue::IssueCode,
    pub description: String,
    pub task_id: Option<i32>,
}
