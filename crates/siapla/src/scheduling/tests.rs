//! Unit tests for the scheduling module.
//!
//! Tests cover: graph operations (remove_groups, reduce_graph),
//! detect_project_issues, plan_individual, milestone_cost,
//! generate_random_individual, and related helpers.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use chrono::{Datelike, NaiveDate, NaiveDateTime, TimeDelta};
use petgraph::Graph;
use petgraph::graph::NodeIndex;
use petgraph::prelude::StableGraph;

use crate::gql::issue::IssueCode;
use crate::scheduling::datastructures::*;
use crate::scheduling::db_layer::{detect_project_issues, reduce_graph, remove_groups};
use crate::scheduling::ga::{
    GASettings, Individual, TaskGene, cost_function, create_random_task_gene,
    generate_random_individual, milestone_cost, plan_individual, plan_task,
};
use crate::scheduling::{Interval, Intervals};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a NaiveDateTime from year/month/day/hour.
fn ndt(year: i32, month: u32, day: u32, hour: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(year, month, day).unwrap().and_hms_opt(hour, 0, 0).unwrap()
}

/// Populate a resource with 8-hour weekday work-slots between `start` and `end`.
fn make_work_slots(resource: &Rc<RefCell<Resource>>, start: NaiveDateTime, end: NaiveDateTime) {
    let mut intervals = Intervals::new();
    let mut day = start;
    while day < end {
        let wd = day.weekday();
        if wd != chrono::Weekday::Sat && wd != chrono::Weekday::Sun {
            intervals
                .insert(Interval::new_lcro(day + TimeDelta::hours(8), day + TimeDelta::hours(16)));
        }
        day += TimeDelta::days(1);
    }
    resource.borrow_mut().slots = vec![Slot {
        range: Interval::new_lcro(start, end),
        extensible: false,
        duration: intervals.length().unwrap(),
        intervals,
    }];
}

/// Create a `Resource` wrapped in `Rc<RefCell<…>>`.
fn make_resource(db_id: i32, name: &str) -> Rc<RefCell<Resource>> {
    Rc::new(RefCell::new(Resource {
        db_id,
        header_id: db_id,
        name: name.to_string(),
        timezone: "UTC".to_string(),
        slots: vec![],
        last_booking_end: None,
    }))
}

/// Create a `Task` (no constraints, no bookings).
fn make_task(db_id: i32, title: &str, effort: f64) -> Rc<RefCell<Task>> {
    Rc::new(RefCell::new(Task {
        parent: None,
        db_id,
        header_id: db_id,
        title: title.to_string(),
        effort,
        constraints: vec![],
        booked_until: None,
        booked_resources: vec![],
        bookings: vec![],
        booked_remaining_effort: 0.0,
        booked_final: false,
    }))
}

/// Create a `Requirement`.
fn make_requirement(
    db_id: i32,
    title: &str,
    earliest_start: NaiveDateTime,
) -> Rc<RefCell<Requirement>> {
    Rc::new(RefCell::new(Requirement {
        db_id,
        header_id: db_id,
        title: title.to_string(),
        earliest_start,
    }))
}

/// Create a `Milestone`.
fn make_milestone(
    db_id: i32,
    title: &str,
    target: NaiveDateTime,
    priority: f64,
) -> Rc<RefCell<Milestone>> {
    Rc::new(RefCell::new(Milestone {
        db_id,
        header_id: db_id,
        title: title.to_string(),
        schedule_target: target,
        priority,
    }))
}

/// Create a `Group`.
fn make_group(db_id: i32) -> Rc<RefCell<Group>> {
    Rc::new(RefCell::new(Group { parent: None, db_id, header_id: db_id, constraints: vec![] }))
}

/// Create a `ResourceConstraint` with a single resource entry.
fn make_constraint(
    db_id: i32,
    resource: &Rc<RefCell<Resource>>,
    speed: f64,
    optional: bool,
) -> ResourceConstraint {
    ResourceConstraint {
        db_id,
        optional,
        speed,
        constraints: vec![ResourceConstraintEntry { db_id, resource: Rc::downgrade(resource) }],
    }
}

/// Build a `Project` directly from its components (no database).
fn make_project(
    start: NaiveDateTime,
    end: NaiveDateTime,
    objs: ProjectObjects,
    g: Graph<Node, ()>,
) -> Project {
    Project { revision: 0, start, calculation_end: end, objs, g, issues: vec![] }
}

/// Convenience: attach a required resource constraint to a task.
fn add_task_constraint(task: &Rc<RefCell<Task>>, resource: &Rc<RefCell<Resource>>, speed: f64) {
    task.borrow_mut().constraints.push(make_constraint(
        resource.borrow().db_id,
        resource,
        speed,
        false,
    ));
}

/// Convenience: attach an optional resource constraint to a task.
fn add_task_optional_constraint(
    task: &Rc<RefCell<Task>>,
    resource: &Rc<RefCell<Resource>>,
    speed: f64,
) {
    task.borrow_mut().constraints.push(make_constraint(
        resource.borrow().db_id,
        resource,
        speed,
        true,
    ));
}

/// Create a simple TaskGene for tests (selectable = first constraint with most entries,
/// required = rest). This mirrors `create_random_task_gene` but deterministically picks
/// the first resource for each constraint.
fn make_task_gene(_project: &Project, task: &Rc<RefCell<Task>>, nidx: NodeIndex) -> TaskGene {
    let borrowed = task.borrow();
    let mut required: HashSet<i32> = HashSet::new();
    let mut selectable: Vec<i32> = Vec::new();
    let mut total_speed = 0.0f64;

    // Find the constraint with the most entries -> selectable
    // Others -> required (pick first entry)
    let mut best_idx: Option<usize> = None;
    let mut best_len = 0;
    for (i, c) in borrowed.constraints.iter().enumerate() {
        if c.constraints.len() > best_len {
            best_len = c.constraints.len();
            best_idx = Some(i);
        }
    }

    for (i, c) in borrowed.constraints.iter().enumerate() {
        if Some(i) == best_idx {
            total_speed += c.speed;
            for entry in &c.constraints {
                let rid = entry.resource.upgrade().expect("resource must exist").borrow().db_id;
                selectable.push(rid);
            }
        } else {
            total_speed += c.speed;
            if let Some(entry) = c.constraints.first() {
                let rid = entry.resource.upgrade().expect("resource must exist").borrow().db_id;
                required.insert(rid);
            }
        }
    }
    if total_speed <= 0.0 {
        total_speed = 1.0;
    }
    TaskGene {
        task: Rc::clone(task),
        task_nidx: nidx,
        required_resource_ids: required,
        selectable_resource_ids: selectable,
        is_booked: false,
        booking_start: None,
        total_speed,
    }
}

// ===========================================================================
// 1. Graph operations
// ===========================================================================

mod graph_ops {
    use super::*;

