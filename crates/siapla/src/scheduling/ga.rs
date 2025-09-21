use anyhow::anyhow;
use chrono::{NaiveDateTime, TimeDelta};
use itertools::Itertools;
use petgraph::{
    Direction::{self, Incoming},
    Graph,
    graph::NodeIndex,
};
use rand::{Rng as _, seq::IndexedRandom as _};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::{Rc, Weak},
};
use tracing::warn;

use crate::scheduling::{Interval, Intervals, Plan, ResourceConstraint, Slot};

use super::datastructures::{Node, Project, Task};

#[derive(Debug, Clone)]
pub struct TaskGene {
    pub task: Rc<RefCell<Task>>,
    pub task_nidx: NodeIndex,
    pub required_resource_ids: HashSet<i32>,
}

#[derive(Debug, Clone)]
pub struct Individual {
    pub tasks: Vec<TaskGene>,
}

pub fn generate_random_individual(project: &Project) -> Individual {
    // TODO: not all allowed random orders are created with the same probability.
    // Example:
    // Assume we have 3 tasks (T1, T2, T3) and T2 depends on T1.
    // Allowed orders are:
    // - T1,T2,T3 (25%)
    // - T1,T3,T2 (25%)
    // - T3,T1,T2 (50%)
    // For larger examples (with longer task chains), we might miss out on relevant parts of the
    // solution space. These possibilities are also never recovered using crossover and are also
    // unlikely to happen during simple swap mutations.
    let mut rng = rand::rng();
    let mut task_genes = vec![];
    let mut possible = project.g.externals(Direction::Incoming).collect::<Vec<_>>();
    let mut handled = HashSet::new();
    while !possible.is_empty() {
        let chosen_idx = rng.random_range(..possible.len());
        let nidx = possible.swap_remove(chosen_idx);
        handled.insert(nidx);
        if let Node::Task(task) = project.g.node_weight(nidx).expect("node must exist") {
            task_genes.push(create_random_task_gene(project, Rc::clone(&task), nidx))
        }
        for candidate in project.g.neighbors_directed(nidx, Direction::Outgoing) {
            let requirements = project
                .g
                .neighbors_directed(candidate, Direction::Incoming)
                .collect::<HashSet<_>>();
            if handled.is_superset(&requirements) {
                possible.push(candidate);
            }
        }
    }
    Individual { tasks: task_genes }
}

pub fn create_random_task_gene(
    _project: &Project,
    task: Rc<RefCell<Task>>,
    nidx: NodeIndex,
) -> TaskGene {
    let borrowed_task = task.borrow();
    let mut req_constraints: Vec<&ResourceConstraint> =
        borrowed_task.constraints.iter().filter(|c| !c.optional).collect();
    let opt_constraints: Vec<&ResourceConstraint> =
        borrowed_task.constraints.iter().filter(|c| c.optional).collect();
    let mut rng = rand::rng();
    let num_opt: usize = rng.random_range(..=opt_constraints.len());
    req_constraints.extend(opt_constraints.choose_multiple(&mut rng, num_opt));

    let required_resource_ids: HashSet<i32> = req_constraints
        .iter()
        .map(|c| {
            Weak::upgrade(
                &c.constraints.choose(&mut rng).expect("constraint must have an entry").resource,
            )
            .expect("resource must still exist")
            .borrow()
            .db_id
        })
        .collect();
    drop(borrowed_task);
    // for c in task.borrow().constraints
    TaskGene { task, task_nidx: nidx, required_resource_ids }
}

