use crate::{entity::allocation, gql::context::Context};
use juniper::graphql_object;
use sea_orm::{EntityTrait as _, QueryOrder as _};

pub struct Plan {}

#[graphql_object]
#[graphql(name = "Plan")]
impl Plan {
    pub async fn allocations(&self, ctx: &Context) -> anyhow::Result<Vec<allocation::Model>> {
        Ok(allocation::Entity::find()
            .order_by_asc(allocation::Column::TaskId)
            .all(ctx.txn().await?)
            .await?)
    }
}
