use sea_orm::{ActiveValue, ColumnTrait as _, EntityTrait, QueryFilter as _};
use sea_query::IntoCondition as _;

use crate::db::{
    DbContext, ResourceIterationsFromBooking,
    dataloader::{ByColRevBatcher, LinkBatcher},
    entity::{booking, booking_resource, resource_iteration, task_iteration},
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

pub struct BookingUpserter {
    pub task_header_id: i32,
    pub db_id: Option<i32>,
}

impl BookingUpserter {
    pub fn new(task_header_id: i32, db_id: Option<i32>) -> Self {
        Self { task_header_id, db_id }
    }
}

impl Upserter for BookingUpserter {
    type Entity = booking::Entity;
    type Key = ();
    type RelData = Vec<i32>;

    fn existing_condition(
        &self,
        _models: &Vec<&<Self::Entity as EntityTrait>::ActiveModel>,
    ) -> sea_orm::Condition {
        match self.db_id {
            Some(id) => booking::Column::Id.eq(id).into_condition(),
            None => sea_orm::Condition::all().add(sea_orm::sea_query::Expr::value(false)),
        }
    }

    fn key(&self, _model: &booking::ActiveModel, _rel_data: &Self::RelData) -> anyhow::Result<()> {
        Ok(())
    }

    fn model_equal(
        &self,
        lhs: &<Self::Entity as EntityTrait>::ActiveModel,
        rhs: &<Self::Entity as EntityTrait>::ActiveModel,
    ) -> bool {
        lhs.task_id == rhs.task_id
            && lhs.start == rhs.start
            && lhs.end == rhs.end
            && lhs.r#final == rhs.r#final
    }

    fn relationships_equal(&self, lhs: &Vec<i32>, rhs: &Vec<i32>) -> bool {
        lhs == rhs
    }

    async fn load_existing_with_rel(
        &self,
        db: &DbContext,
        condition: sea_orm::Condition,
    ) -> anyhow::Result<Vec<(booking::Model, Vec<i32>)>> {
        let results = booking::Entity::find()
            .filter(condition)
            .find_with_related(booking_resource::Entity)
            .all(db.txn().await?)
            .await?;
        Ok(results
            .into_iter()
            .map(|(b, resources)| {
                let mut ids: Vec<i32> = resources.into_iter().map(|r| r.resource_id).collect();
                ids.sort();
                (b, ids)
            })
            .collect())
    }

    async fn after_insert(
        &self,
        db: &DbContext,
        inserted: &Vec<(booking::Model, Vec<i32>)>,
    ) -> anyhow::Result<()> {
        let booking_resource_links: Vec<booking_resource::ActiveModel> = inserted
            .into_iter()
            .flat_map(|(booking, rel_data)| {
                rel_data.into_iter().map(move |rid| booking_resource::ActiveModel {
                    id: ActiveValue::NotSet,
                    booking_id: ActiveValue::Set(booking.id),
                    resource_id: ActiveValue::Set(*rid),
                })
            })
            .collect();
        if !booking_resource_links.is_empty() {
            booking_resource::Entity::insert_many(booking_resource_links)
                .exec(db.txn().await?)
                .await?;
        }
        Ok(())
    }
}
