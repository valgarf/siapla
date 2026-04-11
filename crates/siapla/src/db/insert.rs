use sea_orm::prelude::*;
use sea_orm::{EntityTrait, PrimaryKeyTrait};

use crate::{
    RangeRevColumns,
    db::{DbContext, upsert::LazyRevision},
};
use anyhow;

pub async fn insert_rev<ET: EntityTrait>(
    db: &DbContext,
    revision: &LazyRevision,
    models: impl IntoIterator<Item = ET::ActiveModel>,
) -> anyhow::Result<Option<<<ET as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType>>
where
    ET: RangeRevColumns,
{
    let mut insert_models: Vec<_> = models.into_iter().collect();
    if insert_models.is_empty() {
        return Ok(None);
    }
    let revision_id = revision.get(db).await?;
    for m in &mut insert_models {
        m.set(<ET as RangeRevColumns>::rev_created_column(), revision_id.into());
        m.set(<ET as RangeRevColumns>::rev_deleted_column(), (None as Option<i64>).into());
    }

    let res = ET::insert_many(insert_models).exec(db.txn().await?).await?;

    Ok(Some(res.last_insert_id))
}
