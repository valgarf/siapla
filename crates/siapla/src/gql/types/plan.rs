use crate::{
    db::revisioning::active_for_revision,
    entity::allocation,
    gql::{allocation::GQLAllocation, context::Context, scalars::ExtendedScalarValue},
};
use juniper::graphql_object;
use sea_orm::{EntityTrait as _, QueryFilter as _, QueryOrder as _};

pub struct Plan {
    pub revision: Option<i64>,
}

#[graphql_object]
#[graphql(name = "Plan", context = Context, scalar = ExtendedScalarValue)]
impl Plan {
    pub async fn allocations(&self, ctx: &Context) -> anyhow::Result<Vec<GQLAllocation>> {
        // revision should only be None if there is no plan at all
        let Some(revision) = self.revision else {
            return Ok(Vec::new());
        };
        Ok(allocation::Entity::find()
            .filter(active_for_revision(
                allocation::Column::RevCreated,
                allocation::Column::RevDeleted,
                Some(revision),
            ))
            .order_by_asc(allocation::Column::TaskId)
            .all(ctx.db().txn().await?)
            .await?
            .into_iter()
            .map(|m| GQLAllocation::at_revision(m, revision))
            .collect())
    }
}
