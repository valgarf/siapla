use crate::entity::task;

use super::context::Context;
use juniper::{FieldResult, graphql_object};
use sea_orm::*;

#[derive(Default)]
pub struct Query;

#[graphql_object]
#[graphql(context = Context)]
impl Query {
    async fn hello_world() -> FieldResult<String> {
        // let tasks: Vec<task::Model> = task::Entity::find()
        //     .filter(task::Column::Title.contains("test"))
        //     .order_by_asc(task::Column::Title)
        //     .all(ctx.db().await?)
        //     .await?;
        Ok("Hello World from Juniper!".to_owned())
    }

    async fn tasks(ctx: &Context) -> FieldResult<Vec<task::Model>> {
        let res = task::Entity::find()
            .order_by_asc(task::Column::Title)
            .all(ctx.db().await?)
            .await?;
        Ok(res)
    }
}

impl Query {
    pub fn new() -> Self {
        Default::default()
    }
}
