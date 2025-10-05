use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::rc::{Rc, Weak};
use std::str::FromStr;

use crate::gql::context::Context;
use crate::gql::issue::IssueType;
// availability now loaded via Context::load_combined_availability
use crate::scheduling::{Bound, Interval, Intervals, datastructures::*};
use crate::{entity::*, gql::task::TaskDesignation};
use chrono::NaiveDateTime;
use itertools::Itertools;
use petgraph::Direction::{Incoming, Outgoing};
use petgraph::Graph;
use petgraph::algo::toposort;
use petgraph::algo::tred::{dag_to_toposorted_adjacency_list, dag_transitive_reduction_closure};
use petgraph::dot::{Config, Dot};
use petgraph::graph::NodeIndex;
use petgraph::prelude::StableGraph;
use petgraph::visit::{EdgeRef as _, IntoNodeReferences};
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use tokio::task::JoinSet;

pub async fn query_problem(ctx: &Context) -> anyhow::Result<Project> {
    // Query everything: needs to be done first, we cannot haev an await in the rest of the function
    // TODO: only query tasks not marked as 'done'? (earliest start for following tasks?)
    // alternatively: on marking milestones as done, check which tasks (and requirements) can be
    // marked as not relevant anymore?
    let db = ctx.txn().await?;
    let db_task_vec = task::Entity::find().all(db).await?;
    let db_resource_vec = resource::Entity::find().all(db).await?;
    let db_dependencies_vec = dependency::Entity::find().all(db).await?;
    let task_ids = db_task_vec.iter().map(|t| t.id).collect::<Vec<_>>();
    let db_constraints_vec = resource_constraint::Entity::find()
        .filter(resource_constraint::Column::TaskId.is_in(task_ids))
        .all(db)
        .await?;
    let constraint_ids = db_constraints_vec.iter().map(|t| t.id).collect::<Vec<_>>();
    let db_constraint_entries_vec = resource_constraint_entry::Entity::find()
        .filter(resource_constraint_entry::Column::ResourceConstraintId.is_in(constraint_ids))
        .all(db)
        .await?;

    let db_task_map = db_task_vec.into_iter().map(|t| (t.id, t)).collect::<HashMap<i32, _>>();

    // Build all Task, Requirement, and Milestone objects
    let mut project_objects = ProjectObjects::default();

    let mut db_to_nidx: HashMap<i32, NodeIndex<u32>> = HashMap::new();
    let mut grp_in_idx: HashMap<i32, NodeIndex<u32>> = HashMap::new();
    let mut grp_out_idx: HashMap<i32, NodeIndex<u32>> = HashMap::new();

    let mut g: StableGraph<Node, ()> = StableGraph::new(); // task dependency graph

    // Map task models to Task/Requirement/Milestone objects
    for t in db_task_map.values().sorted_by_key(|t| t.id) {
        let node = if t.designation.as_str() == <&'static str>::from(TaskDesignation::Requirement) {
            let new_ref = Rc::new(RefCell::new(Requirement {
                db_id: t.id,
                title: t.title.clone(),
                earliest_start: t.earliest_start.map(|dt| dt.naive_utc()).unwrap_or_default(),
            }));
            project_objects.requirements.push(Rc::clone(&new_ref));
            Node::Requirement(new_ref.into())
        } else if t.designation.as_str() == <&'static str>::from(TaskDesignation::Milestone) {
            let new_ref = Rc::new(RefCell::new(Milestone {
                db_id: t.id,
                title: t.title.clone(),
                schedule_target: t.schedule_target.map(|dt| dt.naive_utc()).unwrap_or_default(),
            }));
            project_objects.milestones.push(Rc::clone(&new_ref));
            Node::Milestone(new_ref.into())
        } else if t.designation.as_str() == <&'static str>::from(TaskDesignation::Task) {
            let new_ref = Rc::new(RefCell::new(Task {
                db_id: t.id,
                parent: None,
                title: t.title.clone(),
                effort: t.effort.unwrap_or(0.0) as f64,
                constraints: Vec::new(), // filled later
            }));
            project_objects.tasks.push(Rc::clone(&new_ref));
            Node::Task(new_ref.into())
        } else if t.designation.as_str() == <&'static str>::from(TaskDesignation::Group) {
            let new_ref = Rc::new(RefCell::new(Group {
                db_id: t.id,
                parent: None,
                constraints: Vec::new(), // filled later
            }));
            project_objects.groups.push(Rc::clone(&new_ref));
            Node::Group(new_ref.into())
        } else {
            panic!("Unknown task designation: {:?}", t);
        };

        if matches!(node, Node::Group(_)) {
            let in_nidx = g.add_node(node.clone());
            let out_nidx = g.add_node(node);
            grp_in_idx.insert(t.id, in_nidx);
            grp_out_idx.insert(t.id, out_nidx);
            println!("Added to group indices: {} (in: {:?}, out: {:?})", t.id, in_nidx, out_nidx);
            g.add_edge(in_nidx, out_nidx, ());
        } else {
            db_to_nidx.insert(t.id, g.add_node(node));
        }
    }

    let group_map = project_objects
        .groups
        .iter()
        .map(|r| (r.borrow().db_id, Rc::clone(r)))
        .collect::<HashMap<i32, _>>();
    let task_map = project_objects
        .tasks
        .iter()
        .map(|t| (t.borrow().db_id, Rc::clone(t)))
        .collect::<HashMap<i32, _>>();

    // add parent links
    for t in db_task_map.values() {
        let designation =
            TaskDesignation::from_str(&t.designation).expect("Must have a valid designation");
        if let Some(pid) = t.parent_id {
            if let (Some(&in_nidx), Some(&out_nidx)) = (grp_in_idx.get(&pid), grp_out_idx.get(&pid))
            {
                if let (Some(&t_in_nidx), Some(&t_out_nidx)) =
                    (grp_in_idx.get(&t.id), grp_out_idx.get(&t.id))
                {
                    g.add_edge(in_nidx, t_in_nidx, ());
                    g.add_edge(t_out_nidx, out_nidx, ());
                } else {
                    let nidx = db_to_nidx[&t.id];
                    if designation != TaskDesignation::Requirement {
                        g.add_edge(in_nidx, nidx, ());
                    }
                    if designation != TaskDesignation::Milestone {
                        g.add_edge(nidx, out_nidx, ());
                    }
                }
            }
            let parent = group_map.get(&pid).expect("Group must exist.");
            if let Some(child) = group_map.get(&t.id) {
                child.borrow_mut().parent = Some(Rc::downgrade(parent));
            } else if let Some(child) = task_map.get(&t.id) {
                child.borrow_mut().parent = Some(Rc::downgrade(parent));
            } else if ![TaskDesignation::Requirement, TaskDesignation::Milestone]
                .contains(&designation)
            {
                panic!("No task or group with id found.");
            }
        }
    }

    // fill edges from dependencies
    for dep in db_dependencies_vec {
        let pre_id = dep.predecessor_id;
        let suc_id = dep.successor_id;
        let pre_nidx =
            db_to_nidx.get(&pre_id).or_else(|| grp_out_idx.get(&pre_id)).expect("Missing id");
        let suc_nidx =
            db_to_nidx.get(&suc_id).or_else(|| grp_in_idx.get(&suc_id)).expect("Missing id");
        g.add_edge(*pre_nidx, *suc_nidx, ());
    }
    remove_groups(&mut g);
    let mut g = Graph::from(g);
    reduce_graph(&mut g)?;

    // add resources
    project_objects.resources = db_resource_vec
        .into_iter()
        .map(|rm| {
            Rc::new(RefCell::new(Resource {
                db_id: rm.id,
                name: rm.name,
                timezone: rm.timezone,
                slots: vec![],
            }))
        })
        .collect();
    // TODO: query last booking's end time for each resource

    // add resource constraints
    let resource_map = project_objects
        .resources
        .iter()
        .map(|r| (r.borrow().db_id, Rc::clone(r)))
        .collect::<HashMap<i32, _>>();

    let mut constraint_map = db_constraints_vec
        .into_iter()
        .map(|c| {
            (
                c.id,
                (
                    ResourceConstraint {
                        db_id: c.id,
                        optional: c.optional,
                        speed: c.speed as f64,
                        constraints: vec![],
                    },
                    c.task_id,
                ),
            )
        })
        .collect::<HashMap<i32, _>>();
    for ce in db_constraint_entries_vec.into_iter() {
        let (c, _) =
            constraint_map.get_mut(&ce.resource_constraint_id).expect("constraint must exist.");
        let r = resource_map.get(&ce.resource_id).expect("resource must exist.");
        let entry = ResourceConstraintEntry { db_id: ce.id, resource: Rc::downgrade(r) };
        c.constraints.push(entry);
    }
    for (c, task_id) in constraint_map.into_values() {
        if let Some(group) = group_map.get(&task_id) {
            group.borrow_mut().constraints.push(c);
        } else if let Some(task) = task_map.get(&task_id) {
            task.borrow_mut().constraints.push(c);
        } else {
            panic!("No task or group with id found.");
        }
    }

    // copy constraints to children:
    for task in project_objects.tasks.iter() {
        let mut task = task.borrow_mut();
        let mut parent = task.parent.clone();
        while task.constraints.is_empty() && parent.is_some() {
            let group_rc = Weak::upgrade(&parent.as_ref().unwrap()).unwrap();
            let group = group_rc.borrow_mut();
            if !group.constraints.is_empty() {
                task.constraints = group.constraints.clone();
                break;
            } else {
                parent = group.parent.clone();
            }
        }
    }

    // estimate calculation range
    let start = project_objects
        .requirements
        .iter()
        .map(|r| r.borrow().earliest_start)
        .min()
        .ok_or(anyhow::anyhow!("No requirement to set a start date."))?;
    let schedule_target = project_objects
        .milestones
        .iter()
        .map(|r| r.borrow().schedule_target)
        .max()
        .ok_or(anyhow::anyhow!("No milestone to estimate an end date."))?;
    let estimated_end = start + (schedule_target - start) * 2;
    // Query slots
    query_slots(ctx, &project_objects.resources, start, estimated_end).await?;
    // let slot_models = slot::Entity::find().all(db).await?;

    let print_g = g.map(|_, n| PrintNodeName(n), |_, _| PrintEdgeEmpty {});
    println!("{}", Dot::with_config(&print_g, &[Config::EdgeNoLabel]));
    let mut project = Project {
        start,
        calculation_end: estimated_end,
        objs: project_objects,
        g: g,
        issues: vec![],
    };
    project.issues = detect_project_issues(&project);

    Ok(project)
}