pub fn plan_individual(project: &Project, individual: &Individual) -> Plan {
    let mut plan = Plan::default();
    let mut resource_slots = project
        .objs
        .resources
        .iter()
        .map(|r| (r.borrow().db_id, r.borrow().slots.clone()))
        .collect::<HashMap<i32, _>>();
    let mut g_finished = project.g.map(
        |_, n| match n {
            Node::Task(_) => None,
            Node::Requirement(ref_cell) => Some(ref_cell.borrow().earliest_start),
            Node::Milestone(_) => None,
            Node::Group(_) => panic!("Dependency graph should nothave groups anymore"),
        },
        |_, _| (),
    );
    for task_gene in &individual.tasks {
        match plan_task(project, task_gene, &mut resource_slots, &mut g_finished) {
            Ok(assignment) => {
                plan.assignments.insert(task_gene.task.borrow().db_id, assignment);
            }
            Err(reason) => {
                let task = task_gene.task.borrow();
                warn!("Failed planning task {} (id {}): {}", task.title, task.db_id, reason)
            }
        }
    }
    plan
}

struct _SlotIterator<'a> {
    resource_id: i32,
    slots: &'a Vec<Slot>,
    current_idx: usize,
}

impl<'a> _SlotIterator<'a> {
    fn new(resource_id: i32, slots: &'a Vec<Slot>, start: NaiveDateTime) -> Self {
        let mut result = Self { resource_id, slots, current_idx: 0 };
        result.ensure_start(start);
        result
    }

    fn ensure_start(&mut self, start: NaiveDateTime) {
        while let Some(slot) = self.slots.get(self.current_idx)
            && slot.range.end().value().expect("no unbound intervals") <= start
        {
            self.current_idx += 1
        }
    }

    fn current(&self) -> Option<&Slot> {
        self.slots.get(self.current_idx)
    }

    fn advance(&mut self) {
        self.current_idx += 1;
    }
}

fn _ensure_overlapping_slots(slot_iterators: &mut Vec<_SlotIterator>) -> anyhow::Result<()> {
    let mut min_time = None;
    loop {
        if let Some(min_time) = min_time {
            for si in slot_iterators.iter_mut() {
                si.ensure_start(min_time);
            }
        }
        let new_min_time = slot_iterators
            .iter()
            .map(|si| si.current().map(|s| s.range.start().value().expect("No unbound intervals")))
            .min()
            .flatten();
        if new_min_time.is_none() {
            return Err(anyhow::anyhow!("Failed to find overlapping slots"));
        }
        if new_min_time.is_none() || new_min_time == min_time {
            break;
        }
        min_time = new_min_time;
    }
    Ok(())
}

fn _advance_earliest_slot(slot_iterators: &mut Vec<_SlotIterator>) -> anyhow::Result<()> {
    let si = slot_iterators
        .iter_mut()
        .map(|si| (si.current().map(|s| s.range.end().value().expect("No unbound intervals")), si))
        .min_by_key(|(e, _)| *e)
        .map(|(e, si)| if e.is_none() { None } else { Some(si) })
        .flatten();
    if let Some(si) = si {
        si.advance();
        _ensure_overlapping_slots(slot_iterators)?;
    } else {
        return Err(anyhow::anyhow!("Failed to advance slots"));
    }

    Ok(())
}

fn _slot_intervals(
    project: &Project,
    task_start: NaiveDateTime,
    slot_iterators: &mut Vec<_SlotIterator>,
) -> anyhow::Result<Intervals<NaiveDateTime>> {
    let mut result = Intervals::new();
    result.insert(Interval::new_lcro(task_start, project.calculation_end));
    for si in slot_iterators.iter() {
        if let Some(slot) = si.current() {
            result = result.intersection(&slot.intervals);
        } else {
            return Err(anyhow::anyhow!("Failed to combine slot intervals"));
        }
    }
    Ok(result)
}