    /// Req -> T1 -> T2 -> Milestone
    /// After remove_groups + reduce_graph the chain is preserved unchanged.
    #[test]
    fn simple_dependency_chain_preserved() {
        let req = make_requirement(1, "R1", ndt(2025, 1, 1, 0));
        let t1 = make_task(1, "T1", 8.0);
        let t2 = make_task(2, "T2", 8.0);
        let ms = make_milestone(1, "M1", ndt(2025, 3, 1, 0), 1.0);

        let mut g: StableGraph<Node, ()> = StableGraph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_t2 = g.add_node(Node::Task(Rc::clone(&t2)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_t2, ());
        g.add_edge(n_t2, n_ms, ());

        remove_groups(&mut g);

        assert_eq!(g.node_count(), 4, "Chain without groups should keep all 4 nodes");
        assert_eq!(g.edge_count(), 3, "Chain should keep all 3 edges");
        assert!(g.find_edge(n_req, n_t1).is_some(), "Edge Req->T1 must exist");
        assert!(g.find_edge(n_t1, n_t2).is_some(), "Edge T1->T2 must exist");
        assert!(g.find_edge(n_t2, n_ms).is_some(), "Edge T2->Ms must exist");
    }

    /// Req -> Group -> Milestone, Group has children T1, T2.
    /// After remove_groups: Req -> T1, Req -> T2, T1 -> Milestone, T2 -> Milestone.
    #[test]
    fn group_with_children_removed() {
        let req = make_requirement(1, "R1", ndt(2025, 1, 1, 0));
        let t1 = make_task(1, "T1", 8.0);
        let t2 = make_task(2, "T2", 8.0);
        let grp = make_group(10);
        let ms = make_milestone(1, "M1", ndt(2025, 3, 1, 0), 1.0);

        let mut g: StableGraph<Node, ()> = StableGraph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_grp = g.add_node(Node::Group(Rc::clone(&grp)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_t2 = g.add_node(Node::Task(Rc::clone(&t2)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));

        // Req -> Group -> Milestone
        g.add_edge(n_req, n_grp, ());
        g.add_edge(n_grp, n_ms, ());
        // Group -> T1, Group -> T2 (children linked via the group)
        g.add_edge(n_grp, n_t1, ());
        g.add_edge(n_grp, n_t2, ());
        // T1 -> Group (from children back to group, acting as "is part of")
        g.add_edge(n_t1, n_grp, ());
        g.add_edge(n_t2, n_grp, ());

        remove_groups(&mut g);

        // Group node should be removed
        assert!(g.node_weight(n_grp).is_none(), "Group node must be removed after remove_groups");
        // 4 remaining nodes: Req, T1, T2, Milestone
        assert_eq!(
            g.node_count(),
            4,
            "After group removal we should have Req + 2 tasks + Milestone"
        );
    }

    /// Nested groups: Req -> G1 -> G2 -> T1 -> Milestone
    /// After remove_groups both groups should be gone and Req connects to T1.
    #[test]
    fn nested_groups_removed() {
        let req = make_requirement(1, "R1", ndt(2025, 1, 1, 0));
        let g1 = make_group(10);
        let g2 = make_group(11);
        let t1 = make_task(1, "T1", 8.0);
        let ms = make_milestone(1, "M1", ndt(2025, 3, 1, 0), 1.0);

        let mut g: StableGraph<Node, ()> = StableGraph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_g1 = g.add_node(Node::Group(Rc::clone(&g1)));
        let n_g2 = g.add_node(Node::Group(Rc::clone(&g2)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));

        g.add_edge(n_req, n_g1, ());
        g.add_edge(n_g1, n_g2, ());
        g.add_edge(n_g2, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        remove_groups(&mut g);

        assert!(g.node_weight(n_g1).is_none(), "Outer group must be removed");
        assert!(g.node_weight(n_g2).is_none(), "Inner group must be removed");
        assert_eq!(g.node_count(), 3, "Only Req, T1 and Milestone should remain");
        // Req should now reach T1 (possibly via intermediate reconnection)
        assert!(
            g.find_edge(n_req, n_t1).is_some() || {
                // Depending on removal order both groups may have reconnected. The
                // outer group connects Req->G2 and G2 connects Req->T1 after its removal.
                // Either way, the final graph must have Req->T1.
                false
            },
            "Req should connect to T1 after nested group removal"
        );
    }

    /// Transitive reduction removes the redundant edge Req -> T2 when Req -> T1 -> T2 exists.
    #[test]
    fn transitive_reduction_removes_redundant_edge() {
        let req = make_requirement(1, "R1", ndt(2025, 1, 1, 0));
        let t1 = make_task(1, "T1", 8.0);
        let t2 = make_task(2, "T2", 8.0);
        let ms = make_milestone(1, "M1", ndt(2025, 3, 1, 0), 1.0);

        let mut g: Graph<Node, ()> = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_t2 = g.add_node(Node::Task(Rc::clone(&t2)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));

        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_t2, ());
        g.add_edge(n_t2, n_ms, ());
        // redundant edge:
        g.add_edge(n_req, n_t2, ());

        assert_eq!(g.edge_count(), 4, "Before reduction there should be 4 edges");

        reduce_graph(&mut g).expect("reduce_graph should succeed on a DAG");

        assert_eq!(
            g.edge_count(),
            3,
            "After transitive reduction the redundant Req->T2 edge should be removed"
        );
        assert!(g.find_edge(n_req, n_t1).is_some(), "Req->T1 must survive reduction");
        assert!(g.find_edge(n_t1, n_t2).is_some(), "T1->T2 must survive reduction");
        assert!(g.find_edge(n_t2, n_ms).is_some(), "T2->Ms must survive reduction");
        assert!(g.find_edge(n_req, n_t2).is_none(), "Redundant Req->T2 must have been removed");
    }

    /// Transitive reduction with a diamond: Req -> T1, Req -> T2, T1 -> T3, T2 -> T3, T3 -> Ms
    /// and a redundant Req -> T3. After reduction Req -> T3 should be gone.
    #[test]
    fn transitive_reduction_diamond() {
        let req = make_requirement(1, "R1", ndt(2025, 1, 1, 0));
        let t1 = make_task(1, "T1", 8.0);
        let t2 = make_task(2, "T2", 8.0);
        let t3 = make_task(3, "T3", 8.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);

        let mut g: Graph<Node, ()> = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_t2 = g.add_node(Node::Task(Rc::clone(&t2)));
        let n_t3 = g.add_node(Node::Task(Rc::clone(&t3)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));

        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_req, n_t2, ());
        g.add_edge(n_t1, n_t3, ());
        g.add_edge(n_t2, n_t3, ());
        g.add_edge(n_t3, n_ms, ());
        g.add_edge(n_req, n_t3, ()); // redundant

        reduce_graph(&mut g).expect("reduce_graph should succeed");

        assert!(
            g.find_edge(n_req, n_t3).is_none(),
            "Redundant Req->T3 should be removed in diamond reduction"
        );
        // The direct edges should remain
        assert!(g.find_edge(n_req, n_t1).is_some(), "Req->T1 must remain");
        assert!(g.find_edge(n_req, n_t2).is_some(), "Req->T2 must remain");
        assert!(g.find_edge(n_t1, n_t3).is_some(), "T1->T3 must remain");
        assert!(g.find_edge(n_t2, n_t3).is_some(), "T2->T3 must remain");
        assert!(g.find_edge(n_t3, n_ms).is_some(), "T3->Ms must remain");
    }

    /// reduce_graph on an already minimal graph changes nothing.
    #[test]
    fn transitive_reduction_noop_on_minimal_graph() {
        let req = make_requirement(1, "R1", ndt(2025, 1, 1, 0));
        let t1 = make_task(1, "T1", 8.0);
        let ms = make_milestone(1, "M1", ndt(2025, 3, 1, 0), 1.0);

        let mut g: Graph<Node, ()> = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));

        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        reduce_graph(&mut g).expect("should succeed");
        assert_eq!(g.edge_count(), 2, "Minimal graph should keep its 2 edges");
    }
}

// ===========================================================================
// 2. detect_project_issues
// ===========================================================================

mod detect_issues {
    use super::*;

