use petgraph::{Direction, graph::NodeIndex};
use rand::{Rng as _, seq::IndexedRandom as _};
use std::{cell::RefCell, collections::HashSet, rc::Rc};

use super::datastructures::{Node, Project, Task};

#[derive(Debug, Clone)]
pub struct TaskGene {
    pub task: Rc<RefCell<Task>>,
    pub task_nidx: NodeIndex,
}

#[derive(Debug, Clone)]
pub struct Individual {
    pub tasks: Vec<TaskGene>,
}

pub fn generate_random_individual(project: &mut Project) -> Individual {
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
    let mut tasks = vec![];
    let mut possible = project.g.externals(Direction::Incoming).collect::<Vec<_>>();
    let mut handled = HashSet::new();
    while !possible.is_empty() {
        let chosen_idx = rng.random_range(..possible.len());
        let nidx = possible.swap_remove(chosen_idx);
        handled.insert(nidx);
        if let Node::Task(task) = project.g.node_weight(nidx).expect("node must exist") {
            tasks.push(TaskGene { task: Rc::clone(task), task_nidx: nidx })
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
    Individual { tasks }
}
