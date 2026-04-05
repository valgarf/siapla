use crate::{
    db::{
        DbContext,
        dataloader::{ByColBatcher, ByColRevBatcher},
    },
    entity::{resource_constraint, resource_constraint_entry, resource_iteration},
};

impl resource_constraint::Model {
    pub async fn dataloader_entries(
        &self,
        db: &DbContext,
    ) -> anyhow::Result<Vec<resource_constraint_entry::Model>> {
        db.loader(ByColBatcher::<resource_constraint_entry::Entity>::new(
            resource_constraint_entry::Column::ResourceConstraintId,
        ))
        .await
        .load(self.id.into())
        .await
    }
}

impl resource_constraint_entry::Model {
    pub async fn dataloader_resource_constraint(
        &self,
        db: &DbContext,
    ) -> anyhow::Result<resource_constraint::Model> {
        db.loader(ByColBatcher::<resource_constraint::Entity>::new(resource_constraint::Column::Id))
            .await
            .load_exactly_one(self.resource_constraint_id.into())
            .await
    }

    pub async fn dataloader_resource_iteration(
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
