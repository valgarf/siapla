use super::base::{Batcher, BatcherToKey};
use crate::db::context::DbContext;
use crate::db::{ColumnIntoUsize, RangeRevColumns};
use itertools::Itertools as _;
use sea_orm::{ColumnTrait, EntityTrait, ModelTrait, QueryFilter, QueryOrder};
use std::collections::HashMap;

use std::fmt::Debug;

/// ---------------------------------------------------------------------------------
/// generic dataloader to load model by a single column
/// ---------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ByColBatcher<ET: EntityTrait>
where
    ET::Model: Send + Sync,
{
    pub col: ET::Column,
}

impl<ET: EntityTrait> ByColBatcher<ET>
where
    ET::Model: Send + Sync,
{
    pub fn new(col: ET::Column) -> Self {
        Self { col }
    }
}

impl<ET: EntityTrait> Batcher for ByColBatcher<ET>
where
    ET::Model: Send + Sync,
{
    type Key = sea_orm::Value;
    type Value = Vec<ET::Model>;

    async fn load(
        &self,
        db: &DbContext,
        values: &[Self::Key],
    ) -> Result<HashMap<Self::Key, Self::Value>, anyhow::Error> {
        let txn = db.txn().await?;
        let query = ET::find().filter(self.col.is_in(values.to_vec()));

        let models: Vec<ET::Model> = query.order_by_asc(self.col).all(txn).await?;
        let mut res: HashMap<Self::Key, Self::Value> = models
            .into_iter()
            .chunk_by(|model| model.get(self.col))
            .into_iter()
            .map(|(key, models)| (key, models.collect()))
            .collect();
        for v in values {
            res.entry(v.clone()).or_insert_with(Vec::new);
        }
        Ok(res)
    }
}

impl<ET: EntityTrait> BatcherToKey for ByColBatcher<ET>
where
    ET::Column: ColumnIntoUsize,
    ET::Model: Send + Sync,
{
    type MapKey = usize;
    fn loader_map_key(&self) -> Self::MapKey {
        self.col.to_column_index()
    }
}

/// ---------------------------------------------------------------------------------
/// generic dataloader to load a model by a column and a specified revision
/// ---------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ByColRevBatcher<ET: EntityTrait>
where
    ET::Model: Send + Sync,
{
    pub revision: i64,
    pub col: ET::Column,
}

impl<ET: EntityTrait> ByColRevBatcher<ET>
where
    ET::Model: Send + Sync,
{
    pub fn new(col: ET::Column, revision: i64) -> Self {
        Self { col, revision }
    }
}

impl<ET> Batcher for ByColRevBatcher<ET>
where
    ET: RangeRevColumns,
    ET::Model: Send + Sync,
{
    type Key = sea_orm::Value;
    type Value = Vec<ET::Model>;

    async fn load(
        &self,
        db: &DbContext,
        values: &[Self::Key],
    ) -> Result<HashMap<Self::Key, Self::Value>, anyhow::Error> {
        let txn = db.txn().await?;
        let query =
            ET::find().filter(self.col.is_in(values.to_vec())).filter(ET::condition(self.revision));

        let models: Vec<ET::Model> = query.order_by_asc(self.col).all(txn).await?;

        let mut res: HashMap<Self::Key, Self::Value> = models
            .into_iter()
            .chunk_by(|model| model.get(self.col))
            .into_iter()
            .map(|(key, models)| (key, models.collect()))
            .collect();
        for v in values {
            res.entry(v.clone()).or_insert_with(Vec::new);
        }
        Ok(res)
    }
}

impl<ET> BatcherToKey for ByColRevBatcher<ET>
where
    ET: RangeRevColumns,
    ET::Column: ColumnIntoUsize,
    ET::Model: Send + Sync,
{
    type MapKey = (i64, usize);
    fn loader_map_key(&self) -> Self::MapKey {
        (self.revision, self.col.to_column_index())
    }
}

/// ---------------------------------------------------------------------------------
/// generic dataloader to load the latest model of a revision aware entity
/// ---------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ByColLatestBatcher<ET: EntityTrait>
where
    ET::Model: Send + Sync,
{
    pub col: ET::Column,
}

impl<ET: EntityTrait> ByColLatestBatcher<ET>
where
    ET::Model: Send + Sync,
{
    pub fn new(col: ET::Column) -> Self {
        Self { col }
    }
}

impl<ET> Batcher for ByColLatestBatcher<ET>
where
    ET: RangeRevColumns,
    ET::Model: Send + Sync,
{
    type Key = sea_orm::Value;
    type Value = Vec<ET::Model>;

    async fn load(
        &self,
        db: &DbContext,
        values: &[Self::Key],
    ) -> Result<HashMap<Self::Key, Self::Value>, anyhow::Error> {
        let txn = db.txn().await?;
        let query =
            ET::find().filter(self.col.is_in(values.to_vec())).filter(ET::condition_latest());

        let models: Vec<ET::Model> = query.order_by_asc(self.col).all(txn).await?;

        let mut res: HashMap<Self::Key, Self::Value> = models
            .into_iter()
            .chunk_by(|model| model.get(self.col))
            .into_iter()
            .map(|(key, models)| (key, models.collect()))
            .collect();
        for v in values {
            res.entry(v.clone()).or_insert_with(Vec::new);
        }
        Ok(res)
    }
}

impl<ET> BatcherToKey for ByColLatestBatcher<ET>
where
    ET: RangeRevColumns,
    ET::Column: ColumnIntoUsize,
    ET::Model: Send + Sync,
{
    type MapKey = usize;
    fn loader_map_key(&self) -> Self::MapKey {
        self.col.to_column_index()
    }
}