/// Detect project-level and per-task planning issues from the built/reduced graph and objects.
pub fn detect_project_issues(
    project: &Project,
) -> Vec<crate::scheduling::datastructures::PlanningIssue> {
    let mut issues: Vec<crate::scheduling::datastructures::PlanningIssue> = Vec::new();
    // global checks
    if project.objs.requirements.is_empty() {
        issues.push(crate::scheduling::datastructures::PlanningIssue {
            code: crate::gql::issue::IssueCode::RequirementMissing,
            description: "No requirement found in project".to_string(),
            task_id: None,
        });
    }
    if project.objs.milestones.is_empty() {
        issues.push(crate::scheduling::datastructures::PlanningIssue {
            code: crate::gql::issue::IssueCode::MilestoneMissing,
            description: "No milestone found in project".to_string(),
            task_id: None,
        });
    }

    // per-task checks (requirement ancestors and milestone predecessors)
    let mut has_requirement_ancestor: std::collections::HashSet<i32> =
        std::collections::HashSet::new();
    let mut stack = vec![];
    for nidx in project.g.externals(petgraph::Direction::Incoming) {
        if let Some(crate::scheduling::datastructures::Node::Requirement(_)) =
            project.g.node_weight(nidx)
        {
            for nei in project.g.neighbors_directed(nidx, petgraph::Direction::Outgoing) {
                stack.push(nei);
            }
        }
    }
    while let Some(nidx) = stack.pop() {
        if let Some(crate::scheduling::datastructures::Node::Task(t_rc)) =
            project.g.node_weight(nidx)
        {
            has_requirement_ancestor.insert(t_rc.borrow().db_id);
            for nei in project.g.neighbors_directed(nidx, petgraph::Direction::Outgoing) {
                stack.push(nei);
            }
        }
    }

    let mut is_required_by_milestone: std::collections::HashSet<i32> =
        std::collections::HashSet::new();
    for nidx in project.g.externals(petgraph::Direction::Outgoing) {
        if let Some(crate::scheduling::datastructures::Node::Milestone(_)) =
            project.g.node_weight(nidx)
        {
            for pred in project.g.neighbors_directed(nidx, petgraph::Direction::Incoming) {
                stack.push(pred);
            }
        }
    }
    while let Some(nidx) = stack.pop() {
        if let Some(crate::scheduling::datastructures::Node::Task(t_rc)) =
            project.g.node_weight(nidx)
        {
            is_required_by_milestone.insert(t_rc.borrow().db_id);
            for pred in project.g.neighbors_directed(nidx, petgraph::Direction::Incoming) {
                stack.push(pred);
            }
        }
    }

    for t in project.objs.tasks.iter() {
        let tid = t.borrow().db_id;
        if !has_requirement_ancestor.contains(&tid) {
            issues.push(crate::scheduling::datastructures::PlanningIssue {
                code: crate::gql::issue::IssueCode::RequirementMissing,
                description: format!("Task {} has no requirement ancestor", tid),
                task_id: Some(tid),
            });
        }
        if !is_required_by_milestone.contains(&tid) {
            issues.push(crate::scheduling::datastructures::PlanningIssue {
                code: crate::gql::issue::IssueCode::MilestoneMissing,
                description: format!("Task {} is not a predecessor of any milestone", tid),
                task_id: Some(tid),
            });
        }

        // Resource constraint missing: if a Task has no constraints (inherited or direct)
        if t.borrow().constraints.is_empty() {
            issues.push(crate::scheduling::datastructures::PlanningIssue {
                code: crate::gql::issue::IssueCode::ResourceMissing,
                description: format!("Task {} has no resource constraints", tid),
                task_id: Some(tid),
            });
        }
    }
    issues
}

