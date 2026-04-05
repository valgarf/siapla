use chrono::{DateTime, Utc};

use crate::{
    entity::{holiday, resource_iteration, task_iteration},
    gql::scalars::Int64,
    revisioning::{active_for_revision, resolve_plan_revision, resolve_revision},
};

use super::{
    booking::GQLBooking,
    context::Context,
    history::{SearchDirection, TaskHistoryResult},
    holiday::{Country, GQLHoliday, Region},
    issue::GQLIssue,
    plan::Plan,
    resource::GQLResource,
    task::GQLTask,
};
use juniper::graphql_object;
use sea_orm::*;

#[derive(Default)]
pub struct Query;

#[graphql_object]
#[graphql(context = Context, scalar = crate::gql::scalars::MyScalarValue)]
impl Query {
    async fn hello_world() -> anyhow::Result<String> {
        // let tasks: Vec<task_iteration::Model> = task_iteration::Entity::find()
        //     .filter(task_iteration::Column::Title.contains("test"))
        //     .order_by_asc(task_iteration::Column::Title)
        //     .all(ctx.db().await?)
        //     .await?;
        Ok("Hello World from Juniper!".to_owned())
    }

    async fn tasks(ctx: &Context, revision: Option<Int64>) -> anyhow::Result<Vec<GQLTask>> {
        let txn = ctx.txn().await?;
        let revision = resolve_revision(txn, revision.map(i64::from))
            .await?
            .ok_or(anyhow::anyhow!("No revision found in database"))?;
        let res = task_iteration::Entity::find()
            .filter(active_for_revision(
                task_iteration::Column::RevCreated,
                task_iteration::Column::RevDeleted,
                Some(revision),
            ))
            .order_by_asc(task_iteration::Column::Title)
            .all(txn)
            .await?;
        Ok(res.into_iter().map(|m| GQLTask::at_revision(m, revision)).collect())
    }

    async fn resources(ctx: &Context, revision: Option<Int64>) -> anyhow::Result<Vec<GQLResource>> {
        let txn = ctx.txn().await?;
        let revision = resolve_revision(txn, revision.map(i64::from))
            .await?
            .ok_or(anyhow::anyhow!("No revision found in database"))?;
        let res = resource_iteration::Entity::find()
            .filter(active_for_revision(
                resource_iteration::Column::RevCreated,
                resource_iteration::Column::RevDeleted,
                Some(revision),
            ))
            .order_by_asc(resource_iteration::Column::Name)
            .all(txn)
            .await?;
        Ok(res.into_iter().map(|m| GQLResource::at_revision(m, revision)).collect())
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

    async fn current_plan(ctx: &Context, revision: Option<Int64>) -> anyhow::Result<Plan> {
        let txn = ctx.txn().await?;
        let revision = resolve_plan_revision(txn, revision.map(i64::from)).await?;
        Ok(Plan { revision })
    }

    async fn bookings(ctx: &Context, revision: Option<Int64>) -> anyhow::Result<Vec<GQLBooking>> {
        let txn = ctx.txn().await?;
        let revision = resolve_revision(txn, revision.map(i64::from))
            .await?
            .ok_or(anyhow::anyhow!("No revision found in database"))?;
        let res = crate::entity::booking::Entity::find()
            .filter(active_for_revision(
                crate::entity::booking::Column::RevCreated,
                crate::entity::booking::Column::RevDeleted,
                Some(revision),
            ))
            .order_by_asc(crate::entity::booking::Column::TaskId)
            .order_by_asc(crate::entity::booking::Column::Start)
            .all(txn)
            .await?;
        Ok(res.into_iter().map(|m| GQLBooking::at_revision(m, revision)).collect())
    }

    async fn issues(ctx: &Context, revision: Option<Int64>) -> anyhow::Result<Vec<GQLIssue>> {
        let tx = ctx.txn().await?;
        let revision = resolve_plan_revision(tx, revision.map(i64::from)).await?;
        let Some(revision) = revision else {
            return Ok(Vec::new());
        };
        let res = crate::entity::issue::Entity::find()
            .filter(active_for_revision(
                crate::entity::issue::Column::RevCreated,
                crate::entity::issue::Column::RevDeleted,
                Some(revision),
            ))
            .order_by_asc(crate::entity::issue::Column::Id)
            .all(tx)
            .await?;
        Ok(res.into_iter().map(|m| GQLIssue::at_revision(m, revision)).collect())
    }

    async fn latest_revision(ctx: &Context) -> anyhow::Result<Option<Int64>> {
        let txn = ctx.txn().await?;
        let rev = crate::revisioning::latest_revision_id(txn).await?;
        Ok(rev.map(Int64::from))
    }

    async fn task_history(
        ctx: &Context,
        task_header_id: i32,
        from_revision: Option<Int64>,
        from_timestamp: Option<DateTime<Utc>>,
        direction: SearchDirection,
        limit: Option<i32>,
    ) -> anyhow::Result<TaskHistoryResult> {
        super::history::query_task_history(
            ctx,
            task_header_id,
            from_revision,
            from_timestamp,
            direction,
            limit,
        )
        .await
    }
}

impl Query {
    pub fn new() -> Self {
        Default::default()
    }
}