    /// A task with no requirement ancestor gets RequirementMissing.
    #[test]
    fn task_without_requirement_ancestor() {
        let t1 = make_task(1, "T1", 8.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);

        let mut g = Graph::new();
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_t1, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(ndt(2025, 1, 1, 0), ndt(2025, 12, 31, 0), objs, g);
        let issues = detect_project_issues(&project);

        // Should have RequirementMissing globally (no requirements) and per-task
        let task_req_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.code == IssueCode::RequirementMissing && i.task_id == Some(1))
            .collect();
        assert!(
            !task_req_issues.is_empty(),
            "Task without requirement ancestor should produce RequirementMissing issue (task_id=1)"
        );
    }

    /// A task with no milestone successor gets MilestoneMissing.
    #[test]
    fn task_without_milestone_successor() {
        let req = make_requirement(1, "R1", ndt(2025, 1, 1, 0));
        let t1 = make_task(1, "T1", 8.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        g.add_edge(n_req, n_t1, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(ndt(2025, 1, 1, 0), ndt(2025, 12, 31, 0), objs, g);
        let issues = detect_project_issues(&project);

        let task_ms_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.code == IssueCode::MilestoneMissing && i.task_id == Some(1))
            .collect();
        assert!(
            !task_ms_issues.is_empty(),
            "Task without milestone successor should produce MilestoneMissing issue"
        );
    }

    /// A task with no resource constraints gets ResourceMissing.
    #[test]
    fn task_without_resource_constraints() {
        let req = make_requirement(1, "R1", ndt(2025, 1, 1, 0));
        let t1 = make_task(1, "T1", 8.0); // no constraints added
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![],
            groups: vec![],
        };
        let project = make_project(ndt(2025, 1, 1, 0), ndt(2025, 12, 31, 0), objs, g);
        let issues = detect_project_issues(&project);

        let res_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.code == IssueCode::ResourceMissing && i.task_id == Some(1))
            .collect();
        assert!(
            !res_issues.is_empty(),
            "Task without resource constraints should produce ResourceMissing issue"
        );
    }

    /// Project with no requirements at all gets a global RequirementMissing issue.
    #[test]
    fn global_requirement_missing() {
        let t1 = make_task(1, "T1", 8.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);

        let mut g = Graph::new();
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_t1, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(ndt(2025, 1, 1, 0), ndt(2025, 12, 31, 0), objs, g);
        let issues = detect_project_issues(&project);

        let global_req =
            issues.iter().find(|i| i.code == IssueCode::RequirementMissing && i.task_id.is_none());
        assert!(
            global_req.is_some(),
            "Project with no requirements should have a global RequirementMissing issue"
        );
    }

    /// Project with no milestones at all gets a global MilestoneMissing issue.
    #[test]
    fn global_milestone_missing() {
        let req = make_requirement(1, "R1", ndt(2025, 1, 1, 0));
        let t1 = make_task(1, "T1", 8.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        g.add_edge(n_req, n_t1, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(ndt(2025, 1, 1, 0), ndt(2025, 12, 31, 0), objs, g);
        let issues = detect_project_issues(&project);

        let global_ms =
            issues.iter().find(|i| i.code == IssueCode::MilestoneMissing && i.task_id.is_none());
        assert!(
            global_ms.is_some(),
            "Project with no milestones should have a global MilestoneMissing issue"
        );
    }

    /// A well-formed project (Req -> T -> Ms, resource constraint present) has no per-task issues.
    #[test]
    fn well_formed_project_no_task_issues() {
        let req = make_requirement(1, "R1", ndt(2025, 1, 1, 0));
        let t1 = make_task(1, "T1", 8.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(ndt(2025, 1, 1, 0), ndt(2025, 12, 31, 0), objs, g);
        let issues = detect_project_issues(&project);

        let task_issues: Vec<_> = issues.iter().filter(|i| i.task_id.is_some()).collect();
        assert!(
            task_issues.is_empty(),
            "Well-formed project should have no per-task issues, got: {:?}",
            task_issues.iter().map(|i| format!("{:?}", i.code)).collect::<Vec<_>>()
        );
    }

    /// Multiple tasks: one connected properly, one orphaned.
    #[test]
    fn mixed_tasks_some_with_issues() {
        let req = make_requirement(1, "R1", ndt(2025, 1, 1, 0));
        let t1 = make_task(1, "T1", 8.0);
        let t2 = make_task(2, "T2", 8.0); // orphan - no req ancestor, no ms successor
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        add_task_constraint(&t2, &res, 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let _n_t2 = g.add_node(Node::Task(Rc::clone(&t2)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());
        // t2 is not connected to req or ms

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1), Rc::clone(&t2)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(ndt(2025, 1, 1, 0), ndt(2025, 12, 31, 0), objs, g);
        let issues = detect_project_issues(&project);

        // T1 should have no issues, T2 should have RequirementMissing + MilestoneMissing
        let t1_issues: Vec<_> = issues.iter().filter(|i| i.task_id == Some(1)).collect();
        assert!(t1_issues.is_empty(), "T1 is properly connected and should have no issues");

        let t2_req =
            issues.iter().any(|i| i.task_id == Some(2) && i.code == IssueCode::RequirementMissing);
        let t2_ms =
            issues.iter().any(|i| i.task_id == Some(2) && i.code == IssueCode::MilestoneMissing);
        assert!(t2_req, "Orphan T2 should have RequirementMissing");
        assert!(t2_ms, "Orphan T2 should have MilestoneMissing");
    }
}

// ===========================================================================
// 3. plan_individual
// ===========================================================================

mod plan_individual_tests {
    use super::*;

    /// Helper: build a minimal schedulable project and return the project, individual, and node indices.
    fn build_single_task_project() -> (Project, NodeIndex, NodeIndex, NodeIndex) {
        let start = ndt(2025, 1, 6, 0); // Monday
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0); // 1 person-day = 8h
        let ms = make_milestone(1, "M1", ndt(2025, 3, 1, 0), 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        make_work_slots(&res, start, end);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };

        let project = make_project(start, end, objs, g);
        (project, n_req, n_t1, n_ms)
    }

    /// Single task, single resource: the task gets scheduled.
    #[test]
    fn single_task_single_resource() {
        let (project, _n_req, n_t1, _n_ms) = build_single_task_project();

        let t1 = &project.objs.tasks[0];
        let tg = make_task_gene(&project, t1, n_t1);

        let individual =
            Individual { booked_tasks: vec![], tasks: vec![tg], finished_tasks: vec![] };

        let plan = plan_individual(&project, &individual);

        assert!(plan.assignments.contains_key(&1), "Task 1 should be in the plan assignments");
        let assignment = &plan.assignments[&1];
        assert!(assignment.contains_key(&1), "Task 1 should be assigned to resource 1");
        // The slot should have 8 hours of work (1 person-day)
        let slot = &assignment[&1];
        assert_eq!(
            slot.duration,
            TimeDelta::hours(8),
            "Task 1 should be scheduled for 8 hours (1 person-day)"
        );
    }

    /// Two tasks on a single resource: they are scheduled sequentially.
    #[test]
    fn two_tasks_single_resource_sequential() {
        let start = ndt(2025, 1, 6, 0); // Monday
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let t2 = make_task(2, "T2", 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        add_task_constraint(&t2, &res, 1.0);
        make_work_slots(&res, start, end);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_t2 = g.add_node(Node::Task(Rc::clone(&t2)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_req, n_t2, ());
        g.add_edge(n_t1, n_ms, ());
        g.add_edge(n_t2, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1), Rc::clone(&t2)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        let tg1 = make_task_gene(&project, &t1, n_t1);
        let tg2 = make_task_gene(&project, &t2, n_t2);

        let individual =
            Individual { booked_tasks: vec![], tasks: vec![tg1, tg2], finished_tasks: vec![] };

        let plan = plan_individual(&project, &individual);

        assert!(plan.assignments.contains_key(&1), "Task 1 must be scheduled");
        assert!(plan.assignments.contains_key(&2), "Task 2 must be scheduled");

        let end1 = plan.assignments[&1][&1].range.end().value().expect("range must be bounded");
        let start2 = plan.assignments[&2][&1].range.start().value().expect("range must be bounded");

        assert!(
            start2 >= end1,
            "Task 2 (start={}) should begin at or after Task 1 ends ({}) on the same resource",
            start2,
            end1
        );
    }

    /// Two independent tasks on different resources can overlap in time.
    #[test]
    fn two_tasks_two_resources_can_overlap() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let t2 = make_task(2, "T2", 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let res1 = make_resource(1, "Dev1");
        let res2 = make_resource(2, "Dev2");
        add_task_constraint(&t1, &res1, 1.0);
        add_task_constraint(&t2, &res2, 1.0);
        make_work_slots(&res1, start, end);
        make_work_slots(&res2, start, end);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_t2 = g.add_node(Node::Task(Rc::clone(&t2)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_req, n_t2, ());
        g.add_edge(n_t1, n_ms, ());
        g.add_edge(n_t2, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1), Rc::clone(&t2)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res1), Rc::clone(&res2)],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        let tg1 = make_task_gene(&project, &t1, n_t1);
        let tg2 = make_task_gene(&project, &t2, n_t2);

        let individual =
            Individual { booked_tasks: vec![], tasks: vec![tg1, tg2], finished_tasks: vec![] };
        let plan = plan_individual(&project, &individual);

        assert!(
            plan.assignments.contains_key(&1) && plan.assignments.contains_key(&2),
            "Both tasks must be scheduled"
        );

        let range1 = &plan.assignments[&1][&1].range;
        let range2 = &plan.assignments[&2][&2].range;

        // Since they are on different resources and both start from the requirement,
        // they should overlap (start at the same time).
        let start1 = range1.start().value().unwrap();
        let start2 = range2.start().value().unwrap();
        let end1 = range1.end().value().unwrap();
        let end2 = range2.end().value().unwrap();

        let overlap = start1 < end2 && start2 < end1;
        assert!(
            overlap,
            "Two independent tasks on different resources should overlap: T1=[{},{}), T2=[{},{})",
            start1, end1, start2, end2
        );
    }

    /// Dependencies enforced: T2 depends on T1, so T2 starts after T1 finishes.
    #[test]
    fn dependency_enforced() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let t2 = make_task(2, "T2", 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let res1 = make_resource(1, "Dev1");
        let res2 = make_resource(2, "Dev2");
        add_task_constraint(&t1, &res1, 1.0);
        add_task_constraint(&t2, &res2, 1.0);
        make_work_slots(&res1, start, end);
        make_work_slots(&res2, start, end);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_t2 = g.add_node(Node::Task(Rc::clone(&t2)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_t2, ()); // T2 depends on T1
        g.add_edge(n_t2, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1), Rc::clone(&t2)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res1), Rc::clone(&res2)],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        let tg1 = make_task_gene(&project, &t1, n_t1);
        let tg2 = make_task_gene(&project, &t2, n_t2);

        let individual =
            Individual { booked_tasks: vec![], tasks: vec![tg1, tg2], finished_tasks: vec![] };
        let plan = plan_individual(&project, &individual);

        let end1 = plan.assignments[&1][&1].range.end().value().unwrap();
        let start2 = plan.assignments[&2][&2].range.start().value().unwrap();

        assert!(
            start2 >= end1,
            "T2 (start={}) must start at or after T1 ends ({}) due to dependency",
            start2,
            end1
        );
    }

    /// Milestone gets fulfilled when all its predecessor tasks are complete.
    #[test]
    fn milestone_fulfilled_when_predecessors_complete() {
        let (project, _n_req, n_t1, _n_ms) = build_single_task_project();

        let t1 = &project.objs.tasks[0];
        let tg = make_task_gene(&project, t1, n_t1);

        let individual =
            Individual { booked_tasks: vec![], tasks: vec![tg], finished_tasks: vec![] };
        let plan = plan_individual(&project, &individual);

        assert!(
            plan.fulfilled_milestones.contains_key(&1),
            "Milestone 1 should be fulfilled when its predecessor task is scheduled"
        );
        let fm = &plan.fulfilled_milestones[&1];
        assert!(
            fm.date > project.start,
            "Fulfilled milestone date ({}) should be after project start ({})",
            fm.date,
            project.start
        );
    }

    /// Milestone NOT fulfilled when a predecessor cannot be scheduled.
    #[test]
    fn milestone_not_fulfilled_when_predecessor_fails() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        // Task with effort but NO resource constraints → plan_task will fail
        let t1 = make_task(1, "T1", 1.0);
        // Don't add any constraints! The task gene will have empty selectable + required.
        let ms = make_milestone(1, "M1", ndt(2025, 3, 1, 0), 1.0);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        // Build an individual with empty resource ids
        let tg = TaskGene {
            task: Rc::clone(&t1),
            task_nidx: n_t1,
            required_resource_ids: HashSet::new(),
            selectable_resource_ids: vec![],
            is_booked: false,
            booking_start: None,
            total_speed: 1.0,
        };

        let individual =
            Individual { booked_tasks: vec![], tasks: vec![tg], finished_tasks: vec![] };
        let plan = plan_individual(&project, &individual);

        assert!(
            !plan.fulfilled_milestones.contains_key(&1),
            "Milestone should NOT be fulfilled when predecessor task fails to schedule"
        );
        assert!(
            !plan.issues.is_empty(),
            "Plan should contain issues when a task fails to schedule"
        );
    }

    /// Task with optional resource constraint is handled gracefully.
    #[test]
    fn task_with_optional_resource() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let res1 = make_resource(1, "Dev1");
        let res2 = make_resource(2, "Dev2");

        // res1 is required, res2 is optional
        add_task_constraint(&t1, &res1, 1.0);
        add_task_optional_constraint(&t1, &res2, 0.5);
        make_work_slots(&res1, start, end);
        make_work_slots(&res2, start, end);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res1), Rc::clone(&res2)],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        // Build task gene using only the required resource
        let tg = TaskGene {
            task: Rc::clone(&t1),
            task_nidx: n_t1,
            required_resource_ids: HashSet::new(),
            selectable_resource_ids: vec![1], // res1 as selectable
            is_booked: false,
            booking_start: None,
            total_speed: 1.0,
        };

        let individual =
            Individual { booked_tasks: vec![], tasks: vec![tg], finished_tasks: vec![] };
        let plan = plan_individual(&project, &individual);

        assert!(
            plan.assignments.contains_key(&1),
            "Task with optional resource should still be schedulable"
        );
    }

    /// Three tasks in a chain: Req -> T1 -> T2 -> T3 -> Ms, all on different resources.
    /// Verify ordering constraints propagate transitively.
    #[test]
    fn chain_of_three_tasks_ordering() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let t2 = make_task(2, "T2", 1.0);
        let t3 = make_task(3, "T3", 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let r1 = make_resource(1, "Dev1");
        let r2 = make_resource(2, "Dev2");
        let r3 = make_resource(3, "Dev3");
        add_task_constraint(&t1, &r1, 1.0);
        add_task_constraint(&t2, &r2, 1.0);
        add_task_constraint(&t3, &r3, 1.0);
        make_work_slots(&r1, start, end);
        make_work_slots(&r2, start, end);
        make_work_slots(&r3, start, end);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_t2 = g.add_node(Node::Task(Rc::clone(&t2)));
        let n_t3 = g.add_node(Node::Task(Rc::clone(&t3)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_t2, ());
        g.add_edge(n_t2, n_t3, ());
        g.add_edge(n_t3, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1), Rc::clone(&t2), Rc::clone(&t3)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&r1), Rc::clone(&r2), Rc::clone(&r3)],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        let tg1 = make_task_gene(&project, &t1, n_t1);
        let tg2 = make_task_gene(&project, &t2, n_t2);
        let tg3 = make_task_gene(&project, &t3, n_t3);

        let individual =
            Individual { booked_tasks: vec![], tasks: vec![tg1, tg2, tg3], finished_tasks: vec![] };
        let plan = plan_individual(&project, &individual);

        let end1 = plan.assignments[&1][&1].range.end().value().unwrap();
        let start2 = plan.assignments[&2][&2].range.start().value().unwrap();
        let end2 = plan.assignments[&2][&2].range.end().value().unwrap();
        let start3 = plan.assignments[&3][&3].range.start().value().unwrap();

        assert!(start2 >= end1, "T2 start ({}) must be >= T1 end ({})", start2, end1);
        assert!(start3 >= end2, "T3 start ({}) must be >= T2 end ({})", start3, end2);
    }

    /// plan_individual with empty individual produces an empty plan.
    #[test]
    fn empty_individual_produces_empty_plan() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let ms = make_milestone(1, "M1", ndt(2025, 3, 1, 0), 1.0);

        let mut g = Graph::new();
        let _n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let _n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));

        let objs = ProjectObjects {
            tasks: vec![],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        let individual = Individual { booked_tasks: vec![], tasks: vec![], finished_tasks: vec![] };
        let plan = plan_individual(&project, &individual);

        assert!(plan.assignments.is_empty(), "Empty individual should produce empty assignments");
    }
}

