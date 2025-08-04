use std::cell::RefCell;
use std::cmp::{max, min};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::rc::{Rc, Weak};

use crate::gql::context::Context;
use crate::scheduling::{Bound, Interval, Intervals, datastructures::*};
use crate::{entity::*, gql::task::TaskDesignation};
use anymap::any;
use chrono::{
    DateTime, Datelike, Days, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Utc, Weekday,
};
use chrono_tz::Tz;
use itertools::Itertools;
use petgraph::Direction::{Incoming, Outgoing};
use petgraph::Graph;
use petgraph::algo::toposort;
use petgraph::algo::tred::{dag_to_toposorted_adjacency_list, dag_transitive_reduction_closure};
use petgraph::dot::{Config, Dot};
use petgraph::graph::NodeIndex;
use petgraph::prelude::StableGraph;
use petgraph::visit::{EdgeRef as _, IntoNodeReferences};
use sea_orm::prelude::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder};

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

    Ok(Project { start, calculation_end: estimated_end, objs: project_objects, g: g })
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

struct _AvailabilityIterator {
    pub timezone: Tz,
    pub start: DateTime<Tz>,
    pub end: DateTime<Tz>,
    pub durations: HashMap<Weekday, TimeDelta>,
    pub last_end: Option<DateTime<Tz>>,
}

pub fn string_to_weekday(s: &str) -> anyhow::Result<Weekday> {
    match s {
        "Monday" => Ok(Weekday::Mon),
        "Tuesday" => Ok(Weekday::Tue),
        "Wednesday" => Ok(Weekday::Wed),
        "Thursday" => Ok(Weekday::Thu),
        "Friday" => Ok(Weekday::Fri),
        "Saturday" => Ok(Weekday::Sat),
        "Sunday" => Ok(Weekday::Sun),
        _ => Err(anyhow::anyhow!("Unknown weekday: {}", s)),
    }
}

impl _AvailabilityIterator {
    pub fn new(
        timezone: &str,
        start: NaiveDateTime,
        end: NaiveDateTime,
        availabilities: Vec<&availability::Model>,
    ) -> anyhow::Result<Self> {
        let tz: Tz = timezone.parse()?;
        Ok(Self {
            timezone: tz,
            start: start.and_utc().with_timezone(&tz),
            end: end.and_utc().with_timezone(&tz),
            durations: availabilities
                .into_iter()
                .map(|a| -> anyhow::Result<(Weekday, TimeDelta)> {
                    let mut secs = a.duration * Decimal::new(3600, 0);
                    secs.rescale(0); // rounding to whole seconds
                    Ok((string_to_weekday(&a.weekday)?, TimeDelta::seconds(secs.try_into()?)))
                })
                .collect::<anyhow::Result<_>>()?,
            last_end: None,
        })
    }
}

impl Iterator for _AvailabilityIterator {
    type Item = Interval<NaiveDateTime>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut date =
            self.last_end.map(|e| e + TimeDelta::days(1)).unwrap_or(self.start).date_naive();
        loop {
            if date > self.end.date_naive() {
                self.last_end = Some(self.end);
                return None;
            }
            if let Some(dur) = self.durations.get(&date.weekday()) {
                let secs = min(dur.num_seconds() / 2, 12 * 3600);
                if secs <= 0 {
                    date += TimeDelta::days(1);
                    continue;
                }
                let i_start = max(
                    NaiveDateTime::new(
                        date,
                        NaiveTime::from_num_seconds_from_midnight_opt(12 * 3600 - secs as u32, 0)
                            .unwrap(),
                    )
                    .and_local_timezone(self.timezone.clone())
                    .latest()
                    .expect("Cannot determine availability start"),
                    self.start,
                );
                let i_end = min(
                    NaiveDateTime::new(
                        date,
                        NaiveTime::from_num_seconds_from_midnight_opt(12 * 3600 + secs as u32, 0)
                            .unwrap(),
                    )
                    .and_local_timezone(self.timezone.clone())
                    .earliest()
                    .expect("Cannot determine availability end"),
                    self.end,
                );
                self.last_end = Some(i_end);
                return Some(Interval::new_lcro(
                    i_start.to_utc().naive_local(),
                    i_end.to_utc().naive_local(),
                ));
            } else {
                date += TimeDelta::days(1);
            }
        }
    }
}
pub async fn query_slots(
    ctx: &Context,
    resources: &Vec<Rc<RefCell<Resource>>>,
    start: NaiveDateTime,
    end: NaiveDateTime,
) -> anyhow::Result<()> {
    let resource_ids = resources.iter().map(|r| r.borrow().db_id).collect::<HashSet<_>>();
    let db_availabilities = availability::Entity::find()
        .filter(availability::Column::ResourceId.is_in(resource_ids.clone()))
        .all(ctx.txn().await?)
        .await?;
    let db_vacations = vacation::Entity::find()
        .filter(vacation::Column::ResourceId.is_in(resource_ids))
        .filter(vacation::Column::From.lt(end))
        .filter(vacation::Column::Until.gt(start))
        .order_by(vacation::Column::From, Order::Asc)
        .all(ctx.txn().await?)
        .await?;
    for r in resources {
        let mut res = r.borrow_mut();
        let availability_iter = _AvailabilityIterator::new(
            &res.timezone,
            start,
            end,
            db_availabilities.iter().filter(|a| a.resource_id == res.db_id).collect(),
        )?;
        const CIDX: usize = resource::Column::Id as usize;
        let db_res = ctx
            .load_one_by_col::<resource::Entity, CIDX>(res.db_id)
            .await?
            .expect("Resource must exist");
        let holiday_intervals = match db_res.holiday(ctx).await? {
            Some(h) => h
                .entries(
                    ctx,
                    availability_iter.start.date_naive(),
                    availability_iter.end.date_naive(),
                )
                .await?
                .into_iter()
                .map(|he| {
                    let start = NaiveDateTime::new(
                        he.date,
                        NaiveTime::from_hms_opt(0, 0, 0).expect("Must be a valid time"),
                    )
                    .and_local_timezone(availability_iter.timezone)
                    .earliest()
                    .expect("Cannot determine holidays datetime.")
                    .to_utc()
                    .naive_local();
                    let end = start + TimeDelta::hours(24);
                    Interval::new_lcro(start, end)
                })
                .collect(),
            None => Intervals::new(),
        };
        let vacation_intervals: Intervals<NaiveDateTime> = db_vacations
            .iter()
            .map(|v| Interval::new_lcro(v.from.naive_local(), v.until.naive_local()))
            .collect();
        let availability_intervals: Intervals<NaiveDateTime> = availability_iter.collect();
        let intervals =
            availability_intervals.difference(&vacation_intervals).difference(&holiday_intervals);
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
