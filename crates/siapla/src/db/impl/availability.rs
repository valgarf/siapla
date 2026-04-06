use crate::{
    db::{DbContext, dataloader::ByColRevBatcher},
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