// ===========================================================================
// 4. milestone_cost
// ===========================================================================

mod milestone_cost_tests {
    use super::*;

    fn make_settings() -> GASettings {
        GASettings::default()
    }

    fn make_simple_project() -> Project {
        let start = ndt(2025, 1, 1, 0);
        let end = ndt(2025, 12, 31, 0);
        let objs = ProjectObjects::default();
        let g = Graph::new();
        make_project(start, end, objs, g)
    }

    /// Milestone fulfilled before target → negative cost (reward).
    #[test]
    fn before_target_negative_cost() {
        let project = make_simple_project();
        let settings = make_settings();
        let ms = Milestone {
            db_id: 1,
            header_id: 1,
            title: "M1".to_string(),
            schedule_target: ndt(2025, 6, 1, 0),
            priority: 1.0,
        };
        let mut plan = Plan::default();
        plan.fulfilled_milestones.insert(
            1,
            FulfilledMilestone {
                task_id: 1,
                date: ndt(2025, 5, 1, 0), // a month before target
            },
        );

        let cost = milestone_cost(&project, &settings, &plan, &ms);
        assert!(
            cost < 0.0,
            "Finishing before the target should yield a negative cost (reward), got {}",
            cost
        );
    }

    /// Milestone fulfilled after target → positive cost (penalty).
    #[test]
    fn after_target_positive_cost() {
        let project = make_simple_project();
        let settings = make_settings();
        let ms = Milestone {
            db_id: 1,
            header_id: 1,
            title: "M1".to_string(),
            schedule_target: ndt(2025, 6, 1, 0),
            priority: 1.0,
        };
        let mut plan = Plan::default();
        plan.fulfilled_milestones.insert(
            1,
            FulfilledMilestone {
                task_id: 1,
                date: ndt(2025, 7, 1, 0), // a month after target
            },
        );

        let cost = milestone_cost(&project, &settings, &plan, &ms);
        assert!(
            cost > 0.0,
            "Finishing after the target should yield a positive cost (penalty), got {}",
            cost
        );
    }

