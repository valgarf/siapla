use crate::{
    entity::{holiday, resource, task},
    gql::plan::Plan,
};

use super::{
    context::Context,
    holiday::{Country, GQLHoliday, Region},
};
use juniper::graphql_object;
use sea_orm::*;

#[derive(Default)]
pub struct Query;

#[graphql_object]
#[graphql(context = Context)]
impl Query {
    async fn hello_world() -> anyhow::Result<String> {
        // let tasks: Vec<task::Model> = task::Entity::find()
        //     .filter(task::Column::Title.contains("test"))
        //     .order_by_asc(task::Column::Title)
        //     .all(ctx.db().await?)
        //     .await?;
        Ok("Hello World from Juniper!".to_owned())
    }

    async fn tasks(ctx: &Context) -> anyhow::Result<Vec<task::Model>> {
        let res =
            task::Entity::find().order_by_asc(task::Column::Title).all(ctx.txn().await?).await?;
        Ok(res)
    }

    async fn resources(ctx: &Context) -> anyhow::Result<Vec<resource::Model>> {
        let res = resource::Entity::find()
            .order_by_asc(resource::Column::Name)
            .all(ctx.txn().await?)
            .await?;
        Ok(res)
    }

    async fn countries() -> Vec<Country> {
        super::holiday::countries()
            .iter()
            .map(|(code, name)| Country { name: name.clone(), isocode: code.clone() })
            .collect()
    }

    async fn country(isocode: String) -> Option<Country> {
        super::holiday::countries()
            .get(&isocode)
            .map(|name| Country { name: name.clone(), isocode: isocode.clone() })
    }

    async fn region(isocode: String, ctx: &Context) -> anyhow::Result<Option<Region>> {
        let country = super::holiday::countries()
            .get(&isocode[0..2])
            .map(|name| Country { name: name.clone(), isocode: isocode[0..2].to_owned() });
        let country = match country {
            Some(country) => country,
            None => return Ok(None),
        };
        Ok(country.regions(ctx).await?.iter().find(|r| r.isocode == isocode).cloned())
    }

    async fn get_from_open_holidays(
        ctx: &Context,
        isocode: String,
    ) -> anyhow::Result<Option<GQLHoliday>> {
        let txn = ctx.txn().await?;
        let result = holiday::Model::get_from_open_holidays(txn, isocode).await?;

        Ok(Some(GQLHoliday::from_model(result)))
    }

    async fn current_plan(_ctx: &Context) -> Plan {
        Plan {}
    }
}

impl Query {
    pub fn new() -> Self {
        Default::default()
    }
}