struct PrintNodeName<'a>(&'a Node);
struct PrintEdgeEmpty;

impl<'a> Display for PrintNodeName<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Node::Task(ref_cell) => write!(f, "{}", ref_cell.borrow().title),
            Node::Requirement(ref_cell) => write!(f, "R|{}", ref_cell.borrow().title),
            Node::Milestone(ref_cell) => write!(f, "M|{}", ref_cell.borrow().title),
            Node::Group(_) => write!(f, "G"),
        }
    }
}

impl Display for PrintEdgeEmpty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

// query_combined_availability and helpers moved to `crate::gql::dataloader`

/// Extend a single `resource`'s slots with the provided `intervals` for the given range.
pub fn add_slot_availability(
    resource: &Rc<RefCell<Resource>>,
    intervals: Intervals<NaiveDateTime>,
    start: NaiveDateTime,
    end: NaiveDateTime,
) -> anyhow::Result<()> {
    let mut res = resource.borrow_mut();
    if let Some(last_slot) = res.slots.last_mut()
        && last_slot.extensible
        && last_slot.range.end().value().expect("Interval cannot be unbounded") >= start
        && last_slot.range.start().value().expect("Interval cannot be unbounded") <= start
    {
        last_slot.intervals = last_slot.intervals.union(&intervals);
        last_slot.range = Interval::new(last_slot.range.start(), Bound::Open(end));
    } else {
        res.slots.push(Slot {
            range: Interval::new_lcro(start, end),
            extensible: true,
            duration: intervals.length().expect("Intervals cannot be unbounded"),
            intervals,
        });
    }
    Ok(())
}

