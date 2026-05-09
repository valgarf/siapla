use crate::RangeRevColumns;
use crate::db::DbContext;
use crate::db::delete::delete_rev_by_pk;
use crate::db::revisioning::LazyRevision;

use anyhow::anyhow;
use sea_orm::prelude::*;
use sea_orm::{EntityTrait, IntoActiveModel, Iterable, PrimaryKeyTrait};
use sea_query::ValueTuple;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

pub trait Upserter {
    type Entity: EntityTrait;
    type Key: Hash + Eq + Clone;
    type RelData: Default + Send;

    fn existing_condition(
        &self,
        models: &Vec<&<Self::Entity as EntityTrait>::ActiveModel>,
    ) -> sea_orm::Condition;
    fn key(&self, model: &<Self::Entity as EntityTrait>::ActiveModel) -> anyhow::Result<Self::Key>;
    fn model_equal(
        &self,
        lhs: &<Self::Entity as EntityTrait>::ActiveModel,
        rhs: &<Self::Entity as EntityTrait>::ActiveModel,
    ) -> bool;

    fn relationships_equal(&self, _lhs: &Self::RelData, _rhs: &Self::RelData) -> bool {
        true
    }

    fn load_existing_with_rel(
        &self,
        db: &DbContext,
        condition: sea_orm::Condition,
    ) -> impl std::future::Future<
        Output = anyhow::Result<Vec<(<Self::Entity as EntityTrait>::Model, Self::RelData)>>,
    > + Send {
        async move {
            let models = <Self::Entity>::find().filter(condition).all(db.txn().await?).await?;
            Ok(models.into_iter().map(|m| (m, Self::RelData::default())).collect())
        }
    }

    fn after_insert(
        &self,
        _db: &DbContext,
        _inserted: Vec<(<Self::Entity as EntityTrait>::Model, Self::RelData)>,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        async { Ok(()) }
    }
}

pub async fn upsert_rev_many<U: Upserter>(
    db: &DbContext,
    revision: &LazyRevision,
    upserter: U,
    models_with_rel: Vec<(<U::Entity as EntityTrait>::ActiveModel, U::RelData)>,
) -> anyhow::Result<()>
where
    <U::Entity as EntityTrait>::Model: IntoActiveModel<<U::Entity as EntityTrait>::ActiveModel>,
    U::Entity: RangeRevColumns,
    <U::Entity as EntityTrait>::ActiveModel: Send,
{
    let model_refs: Vec<_> = models_with_rel.iter().map(|(m, _)| m).collect();
    let condition = upserter
        .existing_condition(&model_refs)
        .add(<U::Entity as RangeRevColumns>::rev_deleted_column().is_null());
    let existing_with_rel = upserter.load_existing_with_rel(db, condition).await?;

    let mut existing_map: HashMap<U::Key, (<U::Entity as EntityTrait>::ActiveModel, U::RelData)> =
        HashMap::new();
    for (m, rel_data) in existing_with_rel {
        let am = m.into_active_model();
        let k = upserter.key(&am)?;
        existing_map.insert(k, (am, rel_data));
    }

    let mut new_map: HashMap<U::Key, (<U::Entity as EntityTrait>::ActiveModel, U::RelData)> =
        models_with_rel
            .into_iter()
            .map(|(m, rd)| upserter.key(&m).map(|k| (k, (m, rd))))
            .collect::<anyhow::Result<_>>()?;
    let existing_keys: HashSet<_> = existing_map.keys().cloned().collect();
    let new_keys: HashSet<_> = new_map.keys().cloned().collect();

    let delete_keys: HashSet<_> = existing_keys.difference(&new_keys).cloned().collect();
    let insert_keys: HashSet<_> = new_keys.difference(&existing_keys).cloned().collect();
    let update_keys: HashSet<_> = existing_keys
        .intersection(&new_keys)
        .filter(|k| {
            let (existing_am, existing_rd) = &existing_map[k];
            let (new_am, new_rd) = &new_map[k];
            !upserter.model_equal(existing_am, new_am)
                || !upserter.relationships_equal(existing_rd, new_rd)
        })
        .cloned()
        .collect();

    if delete_keys.is_empty() && insert_keys.is_empty() && update_keys.is_empty() {
        return Ok(());
    }
    let revision_id = revision.get(db).await?;

    if <<U::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType::ARITY == 1 {
        let delete_pks: Vec<_> = delete_keys
            .union(&update_keys)
            .map(|k| existing_map[k].0.get_primary_key_value())
            .collect::<Option<Vec<_>>>()
            .ok_or(anyhow!("Existing model lacks primary key"))?;
        let delete_pks: Vec<_> = delete_pks
            .iter()
            .map(|tup| match tup {
                ValueTuple::One(value) => value,
                _ => panic!("This can only have a single value"),
            })
            .cloned()
            .collect();
        delete_rev_by_pk::<U::Entity>(db, &revision, delete_pks).await?;
    } else {
        for k in delete_keys.union(&update_keys) {
            if let Some((am, _)) = existing_map.remove(k) {
                am.delete(db.txn().await?).await?;
            }
        }
    }

    let keys_to_insert: Vec<_> = insert_keys.union(&update_keys).cloned().collect();
    if !keys_to_insert.is_empty() {
        let txn = db.txn().await?;
        let mut insert_models = Vec::new();
        let mut insert_rd = Vec::new();
        for k in keys_to_insert {
            if let Some((mut am, rd)) = new_map.remove(&k) {
                am.set(<U::Entity as RangeRevColumns>::rev_created_column(), revision_id.into());
                am.set(
                    <U::Entity as RangeRevColumns>::rev_deleted_column(),
                    (None as Option<i64>).into(),
                );
                for col in <U::Entity as EntityTrait>::PrimaryKey::iter() {
                    am.not_set(col.into_column());
                }
                insert_models.push(am);
                insert_rd.push(rd);
            }
        }

        let res = U::Entity::insert_many(insert_models).exec_with_returning_many(txn).await?;
        let inserted_with_rel = res.into_iter().zip(insert_rd.into_iter()).collect::<Vec<_>>();
        upserter.after_insert(db, inserted_with_rel).await?;
    }

    Ok(())
}

