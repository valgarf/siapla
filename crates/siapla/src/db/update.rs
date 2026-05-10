use crate::RangeRevColumns;
use crate::db::DbContext;
use crate::db::delete::delete_rev_by_pk;
use crate::db::revisioning::LazyRevision;

use anyhow::anyhow;
use sea_orm::prelude::*;
use sea_orm::{EntityTrait, IntoActiveModel, Iterable, PrimaryKeyTrait};
use sea_query::ValueTuple;

pub trait Updater {
    type Entity: EntityTrait;

    fn existing_condition(&self) -> sea_orm::Condition;
    fn apply_changes(
        &self,
        existing: &mut <Self::Entity as EntityTrait>::ActiveModel,
    ) -> anyhow::Result<()>;
    /* TODO: model_eq is similar between upserter / updater. We should find a way to share an
     * implementation, with the option to overwrite it.
     * Idea: have a ModelEq trait that could be implemented. This is then used by default in
     * updater / upserter `model_equal`, but can be overwritten if needed.
     */
    fn model_equal(
        &self,
        lhs: &<Self::Entity as EntityTrait>::ActiveModel,
        rhs: &<Self::Entity as EntityTrait>::ActiveModel,
    ) -> bool;

    fn load_existing(
        &self,
        db: &DbContext,
        condition: sea_orm::Condition,
    ) -> impl std::future::Future<Output = anyhow::Result<Vec<<Self::Entity as EntityTrait>::Model>>>
    + Send {
        async move { Ok(<Self::Entity>::find().filter(condition).all(db.txn().await?).await?) }
    }

    fn after_insert(
        &self,
        _db: &DbContext,
        _inserted: Vec<<Self::Entity as EntityTrait>::Model>,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        async { Ok(()) }
    }
}

pub async fn update_rev_many<U: Updater>(
    db: &DbContext,
    revision: &LazyRevision,
    updater: U,
) -> anyhow::Result<()>
where
    <U::Entity as EntityTrait>::Model: IntoActiveModel<<U::Entity as EntityTrait>::ActiveModel>,
    U::Entity: RangeRevColumns,
    <U::Entity as EntityTrait>::ActiveModel: Send,
{
    if <<U::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType::ARITY != 1 {
        return Err(anyhow!("update_rev_many only supports single-column primary keys"));
    }

    let condition = sea_orm::Condition::all()
        .add(updater.existing_condition())
        .add(<U::Entity as RangeRevColumns>::rev_deleted_column().is_null());
    let existing_models = updater.load_existing(db, condition).await?;

    if existing_models.is_empty() {
        return Ok(());
    }

    let mut updated_models = Vec::new();
    let mut delete_pks = Vec::new();

    for existing_model in existing_models {
        let existing_am = existing_model.into_active_model();
        let mut updated_am = existing_am.clone();
        updater.apply_changes(&mut updated_am)?;
        if !updater.model_equal(&existing_am, &updated_am) {
            let pk = existing_am
                .get_primary_key_value()
                .ok_or(anyhow!("Existing model lacks primary key"))?;
            let pk = match pk {
                ValueTuple::One(value) => value,
                _ => panic!("This can only have a single value"),
            };
            delete_pks.push(pk);
            updated_models.push(updated_am);
        }
    }

    if updated_models.is_empty() {
        return Ok(());
    }

    let revision_id = revision.get(db).await?;
    delete_rev_by_pk::<U::Entity>(db, revision, delete_pks).await?;

    for model in &mut updated_models {
        model.set(<U::Entity as RangeRevColumns>::rev_created_column(), revision_id.into());
        model.set(
            <U::Entity as RangeRevColumns>::rev_deleted_column(),
            (None as Option<i64>).into(),
        );
        for col in <U::Entity as EntityTrait>::PrimaryKey::iter() {
            model.not_set(col.into_column());
        }
    }

    let inserted =
        U::Entity::insert_many(updated_models).exec_with_returning_many(db.txn().await?).await?;
    updater.after_insert(db, inserted).await?;

    // TODO: we could return unchanged / updated models here
    Ok(())
}
