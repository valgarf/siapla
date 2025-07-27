use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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
use sea_orm::{DatabaseConnection, EntityTrait};

pub async fn query_problem(db: &DatabaseConnection) -> Result<Project, anyhow::Error> {
    // Query all tasks (including requirements and milestones)
    // TODO: only query tasks not marked as 'done'
    let db_task_models_vec = task::Entity::find().all(db).await?;
    // Query all resources
    let resource_models = resource::Entity::find().all(db).await?;
    // QUery all dependencies
    let db_dependencies = dependency::Entity::find().all(db).await?;

    let db_task_models: HashMap<i32, _> =
        db_task_models_vec.into_iter().map(|t| (t.id, t)).collect();

    // Build all Task, Requirement, Milestone objects
    let mut project_objects = ProjectObjects::default();

    let mut db_to_nidx: HashMap<i32, NodeIndex<u32>> = HashMap::new();
    let mut grp_in_idx: HashMap<i32, NodeIndex<u32>> = HashMap::new();
    let mut grp_out_idx: HashMap<i32, NodeIndex<u32>> = HashMap::new();

    let mut g: StableGraph<Node, ()> = StableGraph::new();

    // Map task models to Task/Requirement/Milestone objects
    for t in db_task_models.values() {
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
                title: t.title.clone(),
                effort: t.effort.unwrap_or(0.0) as f64,
                constraints: Vec::new(), // filled later
            }));
            project_objects.tasks.push(Rc::clone(&new_ref));
            Node::Task(new_ref.into())
        } else if t.designation.as_str() == <&'static str>::from(TaskDesignation::Group) {
            Node::Group()
        } else {
            panic!("Unknown task designation: {:?}", t);
        };

        if matches!(node, Node::Group()) {
            let in_nidx = g.add_node(node.clone());
            let out_nidx = g.add_node(node);
            grp_in_idx.insert(t.id, in_nidx);
            grp_out_idx.insert(t.id, out_nidx);
            g.add_edge(in_nidx, out_nidx, ());
        } else {
            db_to_nidx.insert(t.id, g.add_node(node));
        }
    }

    // add parent links
    for t in db_task_models.values() {
        if let Some(pid) = t.parent_id {
            if let (Some(&in_nidx), Some(&out_nidx)) = (grp_in_idx.get(&pid), grp_out_idx.get(&pid))
            {
                let nidx = db_to_nidx[&t.id];
                g.add_edge(in_nidx, nidx, ());
                g.add_edge(nidx, out_nidx, ());
            }
        }
    }

    // fill edges from dependencies

    for dep in db_dependencies {
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
    project_objects.resources = resource_models
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

    // Query all slots
    // let slot_models = slot::Entity::find().all(db).await?;
    // Query all resource constraints
    // let rc_models = resource_constraint::Entity::find().all(db).await?;
    // // Query all resource constraint entries
    // let rce_models = resource_constraint_entry::Entity::find().all(db).await?;

    println!("{:?}", Dot::with_config(&g, &[Config::EdgeNoLabel]));

    Ok(Project {
        start_date: chrono::NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
        objs: project_objects,
        g: g,
    })
}

pub fn remove_groups(g: &mut StableGraph<Node, ()>) {
    let to_remove: Vec<_> = g
        .node_references()
        .filter_map(|(nidx, n)| if matches!(n, Node::Group()) { Some(nidx) } else { None })
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
