use std::{collections::HashMap, sync::Arc, sync::Weak};

use dataloader::cached::Loader;
use itertools::Itertools as _;
use sea_orm::{
    ColumnTrait, EntityTrait, ModelTrait, QueryFilter, QueryOrder, strum::IntoEnumIterator,
};

use super::context::Context;
use crate::SiaplaError;

pub struct ByColBatcher<ET: EntityTrait, const CIDX: usize>
where
    ET::Column: IntoEnumIterator,
{
    pub ctx: Weak<Context>,
    pub pd: std::marker::PhantomData<ET>,
}

async fn fallible_load<ET: EntityTrait, const CIDX: usize>(
    ctx: &Weak<Context>,
    values: &[sea_orm::Value],
) -> Result<HashMap<sea_orm::Value, Result<Vec<ET::Model>, Arc<anyhow::Error>>>, anyhow::Error>
where
    ET::Column: IntoEnumIterator,
{
    let col: ET::Column = ET::Column::iter()
        .nth(CIDX)
        .expect("Loader with invalid column index");
    let ctx = ctx
        .upgrade()
        .ok_or(SiaplaError::new("Weak ref not upgradable in dataloader."))?;
    let db = ctx.db().await?;
    let tasks: Vec<ET::Model> = ET::find()
        .filter(col.is_in(values.to_vec()))
        .order_by_asc(col)
        .all(db)
        .await?;
    Ok(tasks
        .into_iter()
        .chunk_by(|task| task.get(col))
        .into_iter()
        .map(|(key, tasks)| (key, Ok(tasks.collect())))
        .collect())
}

impl<ET: EntityTrait, const CIDX: usize>
    dataloader::BatchFn<sea_orm::Value, Result<Vec<ET::Model>, Arc<anyhow::Error>>>
    for ByColBatcher<ET, CIDX>
where
    ET::Column: IntoEnumIterator,
{
    async fn load(
        &mut self,
        values: &[sea_orm::Value],
    ) -> HashMap<sea_orm::Value, Result<Vec<ET::Model>, Arc<anyhow::Error>>> {
        match fallible_load::<ET, CIDX>(&self.ctx, values).await {
            Ok(data) => data,
            Err(err) => {
                let clonable_err = Arc::new(err);
                values
                    .iter()
                    .map(|k| (k.clone(), Err(clonable_err.clone())))
                    .collect()
            }
        }
    }
}

pub type ByColLoader<ET, const CIDX: usize> = Loader<
    sea_orm::Value,
    Result<Vec<<ET as EntityTrait>::Model>, Arc<anyhow::Error>>,
    ByColBatcher<ET, CIDX>,
>;
