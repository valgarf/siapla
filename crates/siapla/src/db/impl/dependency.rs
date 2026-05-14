use anyhow::anyhow;
use sea_orm::{ColumnTrait as _, EntityTrait};
use sea_query::IntoCondition as _;

use crate::{db::upsert::Upserter, entity::dependency};

#[derive(Clone, Copy)]
enum TaskDependencyScope {
    BySuccessor,
    ByPredecessor,
}

pub struct TaskDependencyUpserter {
    task_header_id: i32,
    scope: TaskDependencyScope,
}

impl TaskDependencyUpserter {
    pub fn new_for_predecessors(task_header_id: i32) -> Self {
        Self { task_header_id, scope: TaskDependencyScope::BySuccessor }
    }

    pub fn new_for_successors(task_header_id: i32) -> Self {
        Self { task_header_id, scope: TaskDependencyScope::ByPredecessor }
    }
}

impl Upserter for TaskDependencyUpserter {
    type Entity = dependency::Entity;
    type Key = (i32, i32);
    type RelData = ();

    fn existing_condition(
        &self,
        _: &Vec<&<Self::Entity as EntityTrait>::ActiveModel>,
    ) -> sea_orm::Condition {
        match self.scope {
            TaskDependencyScope::BySuccessor => {
                dependency::Column::SuccessorId.eq(self.task_header_id).into_condition()
            }
            TaskDependencyScope::ByPredecessor => {
                dependency::Column::PredecessorId.eq(self.task_header_id).into_condition()
            }
        }
    }

    fn key(&self, model: &dependency::ActiveModel, _: &()) -> anyhow::Result<Self::Key> {
        let predecessor_id = model
            .predecessor_id
            .try_as_ref()
            .copied()
            .ok_or_else(|| anyhow!("Dependency model is missing predecessor_id"))?;
        let successor_id = model
            .successor_id
            .try_as_ref()
            .copied()
            .ok_or_else(|| anyhow!("Dependency model is missing successor_id"))?;
        Ok((predecessor_id, successor_id))
    }

    fn model_equal(
        &self,
        lhs: &<Self::Entity as EntityTrait>::ActiveModel,
        rhs: &<Self::Entity as EntityTrait>::ActiveModel,
    ) -> bool {
        lhs.predecessor_id.try_as_ref() == rhs.predecessor_id.try_as_ref()
            && lhs.successor_id.try_as_ref() == rhs.successor_id.try_as_ref()
    }
}
