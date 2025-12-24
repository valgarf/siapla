// anyhow not required here; planner uses PlanningIssue for errors
use chrono::{NaiveDateTime, TimeDelta};
use itertools::Itertools;
use petgraph::Direction::Outgoing;
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
    time::Instant,
};
use tracing::warn;

use crate::scheduling::{
    Interval, Intervals, Milestone, Plan, PlanningIssue, ResourceConstraint, Slot,
};

use super::datastructures::{Node, Project, Task};

/// Settings for the genetic algorithm.
pub struct GASettings {
    pub iterations: usize,
    pub population: usize,
    pub keep_seeds: usize,
    /// probabilities for how to generate a new individual from parents/mutations
    /// (mutation_only, crossover_only, both)
    pub prob_mutation_only: f64,
    pub prob_crossover_only: f64,
    pub prob_both: f64,
    /// probability for mutating resources (applied per task)
    pub prob_mutate_resources: f64,
    /// probability for mutating order by swapping two adjacent tasks (per individual)
    pub prob_mutate_order: f64,
    /// probability for creating a crossover point at any given index between tasks
    pub prob_crossover_point: f64,
    /// slopes for cost function: (low, medium, high) before target
    pub cost_before: [f64; 3],
    /// slopes for cost function: (low, medium, high) after target
    pub cost_after: [f64; 3],
}

