use crate::{
    entity::allocation,
    gql::{allocation::GQLAllocation, context::Context},
    revisioning::active_for_revision,
};
use juniper::graphql_object;
use sea_orm::{EntityTrait as _, QueryFilter as _, QueryOrder as _};

pub struct Plan {
    pub revision: Option<i64>,
}

#[graphql_object]
#[graphql(name = "Plan")]
impl Plan {
    pub async fn allocations(&self, ctx: &Context) -> anyhow::Result<Vec<GQLAllocation>> {
        let txn = ctx.txn().await?;
        let revision = crate::revisioning::resolve_plan_revision(txn, self.revision).await?;
        let Some(revision) = revision else {
            return Ok(Vec::new());
        };
        Ok(allocation::Entity::find()
            .filter(active_for_revision(
                allocation::Column::RevCreated,
                allocation::Column::RevDeleted,
                Some(revision),
            ))
            .order_by_asc(allocation::Column::TaskId)
            .all(txn)
            .await?
            .into_iter()
            .map(|m| GQLAllocation::at_revision(m, Some(revision)))
            .collect())
    }
}
