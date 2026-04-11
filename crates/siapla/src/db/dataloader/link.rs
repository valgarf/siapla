use sea_orm::{
    ColumnTrait, EntityTrait, JoinType, ModelTrait, QueryFilter, QuerySelect as _, RelationDef,
};
use std::{any::type_name, collections::HashMap};

use super::base::{Batcher, BatcherToKey};
use crate::db::context::DbContext;
use crate::db::revisioning::active_for_revision;
use crate::db::{Link, RangeRevColumns};

use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct LinkBatcher<L: Link> {
    pub revision: i64,
    _phantom: std::marker::PhantomData<L>,
}

impl<L: Link> LinkBatcher<L> {
    pub fn new(revision: i64) -> Self {
        Self { revision, _phantom: std::marker::PhantomData }
    }
}

impl<L: Link> Batcher for LinkBatcher<L>
where
    <L::LinkEntity as EntityTrait>::Model: Send + Sync + Clone,
    <L::TargetEntity as EntityTrait>::Model: Send + Sync + Clone,
    L::TargetEntity: RangeRevColumns,
    L::LinkEntity: EntityTrait + Default,
{
    type Key = sea_orm::Value;
    type Value = Vec<<L::TargetEntity as EntityTrait>::Model>;

    async fn load(
        &self,
        db: &DbContext,
        keys: &[Self::Key],
    ) -> Result<HashMap<Self::Key, Self::Value>, anyhow::Error> {
        let txn = db.txn().await?;
        let relation_def: RelationDef = L::TargetEntity::belongs_to(L::LinkEntity::default())
            .from(L::join_target_column())
            .to(L::join_link_column())
            .into();
        let query = L::LinkEntity::find().filter(L::filter_column().is_in(keys.to_vec()));
        let query = if let (Some(rev_created_col), Some(rev_deleted_col)) =
            (L::link_rev_created_column(), L::link_rev_deleted_column())
        {
            query.filter(active_for_revision(rev_created_col, rev_deleted_col, Some(self.revision)))
        } else {
            query
        };
        let query = query
            .join_rev(JoinType::InnerJoin, relation_def)
            .select_also(L::TargetEntity::default())
            .filter(<L::TargetEntity as RangeRevColumns>::condition(self.revision));

        let targets: Vec<(
            <L::LinkEntity as EntityTrait>::Model,
            Option<<L::TargetEntity as EntityTrait>::Model>,
        )> = query.all(txn).await?;

        let mut res: HashMap<sea_orm::Value, Self::Value> = HashMap::new();
        for (link_model, target) in targets.into_iter() {
            if let Some(target) = target {
                let source_key: sea_orm::Value = link_model.get(L::filter_column());
                res.entry(source_key).or_default().push(target);
            }
        }

        for k in keys {
            res.entry(k.clone()).or_default();
        }

        Ok(res)
    }
}

impl<L: Link> BatcherToKey for LinkBatcher<L>
where
    <L::LinkEntity as EntityTrait>::Model: Send + Sync + Clone,
    <L::TargetEntity as EntityTrait>::Model: Send + Sync + Clone,
    L::TargetEntity: RangeRevColumns,
{
    type MapKey = (&'static str, i64);
    fn loader_map_key(&self) -> Self::MapKey {
        (type_name::<L>(), self.revision)
    }
}
