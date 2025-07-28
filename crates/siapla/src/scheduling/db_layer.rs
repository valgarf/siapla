use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::scheduling::datastructures::*;
use crate::{entity::*, gql::task::TaskDesignation};
use petgraph::Direction::{Incoming, Outgoing};
use petgraph::Graph;
use petgraph::algo::toposort;
use petgraph::algo::tred::{dag_to_toposorted_adjacency_list, dag_transitive_reduction_closure};
use petgraph::dot::{Config, Dot};
use petgraph::graph::NodeIndex;
use petgraph::prelude::StableGraph;
use petgraph::visit::{EdgeRef as _, IntoNodeReferences};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

pub async fn query_problem(db: &DatabaseConnection) -> Result<Project, anyhow::Error> {
    // Query everything: needs to be done first, we cannot haev an await in the rest of the function
    // TODO: only query tasks not marked as 'done'
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
    for t in db_task_map.values() {
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
        if let Some(pid) = t.parent_id {
            if let (Some(&in_nidx), Some(&out_nidx)) = (grp_in_idx.get(&pid), grp_out_idx.get(&pid))
            {
                let nidx = db_to_nidx[&t.id];
                g.add_edge(in_nidx, nidx, ());
                g.add_edge(nidx, out_nidx, ());
            }
            let parent = group_map.get(&pid).expect("Group must exist.");
            if let Some(child) = group_map.get(&t.id) {
                child.borrow_mut().parent = Some(Rc::downgrade(parent));
            } else if let Some(child) = task_map.get(&t.id) {
                child.borrow_mut().parent = Some(Rc::downgrade(parent));
            } else {
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
        let r = resource_map.get(&ce.resource_constraint_id).expect("resource must exist.");
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

    // Query slots?
    // let slot_models = slot::Entity::find().all(db).await?;

    println!("{:?}", Dot::with_config(&g, &[Config::EdgeNoLabel]));

    Ok(Project {
        start_date: chrono::NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
        objs: project_objects,
        g: g,
    })
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

/// Transitive reduction of the graph (removes unnecessary dependencies between taks)
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
