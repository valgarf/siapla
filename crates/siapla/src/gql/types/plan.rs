use crate::{entity::allocation, gql::context::Context};
use juniper::graphql_object;
use sea_orm::{ColumnTrait as _, EntityTrait as _, QueryFilter as _, QueryOrder as _};

pub struct Plan {
    pub revision: Option<i64>,
}

#[graphql_object]
#[graphql(name = "Plan")]
impl Plan {
    pub async fn allocations(&self, ctx: &Context) -> anyhow::Result<Vec<allocation::Model>> {
        let txn = ctx.txn().await?;
        let revision = crate::revisioning::resolve_revision(txn, self.revision).await?;
        let Some(revision) = revision else {
            return Ok(Vec::new());
        };
        Ok(allocation::Entity::find()
            .filter(allocation::Column::Revision.eq(revision))
            .order_by_asc(allocation::Column::TaskId)
            .all(txn)
            .await?)
    }
}
