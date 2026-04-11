use sea_orm::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, Iterable, PrimaryKeyArity, PrimaryKeyTrait, Value};
use sea_query::Expr;

use crate::{
    RangeRevColumns,
    db::{DbContext, revisioning::LazyRevision},
};
use anyhow;

pub async fn delete_rev_by_pk<ET: EntityTrait>(
    db: &DbContext,
    revision: &LazyRevision,
    pks: impl IntoIterator<Item = impl Into<Value>>,
) -> anyhow::Result<u64>
where
    ET: RangeRevColumns,
{
    assert!(
        <ET::PrimaryKey as PrimaryKeyTrait>::ValueType::ARITY == 1,
        "delete_rev_by_pk only supports single-column primary keys"
    );
    let columns: Vec<_> = <ET as EntityTrait>::PrimaryKey::iter().collect();
    if columns.len() != 1 {
        return Err(anyhow::anyhow!("Expected exactly one primary key column"));
    }
    let column = columns[0].into_column();

    delete_rev_by_col::<ET>(db, column, revision, pks).await
}

pub async fn delete_rev_by_col<ET: EntityTrait>(
    db: &DbContext,
    column: ET::Column,
    revision: &LazyRevision,
    values: impl IntoIterator<Item = impl Into<Value>>,
) -> anyhow::Result<u64>
where
    ET: RangeRevColumns,
    ET::Column: ColumnTrait,
{
    let sea_orm_values: Vec<_> = values.into_iter().map(|pk| pk.into()).collect();
    if sea_orm_values.is_empty() {
        return Ok(0);
    }

    let txn = db.txn().await?;
    let revision_id = revision.get(db).await?;

    let res = ET::update_many()
        .col_expr(
            <ET as RangeRevColumns>::rev_deleted_column(),
            Expr::value(Value::BigInt(Some(revision_id))),
        )
        .filter(column.is_in(sea_orm_values))
        .exec(txn)
        .await?;

    Ok(res.rows_affected)
}