    /// Milestone fulfilled exactly at target → cost should be zero.
    #[test]
    fn at_target_zero_cost() {
        let project = make_simple_project();
        let settings = make_settings();
        let ms = Milestone {
            db_id: 1,
            header_id: 1,
            title: "M1".to_string(),
            schedule_target: ndt(2025, 6, 1, 0),
            priority: 1.0,
        };
        let mut plan = Plan::default();
        plan.fulfilled_milestones.insert(
            1,
            FulfilledMilestone {
                task_id: 1,
                date: ndt(2025, 6, 1, 0), // exactly at target
            },
        );

        let cost = milestone_cost(&project, &settings, &plan, &ms);
        assert!(
            cost.abs() < 1e-6,
            "Finishing exactly at target should yield ~0 cost, got {}",
            cost
        );
    }

    /// Unfulfilled milestone → high penalty (positive cost).
    #[test]
    fn unfulfilled_milestone_high_penalty() {
        let project = make_simple_project();
        let settings = make_settings();
        let ms = Milestone {
            db_id: 1,
            header_id: 1,
            title: "M1".to_string(),
            schedule_target: ndt(2025, 6, 1, 0),
            priority: 1.0,
        };
        let plan = Plan::default(); // no fulfilled milestones

        let cost = milestone_cost(&project, &settings, &plan, &ms);
        assert!(
            cost > 0.0,
            "Unfulfilled milestone should have a large positive cost, got {}",
            cost
        );
    }

    /// Unfulfilled milestone penalty should be greater than a late fulfillment.
    #[test]
    fn unfulfilled_worse_than_late() {
        let project = make_simple_project();
        let settings = make_settings();
        let ms = Milestone {
            db_id: 1,
            header_id: 1,
            title: "M1".to_string(),
            schedule_target: ndt(2025, 6, 1, 0),
            priority: 1.0,
        };

        // Late fulfillment
        let mut late_plan = Plan::default();
        late_plan.fulfilled_milestones.insert(
            1,
            FulfilledMilestone {
                task_id: 1,
                date: ndt(2025, 8, 1, 0), // 2 months late
            },
        );
        let late_cost = milestone_cost(&project, &settings, &late_plan, &ms);

        // Unfulfilled
        let unfulfilled_plan = Plan::default();
        let unfulfilled_cost = milestone_cost(&project, &settings, &unfulfilled_plan, &ms);

        assert!(
            unfulfilled_cost > late_cost,
            "Unfulfilled cost ({}) should be > late cost ({})",
            unfulfilled_cost,
            late_cost
        );
    }

    /// Higher priority milestone produces larger absolute cost.
    #[test]
    fn higher_priority_larger_cost() {
        let project = make_simple_project();
        let settings = make_settings();

        let ms_low = Milestone {
            db_id: 1,
            header_id: 1,
            title: "Low".to_string(),
            schedule_target: ndt(2025, 6, 1, 0),
            priority: 1.0,
        };
        let ms_high = Milestone {
            db_id: 2,
            header_id: 2,
            title: "High".to_string(),
            schedule_target: ndt(2025, 6, 1, 0),
            priority: 5.0,
        };

        let mut plan = Plan::default();
        plan.fulfilled_milestones
            .insert(1, FulfilledMilestone { task_id: 1, date: ndt(2025, 7, 1, 0) });
        plan.fulfilled_milestones
            .insert(2, FulfilledMilestone { task_id: 2, date: ndt(2025, 7, 1, 0) });

        let cost_low = milestone_cost(&project, &settings, &plan, &ms_low);
        let cost_high = milestone_cost(&project, &settings, &plan, &ms_high);

        assert!(
            cost_high > cost_low,
            "Higher priority ({}) should produce larger cost than lower priority ({})",
            cost_high,
            cost_low
        );
    }

    /// Zero priority milestone always yields zero cost.
    #[test]
    fn zero_priority_zero_cost() {
        let project = make_simple_project();
        let settings = make_settings();
        let ms = Milestone {
            db_id: 1,
            header_id: 1,
            title: "M1".to_string(),
            schedule_target: ndt(2025, 6, 1, 0),
            priority: 0.0,
        };
        let plan = Plan::default(); // unfulfilled

        let cost = milestone_cost(&project, &settings, &plan, &ms);
        assert!(cost.abs() < 1e-6, "Zero-priority milestone should have ~0 cost, got {}", cost);
    }

    /// Larger late delay → larger cost (quadratic penalty).
    #[test]
    fn later_is_more_expensive() {
        let project = make_simple_project();
        let settings = make_settings();
        let ms = Milestone {
            db_id: 1,
            header_id: 1,
            title: "M1".to_string(),
            schedule_target: ndt(2025, 6, 1, 0),
            priority: 1.0,
        };

        let mut plan_30d = Plan::default();
        plan_30d
            .fulfilled_milestones
            .insert(1, FulfilledMilestone { task_id: 1, date: ndt(2025, 7, 1, 0) });

        let mut plan_60d = Plan::default();
        plan_60d
            .fulfilled_milestones
            .insert(1, FulfilledMilestone { task_id: 1, date: ndt(2025, 8, 1, 0) });

        let cost_30 = milestone_cost(&project, &settings, &plan_30d, &ms);
        let cost_60 = milestone_cost(&project, &settings, &plan_60d, &ms);

        assert!(
            cost_60 > cost_30,
            "60-day late cost ({}) should be > 30-day late cost ({})",
            cost_60,
            cost_30
        );
    }
}

// ===========================================================================
// 5. generate_random_individual
// ===========================================================================

mod generate_individual_tests {
    use super::*;

