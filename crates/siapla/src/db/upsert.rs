use crate::RangeRevColumns;
use crate::db::DbContext;
use crate::db::delete::delete_rev_by_pk;
use crate::db::revisioning::LazyRevision;

use anyhow::anyhow;
use sea_orm::prelude::*;
use sea_orm::{EntityTrait, IntoActiveModel, Iterable, PrimaryKeyTrait, TryIntoModel};
use sea_query::ValueTuple;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

// convenient type accessors
type Model<U> = <<U as Upserter>::Entity as EntityTrait>::Model;
type ActiveModel<U> = <<U as Upserter>::Entity as EntityTrait>::ActiveModel;
type PrimaryKey<U> = <<U as Upserter>::Entity as EntityTrait>::PrimaryKey;

pub trait Upserter {
    type Entity: EntityTrait;
    type Key: Hash + Eq + Clone;
    type RelData: Default + Send;

    fn existing_condition(&self, models: &Vec<&ActiveModel<Self>>) -> sea_orm::Condition;
    fn key(&self, model: &ActiveModel<Self>) -> anyhow::Result<Self::Key>;
    fn model_equal(&self, lhs: &ActiveModel<Self>, rhs: &ActiveModel<Self>) -> bool;

    fn relationships_equal(&self, _lhs: &Self::RelData, _rhs: &Self::RelData) -> bool {
        true
    }

    fn load_existing_with_rel(
        &self,
        db: &DbContext,
        condition: sea_orm::Condition,
    ) -> impl std::future::Future<Output = anyhow::Result<Vec<(Model<Self>, Self::RelData)>>> + Send
    {
        async move {
            let models = <Self::Entity>::find().filter(condition).all(db.txn().await?).await?;
            Ok(models.into_iter().map(|m| (m, Self::RelData::default())).collect())
        }
    }

    fn after_insert(
        &self,
        _db: &DbContext,
        _inserted: &Vec<(Model<Self>, Self::RelData)>,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        async { Ok(()) }
    }
}

/// Upsert multiple models with revisioning. Returns (deleted_models, unchanged_models,
/// created_models).
pub async fn upsert_rev_many<U: Upserter>(
    db: &DbContext,
    revision: &LazyRevision,
    upserter: U,
    models_with_rel: Vec<(ActiveModel<U>, U::RelData)>,
) -> anyhow::Result<(Vec<Model<U>>, Vec<Model<U>>, Vec<Model<U>>)>
where
    Model<U>: IntoActiveModel<ActiveModel<U>>,
    U::Entity: RangeRevColumns,
    ActiveModel<U>: Send + TryIntoModel<Model<U>>,
{
    let model_refs: Vec<_> = models_with_rel.iter().map(|(m, _)| m).collect();
    let condition = upserter
        .existing_condition(&model_refs)
        .add(<U::Entity as RangeRevColumns>::rev_deleted_column().is_null());
    let existing_with_rel = upserter.load_existing_with_rel(db, condition).await?;

    let mut existing_map: HashMap<U::Key, (ActiveModel<U>, U::RelData)> = HashMap::new();
    for (m, rel_data) in existing_with_rel {
        let am = m.into_active_model();
        let k = upserter.key(&am)?;
        existing_map.insert(k, (am, rel_data));
    }

    let mut new_map: HashMap<U::Key, (ActiveModel<U>, U::RelData)> = models_with_rel
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
        return Ok(Default::default());
    }
    let revision_id = revision.get(db).await?;

    if <PrimaryKey<U> as PrimaryKeyTrait>::ValueType::ARITY == 1 {
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
            let (am, _) = existing_map.get(k).expect("Existing map should have all models");
            am.clone().delete(db.txn().await?).await?;
        }
    }
    // get deleted + unchanged models from existing_map
    let deleted_models: Vec<Model<U>> = delete_keys
        .union(&update_keys)
        .map(|k| {
            let (am, _) = existing_map.remove(k).expect("Existing map should have all models");
            am.try_into_model().expect("Existing active model should be convertible into model")
        })
        .collect();
    let unchanged_models: Vec<Model<U>> = existing_map
        .into_values()
        .map(|(am, _)| {
            am.try_into_model().expect("Existing active model should be convertible into model")
        })
        .collect();

    let keys_to_insert: Vec<_> = insert_keys.union(&update_keys).cloned().collect();
    let created_models: Vec<Model<U>> = if !keys_to_insert.is_empty() {
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
                for col in PrimaryKey::<U>::iter() {
                    am.not_set(col.into_column());
                }
                insert_models.push(am);
                insert_rd.push(rd);
            }
        }

        let res = U::Entity::insert_many(insert_models).exec_with_returning_many(txn).await?;
        let inserted_with_rel = res.into_iter().zip(insert_rd.into_iter()).collect::<Vec<_>>();
        upserter.after_insert(db, &inserted_with_rel).await?;
        inserted_with_rel.into_iter().map(|(m, _)| m).collect()
    } else {
        Vec::new()
    };

    Ok((deleted_models, unchanged_models, created_models))
}

/// Upsert a single model with revisioning. Returns (deleted_model, upserted_model).
/// Checks that only one model is deleted/updated/inserted. If you want to replace possibly multiple
/// models with a single model, use upsert_rev_many.
pub async fn upsert_rev_one<U: Upserter>(
    db: &DbContext,
    revision: &LazyRevision,
    upserter: U,
    model: ActiveModel<U>,
    new_rel_data: U::RelData,
) -> anyhow::Result<(Option<Model<U>>, Model<U>)>
where
    Model<U>: IntoActiveModel<ActiveModel<U>>,
    U::Entity: RangeRevColumns,
    ActiveModel<U>: Send + TryIntoModel<Model<U>>,
{
    let (deleted, unchanged, created) =
        upsert_rev_many(db, revision, upserter, vec![(model, new_rel_data)]).await?;
    if !unchanged.is_empty() {
        if unchanged.len() > 1 {
            return Err(anyhow!("Expected at most one deleted model, got {}", deleted.len()));
        }
        if !deleted.is_empty() || !created.is_empty() {
            return Err(anyhow!("Model is both unchanged and deleted/created"));
        }
        Ok((None, unchanged.into_iter().next().expect("We checked that unchanged is not empty.")))
    } else {
        if deleted.len() > 1 {
            return Err(anyhow!("Expected at most one deleted model, got {}.", deleted.len()));
        }
        if created.len() != 1 {
            return Err(anyhow!(
                "Expected exactly one created model, got {}. There are no unchanged models.",
                deleted.len()
            ));
        }
        Ok((
            deleted.into_iter().next(),
            created.into_iter().next().expect("We checked that created is not empty."),
        ))
    }
}