impl Default for GASettings {
    fn default() -> Self {
        Self {
            iterations: 100,
            population: 100,
            keep_seeds: 10,
            prob_mutation_only: 0.25,
            prob_crossover_only: 0.25,
            prob_both: 0.25,
            prob_mutate_resources: 0.05,
            prob_mutate_order: 0.2,
            prob_crossover_point: 0.3,

            cost_before: [-0.2, -0.4, -0.6],
            cost_after: [0.2, 0.4, 0.6],
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskGene {
    pub task: Rc<RefCell<Task>>,
    pub task_nidx: NodeIndex,
    pub required_resource_ids: HashSet<i32>,
    pub selectable_resource_ids: Vec<i32>,
    // booking metadata: whether this task has bookings and the first booking start
    pub is_booked: bool,
    pub booking_start: Option<NaiveDateTime>,
    // sum of speeds of the constraints used for this gene
    pub total_speed: f64,
}

#[derive(Debug, Clone)]
pub struct Individual {
    // booked tasks (have bookings and may have remaining effort)
    pub booked_tasks: Vec<TaskGene>,
    // unbooked tasks (subject to GA crossover/mutation)
    pub tasks: Vec<TaskGene>,
    // finished tasks (single final booking) - considered done
    pub finished_tasks: Vec<TaskGene>,
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
    // split into booked (non-final), finished (final booking) and remaining tasks
    let mut booked_tasks: Vec<TaskGene> = Vec::new();
    let mut finished_tasks: Vec<TaskGene> = Vec::new();
    let mut other_tasks: Vec<TaskGene> = Vec::new();
    for tg in task_genes.into_iter() {
        if tg.is_booked {
            if tg.task.borrow().booked_final {
                finished_tasks.push(tg);
            } else {
                booked_tasks.push(tg);
            }
        } else {
            other_tasks.push(tg);
        }
    }
    booked_tasks.sort_by_key(|tg| tg.booking_start.clone());
    Individual { booked_tasks, tasks: other_tasks, finished_tasks }
}

pub fn milestone_cost(
    project: &Project,
    settings: &GASettings,
    plan: &Plan,
    milestone: &Milestone,
) -> f64 {
    // since no priority metadata exists, use medium priority index = 1
    let pri_idx = 1usize;
    let day = 3600.0 * 24.0;
    if let Some(fulfilled_milestone) = plan.fulfilled_milestones.get(&milestone.db_id) {
        let diff = fulfilled_milestone.date - milestone.schedule_target;
        let days = diff.as_seconds_f64() / day;
        if days < 0.0 {
            // finished before target -> negative cost
            (days.abs() + 1.0).ln() * settings.cost_before[pri_idx]
        } else {
            // finished after -> positive cost
            (days.powi(2) + days) * settings.cost_after[pri_idx]
        }
    } else {
        // milestone not fulfilled: penalize as if finished at calculation_end + (end - start)
        let diff = project.calculation_end - milestone.schedule_target;
        let project_length = project.calculation_end - project.start;
        let days = (diff.as_seconds_f64() + project_length.as_seconds_f64()) / day;
        (days.powi(2) + days) * settings.cost_after[pri_idx]
    }
}

// helper: compute cost for an individual (lower is better)
pub fn cost_function(project: &Project, settings: &GASettings, ind: &Individual) -> f64 {
    // plan the individual
    let plan = plan_individual(project, ind);
    let mut total_cost = 0.0f64;
    for m in project.objs.milestones.iter() {
        total_cost += milestone_cost(project, settings, &plan, &m.borrow());
    }
    total_cost
}

// c* ln(t +delta) -> c/(t+delta) ->
// c*sqrt(t+delta) -> c * 0.5 *(t+delta)**3/2

/// Run the genetic algorithm and return the best found individual.
///
/// NOTE: The project model currently does not contain per-milestone priority metadata.
/// As a pragmatic default we treat all milestones as 'medium' priority. If you later
/// add priority information to milestones, the cost function here should be adapted to
/// read it and pick the appropriate slope index.
pub fn run_ga(project: &Project, settings: &GASettings) -> Individual {
    let start_time = Instant::now();
    let mut rng = rand::rng();

    let cost_of = |ind: &Individual| -> f64 { cost_function(project, settings, ind) };
    // initial population
    let mut population: Vec<(Individual, f64)> = (0..settings.population)
        .map(|_| {
            let ind = generate_random_individual(project);
            let c = cost_of(&ind);
            (ind, c)
        })
        .collect();

    // ensure keep_seeds is not larger than population
    let keep_seeds = settings.keep_seeds.min(settings.population);

    // iterate
    for _it in 0..settings.iterations {
        // sort by cost ascending
        population.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        println!(
            "iteration: {} | Best: {} | Worst: {}",
            _it,
            population.first().expect("Cannot be empty").1,
            population.last().expect("Cannot be empty").1
        );

        // keep the best individual unchanged
        let best = population.first().expect("must have at least one entry").0.clone();

        // select seeds (best N)
        let seeds: Vec<Individual> =
            population.into_iter().take(keep_seeds).map(|(i, _)| i).collect();

        // prepare next generation, start with seeds copied
        let mut next: Vec<Individual> = vec![];

        // generate rest of population (except the preserved best)
        while next.len() + 1 < settings.population {
            // pick generation mode
            let r: f64 = rng.random();
            let take_mut = r < settings.prob_mutation_only;
            let take_cross = r >= settings.prob_mutation_only
                && r < (settings.prob_mutation_only + settings.prob_crossover_only);
            let take_both = r >= (settings.prob_mutation_only + settings.prob_crossover_only)
                && r < (settings.prob_mutation_only
                    + settings.prob_crossover_only
                    + settings.prob_both);

            // choose parents
            let parent_from_seed = |rng: &mut _| -> Individual {
                if seeds.is_empty() {
                    generate_random_individual(project)
                } else {
                    seeds.choose(rng).unwrap().clone()
                }
            };

            let mut child = if take_cross || take_both {
                // crossover between two parents
                let p1 = parent_from_seed(&mut rng);
                let p2 = parent_from_seed(&mut rng);
                // if either parent has empty task list, fallback
                if p1.tasks.is_empty() || p2.tasks.is_empty() {
                    parent_from_seed(&mut rng)
                } else {
                    // create crossover points between 1..len-1 positions
                    let len = p1.tasks.len().max(p2.tasks.len());
                    let mut points = vec![rng.random_range(..len)];
                    while rng.random::<f64>() < settings.prob_crossover_point {
                        points.push(rng.random_range(..len));
                    }
                    points.sort_unstable();
                    points.dedup();

                    let mut use_first = true;
                    let mut idx1 = 0usize;
                    let mut idx2 = 0usize;
                    let mut child_tasks: Vec<TaskGene> = Vec::with_capacity(len);
                    let mut next_point_iter = points.into_iter();
                    let mut next_point = next_point_iter.next();
                    for pos in 0..len {
                        if let Some(np) = next_point {
                            if pos >= np {
                                use_first = !use_first;
                                next_point = next_point_iter.next();
                            }
                        }
                        let source = if use_first { &p1.tasks } else { &p2.tasks };
                        // take next task from source skipping duplicates
                        let mut taken = false;
                        while (if use_first { idx1 } else { idx2 }) < source.len() {
                            let idx = if use_first { idx1 } else { idx2 };
                            let tg = &source[idx];
                            let tid = tg.task.borrow().db_id;
                            let already = child_tasks.iter().any(|c| c.task.borrow().db_id == tid);
                            if use_first {
                                idx1 += 1
                            } else {
                                idx2 += 1
                            }
                            if !already {
                                child_tasks.push(tg.clone());
                                taken = true;
                                break;
                            }
                        }
                        if !taken {
                            // nothing available from selected source, fill remaining from other
                            // Is this even possible?
                            let other = if use_first { &p2.tasks } else { &p1.tasks };
                            let mut other_idx = 0usize;
                            while other_idx < other.len() {
                                let tg = &other[other_idx];
                                let tid = tg.task.borrow().db_id;
                                let already =
                                    child_tasks.iter().any(|c| c.task.borrow().db_id == tid);
                                if !already {
                                    child_tasks.push(tg.clone());
                                    break;
                                }
                                other_idx += 1;
                            }
                            break;
                        }
                    }

                    Individual {
                        booked_tasks: p1.booked_tasks.clone(),
                        tasks: child_tasks,
                        finished_tasks: p1.finished_tasks.clone(),
                    }
                }
            } else if take_mut {
                // mutation-only: pick a seed and mutate
                parent_from_seed(&mut rng)
            } else {
                // default: random individual
                generate_random_individual(project)
            };

            // apply mutation if requested or mode==both or mode==mutation
            // mutation: with prob_mutate_order perform one adjacent swap (if allowed)
            while rng.random::<f64>() < settings.prob_mutate_order && child.tasks.len() >= 2 {
                let idx = rng.random_range(..(child.tasks.len() - 1));
                let first_n = child.tasks[idx].task_nidx;
                let second_n = child.tasks[idx + 1].task_nidx;
                // only swap if second does NOT depend on previous one (no edge first -> second)
                if project.g.find_edge(first_n, second_n).is_none() {
                    child.tasks.swap(idx, idx + 1);
                }
            }

            // resource mutation: per-task probability
            for t_idx in 0..child.tasks.len() {
                if rng.random::<f64>() < settings.prob_mutate_resources {
                    let tg = &child.tasks[t_idx];
                    // skip mutation for booked tasks (locked resources)
                    if tg.is_booked {
                        continue;
                    }
                    let new_tg =
                        create_random_task_gene(project, Rc::clone(&tg.task), tg.task_nidx);
                    // replace resource-related fields (keep Rc pointers)
                    child.tasks[t_idx].required_resource_ids = new_tg.required_resource_ids;
                    child.tasks[t_idx].selectable_resource_ids = new_tg.selectable_resource_ids;
                }
            }

            next.push(child);
        }

        // evaluate next generation
        population = next
            .into_iter()
            .map(|ind| {
                let c = cost_of(&ind);
                (ind, c)
            })
            .collect();

        // ensure best individual so far is preserved
        population.push((best.clone(), cost_of(&best)));
    }

    // final sort and return best individual
    population.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let end_time = Instant::now();
    println!(
        "final result | Best: {} | Worst: {}",
        population.first().expect("Cannot be empty").1,
        population.last().expect("Cannot be empty").1
    );
    println!("Took {} seconds", (end_time - start_time).as_secs_f64());
    population.first().expect("population must not be empty").0.clone()
}

pub fn create_random_task_gene(
    _project: &Project,
    task: Rc<RefCell<Task>>,
    nidx: NodeIndex,
) -> TaskGene {
    let borrowed_task = task.borrow();
    // constraints are available via borrowed_task.constraints
    let mut rng = rand::rng();
    let mut required_resource_ids: HashSet<i32> = HashSet::new();
    let mut used_constraint_speeds: Vec<f64> = Vec::new();

    // Build required_resource_ids and collect used constraint speeds. Booking handling
    // should be done up-front: collect booked resource ids and if bookings exist
    // prefer to use those resources when they match constraints. We only alter
    // selection logic at the end based on whether bookings were present.
    let mut booked_res_ids: HashSet<i32> = HashSet::new();
    for (_s, _e, ress, _f) in borrowed_task.bookings.iter() {
        for r in ress.iter() {
            booked_res_ids.insert(*r);
        }
    }

    // partition constraints into required and optional
    let mut req_constraints: Vec<&ResourceConstraint> = vec![];
    let mut opt_constraints: Vec<&ResourceConstraint> = vec![];

    // If bookings exist, prefer booked resources that match constraints.
    // Otherwise put the constraint into the required / optional vec.
    for c in borrowed_task.constraints.iter() {
        // try to find a booked resource matching this constraint
        let mut chosen: Option<i32> = None;
        if !booked_res_ids.is_empty() {
            for entry in c.constraints.iter() {
                let rid = Weak::upgrade(&entry.resource)
                    .expect("resource must still exist")
                    .borrow()
                    .db_id;
                if booked_res_ids.contains(&rid) {
                    chosen = Some(rid);
                    break;
                }
            }
        }

        if let Some(rid) = chosen {
            required_resource_ids.insert(rid);
            used_constraint_speeds.push(c.speed);
        } else {
            if c.optional {
                opt_constraints.push(c);
            } else {
                req_constraints.push(c);
            }
        }
    }

    // Optionals: if no bookings exist, randomly pick some optional constraints and add them to the
    // required constraints
    if booked_res_ids.is_empty() && !opt_constraints.is_empty() {
        let num_opt: usize = rng.random_range(..=opt_constraints.len());
        req_constraints.extend(opt_constraints.choose_multiple(&mut rng, num_opt));
    }

    // Determine selectable_resource_ids: pick the largest required constraint
    let mut selectable_resource_ids: Vec<i32> = Vec::new();
    if !req_constraints.is_empty() {
        let max_idx = req_constraints
            .iter()
            .enumerate()
            .max_by_key(|(_, c)| c.constraints.len())
            .map(|(i, _)| i)
            .unwrap();
        let chosen = req_constraints.remove(max_idx);
        used_constraint_speeds.push(chosen.speed);
        for entry in chosen.constraints.iter() {
            let rid =
                Weak::upgrade(&entry.resource).expect("resource must still exist").borrow().db_id;
            selectable_resource_ids.push(rid);
        }
    }

    // choose a resource randomly for the remaining required constraints
    for c in req_constraints {
        let entry = c.constraints.choose(&mut rng).expect("constraint must have an entry");
        let rid = Weak::upgrade(&entry.resource).expect("resource must still exist").borrow().db_id;
        required_resource_ids.insert(rid);
        used_constraint_speeds.push(c.speed);
    }

    // Now finalize booking metadata (at the end as requested)
    let mut is_booked = false;
    let mut booking_start: Option<NaiveDateTime> = None;
    if !borrowed_task.bookings.is_empty() {
        is_booked = true;
        booking_start = Some(borrowed_task.bookings.iter().map(|(s, _, _, _)| *s).min().unwrap());
    }

    let mut total_speed: f64 = used_constraint_speeds.iter().copied().sum();
    if total_speed <= 0.0 {
        total_speed = 1.0;
    }

    TaskGene {
        task: Rc::clone(&task),
        task_nidx: nidx,
        required_resource_ids,
        selectable_resource_ids,
        is_booked,
        booking_start,
        total_speed,
    }
}

pub fn plan_individual(project: &Project, individual: &Individual) -> Plan {
    let mut plan = Plan::default();
    // prepare resource slots (do not truncate by booking here; query_slots already requested per-resource ranges)
    let mut resource_slots = project
        .objs
        .resources
        .iter()
        .map(|r| {
            let rb = r.borrow();
            let slots = rb.slots.clone();
            (rb.db_id, slots)
        })
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
    // add finished tasks (final bookings) to g_finished so successors can start after them
    for ft in &individual.finished_tasks {
        // find the end time of the final booking
        let end_time_opt =
            ft.task.borrow().bookings.iter().find(|(_, _, _, f)| *f).map(|(_, e, _, _)| *e);
        if let Some(end_time) = end_time_opt {
            let nw = g_finished.node_weight_mut(ft.task_nidx).expect("Node must exist");
            *nw = Some(end_time);
        }
    }

    // schedule booked tasks first (non-final), then unbooked tasks
    // Build initial ordered vector preserving booked/task grouping, then
    // produce a dependency-respecting order that prefers the initial order.
    let initial_order: Vec<TaskGene> =
        individual.booked_tasks.iter().chain(individual.tasks.iter()).cloned().collect();

    // build indegree map counting only predecessors that are in our selected set
    let mut indegree: HashMap<NodeIndex, usize> = HashMap::new();
    let selected: HashSet<NodeIndex> = initial_order.iter().map(|tg| tg.task_nidx).collect();
    for tg in initial_order.iter() {
        let count = project
            .g
            .neighbors_directed(tg.task_nidx, Incoming)
            .filter(|pidx| selected.contains(pidx))
            .count();
        indegree.insert(tg.task_nidx, count);
    }

    let mut remaining = initial_order;
    let mut ordered_vec: Vec<TaskGene> = Vec::with_capacity(indegree.len());
    while !remaining.is_empty() {
        // pick the first task in remaining with indegree == 0 to preserve original order
        if let Some(pos) =
            remaining.iter().position(|tg| *indegree.get(&tg.task_nidx).unwrap_or(&0) == 0)
        {
            let tg = remaining.remove(pos);
            // append and decrease indegree of successors within selected set
            for succ in project.g.neighbors_directed(tg.task_nidx, Outgoing) {
                if indegree.contains_key(&succ) {
                    if let Some(v) = indegree.get_mut(&succ) {
                        *v = v.saturating_sub(1);
                    }
                }
            }
            ordered_vec.push(tg);
        } else {
            // cycle or unresolved dependencies among remaining: append them as-is to avoid infinite loop
            ordered_vec.extend(remaining.into_iter());
            break;
        }
    }

    for task_gene in ordered_vec.iter() {
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
            let pred_tasks: Vec<Rc<RefCell<Task>>> = project
                .g
                .neighbors_directed(nidx, Direction::Incoming)
                .filter_map(|pidx| match project.g.node_weight(pidx) {
                    Some(Node::Task(t)) => Some(Rc::clone(t)),
                    _ => None,
                })
                .collect();

            if pred_tasks.is_empty() {
                continue;
            }

            let mut max_end: Option<NaiveDateTime> = None;
            let mut all_assigned = true;
            for t in pred_tasks.iter() {
                let borrowd_task = t.borrow();
                let tid = borrowd_task.db_id;
                let booked_finishes =
                    if borrowd_task.booked_final { borrowd_task.booked_until } else { None };
                drop(borrowd_task);
                if let Some(finished_at) = booked_finishes {
                    max_end = match max_end {
                        None => Some(finished_at),
                        Some(prev) => Some(std::cmp::max(prev, finished_at)),
                    };
                }
                else if let Some(assign_map) = plan.assignments.get(&tid) {
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
                    plan.fulfilled_milestones.insert(
                        milestone.db_id,
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
            description:
                "Failed to determine start timestamp - might be an issue in a predecessor."
                    .to_string(),
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
    // divide effort by total_speed to account for faster/slower constraints
    let effective_hours = task.effort / task_gene.total_speed;
    let effort = TimeDelta::seconds((effective_hours * 8.0 * 3600.0).round() as i64);

    // Determine selectable resources: prefer gene.selectable. If empty, we will
    // try to schedule using primary resources only (no selectable iterator loop).
    let task_selectable = task_gene.selectable_resource_ids.clone();

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

    if task_selectable.is_empty() && primary_iterators.is_empty() {
        return Err(Some(PlanningIssue {
            code: crate::gql::issue::IssueCode::NoSlotFound,
            description: format!("Task has no resource constraint"),
            task_id: Some(task.db_id),
        }));
    }
    // Create selectable iterators once (may be empty)
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

        // Try each selectable iterator in-place (advance them as needed). If there
        // are no selectable iterators, we'll attempt a primary-only candidate below.
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

        // If there are no selectable iterators (and thus no best_candidate so far) try primary-only candidate
        if best_candidate.is_none() && selectable_iterators.is_empty() {
            // We need a contiguous intersection among primary_iterators of length >= effort
            if primary_intervals.length().unwrap_or_default() >= effort {
                let assigned_intervals = _reduce_intervals(primary_intervals.clone(), effort);
                let hull = assigned_intervals.hull().expect("Cannot be empty");
                let end_ts = hull.end().value().expect("no unbounded intervals");
                let assigned_slot = Slot {
                    range: hull,
                    extensible: false,
                    duration: effort,
                    intervals: assigned_intervals,
                };
                let mut result_map: HashMap<i32, Slot> = HashMap::new();
                let mut removals: HashMap<i32, usize> = HashMap::new();
                for pi in primary_iterators.iter() {
                    let idx = pi.current_idx;
                    removals.insert(pi.resource_id, idx);
                    result_map.insert(pi.resource_id, assigned_slot.clone());
                }
                best_candidate = Some((result_map, removals, end_ts));
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
