use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::{
    db::{
        context::DbContext,
        dataloader::{ByColBatcher, ByColRevBatcher},
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

    pub async fn query_availability_latest(
        &self,
        db: &DbContext,
    ) -> anyhow::Result<Vec<availability::Model>> {
        let resource_id = self.header_id;
        let availability = availability::Entity::find()
            .filter(availability::Column::ResourceId.eq(resource_id))
            .filter(availability::Column::RevDeleted.is_null())
            .all(db.txn().await?)
            .await?;
        Ok(availability)
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
