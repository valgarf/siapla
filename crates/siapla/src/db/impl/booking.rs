use crate::db::{
    ResourceIterationsFromBooking,
    context::DbContext,
    dataloader::{ByColBatcher, ByColRevBatcher, LinkBatcher},
    entity::{booking, resource_iteration, task_iteration},
};

impl booking::Model {
    pub async fn query_resource_iterations(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Vec<resource_iteration::Model>> {
        db.loader(LinkBatcher::<ResourceIterationsFromBooking>::new(revision))
            .await
            .load(self.id.into())
            .await
    }

    pub async fn query_task_iteration(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Option<task_iteration::Model>> {
        let current = db
            .loader(ByColRevBatcher::<task_iteration::Entity> {
                revision,
                col: task_iteration::Column::HeaderId,
            })
            .await
            .load_one(self.task_id.into())
            .await?;
        if current.is_some() {
            return Ok(current);
        }
        db.loader(ByColBatcher::<task_iteration::Entity> { col: task_iteration::Column::Id })
            .await
            .load_one(self.task_id.into())
            .await
    }
}