pub fn plan_task(
    project: &Project,
    task_gene: &TaskGene,
    resource_slots: &mut HashMap<i32, Vec<Slot>>,
    g_finished: &mut Graph<Option<NaiveDateTime>, ()>,
) -> anyhow::Result<HashMap<i32, Slot>> {
    let task = task_gene.task.borrow();
    let res_ids: Vec<_> = task_gene.required_resource_ids.iter().cloned().collect();
    let task_start_opt = match g_finished
        .neighbors_directed(task_gene.task_nidx, Incoming)
        .map(|nidx| g_finished.node_weight(nidx).cloned().flatten())
        .minmax()
    {
        itertools::MinMaxResult::NoElements => Some(project.start), // no requirement or previous tasks
        itertools::MinMaxResult::OneElement(v) => v,
        itertools::MinMaxResult::MinMax(min, max) => {
            if min.is_none() {
                None
            } else {
                max
            }
        }
    };
    let task_start = if let Some(task_start) = task_start_opt {
        task_start
    } else {
        return Err(anyhow!("Failed to determine start timestamp."));
    };
    let mut slot_iterators: Vec<_SlotIterator> = res_ids
        .into_iter()
        .map(|res_id| {
            _SlotIterator::new(
                res_id,
                resource_slots.get(&res_id).expect("Resource slots must exist"),
                task_start,
            )
        })
        .collect();
    _ensure_overlapping_slots(&mut slot_iterators)?;
    let effort = TimeDelta::seconds((task.effort * 8.0 * 3600.0).round() as i64);
    // TODO speed? configure days != 8 hours?
    loop {
        let intervals = _slot_intervals(project, task_start, &mut slot_iterators)?;
        if intervals.length().expect("No unbounded intervals") >= effort {
            let mut result: HashMap<i32, Slot> = HashMap::new();
            let assigned_intervals = _reduce_intervals(intervals, effort);
            let slot = Slot {
                range: assigned_intervals.hull().expect("Cannot be empty"),
                extensible: false,
                duration: effort,
                intervals: assigned_intervals,
            };
            let mut removal_indices: HashMap<i32, usize> = HashMap::new();
            for si in &mut slot_iterators {
                result.insert(si.resource_id, slot.clone());
                removal_indices.insert(si.resource_id, si.current_idx);
            }
            drop(slot_iterators);
            for (res_id, idx) in removal_indices {
                remove_slot(
                    resource_slots.get_mut(&res_id).expect("Resource must exist"),
                    idx,
                    &slot,
                );
            }
            let nw = g_finished.node_weight_mut(task_gene.task_nidx).expect("Node must exist");
            *nw = slot.range.end().value();
            return Ok(result);
        } else {
            _advance_earliest_slot(&mut slot_iterators)?;
        }
    }
}

fn remove_slot(slots: &mut Vec<Slot>, idx: usize, slot: &Slot) {
    let orig_slot = slots.get_mut(idx).expect("Index must exist");
    let ranges = orig_slot.range.difference(&slot.range);
    let new_slot = if orig_slot.range.start() == slot.range.start()
        || orig_slot.range.end() == slot.range.end()
    {
        assert!(ranges.len() == 1);
        if orig_slot.range.end() == slot.range.end() {
            orig_slot.extensible = false;
        }
        None
    } else {
        assert!(ranges.len() == 2);
        let new_range = ranges[1];
        let new_intervals = orig_slot.intervals.intersection(&new_range.into());
        orig_slot.extensible = false;
        Some(Slot {
            duration: new_intervals.length().expect("No unbound intervals"),
            extensible: orig_slot.extensible,
            intervals: new_intervals,
            range: new_range,
        })
    };
    orig_slot.range = ranges[0];
    orig_slot.intervals = orig_slot.intervals.intersection(&orig_slot.range.into());
    orig_slot.duration = orig_slot.intervals.length().expect("No unbound intervals");
    if let Some(new_slot) = new_slot {
        slots.insert(idx + 1, new_slot);
    }
}

fn _reduce_intervals(
    intervals: Intervals<NaiveDateTime>,
    mut duration: TimeDelta,
) -> Intervals<NaiveDateTime> {
    let mut result = Intervals::<NaiveDateTime>::new();
    for iv in intervals {
        let iv_length = iv.length().expect("no unbounded intervals");
        if iv_length < duration {
            result.insert(iv);
            duration -= iv_length;
        } else {
            let iv_start = iv.start().value().expect("no unbound intervals");
            result.insert(Interval::new_lcro(iv_start, iv_start + duration));
            duration -= duration;
            break;
        }
    }
    if !duration.is_zero() {
        panic!("Intervals not long enough to reduce!")
    }
    result
}
