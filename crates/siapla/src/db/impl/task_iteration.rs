use crate::{
    db::{
        PredecessorTaskIterations, SuccessorTaskIterations,
        context::DbContext,
        dataloader::{ByColBatcher, ByColRevBatcher, LinkBatcher},
        entity::{allocation, issue, task_iteration},
    },
    entity::{resource_constraint, task_header},
};

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