pub async fn query_slots(
    ctx: &Context,
    resources: &Vec<Rc<RefCell<Resource>>>,
    start: NaiveDateTime,
    end: NaiveDateTime,
) -> anyhow::Result<()> {
    // Load availability per-resource in parallel using a JoinSet, then apply in original order.
    let mut set: JoinSet<(usize, anyhow::Result<Intervals<NaiveDateTime>>)> = JoinSet::new();
    for (idx, r) in resources.iter().enumerate() {
        let rid = r.borrow().db_id;
        let fut = ctx.load_combined_availability(rid, start, end);
        set.spawn(async move { (idx, fut.await) });
    }

    while let Some(join_res) = set.join_next().await {
        match join_res {
            Ok((idx, Ok(iv))) => {
                // apply availability immediately for the resource at `idx`
                let r = &resources[idx];
                add_slot_availability(r, iv, start, end)?;
            }
            Ok((_, Err(e))) => return Err(e),
            Err(e) => return Err(anyhow::anyhow!("Join error: {}", e)),
        }
    }
    Ok(())
}

/// Remove groups from graph by directly connecting all incoming and outgoing node for each group
pub fn remove_groups(g: &mut StableGraph<Node, ()>) {
    let to_remove: Vec<_> = g
        .node_references()
        .filter_map(|(nidx, n)| if matches!(n, Node::Group(_)) { Some(nidx) } else { None })
        .collect();
    for nidx in to_remove.into_iter() {
        let mut new_edges: Vec<(NodeIndex, NodeIndex)> = vec![];
        for e_in in g.edges_directed(nidx, Incoming) {
            let in_nidx = e_in.source();
            for e_out in g.edges_directed(nidx, Outgoing) {
                let out_nidx = e_out.target();
                new_edges.push((in_nidx, out_nidx));
            }
        }
        for (idx1, idx2) in new_edges.into_iter() {
            g.add_edge(idx1, idx2, ());
        }
        g.remove_node(nidx);
    }
}

