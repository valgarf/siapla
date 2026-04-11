use crate::RangeRevColumns;
use crate::db::DbContext;
use crate::db::delete::delete_rev_by_pk;
use crate::db::entity::revision;
use crate::revisioning::PlanState;
use anyhow::anyhow;
use sea_orm::{ActiveValue, prelude::*};
use sea_orm::{EntityTrait, IntoActiveModel, PrimaryKeyTrait};
use sea_query::ValueTuple;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use tokio::sync::OnceCell;

pub trait Upserter {
    type Entity: EntityTrait;
    type Key: Hash + Eq + Clone;
    fn existing_condition(&self) -> sea_orm::Condition;
    fn key(&self, model: &<Self::Entity as EntityTrait>::ActiveModel) -> anyhow::Result<Self::Key>;
    fn model_equal(
        &self,
        lhs: &<Self::Entity as EntityTrait>::ActiveModel,
        rhs: &<Self::Entity as EntityTrait>::ActiveModel,
    ) -> bool;
}

#[derive(Debug, Default)]
pub struct LazyRevision {
    revision: OnceCell<i64>,
}

impl LazyRevision {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn from_revision(revision_id: i64) -> Self {
        Self { revision: OnceCell::from(revision_id) }
    }
    pub async fn get(&self, db: &DbContext) -> anyhow::Result<i64> {
        self.revision
            .get_or_try_init(|| async {
                let rev_model = revision::Entity::insert(revision::ActiveModel {
                    timestamp: ActiveValue::Set(chrono::Utc::now()),
                    plan_state: ActiveValue::Set(PlanState::NotCalculated.as_str().to_string()),
                    ..Default::default()
                })
                .exec(db.txn().await?)
                .await?;
                Ok(rev_model.last_insert_id.into())
            })
            .await
            .copied()
    }

    pub fn take(self) -> Option<i64> {
        self.revision.into_inner()
    }
}

pub async fn upsert_rev<U: Upserter>(
    db: &DbContext,
    revision: &LazyRevision,
    upserter: U,
    models: Vec<<U::Entity as EntityTrait>::ActiveModel>,
) -> anyhow::Result<()>
where
    <U::Entity as EntityTrait>::Model: IntoActiveModel<<U::Entity as EntityTrait>::ActiveModel>,
    U::Entity: RangeRevColumns,
    <U::Entity as EntityTrait>::ActiveModel: Send,
{
    let txn = db.txn().await?;
    let existing_models = U::Entity::find()
        .filter(upserter.existing_condition())
        .filter(<U::Entity as RangeRevColumns>::rev_deleted_column().is_null())
        .all(txn)
        .await?
        .into_iter()
        .map(|m| m.into_active_model());

    let mut existing_map: HashMap<_, _> =
        existing_models.map(|m| upserter.key(&m).map(|k| (k, m))).collect::<anyhow::Result<_>>()?;
    let mut new_map: HashMap<_, _> = models
        .into_iter()
        .map(|m| upserter.key(&m).map(|k| (k, m)))
        .collect::<anyhow::Result<_>>()?;
    let existing_keys: HashSet<_> = existing_map.keys().cloned().collect();
    let new_keys: HashSet<_> = new_map.keys().cloned().collect();

    let delete_keys: HashSet<_> = existing_keys.difference(&new_keys).cloned().collect();
    let insert_keys: HashSet<_> = new_keys.difference(&existing_keys).cloned().collect();
    let update_keys: HashSet<_> = existing_keys
        .intersection(&new_keys)
        .filter(|k| !upserter.model_equal(&existing_map[k], &new_map[k]))
        .cloned()
        .collect();

    if delete_keys.is_empty() && insert_keys.is_empty() && update_keys.is_empty() {
        return Ok(());
    }
    let revision_id = revision.get(db).await?;

    if <<U::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType::ARITY == 1 {
        let delete_pks: Vec<_> = delete_keys
            .union(&update_keys)
            .map(|k| existing_map[k].get_primary_key_value())
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
                v.delete(db.txn().await?).await?;
            }
        }
    }

    let mut insert_models: Vec<_> = insert_keys
        .union(&update_keys)
        .filter_map(|k| new_map.remove_entry(k))
        .map(|(_, v)| v)
        .collect();
    if !insert_models.is_empty() {
        for model in &mut insert_models {
            model.set(<U::Entity as RangeRevColumns>::rev_created_column(), revision_id.into());
            model.set(
                <U::Entity as RangeRevColumns>::rev_deleted_column(),
                (None as Option<i64>).into(),
            );
        }
        U::Entity::insert_many(insert_models).exec(txn).await?;
    }

    Ok(())
}
