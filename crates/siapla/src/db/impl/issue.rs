use crate::db::{
    context::DbContext,
    dataloader::{ByColBatcher, ByColRevBatcher},
    entity::{issue, task_iteration},
};

impl issue::Model {
    pub async fn query_task_iteration(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Option<task_iteration::Model>> {
        let Some(task_id) = self.task_id else {
            return Ok(None);
        };
        let current = db
            .loader(ByColRevBatcher::<task_iteration::Entity> {
                revision,
                col: task_iteration::Column::HeaderId,
            })
            .await
            .load_one(task_id.into())
            .await?;
        if current.is_some() {
            return Ok(current);
        }
        db.loader(ByColBatcher::<task_iteration::Entity> { col: task_iteration::Column::Id })
            .await
            .load_one(task_id.into())
            .await
    }
}
