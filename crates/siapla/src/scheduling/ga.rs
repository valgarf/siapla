// anyhow not required here; planner uses PlanningIssue for errors
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

use crate::scheduling::{Interval, Intervals, Plan, PlanningIssue, ResourceConstraint, Slot};

use super::datastructures::{Node, Project, Task};

#[derive(Debug, Clone)]
pub struct TaskGene {
    pub task: Rc<RefCell<Task>>,
    pub task_nidx: NodeIndex,
    pub required_resource_ids: HashSet<i32>,
    pub selectable_resource_ids: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct Individual {
    pub tasks: Vec<TaskGene>,
}

pub fn generate_random_individual(project: &Project) -> Individual {
    // TODO: not all allowed random orders are created with the same probability.
    // Example:
    // Assume we have 3 tasks (T1, T2, T3) and T2 depends on T1.
    // Allowed orders are (with generation probability):
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

    // From the required constraints, pick the one with the most entries and make it
    // selectable (we will choose one of these resources later during planning).
    let mut selectable_resource_ids: Vec<i32> = Vec::new();
    if !req_constraints.is_empty() {
        // find index of constraint with the maximum number of entries
        let max_idx = req_constraints
            .iter()
            .enumerate()
            .max_by_key(|(_, c)| c.constraints.len())
            .map(|(i, _)| i)
            .unwrap();
        // remove the chosen constraint from req_constraints and collect its resource ids
        let chosen = req_constraints.remove(max_idx);
        for entry in chosen.constraints.iter() {
            let rid =
                Weak::upgrade(&entry.resource).expect("resource must still exist").borrow().db_id;
            selectable_resource_ids.push(rid);
        }
    }

    // For the remaining required constraints, pick exactly one resource each as before.
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
    TaskGene { task, task_nidx: nidx, required_resource_ids, selectable_resource_ids }
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
            Node::Group(_) => panic!("Dependency graph should not have groups anymore"),
        },
        |_, _| (),
    );
    for task_gene in &individual.tasks {
        match plan_task(project, task_gene, &mut resource_slots, &mut g_finished) {
            Ok(assignment) => {
                plan.assignments.insert(task_gene.task.borrow().db_id, assignment);
            }
            Err(Some(issue)) => {
                let task = task_gene.task.borrow();
                warn!(
                    "Failed planning task {} (id {}): {}",
                    task.title, task.db_id, issue.description
                );
                plan.issues.push(issue);
            }
            Err(None) => {
                let task = task_gene.task.borrow();
                warn!("Failed planning task {} (id {})", task.title, task.db_id);
            }
        }
    }

    // Calculate fulfilled milestones: for each milestone node, check predecessors. If all
    // predecessors have assignments, the milestone is fulfilled at the maximum end time of
    // predecessor allocations.
    for nidx in project.g.node_indices() {
        if let Node::Milestone(m_rc) = project.g.node_weight(nidx).expect("node must exist") {
            let milestone = m_rc.borrow();
            // collect predecessor task ids
            let pred_task_ids: Vec<i32> = project
                .g
                .neighbors_directed(nidx, Direction::Incoming)
                .filter_map(|pidx| match project.g.node_weight(pidx) {
                    Some(Node::Task(t)) => Some(t.borrow().db_id),
                    _ => None,
                })
                .collect();

            if pred_task_ids.is_empty() {
                continue;
            }

            let mut max_end: Option<NaiveDateTime> = None;
            let mut all_assigned = true;
            for tid in pred_task_ids.iter() {
                if let Some(assign_map) = plan.assignments.get(tid) {
                    // flatten the assign_map into an iterator of end timestamps and take the max
                    let task_max_end = itertools::max(
                        assign_map
                            .values()
                            .map(|slot| slot.range.end().value().expect("no unbound intervals")),
                    );
                    if let Some(end) = task_max_end {
                        max_end = match max_end {
                            None => Some(end),
                            Some(prev) => Some(std::cmp::max(prev, end)),
                        };
                    }
                } else {
                    all_assigned = false;
                    break;
                }
            }

            if all_assigned {
                if let Some(date) = max_end {
                    plan.fulfilled_milestones.push(
                        crate::scheduling::datastructures::FulfilledMilestone {
                            task_id: milestone.db_id,
                            date,
                        },
                    );
                }
            }
        }
    }

    plan
}

