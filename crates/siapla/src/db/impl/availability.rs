use sea_orm::{ColumnTrait as _, EntityTrait};
use sea_query::IntoCondition as _;

use crate::{
    db::{DbContext, dataloader::ByColRevBatcher, upsert::Upserter},
    entity::{availability, resource_iteration},
};

impl availability::Model {
    pub async fn dataloader_resource(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<resource_iteration::Model> {
        db.loader(ByColRevBatcher::<resource_iteration::Entity>::new(
            resource_iteration::Column::HeaderId,
            revision,
        ))
        .await
        .load_exactly_one(self.resource_id.into())
        .await
    }
}

/// Upserter trait implementation for availability models
pub struct AvailabilityUpserter {
    pub resource_header_id: i32,
}

impl AvailabilityUpserter {
    pub fn new(resource_header_id: i32) -> Self {
        Self { resource_header_id }
    }
}

impl Upserter for AvailabilityUpserter {
    type Entity = availability::Entity;
    type Key = Option<String>;
    type RelData = ();

    fn existing_condition(
        &self,
        _: &Vec<&<Self::Entity as EntityTrait>::ActiveModel>,
    ) -> sea_orm::Condition {
        availability::Column::ResourceId.eq(self.resource_header_id).into_condition()
    }

    fn key(&self, model: &availability::ActiveModel) -> anyhow::Result<Self::Key> {
        Ok(model.weekday.try_as_ref().cloned())
    }

    fn model_equal(
        &self,
        lhs: &<Self::Entity as EntityTrait>::ActiveModel,
        rhs: &<Self::Entity as EntityTrait>::ActiveModel,
    ) -> bool {
        lhs.duration == rhs.duration
    }
}