    /// All tasks present in the generated individual.
    #[test]
    fn all_tasks_present() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let t2 = make_task(2, "T2", 1.0);
        let t3 = make_task(3, "T3", 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        add_task_constraint(&t2, &res, 1.0);
        add_task_constraint(&t3, &res, 1.0);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_t2 = g.add_node(Node::Task(Rc::clone(&t2)));
        let n_t3 = g.add_node(Node::Task(Rc::clone(&t3)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_req, n_t2, ());
        g.add_edge(n_req, n_t3, ());
        g.add_edge(n_t1, n_ms, ());
        g.add_edge(n_t2, n_ms, ());
        g.add_edge(n_t3, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1), Rc::clone(&t2), Rc::clone(&t3)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        let individual = generate_random_individual(&project);

        let all_gene_tasks: Vec<i32> = individual
            .booked_tasks
            .iter()
            .chain(individual.tasks.iter())
            .chain(individual.finished_tasks.iter())
            .map(|tg| tg.task.borrow().db_id)
            .collect();

        let expected_ids: HashSet<i32> = [1, 2, 3].into_iter().collect();
        let actual_ids: HashSet<i32> = all_gene_tasks.into_iter().collect();

        assert_eq!(
            actual_ids, expected_ids,
            "Individual should contain all 3 tasks, got ids: {:?}",
            actual_ids
        );
    }

    /// Dependencies are respected: if T2 depends on T1, T1 appears before T2.
    #[test]
    fn dependencies_respected_in_ordering() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let t2 = make_task(2, "T2", 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        add_task_constraint(&t2, &res, 1.0);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_t2 = g.add_node(Node::Task(Rc::clone(&t2)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_t2, ()); // T2 depends on T1
        g.add_edge(n_t2, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1), Rc::clone(&t2)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        // Run multiple times (randomized) to gain confidence
        for i in 0..20 {
            let individual = generate_random_individual(&project);
            let all_tasks: Vec<i32> = individual
                .booked_tasks
                .iter()
                .chain(individual.tasks.iter())
                .chain(individual.finished_tasks.iter())
                .map(|tg| tg.task.borrow().db_id)
                .collect();

            let pos_t1 =
                all_tasks.iter().position(|&id| id == 1).expect("T1 must exist in individual");
            let pos_t2 =
                all_tasks.iter().position(|&id| id == 2).expect("T2 must exist in individual");
            assert!(
                pos_t1 < pos_t2,
                "Iteration {}: T1 (pos {}) must come before T2 (pos {}) due to dependency. Order: {:?}",
                i,
                pos_t1,
                pos_t2,
                all_tasks
            );
        }
    }

    /// Single task project yields individual with exactly one task.
    #[test]
    fn single_task_individual() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        let individual = generate_random_individual(&project);
        let total = individual.booked_tasks.len()
            + individual.tasks.len()
            + individual.finished_tasks.len();
        assert_eq!(total, 1, "Single-task project should produce individual with 1 task gene");
    }

    /// No tasks in the project → empty individual.
    #[test]
    fn no_tasks_empty_individual() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);

        let mut g = Graph::new();
        let _n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let _n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));

        let objs = ProjectObjects {
            tasks: vec![],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        let individual = generate_random_individual(&project);
        let total = individual.booked_tasks.len()
            + individual.tasks.len()
            + individual.finished_tasks.len();
        assert_eq!(total, 0, "Project with no tasks should produce empty individual");
    }

    /// generate_random_individual with a longer chain: Req -> T1 -> T2 -> T3 -> Ms
    #[test]
    fn chain_dependency_order_maintained() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let t2 = make_task(2, "T2", 1.0);
        let t3 = make_task(3, "T3", 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        add_task_constraint(&t2, &res, 1.0);
        add_task_constraint(&t3, &res, 1.0);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_t2 = g.add_node(Node::Task(Rc::clone(&t2)));
        let n_t3 = g.add_node(Node::Task(Rc::clone(&t3)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_t2, ());
        g.add_edge(n_t2, n_t3, ());
        g.add_edge(n_t3, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1), Rc::clone(&t2), Rc::clone(&t3)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        for _ in 0..20 {
            let individual = generate_random_individual(&project);
            let ids: Vec<i32> = individual
                .booked_tasks
                .iter()
                .chain(individual.tasks.iter())
                .chain(individual.finished_tasks.iter())
                .map(|tg| tg.task.borrow().db_id)
                .collect();

            let pos1 = ids.iter().position(|&id| id == 1).unwrap();
            let pos2 = ids.iter().position(|&id| id == 2).unwrap();
            let pos3 = ids.iter().position(|&id| id == 3).unwrap();
            assert!(
                pos1 < pos2 && pos2 < pos3,
                "Chain must be ordered: T1 < T2 < T3, got positions: T1={}, T2={}, T3={}",
                pos1,
                pos2,
                pos3,
            );
        }
    }
}

// ===========================================================================
// 6. cost_function integration
// ===========================================================================

mod cost_function_tests {
    use super::*;

    /// cost_function returns a finite value for a well-formed project.
    #[test]
    fn cost_function_finite() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 12, 31, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        make_work_slots(&res, start, end);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);
        let settings = GASettings::default();

        let tg = make_task_gene(&project, &t1, n_t1);
        let individual =
            Individual { booked_tasks: vec![], tasks: vec![tg], finished_tasks: vec![] };

        let cost = cost_function(&project, &settings, &individual);
        assert!(cost.is_finite(), "cost_function should return a finite value, got {}", cost);
    }

    /// Earlier completion should have lower (or equal) cost than later completion.
    #[test]
    fn earlier_completion_lower_cost() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 12, 31, 0);
        let ms_target = ndt(2025, 9, 1, 0); // target well in the future

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let t2 = make_task(2, "T2", 5.0); // bigger task
        let ms = make_milestone(1, "M1", ms_target, 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        add_task_constraint(&t2, &res, 1.0);
        make_work_slots(&res, start, end);

        // Project with small task (finishes sooner)
        let mut g1 = Graph::new();
        let n_req1 = g1.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g1.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms1 = g1.add_node(Node::Milestone(Rc::clone(&ms)));
        g1.add_edge(n_req1, n_t1, ());
        g1.add_edge(n_t1, n_ms1, ());

        let objs1 = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project1 = make_project(start, end, objs1, g1);
        let settings = GASettings::default();

        let tg1 = make_task_gene(&project1, &t1, n_t1);
        let ind1 = Individual { booked_tasks: vec![], tasks: vec![tg1], finished_tasks: vec![] };
        let cost1 = cost_function(&project1, &settings, &ind1);

        // Re-create slots for second project (they get consumed during planning)
        make_work_slots(&res, start, end);

        // Project with bigger task (finishes later)
        let mut g2 = Graph::new();
        let n_req2 = g2.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t2 = g2.add_node(Node::Task(Rc::clone(&t2)));
        let n_ms2 = g2.add_node(Node::Milestone(Rc::clone(&ms)));
        g2.add_edge(n_req2, n_t2, ());
        g2.add_edge(n_t2, n_ms2, ());

        let objs2 = ProjectObjects {
            tasks: vec![Rc::clone(&t2)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project2 = make_project(start, end, objs2, g2);

        let tg2 = make_task_gene(&project2, &t2, n_t2);
        let ind2 = Individual { booked_tasks: vec![], tasks: vec![tg2], finished_tasks: vec![] };
        let cost2 = cost_function(&project2, &settings, &ind2);

        // Both should finish before the target, so both are negative, but the shorter
        // task finishes earlier (more ahead of schedule) → more negative cost
        assert!(
            cost1 <= cost2,
            "Shorter task cost ({}) should be <= longer task cost ({})",
            cost1,
            cost2
        );
    }
}

// ===========================================================================
// 7. create_random_task_gene
// ===========================================================================

mod create_task_gene_tests {
    use super::*;

    /// TaskGene for a task with one required constraint always picks that resource.
    #[test]
    fn single_required_constraint_picked() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        // Run multiple times due to randomness
        for _ in 0..10 {
            let tg = create_random_task_gene(&project, Rc::clone(&t1), n_t1);
            let all_res: HashSet<i32> = tg
                .required_resource_ids
                .iter()
                .chain(tg.selectable_resource_ids.iter())
                .cloned()
                .collect();
            assert!(
                all_res.contains(&1),
                "TaskGene should include resource 1 (the only required resource)"
            );
        }
    }

    /// TaskGene total_speed reflects constraint speed.
    #[test]
    fn total_speed_matches_constraint() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 2.0); // speed = 2

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        let tg = create_random_task_gene(&project, Rc::clone(&t1), n_t1);
        assert!(
            (tg.total_speed - 2.0).abs() < 1e-6,
            "Total speed should be 2.0 (matching the constraint), got {}",
            tg.total_speed
        );
    }

    /// TaskGene for a task with no constraints has empty resource sets and total_speed = 1.
    #[test]
    fn no_constraints_empty_resources() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0); // no constraints
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        let tg = create_random_task_gene(&project, Rc::clone(&t1), n_t1);
        assert!(
            tg.required_resource_ids.is_empty() && tg.selectable_resource_ids.is_empty(),
            "Task with no constraints should have empty resource sets"
        );
        assert!(
            (tg.total_speed - 1.0).abs() < 1e-6,
            "total_speed should default to 1.0 when no constraints, got {}",
            tg.total_speed
        );
    }
}

