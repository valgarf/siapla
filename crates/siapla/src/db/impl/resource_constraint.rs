use anyhow::anyhow;
use ordered_float::OrderedFloat;
use sea_orm::{ActiveValue, ColumnTrait as _, EntityTrait, QueryFilter as _, QueryOrder as _};
use sea_query::IntoCondition as _;

use crate::{
    db::{
        DbContext,
        dataloader::{ByColBatcher, ByColRevBatcher},
        upsert::Upserter,
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResourceConstraintKey {
    pub position: i32,
    pub r#type: String,
    pub optional: bool,
    pub speed: OrderedFloat<f64>,
    pub resource_ids: Vec<i32>,
}

pub struct ResourceConstraintUpserter {
    pub task_header_id: i32,
}

impl ResourceConstraintUpserter {
    pub fn new(task_header_id: i32) -> Self {
        Self { task_header_id }
    }
}

impl Upserter for ResourceConstraintUpserter {
    type Entity = resource_constraint::Entity;
    type Key = ResourceConstraintKey;
    type RelData = Vec<i32>;

    fn existing_condition(
        &self,
        _models: &Vec<&<Self::Entity as EntityTrait>::ActiveModel>,
    ) -> sea_orm::Condition {
        resource_constraint::Column::TaskId.eq(self.task_header_id).into_condition()
    }

    fn key(
        &self,
        model: &resource_constraint::ActiveModel,
        rel_data: &Self::RelData,
    ) -> anyhow::Result<Self::Key> {
        Ok(ResourceConstraintKey {
            position: model
                .position
                .try_as_ref()
                .copied()
                .ok_or_else(|| anyhow!("resource constraint active model is missing position"))?,
            r#type: model
                .r#type
                .try_as_ref()
                .cloned()
                .ok_or_else(|| anyhow!("resource constraint active model is missing type"))?,
            optional: model
                .optional
                .try_as_ref()
                .copied()
                .ok_or_else(|| anyhow!("resource constraint active model is missing optional"))?,
            speed: OrderedFloat(f64::from(
                model
                    .speed
                    .try_as_ref()
                    .copied()
                    .ok_or_else(|| anyhow!("resource constraint active model is missing speed"))?,
            )),
            resource_ids: rel_data.clone(),
        })
    }

    fn model_equal(
        &self,
        lhs: &<Self::Entity as EntityTrait>::ActiveModel,
        rhs: &<Self::Entity as EntityTrait>::ActiveModel,
    ) -> bool {
        lhs.task_id.try_as_ref() == rhs.task_id.try_as_ref()
            && lhs.r#type.try_as_ref() == rhs.r#type.try_as_ref()
            && lhs.optional.try_as_ref() == rhs.optional.try_as_ref()
            && lhs.speed.try_as_ref() == rhs.speed.try_as_ref()
            && lhs.position.try_as_ref() == rhs.position.try_as_ref()
    }

    fn relationships_equal(&self, lhs: &Self::RelData, rhs: &Self::RelData) -> bool {
        lhs == rhs
    }

    async fn load_existing_with_rel(
        &self,
        db: &DbContext,
        condition: sea_orm::Condition,
    ) -> anyhow::Result<Vec<(resource_constraint::Model, Self::RelData)>> {
        let results = resource_constraint::Entity::find()
            .filter(condition)
            .order_by_asc(resource_constraint::Column::Position)
            .find_with_related(resource_constraint_entry::Entity)
            .all(db.txn().await?)
            .await?;

        Ok(results
            .into_iter()
            .map(|(constraint, entries)| {
                let mut resource_ids =
                    entries.into_iter().map(|entry| entry.resource_id).collect::<Vec<_>>();
                resource_ids.sort_unstable();
                (constraint, resource_ids)
            })
            .collect())
    }

    async fn after_insert(
        &self,
        db: &DbContext,
        inserted: &Vec<(resource_constraint::Model, Self::RelData)>,
    ) -> anyhow::Result<()> {
        let entry_models = inserted
            .into_iter()
            .flat_map(|(constraint, resource_ids)| {
                resource_ids.into_iter().map(move |resource_id| {
                    resource_constraint_entry::ActiveModel {
                        id: ActiveValue::NotSet,
                        resource_constraint_id: ActiveValue::Set(constraint.id),
                        resource_id: ActiveValue::Set(*resource_id),
                    }
                })
            })
            .collect::<Vec<_>>();

        if !entry_models.is_empty() {
            resource_constraint_entry::Entity::insert_many(entry_models)
                .exec(db.txn().await?)
                .await?;
        }

        Ok(())
    }
}
