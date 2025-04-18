use crate::entity::{holiday, task};

use super::{
    context::Context,
    holiday::{Country, GQLHoliday},
};
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

    async fn countries() -> Vec<Country> {
        super::holiday::countries()
            .iter()
            .map(|(code, name)| Country {
                name: name.clone(),
                isocode: code.clone(),
            })
            .collect()
    }

    async fn country(isocode: String) -> Option<Country> {
        super::holiday::countries()
            .get(&isocode)
            .map(|name| Country {
                name: name.clone(),
                isocode: isocode.clone(),
            })
    }

    async fn get_from_open_holidays(
        ctx: &Context,
        isocode: String,
    ) -> FieldResult<Option<GQLHoliday>> {
        let db = ctx.db().await?;
        let txn = db.begin().await?;
        let result = holiday::Model::get_from_open_holidays(&txn, isocode).await?;
        txn.commit().await?;

        Ok(Some(GQLHoliday::from_model(result)))
    }
}

impl Query {
    pub fn new() -> Self {
        Default::default()
    }
}
