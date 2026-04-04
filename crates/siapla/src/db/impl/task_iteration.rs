use crate::db::{
    PredecessorTaskIterations, SuccessorTaskIterations,
    context::DbContext,
    dataloader::{ByColRevBatcher, LinkBatcher},
    entity::{allocation, issue, task_iteration},
};

impl task_iteration::Model {
    pub async fn query_predecessors(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Vec<task_iteration::Model>> {
        db.loader(LinkBatcher::<PredecessorTaskIterations>::new(revision))
            .await
            .load(self.header_id.into())
            .await
    }

    pub async fn query_successors(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Vec<task_iteration::Model>> {
        db.loader(LinkBatcher::<SuccessorTaskIterations>::new(revision))
            .await
            .load(self.header_id.into())
            .await
    }

    pub async fn query_issues(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Vec<issue::Model>> {
        db.loader(ByColRevBatcher::<issue::Entity> { revision, col: issue::Column::TaskId })
            .await
            .load(self.header_id.into())
            .await
    }

    pub async fn query_allocations(
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
}