// ===========================================================================
// 8. plan_task
// ===========================================================================

mod plan_task_tests {
    use super::*;

    /// plan_task fails with NoEffort for tasks with zero effort.
    #[test]
    fn zero_effort_returns_no_effort_issue() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 0.0); // zero effort!
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        make_work_slots(&res, start, end);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        let project = make_project(
            start,
            end,
            ProjectObjects {
                tasks: vec![Rc::clone(&t1)],
                requirements: vec![Rc::clone(&req)],
                milestones: vec![Rc::clone(&ms)],
                resources: vec![Rc::clone(&res)],
                groups: vec![],
            },
            g,
        );

        let tg = make_task_gene(&project, &t1, n_t1);
        let mut resource_slots: HashMap<i32, Vec<Slot>> = project
            .objs
            .resources
            .iter()
            .map(|r| {
                let rb = r.borrow();
                (rb.db_id, rb.slots.clone())
            })
            .collect();
        let mut g_finished = project.g.map(
            |_, n| match n {
                Node::Task(_) => None,
                Node::Requirement(rc) => Some(rc.borrow().earliest_start),
                Node::Milestone(_) => None,
                Node::Group(_) => None,
            },
            |_, _| (),
        );

        let result = plan_task(&project, &tg, &mut resource_slots, &mut g_finished);
        assert!(result.is_err(), "plan_task should fail for zero-effort task");
        if let Err(Some(issue)) = result {
            assert_eq!(
                issue.code,
                IssueCode::NoEffort,
                "Issue should be NoEffort, got {:?}",
                issue.code
            );
        }
    }

    /// plan_task fails with NoSlotFound when no resources are assigned.
    #[test]
    fn no_resources_returns_no_slot_found() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0); // no constraints
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        let project = make_project(
            start,
            end,
            ProjectObjects {
                tasks: vec![Rc::clone(&t1)],
                requirements: vec![Rc::clone(&req)],
                milestones: vec![Rc::clone(&ms)],
                resources: vec![],
                groups: vec![],
            },
            g,
        );

        let tg = TaskGene {
            task: Rc::clone(&t1),
            task_nidx: n_t1,
            required_resource_ids: HashSet::new(),
            selectable_resource_ids: vec![],
            is_booked: false,
            booking_start: None,
            total_speed: 1.0,
        };
        let mut resource_slots: HashMap<i32, Vec<Slot>> = HashMap::new();
        let mut g_finished = project.g.map(
            |_, n| match n {
                Node::Task(_) => None,
                Node::Requirement(rc) => Some(rc.borrow().earliest_start),
                Node::Milestone(_) => None,
                Node::Group(_) => None,
            },
            |_, _| (),
        );

        let result = plan_task(&project, &tg, &mut resource_slots, &mut g_finished);
        assert!(result.is_err(), "plan_task should fail with no resources");
        if let Err(Some(issue)) = result {
            assert_eq!(
                issue.code,
                IssueCode::NoSlotFound,
                "Issue should be NoSlotFound, got {:?}",
                issue.code
            );
        }
    }

    /// plan_task succeeds for a normal single-resource task.
    #[test]
    fn normal_task_succeeds() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        make_work_slots(&res, start, end);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        let project = make_project(
            start,
            end,
            ProjectObjects {
                tasks: vec![Rc::clone(&t1)],
                requirements: vec![Rc::clone(&req)],
                milestones: vec![Rc::clone(&ms)],
                resources: vec![Rc::clone(&res)],
                groups: vec![],
            },
            g,
        );

        let tg = make_task_gene(&project, &t1, n_t1);
        let mut resource_slots: HashMap<i32, Vec<Slot>> = project
            .objs
            .resources
            .iter()
            .map(|r| {
                let rb = r.borrow();
                (rb.db_id, rb.slots.clone())
            })
            .collect();
        let mut g_finished = project.g.map(
            |_, n| match n {
                Node::Task(_) => None,
                Node::Requirement(rc) => Some(rc.borrow().earliest_start),
                Node::Milestone(_) => None,
                Node::Group(_) => None,
            },
            |_, _| (),
        );

        let result = plan_task(&project, &tg, &mut resource_slots, &mut g_finished);
        assert!(
            result.is_ok(),
            "plan_task should succeed for a normal task, got error: {:?}",
            result.err()
        );
        let assignment = result.unwrap();
        assert!(assignment.contains_key(&1), "Assignment should contain resource 1");

        let slot = &assignment[&1];
        assert_eq!(slot.duration, TimeDelta::hours(8), "Assigned slot should have 8h duration");

        // Verify g_finished was updated
        let finished_time = g_finished.node_weight(n_t1).unwrap();
        assert!(finished_time.is_some(), "g_finished should be updated for the task node");
    }

    /// plan_task with speed=2.0 constraint should use half the calendar time.
    #[test]
    fn speed_affects_duration() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        // 2 person-days effort at speed 2 = 1 effective day
        let t_fast = make_task(10, "T_fast", 2.0);
        let t_normal = make_task(11, "T_normal", 2.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let res_fast = make_resource(1, "DevFast");
        let res_normal = make_resource(2, "DevNormal");
        add_task_constraint(&t_fast, &res_fast, 2.0);
        add_task_constraint(&t_normal, &res_normal, 1.0);
        make_work_slots(&res_fast, start, end);
        make_work_slots(&res_normal, start, end);

        // Fast task project
        let mut g1 = Graph::new();
        let n_req1 = g1.add_node(Node::Requirement(Rc::clone(&req)));
        let n_tf = g1.add_node(Node::Task(Rc::clone(&t_fast)));
        let n_ms1 = g1.add_node(Node::Milestone(Rc::clone(&ms)));
        g1.add_edge(n_req1, n_tf, ());
        g1.add_edge(n_tf, n_ms1, ());

        let project1 = make_project(
            start,
            end,
            ProjectObjects {
                tasks: vec![Rc::clone(&t_fast)],
                requirements: vec![Rc::clone(&req)],
                milestones: vec![Rc::clone(&ms)],
                resources: vec![Rc::clone(&res_fast)],
                groups: vec![],
            },
            g1,
        );

        let tg_fast = make_task_gene(&project1, &t_fast, n_tf);
        let mut rs1: HashMap<i32, Vec<Slot>> = project1
            .objs
            .resources
            .iter()
            .map(|r| (r.borrow().db_id, r.borrow().slots.clone()))
            .collect();
        let mut gf1 = project1.g.map(
            |_, n| match n {
                Node::Task(_) => None,
                Node::Requirement(rc) => Some(rc.borrow().earliest_start),
                _ => None,
            },
            |_, _| (),
        );

        let result_fast = plan_task(&project1, &tg_fast, &mut rs1, &mut gf1).unwrap();
        let fast_end = result_fast[&1].range.end().value().unwrap();

        // Normal task project
        let mut g2 = Graph::new();
        let n_req2 = g2.add_node(Node::Requirement(Rc::clone(&req)));
        let n_tn = g2.add_node(Node::Task(Rc::clone(&t_normal)));
        let n_ms2 = g2.add_node(Node::Milestone(Rc::clone(&ms)));
        g2.add_edge(n_req2, n_tn, ());
        g2.add_edge(n_tn, n_ms2, ());

        let project2 = make_project(
            start,
            end,
            ProjectObjects {
                tasks: vec![Rc::clone(&t_normal)],
                requirements: vec![Rc::clone(&req)],
                milestones: vec![Rc::clone(&ms)],
                resources: vec![Rc::clone(&res_normal)],
                groups: vec![],
            },
            g2,
        );

        let tg_normal = make_task_gene(&project2, &t_normal, n_tn);
        let mut rs2: HashMap<i32, Vec<Slot>> = project2
            .objs
            .resources
            .iter()
            .map(|r| (r.borrow().db_id, r.borrow().slots.clone()))
            .collect();
        let mut gf2 = project2.g.map(
            |_, n| match n {
                Node::Task(_) => None,
                Node::Requirement(rc) => Some(rc.borrow().earliest_start),
                _ => None,
            },
            |_, _| (),
        );

        let result_normal = plan_task(&project2, &tg_normal, &mut rs2, &mut gf2).unwrap();
        let normal_end = result_normal[&2].range.end().value().unwrap();

        assert!(
            fast_end < normal_end,
            "Task at speed=2 should finish earlier ({}) than task at speed=1 ({}) for same effort",
            fast_end,
            normal_end
        );
    }
}