#[derive(Clone)]
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
) -> Result<HashMap<i32, Slot>, Option<PlanningIssue>> {
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
        return Err(Some(PlanningIssue {
            code: crate::gql::issue::IssueCode::PredIssue,
            description: "Failed to determine start timestamp.".to_string(),
            task_id: Some(task.db_id),
        }));
    };
    if task.effort <= 0.0 {
        return Err(Some(PlanningIssue {
            code: crate::gql::issue::IssueCode::NoEffort,
            description: "No effort set for this task.".to_string(),
            task_id: Some(task.db_id),
        })); // detected on creation
    }
    let effort = TimeDelta::seconds((task.effort * 8.0 * 3600.0).round() as i64);

    // If there are no selectable resources, just attempt with primary iterators
    let task_selectable = task_gene.selectable_resource_ids.clone();
    if task_selectable.is_empty() {
        return Err(Some(PlanningIssue {
            code: crate::gql::issue::IssueCode::NoSlotFound,
            description: format!("Task has no resource constraint"),
            task_id: Some(task.db_id),
        }));
    }

    // Create primary slot iterators once and for all
    let mut primary_iterators: Vec<_SlotIterator> = res_ids
        .iter()
        .map(|&res_id| {
            _SlotIterator::new(
                res_id,
                resource_slots.get(&res_id).expect("Resource slots must exist"),
                task_start,
            )
        })
        .collect();

    // Create selectable iterators once
    let mut selectable_iterators: Vec<_SlotIterator> = task_selectable
        .iter()
        .map(|&rid| {
            _SlotIterator::new(
                rid,
                resource_slots.get(&rid).expect("Resource slots must exist"),
                task_start,
            )
        })
        .collect();

    // Main loop: advance primary iterators until we find a candidate or run out
    loop {
        // Ensure primary iterators overlap
        if !primary_iterators.is_empty() {
            if let Err(_) = _ensure_overlapping_slots(&mut primary_iterators) {
                return Err(Some(PlanningIssue {
                    code: crate::gql::issue::IssueCode::NoSlotFound,
                    description:
                        "Failed to find overlapping slots for the given resource constraints."
                            .to_string(),
                    task_id: Some(task.db_id),
                }));
            }
        }

        // Compute primary_intervals (either full span or intersection of primary slots)
        let primary_intervals = if primary_iterators.is_empty() {
            let mut tmp = Intervals::new();
            tmp.insert(Interval::new_lcro(task_start, project.calculation_end));
            tmp
        } else {
            match _slot_intervals(project, task_start, &mut primary_iterators) {
                Ok(iv) => iv,
                Err(_) => {
                    return Err(Some(PlanningIssue {
                        code: crate::gql::issue::IssueCode::NoSlotFound,
                        description: "Failed to compute overlapping intervals.".to_string(),
                        task_id: Some(task.db_id),
                    }));
                }
            }
        };

        // Try each selectable iterator in-place (advance them as needed)
        let mut best_candidate: Option<(HashMap<i32, Slot>, HashMap<i32, usize>, NaiveDateTime)> =
            None;
        for sel_iter in selectable_iterators.iter_mut() {
            loop {
                if let Some(sel_slot) = sel_iter.current() {
                    let inter = primary_intervals.intersection(&sel_slot.intervals);
                    if inter.length().unwrap_or_default() >= effort {
                        // feasible candidate: build result map and removals
                        let mut result_map: HashMap<i32, Slot> = HashMap::new();
                        let mut removals: HashMap<i32, usize> = HashMap::new();
                        for pi in primary_iterators.iter() {
                            let idx = pi.current_idx;
                            let slot = pi.current().expect("slot must exist").clone();
                            result_map.insert(pi.resource_id, slot.clone());
                            removals.insert(pi.resource_id, idx);
                        }
                        let sidx = sel_iter.current_idx;
                        let sslot = sel_iter.current().expect("slot must exist").clone();
                        result_map.insert(sel_iter.resource_id, sslot.clone());
                        removals.insert(sel_iter.resource_id, sidx);

                        let assigned_intervals = _reduce_intervals(
                            primary_intervals.intersection(&sslot.intervals),
                            effort,
                        );
                        let hull = assigned_intervals.hull().expect("Cannot be empty");
                        let end_ts = hull.end().value().expect("no unbounded intervals");
                        let assigned_slot = Slot {
                            range: hull,
                            extensible: false,
                            duration: effort,
                            intervals: assigned_intervals,
                        };
                        for rid in result_map.keys().cloned().collect::<Vec<_>>() {
                            result_map.insert(rid, assigned_slot.clone());
                        }

                        if let Some((_, _, best_end)) = &best_candidate {
                            if end_ts < *best_end {
                                best_candidate = Some((result_map, removals, end_ts));
                            }
                        } else {
                            best_candidate = Some((result_map, removals, end_ts));
                        }
                        break;
                    } else {
                        // not enough overlap: advance selectable iterator and try again
                        sel_iter.advance();
                        // stop if selectable no longer overlaps the primary interval
                        let primary_max_end = primary_intervals
                            .clone()
                            .into_iter()
                            .map(|iv| iv.end().value().expect("no unbound intervals"))
                            .max()
                            .unwrap_or(project.calculation_end);
                        if let Some(curr) = sel_iter.current() {
                            let sel_start =
                                curr.range.start().value().expect("no unbound intervals");
                            if sel_start >= primary_max_end {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
        }

        if let Some((result_map, removals, end_ts)) = best_candidate {
            // drop iterators before mutating resource_slots
            drop(primary_iterators);
            drop(selectable_iterators);
            for (res_id, idx) in removals.iter() {
                remove_slot(
                    resource_slots.get_mut(res_id).expect("Resource must exist"),
                    *idx,
                    &result_map.get(res_id).expect("slot must exist"),
                );
            }
            let nw = g_finished.node_weight_mut(task_gene.task_nidx).expect("Node must exist");
            *nw = Some(end_ts);
            return Ok(result_map);
        }

        // no candidate found for current primary positions -> advance earliest primary slot
        if let Err(_) = _advance_earliest_slot(&mut primary_iterators) {
            return Err(Some(PlanningIssue {
                code: crate::gql::issue::IssueCode::NoSlotFound,
                description: "Failed to find overlapping slots for the given resource constraints."
                    .to_string(),
                task_id: Some(task.db_id),
            }));
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
