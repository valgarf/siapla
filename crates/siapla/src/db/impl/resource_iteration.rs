use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait as _, EntityTrait};
use sea_query::IntoCondition as _;

use crate::{
    db::{
        context::DbContext,
        dataloader::{
            AvailabilityBatcher, ByColBatcher, ByColRevBatcher, by_col::ByColLatestBatcher,
        },
        entity::{availability, holiday, resource_iteration, vacation},
        upsert::Upserter,
    },
    entity::resource_header,
    scheduling::Intervals,
};

impl resource_iteration::Model {
    pub async fn dataloader_holiday(
        &self,
        db: &DbContext,
    ) -> anyhow::Result<Option<holiday::Model>> {
        db.loader(ByColBatcher::<holiday::Entity> { col: holiday::Column::Id })
            .await
            .load_one(self.holiday_id.into())
            .await
    }

    pub async fn dataloader_availability(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Vec<availability::Model>> {
        db.loader(ByColRevBatcher::<availability::Entity> {
            revision,
            col: availability::Column::ResourceId,
        })
        .await
        .load(self.header_id.into())
        .await
    }

    pub async fn dataloader_combined_availability(
        &self,
        ctx: &DbContext,
        start: NaiveDateTime,
        end: NaiveDateTime,
        revision: i64,
    ) -> anyhow::Result<Intervals<NaiveDateTime>> {
        ctx.loader(AvailabilityBatcher { start, end, revision }).await.load(self.id).await
    }

    pub async fn dataloader_vacation(
        &self,
        db: &DbContext,
        revision: i64,
    ) -> anyhow::Result<Vec<vacation::Model>> {
        db.loader(ByColRevBatcher::<vacation::Entity> {
            revision,
            col: vacation::Column::ResourceId,
        })
        .await
        .load(self.header_id.into())
        .await
    }

    pub async fn dataloader_availability_latest(
        &self,
        db: &DbContext,
    ) -> anyhow::Result<Vec<availability::Model>> {
        db.loader(ByColLatestBatcher::<availability::Entity> {
            col: availability::Column::ResourceId,
        })
        .await
        .load(self.header_id.into())
        .await
    }

    pub async fn dataloader_header(
        &self,
        db: &DbContext,
    ) -> anyhow::Result<resource_header::Model> {
        db.loader(ByColBatcher::<resource_header::Entity> { col: resource_header::Column::Id })
            .await
            .load_exactly_one(self.header_id.into())
            .await
    }

    pub async fn dataloader_by_header_at_rev(
        db: &DbContext,
        header_id: i32,
        revision: i64,
    ) -> anyhow::Result<Self> {
        db.loader(ByColRevBatcher::<resource_iteration::Entity> {
            revision,
            col: resource_iteration::Column::HeaderId,
        })
        .await
        .load_exactly_one(header_id.into())
        .await
    }
}

/// Upserter trait implementation for availability models
pub struct ResourceIterationUpserter {
    pub resource_header_id: i32,
}

impl ResourceIterationUpserter {
    pub fn new(resource_header_id: i32) -> Self {
        Self { resource_header_id }
    }
}

impl Upserter for ResourceIterationUpserter {
    type Entity = resource_iteration::Entity;
    type Key = Option<i32>;
    type RelData = ();

    fn existing_condition(
        &self,
        _: &Vec<&<Self::Entity as EntityTrait>::ActiveModel>,
    ) -> sea_orm::Condition {
        resource_iteration::Column::HeaderId.eq(self.resource_header_id).into_condition()
    }

    fn key(&self, model: &resource_iteration::ActiveModel) -> anyhow::Result<Self::Key> {
        Ok(model.id.try_as_ref().cloned())
    }

    fn model_equal(
        &self,
        lhs: &<Self::Entity as EntityTrait>::ActiveModel,
        rhs: &<Self::Entity as EntityTrait>::ActiveModel,
    ) -> bool {
        lhs.holiday_id == rhs.holiday_id
            && lhs.name == rhs.name
            && lhs.timezone == rhs.timezone
            && lhs.added == rhs.added
            && lhs.removed == rhs.removed
    }
}
