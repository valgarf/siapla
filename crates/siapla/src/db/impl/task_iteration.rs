use crate::{
    db::{
        PredecessorTaskIterations, SuccessorTaskIterations,
        context::DbContext,
        dataloader::{ByColBatcher, ByColRevBatcher, LinkBatcher},
        entity::{allocation, issue, task_iteration},
        upsert::Upserter,
    },
    entity::{resource_constraint, task_header},
};
use sea_orm::{ColumnTrait as _, EntityTrait};
use sea_query::IntoCondition as _;

impl task_iteration::Model {
    pub async fn dataloader_predecessors(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Vec<task_iteration::Model>> {
        db.loader(LinkBatcher::<PredecessorTaskIterations>::new(revision))
            .await
            .load(self.header_id.into())
            .await
    }

    pub async fn dataloader_successors(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Vec<task_iteration::Model>> {
        db.loader(LinkBatcher::<SuccessorTaskIterations>::new(revision))
            .await
            .load(self.header_id.into())
            .await
    }

    pub async fn dataloader_children(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Vec<task_iteration::Model>> {
        db.loader(ByColRevBatcher::<task_iteration::Entity>::new(
            task_iteration::Column::ParentId,
            revision,
        ))
        .await
        .load(self.header_id.into())
        .await
    }

    pub async fn dataloader_parent(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Option<task_iteration::Model>> {
        db.loader(ByColRevBatcher::<task_iteration::Entity>::new(
            task_iteration::Column::HeaderId,
            revision,
        ))
        .await
        .load_one(self.parent_id.into())
        .await
    }

    pub async fn dataloader_issues(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Vec<issue::Model>> {
        db.loader(ByColRevBatcher::<issue::Entity> { revision, col: issue::Column::TaskId })
            .await
            .load(self.header_id.into())
            .await
    }

    pub async fn dataloader_allocations(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Vec<allocation::Model>> {
        db.loader(ByColRevBatcher::<allocation::Entity> {
            revision,
            col: allocation::Column::TaskId,
        })
        .await
        .load(self.header_id.into())
        .await
    }

    pub async fn dataloader_resource_constraints(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Vec<resource_constraint::Model>> {
        db.loader(ByColRevBatcher::<resource_constraint::Entity> {
            revision,
            col: resource_constraint::Column::TaskId,
        })
        .await
        .load(self.header_id.into())
        .await
    }

    pub async fn dataloader_header(&self, db: &DbContext) -> anyhow::Result<task_header::Model> {
        db.loader(ByColBatcher::<task_header::Entity> { col: task_header::Column::Id })
            .await
            .load_exactly_one(self.header_id.into())
            .await
    }
}

pub struct TaskIterationUpserter {
    pub task_header_id: i32,
}

impl TaskIterationUpserter {
    pub fn new(task_header_id: i32) -> Self {
        Self { task_header_id }
    }
}

impl Upserter for TaskIterationUpserter {
    type Entity = task_iteration::Entity;
    type Key = ();
    type RelData = ();

    fn existing_condition(
        &self,
        _: &Vec<&<Self::Entity as EntityTrait>::ActiveModel>,
    ) -> sea_orm::Condition {
        task_iteration::Column::HeaderId.eq(self.task_header_id).into_condition()
    }

    fn key(&self, _: &task_iteration::ActiveModel) -> anyhow::Result<()> {
        Ok(())
    }

    fn model_equal(
        &self,
        lhs: &<Self::Entity as EntityTrait>::ActiveModel,
        rhs: &<Self::Entity as EntityTrait>::ActiveModel,
    ) -> bool {
        lhs.header_id.try_as_ref() == rhs.header_id.try_as_ref()
            && lhs.title.try_as_ref() == rhs.title.try_as_ref()
            && lhs.description.try_as_ref() == rhs.description.try_as_ref()
            && lhs.designation.try_as_ref() == rhs.designation.try_as_ref()
            && lhs.parent_id.try_as_ref().cloned().flatten()
                == rhs.parent_id.try_as_ref().cloned().flatten()
            && lhs.earliest_start.try_as_ref().cloned().flatten()
                == rhs.earliest_start.try_as_ref().cloned().flatten()
            && lhs.schedule_target.try_as_ref().cloned().flatten()
                == rhs.schedule_target.try_as_ref().cloned().flatten()
            && lhs.effort.try_as_ref().cloned().flatten().map(f32::to_bits)
                == rhs.effort.try_as_ref().cloned().flatten().map(f32::to_bits)
            && lhs.priority.try_as_ref().map(|value| value.to_bits())
                == rhs.priority.try_as_ref().map(|value| value.to_bits())
    }
}