pub async fn upsert_rev_one<U: Upserter>(
    db: &DbContext,
    revision: &LazyRevision,
    upserter: U,
    mut model: <U::Entity as EntityTrait>::ActiveModel,
    new_rel_data: U::RelData,
) -> anyhow::Result<<U::Entity as EntityTrait>::Model>
where
    <U::Entity as EntityTrait>::Model: IntoActiveModel<<U::Entity as EntityTrait>::ActiveModel>,
    U::Entity: RangeRevColumns,
    <U::Entity as EntityTrait>::ActiveModel: Send,
{
    let condition = upserter
        .existing_condition(&vec![&model])
        .add(<U::Entity as RangeRevColumns>::rev_deleted_column().is_null());
    let existing_with_rel = upserter.load_existing_with_rel(db, condition).await?;

    let mut existing_map: HashMap<
        U::Key,
        (<U::Entity as EntityTrait>::Model, <U::Entity as EntityTrait>::ActiveModel, U::RelData),
    > = HashMap::new();
    for (m, rel_data) in existing_with_rel {
        let am = m.clone().into_active_model();
        let k = upserter.key(&am)?;
        existing_map.insert(k, (m, am, rel_data));
    }

    let new_key = upserter.key(&model)?;
    let existing_keys: HashSet<_> = existing_map.keys().cloned().collect();
    let new_keys: HashSet<_> = HashSet::from_iter(vec![new_key.clone()]);

    let delete_keys: HashSet<_> = existing_keys.difference(&new_keys).cloned().collect();
    let insert_keys: HashSet<_> = new_keys.difference(&existing_keys).cloned().collect();
    let update_keys: HashSet<_> = existing_keys
        .intersection(&new_keys)
        .filter(|k| {
            let existing = &existing_map[k];
            !upserter.model_equal(&existing.1, &model)
                || !upserter.relationships_equal(&existing.2, &new_rel_data)
        })
        .cloned()
        .collect();

    if delete_keys.is_empty() && insert_keys.is_empty() && update_keys.is_empty() {
        let (model, _, _) = existing_map.remove(&new_key).ok_or(anyhow!("No existing model found in map, even though we do not plan on inserting the model. This should not happen."))?;
        return Ok(model);
    }
    let revision_id = revision.get(db).await?;

    if <<U::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType::ARITY == 1 {
        let delete_pks: Vec<_> = delete_keys
            .union(&update_keys)
            .map(|k| existing_map[k].1.get_primary_key_value())
            .collect::<Option<Vec<_>>>()
            .ok_or(anyhow!("Existing model lacks primary key"))?;
        let delete_pks: Vec<_> = delete_pks
            .iter()
            .map(|tup| match tup {
                ValueTuple::One(value) => value,
                _ => panic!("This can only have a single value"),
            })
            .cloned()
            .collect();
        delete_rev_by_pk::<U::Entity>(db, &revision, delete_pks).await?;
    } else {
        for k in delete_keys.union(&update_keys) {
            if let Some(v) = existing_map.remove_entry(k).map(|(_, v)| v) {
                v.1.delete(db.txn().await?).await?;
            }
        }
    }

    let union_insert_keys: Vec<_> = insert_keys.union(&update_keys).cloned().collect();
    let res = if !union_insert_keys.is_empty() {
        assert!(union_insert_keys == vec![new_key]);

        model.set(<U::Entity as RangeRevColumns>::rev_created_column(), revision_id.into());
        model.set(
            <U::Entity as RangeRevColumns>::rev_deleted_column(),
            (None as Option<i64>).into(),
        );
        for col in <U::Entity as EntityTrait>::PrimaryKey::iter() {
            model.not_set(col.into_column());
        }
        let inserted = model.insert(db.txn().await?).await?;
        upserter.after_insert(db, vec![(inserted.clone(), new_rel_data)]).await?;
        inserted
    } else {
        let (model, _, _) = existing_map.remove(&new_key).ok_or(anyhow!("No existing model found in map, even though we did not insert a new one. This should not happen."))?;
        model
    };

    Ok(res)
}
