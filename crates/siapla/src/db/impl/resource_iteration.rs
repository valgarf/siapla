use crate::{
    db::{
        context::DbContext,
        dataloader::{ByColBatcher, ByColRevBatcher, by_col::ByColLatestBatcher},
        entity::{availability, holiday, resource_iteration, vacation},
    },
    entity::resource_header,
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
}