/// Transitive reduction of the graph (removes unnecessary dependencies between tasks)
pub fn reduce_graph(g: &mut Graph<Node, ()>) -> anyhow::Result<()> {
    let gref: &Graph<Node, ()> = g;
    let sorted = toposort(gref, None).map_err(|_cycle| anyhow::anyhow!("Cycle detected"))?;
    let (adj_list, revmap) = dag_to_toposorted_adjacency_list::<_, NodeIndex<u32>>(gref, &sorted);
    let (red, _) = dag_transitive_reduction_closure(&adj_list);
    g.retain_edges(|g, eidx| {
        let (idx1, idx2) = g.edge_endpoints(eidx).expect("Edge should exist");
        red.contains_edge(revmap[idx1.index()], revmap[idx2.index()])
    });
    Ok(())
}

pub async fn store_plan(ctx: &Context, project: &Project, plan: &Plan) -> anyhow::Result<()> {
    let txn = ctx.txn().await?;
    allocated_resource::Entity::delete_many().exec(txn).await?;
    allocation::Entity::delete_many().exec(txn).await?;
    // remove previous planning issues
    crate::entity::issue::Entity::delete_many()
        .filter(
            crate::entity::issue::Column::Type
                .eq(<&'static str>::from(IssueType::PlanningTask))
                .or(crate::entity::issue::Column::Type
                    .eq(<&'static str>::from(IssueType::PlanningGeneral))),
        )
        .exec(txn)
        .await?;
    for (task_id, assignment) in &plan.assignments {
        let range = assignment
            .values()
            .map(|slot| slot.range)
            .fold(None, |acc, el| match acc {
                None => Some(el),
                Some(iv) => Some(iv.union(&el).expect("Should overlap")),
            })
            .expect("At least one range must exist");
        let am = allocation::ActiveModel {
            id: ActiveValue::NotSet,
            task_id: ActiveValue::Set(*task_id),
            start: ActiveValue::Set(range.start().value().expect("No unbound intervals").and_utc()),
            end: ActiveValue::Set(range.end().value().expect("No unbound intervals").and_utc()),
        };
        let db_alloc = am.insert(txn).await?;
        for (res_id, _) in assignment {
            let am = allocated_resource::ActiveModel {
                id: ActiveValue::NotSet,
                allocation_id: ActiveValue::Set(db_alloc.id),
                resource_id: ActiveValue::Set(*res_id),
            };
            am.insert(txn).await?;
        }
    }
    // store fulfilled milestones as allocations (no allocated_resource entries)
    for fm in &plan.fulfilled_milestones {
        let am = allocation::ActiveModel {
            id: ActiveValue::NotSet,
            task_id: ActiveValue::Set(fm.task_id),
            start: ActiveValue::Set(fm.date.and_utc()),
            end: ActiveValue::Set(fm.date.and_utc()),
        };
        am.insert(txn).await?;
    }
    // Persist project-level issues (from query_problem)
    for pi in &project.issues {
        let issue_type_str: String = if pi.task_id.is_some() {
            <&'static str>::from(IssueType::PlanningTask).into()
        } else {
            <&'static str>::from(IssueType::PlanningGeneral).into()
        };
        let issue_am = crate::entity::issue::ActiveModel {
            id: sea_orm::ActiveValue::NotSet,
            code: sea_orm::ActiveValue::Set(pi.code as i32),
            description: sea_orm::ActiveValue::Set(pi.description.clone()),
            r#type: sea_orm::ActiveValue::Set(issue_type_str),
            task_id: sea_orm::ActiveValue::Set(pi.task_id),
        };
        issue_am.insert(txn).await?;
    }

    // Persist planning issues collected in Plan
    for pi in &plan.issues {
        let issue_type_str: String = if pi.task_id.is_some() {
            <&'static str>::from(IssueType::PlanningTask).into()
        } else {
            <&'static str>::from(IssueType::PlanningGeneral).into()
        };
        let issue_am = crate::entity::issue::ActiveModel {
            id: sea_orm::ActiveValue::NotSet,
            code: sea_orm::ActiveValue::Set(pi.code as i32),
            description: sea_orm::ActiveValue::Set(pi.description.clone()),
            r#type: sea_orm::ActiveValue::Set(issue_type_str),
            task_id: sea_orm::ActiveValue::Set(pi.task_id),
        };
        issue_am.insert(txn).await?;
    }
    Ok(())
}
