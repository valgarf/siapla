use crate::db::DbContext;
use crate::db::entity::revision;
use anyhow::anyhow;
use sea_orm::EntityTrait;
use sea_orm::{
    ActiveModelTrait as _, ActiveValue, ColumnTrait, Condition, DatabaseTransaction,
    QueryFilter as _, QueryOrder as _,
};
use strum::{EnumString, IntoStaticStr};
use tokio::sync::OnceCell;

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
pub enum PlanState {
    #[strum(serialize = "NOT_CALCULATED")]
    NotCalculated,
    #[strum(serialize = "AVAILABLE")]
    Available,
    #[strum(serialize = "DELETED")]
    Deleted,
    #[strum(serialize = "MILESTONES_ONLY")]
    MilestonesOnly,
}

impl PlanState {
    pub fn as_str(self) -> &'static str {
        self.into()
    }
}

pub async fn latest_revision_id(txn: &DatabaseTransaction) -> anyhow::Result<Option<i64>> {
    let revision =
        revision::Entity::find().order_by_desc(revision::Column::Id).one(txn).await?.map(|r| r.id);
    match revision {
        Some(revision_id) => Ok(Some(revision_id)),
        None => Ok(None),
    }
}

pub async fn ensure_revision_id(txn: &DatabaseTransaction) -> anyhow::Result<i64> {
    if let Some(revision_id) = latest_revision_id(txn).await? {
        return Ok(revision_id);
    }
    create_revision(txn, PlanState::NotCalculated).await
}

pub async fn create_revision(
    txn: &DatabaseTransaction,
    plan_state: PlanState,
) -> anyhow::Result<i64> {
    let model = revision::ActiveModel {
        id: ActiveValue::NotSet,
        timestamp: ActiveValue::Set(chrono::Utc::now()),
        plan_state: ActiveValue::Set(plan_state.as_str().to_string()),
    }
    .insert(txn)
    .await?;
    Ok(model.id)
}

pub async fn resolve_revision(
    txn: &DatabaseTransaction,
    revision: Option<i64>,
) -> anyhow::Result<Option<i64>> {
    if revision.is_some() { Ok(revision) } else { latest_revision_id(txn).await }
}

/// Like `resolve_revision`, but when no explicit revision is given, returns the
/// latest revision whose `plan_state` is `AVAILABLE`. This ensures that plan
/// queries keep showing the last successfully calculated plan even after
/// mutations create newer `NOT_CALCULATED` revisions.
pub async fn resolve_plan_revision(
    txn: &DatabaseTransaction,
    revision: Option<i64>,
) -> anyhow::Result<Option<i64>> {
    if revision.is_some() {
        return Ok(revision);
    }
    let rev = revision::Entity::find()
        .filter(revision::Column::PlanState.eq(PlanState::Available.as_str()))
        .order_by_desc(revision::Column::Id)
        .one(txn)
        .await?;
    Ok(rev.map(|r| r.id))
}

pub fn active_for_revision<C: ColumnTrait>(
    rev_created_column: C,
    rev_deleted_column: C,
    revision: Option<i64>,
) -> Condition {
    match revision {
        Some(rev) => Condition::all().add(rev_created_column.lte(rev)).add(
            Condition::any().add(rev_deleted_column.is_null()).add(rev_deleted_column.gt(rev)),
        ),
        None => Condition::all().add(rev_deleted_column.is_null()),
    }
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

    pub async fn resolve(self, db: &DbContext) -> anyhow::Result<(bool, i64)> {
        if let Some(revision_id) = self.take() {
            Ok((true, revision_id))
        } else {
            let revision_id = latest_revision_id(db.txn().await?)
                .await?
                .ok_or(anyhow!("Cannot find the latest revision."))?;
            Ok((false, revision_id))
        }
    }
}
