use crate::db::{
    ResourceIterationsFromAllocation,
    context::DbContext,
    dataloader::{ByColRevBatcher, LinkBatcher},
    entity::{allocation, resource_iteration, task_iteration},
};

impl allocation::Model {
    pub async fn dataloader_resource_iterations(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Vec<resource_iteration::Model>> {
        db.loader(LinkBatcher::<ResourceIterationsFromAllocation>::new(revision))
            .await
            .load(self.id.into())
            .await
    }

    pub async fn dataloader_task_iteration(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Option<task_iteration::Model>> {
        db.loader(ByColRevBatcher::<task_iteration::Entity> {
            revision,
            col: task_iteration::Column::HeaderId,
        })
        .await
        .load_one(self.task_id.into())
        .await
    }
}
