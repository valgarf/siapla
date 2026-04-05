use crate::db::{
    context::DbContext,
    dataloader::ByColRevBatcher,
    entity::{issue, task_iteration},
};

impl issue::Model {
    pub async fn dataloader_task_iteration(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Option<task_iteration::Model>> {
        let Some(task_id) = self.task_id else {
            return Ok(None);
        };
        db.loader(ByColRevBatcher::<task_iteration::Entity> {
            revision,
            col: task_iteration::Column::HeaderId,
        })
        .await
        .load_one(task_id.into())
        .await
    }
}