// ===========================================================================
// 9. run_ga (smoke test - fast settings)
// ===========================================================================

mod ga_smoke {
    use super::*;
    use crate::scheduling::ga::run_ga;

    /// Smoke test: run_ga produces a valid individual for a trivial project.
    #[test]
    fn run_ga_trivial_project() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        make_work_slots(&res, start, end);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let mut project = make_project(start, end, objs, g);

        // Use minimal settings for speed
        let settings =
            GASettings { iterations: 5, population: 10, keep_seeds: 3, ..GASettings::default() };

        let best = run_ga(&mut project, &settings);
        let total_tasks = best.booked_tasks.len() + best.tasks.len() + best.finished_tasks.len();
        assert_eq!(total_tasks, 1, "run_ga result should contain exactly 1 task");

        // Verify the plan from the best individual actually schedules the task
        let plan = plan_individual(&project, &best);
        assert!(plan.assignments.contains_key(&1), "Best individual's plan should assign task 1");
        assert!(
            plan.fulfilled_milestones.contains_key(&1),
            "Best individual's plan should fulfill milestone 1"
        );
    }

    /// run_ga with two tasks: the resulting plan should fulfil the milestone.
    #[test]
    fn run_ga_two_tasks_fulfils_milestone() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 12, 31, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let t2 = make_task(2, "T2", 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 9, 1, 0), 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        add_task_constraint(&t2, &res, 1.0);
        make_work_slots(&res, start, end);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_t2 = g.add_node(Node::Task(Rc::clone(&t2)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_req, n_t2, ());
        g.add_edge(n_t1, n_ms, ());
        g.add_edge(n_t2, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1), Rc::clone(&t2)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let mut project = make_project(start, end, objs, g);

        let settings =
            GASettings { iterations: 5, population: 10, keep_seeds: 3, ..GASettings::default() };

        let best = run_ga(&mut project, &settings);
        let plan = plan_individual(&project, &best);
        assert!(
            plan.fulfilled_milestones.contains_key(&1),
            "Milestone should be fulfilled after GA optimization with 2 tasks"
        );
    }
}

// ===========================================================================
// 10. Edge cases and integration
// ===========================================================================

mod edge_cases {
    use super::*;

    /// A task with very large effort still schedules (uses multiple days).
    #[test]
    fn large_effort_schedules_across_days() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 12, 31, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "BigTask", 10.0); // 10 person-days
        let ms = make_milestone(1, "M1", ndt(2025, 12, 1, 0), 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        make_work_slots(&res, start, end);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        let tg = make_task_gene(&project, &t1, n_t1);
        let individual =
            Individual { booked_tasks: vec![], tasks: vec![tg], finished_tasks: vec![] };
        let plan = plan_individual(&project, &individual);

        assert!(plan.assignments.contains_key(&1), "Large-effort task should still be schedulable");
        let slot = &plan.assignments[&1][&1];
        // 10 person-days = 80 hours
        assert_eq!(
            slot.duration,
            TimeDelta::hours(80),
            "Slot duration should be 80h for 10 person-days"
        );
        // Should span multiple calendar days
        let slot_start = slot.range.start().value().unwrap();
        let slot_end = slot.range.end().value().unwrap();
        let calendar_days = (slot_end - slot_start).num_days();
        assert!(
            calendar_days >= 10,
            "10 person-days should span at least 10 calendar days (weekends), got {}",
            calendar_days
        );
    }

    /// Requirement earliest_start is respected: task doesn't start before it.
    #[test]
    fn requirement_earliest_start_respected() {
        let project_start = ndt(2025, 1, 6, 0);
        let earliest = ndt(2025, 2, 3, 0); // Requirement says "not before Feb 3"
        let end = ndt(2025, 6, 30, 0);

        let req = make_requirement(1, "R1", earliest);
        let t1 = make_task(1, "T1", 1.0);
        let ms = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        make_work_slots(&res, project_start, end);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_ms = g.add_node(Node::Milestone(Rc::clone(&ms)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_t1, n_ms, ());

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(project_start, end, objs, g);

        let tg = make_task_gene(&project, &t1, n_t1);
        let individual =
            Individual { booked_tasks: vec![], tasks: vec![tg], finished_tasks: vec![] };
        let plan = plan_individual(&project, &individual);

        let task_start = plan.assignments[&1][&1].range.start().value().unwrap();
        assert!(
            task_start >= earliest,
            "Task should not start ({}) before requirement earliest_start ({})",
            task_start,
            earliest
        );
    }

    /// Multiple milestones: each predecessor set is tracked independently.
    #[test]
    fn multiple_milestones_independent() {
        let start = ndt(2025, 1, 6, 0);
        let end = ndt(2025, 12, 31, 0);

        let req = make_requirement(1, "R1", start);
        let t1 = make_task(1, "T1", 1.0);
        let t2 = make_task(2, "T2", 2.0);
        let ms1 = make_milestone(1, "M1", ndt(2025, 6, 1, 0), 1.0);
        let ms2 = make_milestone(2, "M2", ndt(2025, 9, 1, 0), 1.0);
        let res = make_resource(1, "Dev");
        add_task_constraint(&t1, &res, 1.0);
        add_task_constraint(&t2, &res, 1.0);
        make_work_slots(&res, start, end);

        let mut g = Graph::new();
        let n_req = g.add_node(Node::Requirement(Rc::clone(&req)));
        let n_t1 = g.add_node(Node::Task(Rc::clone(&t1)));
        let n_t2 = g.add_node(Node::Task(Rc::clone(&t2)));
        let n_ms1 = g.add_node(Node::Milestone(Rc::clone(&ms1)));
        let n_ms2 = g.add_node(Node::Milestone(Rc::clone(&ms2)));
        g.add_edge(n_req, n_t1, ());
        g.add_edge(n_req, n_t2, ());
        g.add_edge(n_t1, n_ms1, ()); // M1 only needs T1
        g.add_edge(n_t2, n_ms2, ()); // M2 only needs T2

        let objs = ProjectObjects {
            tasks: vec![Rc::clone(&t1), Rc::clone(&t2)],
            requirements: vec![Rc::clone(&req)],
            milestones: vec![Rc::clone(&ms1), Rc::clone(&ms2)],
            resources: vec![Rc::clone(&res)],
            groups: vec![],
        };
        let project = make_project(start, end, objs, g);

        let tg1 = make_task_gene(&project, &t1, n_t1);
        let tg2 = make_task_gene(&project, &t2, n_t2);
        let individual =
            Individual { booked_tasks: vec![], tasks: vec![tg1, tg2], finished_tasks: vec![] };
        let plan = plan_individual(&project, &individual);

        assert!(plan.fulfilled_milestones.contains_key(&1), "Milestone 1 should be fulfilled");
        assert!(plan.fulfilled_milestones.contains_key(&2), "Milestone 2 should be fulfilled");

        let m1_date = plan.fulfilled_milestones[&1].date;
        let m2_date = plan.fulfilled_milestones[&2].date;
        // M2 depends on T2 which has 2x the effort, so it should be fulfilled later
        // (they share the same resource and T1 is scheduled first)
        assert!(
            m2_date >= m1_date,
            "M2 ({}) should be fulfilled at or after M1 ({}), since T2 has more effort and they share a resource",
            m2_date,
            m1_date
        );
    }
}
