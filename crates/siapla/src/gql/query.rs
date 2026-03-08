use crate::{
    entity::{holiday, issue, resource_iteration as resource, task_iteration as task},
    gql::plan::Plan,
    gql::scalars::Int64,
    revisioning::{active_for_revision, resolve_revision},
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
#[graphql(context = Context, scalar = crate::gql::scalars::MyScalarValue)]
impl Query {
    async fn hello_world() -> anyhow::Result<String> {
        // let tasks: Vec<task::Model> = task::Entity::find()
        //     .filter(task::Column::Title.contains("test"))
        //     .order_by_asc(task::Column::Title)
        //     .all(ctx.db().await?)
        //     .await?;
        Ok("Hello World from Juniper!".to_owned())
    }

    async fn tasks(ctx: &Context, revision: Option<Int64>) -> anyhow::Result<Vec<task::Model>> {
        let txn = ctx.txn().await?;
        let revision = resolve_revision(txn, revision.map(i64::from)).await?;
        let res = task::Entity::find()
            .filter(active_for_revision(
                task::Column::RevCreated,
                task::Column::RevDeleted,
                revision,
            )?)
            .order_by_asc(task::Column::Title)
            .all(txn)
            .await?;
        Ok(res)
    }

    async fn resources(
        ctx: &Context,
        revision: Option<Int64>,
    ) -> anyhow::Result<Vec<resource::Model>> {
        let txn = ctx.txn().await?;
        let revision = resolve_revision(txn, revision.map(i64::from)).await?;
        let res = resource::Entity::find()
            .filter(active_for_revision(
                resource::Column::RevCreated,
                resource::Column::RevDeleted,
                revision,
            )?)
            .order_by_asc(resource::Column::Name)
            .all(txn)
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

    async fn current_plan(_ctx: &Context, revision: Option<Int64>) -> anyhow::Result<Plan> {
        Ok(Plan { revision: revision.map(i64::from) })
    }

    async fn issues(ctx: &Context, revision: Option<Int64>) -> anyhow::Result<Vec<issue::Model>> {
        let tx = ctx.txn().await?;
        let revision = resolve_revision(tx, revision.map(i64::from)).await?;
        let Some(revision) = revision else {
            return Ok(Vec::new());
        };
        let res = issue::Entity::find()
            .filter(issue::Column::Revision.eq(revision))
            .order_by_asc(issue::Column::Id)
            .all(tx)
            .await?;
        Ok(res)
    }
}

impl Query {
    pub fn new() -> Self {
        Default::default()
    }
}
