use sea_orm::{EntityTrait, Value};

use crate::db::{
    ResourceIterationsFromBooking,
    context::DbContext,
    dataloader::{ByColRevBatcher, LinkBatcher},
    entity::{booking, resource_iteration, task_iteration},
    upsert::Upserter,
};

impl booking::Model {
    pub async fn dataloader_resource_iterations(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Vec<resource_iteration::Model>> {
        db.loader(LinkBatcher::<ResourceIterationsFromBooking>::new(revision))
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

/// Upserter trait implementation for availability models
pub struct BookingUpserter {
    pub task_header_id: i32,
}

impl BookingUpserter {
    pub fn new(task_header_id: i32) -> Self {
        Self { task_header_id }
    }
}

impl Upserter for BookingUpserter {
    type Entity = booking::Entity;
    type Key = Option<Value>;

    fn existing_condition(
        &self,
        models: &Vec<&<Self::Entity as EntityTrait>::ActiveModel>,
    ) -> sea_orm::Condition {
        let cond = booking::Column::TaskId.eq(self.task_header_id).into_condition();
        if models.len() == 1 { cond.add(booking::Column::Id.eq(models[0].id)) } else { cond }
    }

    fn key(&self, model: &booking::ActiveModel) -> anyhow::Result<Self::Key> {
        Ok(model.id.clone().into_value())
    }

    fn model_equal(
        &self,
        lhs: &<Self::Entity as EntityTrait>::ActiveModel,
        rhs: &<Self::Entity as EntityTrait>::ActiveModel,
    ) -> bool {
        lhs.start == rhs.start && lhs.end == rhs.end && lhs.
    }
}
