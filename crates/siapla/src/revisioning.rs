use sea_orm::{
    ActiveModelTrait as _, ActiveValue, ColumnTrait, Condition, DatabaseTransaction,
    EntityTrait as _, QueryOrder as _,
};
use strum::{EnumString, IntoStaticStr};

use crate::entity::revision;

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

pub fn db_to_revision_id(revision_id: i32) -> anyhow::Result<u64> {
    u64::try_from(revision_id)
        .map_err(|_| anyhow::anyhow!("Revision id {} cannot be converted to u64", revision_id))
}

pub fn revision_to_db_id(revision_id: u64) -> u64 {
    revision_id
}

pub async fn latest_revision_id(txn: &DatabaseTransaction) -> anyhow::Result<Option<u64>> {
    let revision =
        revision::Entity::find().order_by_desc(revision::Column::Id).one(txn).await?.map(|r| r.id);
    match revision {
        Some(revision_id) => Ok(Some(revision_id)),
        None => Ok(None),
    }
}

pub async fn ensure_revision_id(txn: &DatabaseTransaction) -> anyhow::Result<u64> {
    if let Some(revision_id) = latest_revision_id(txn).await? {
        return Ok(revision_id);
    }
    create_revision(txn, PlanState::NotCalculated).await
}

pub async fn create_revision(
    txn: &DatabaseTransaction,
    plan_state: PlanState,
) -> anyhow::Result<u64> {
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
    revision: Option<u64>,
) -> anyhow::Result<Option<u64>> {
    if revision.is_some() { Ok(revision) } else { latest_revision_id(txn).await }
}

pub async fn resolve_revision_arg_i32(
    txn: &DatabaseTransaction,
    revision: Option<i32>,
) -> anyhow::Result<Option<u64>> {
    let revision = revision.map(db_to_revision_id).transpose()?;
    resolve_revision(txn, revision).await
}

pub fn active_for_revision<C: ColumnTrait>(
    rev_created_column: C,
    rev_deleted_column: C,
    revision: Option<u64>,
) -> anyhow::Result<Condition> {
    match revision {
        Some(rev) => {
            let db_rev = revision_to_db_id(rev);
            Ok(Condition::all().add(rev_created_column.lte(db_rev)).add(
                Condition::any()
                    .add(rev_deleted_column.is_null())
                    .add(rev_deleted_column.gt(db_rev)),
            ))
        }
        None => Ok(Condition::all().add(rev_deleted_column.is_null())),
    }
}
