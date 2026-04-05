use crate::db::{
    context::DbContext,
    dataloader::ByColBatcher,
    entity::{holiday, holiday_entry},
};

impl holiday_entry::Model {
    pub async fn dataloader_holiday(
        &self,
        db: &DbContext,
    ) -> anyhow::Result<Option<holiday::Model>> {
        db.loader(ByColBatcher::<holiday::Entity> { col: holiday::Column::Id })
            .await
            .load_one(self.holiday_id.into())
            .await
    }
}
