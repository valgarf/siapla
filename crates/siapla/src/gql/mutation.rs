use juniper::graphql_object;
use sea_orm::ActiveModelTrait;
use sea_orm::{ActiveValue, prelude::*};

use crate::entity::{booking, booking_resource};
use crate::entity::{resource_iteration as resource, task_iteration as task};
use crate::revisioning::{PlanState, create_revision};

use super::{
    context::Context,
    resource::{ResourceSaveInput, resource_save},
    task::{TaskSaveInput, task_save},
};

#[derive(Default)]
pub struct Mutation {}

#[graphql_object]
#[graphql(context = Context)]
impl Mutation {
    pub fn new() -> Self {
        Default::default()
    }

    async fn task_save(ctx: &Context, task: TaskSaveInput) -> anyhow::Result<task::Model> {
        let res = match task_save(ctx, task).await {
            Ok(res) => res,
            Err(err) => {
                ctx.failed().await;
                Err(err)?
            }
        };
        // notify modification channel
        ctx.app_state().notify_modified("graphql".to_string());
        Ok(res)
    }

    async fn task_delete(ctx: &Context, task_id: i32) -> anyhow::Result<bool> {
        let txn = ctx.txn().await?;
        let revision_id = create_revision(txn, PlanState::NotCalculated).await?;
        let res = task::Entity::update_many()
            .col_expr(task::Column::RevDeleted, Expr::value(Value::BigInt(Some(revision_id))))
            .filter(task::Column::Id.eq(task_id))
            .filter(task::Column::RevDeleted.is_null())
            .exec(txn)
            .await?;
        let ok = res.rows_affected > 0;
        if ok {
            ctx.app_state().notify_modified("graphql".to_string());
        }
        Ok(ok)
    }

    async fn resource_save(
        ctx: &Context,
        resource: ResourceSaveInput,
    ) -> anyhow::Result<resource::Model> {
        let res = match resource_save(ctx, resource).await {
            Ok(res) => res,
            Err(err) => {
                ctx.failed().await;
                Err(err)?
            }
        };
        ctx.app_state().notify_modified("graphql".to_string());
        Ok(res)
    }

    async fn resource_delete(ctx: &Context, resource_id: i32) -> anyhow::Result<bool> {
        let txn = ctx.txn().await?;
        let revision_id = create_revision(txn, PlanState::NotCalculated).await?;
        let res = resource::Entity::update_many()
            .col_expr(resource::Column::RevDeleted, Expr::value(Value::BigInt(Some(revision_id))))
            .filter(resource::Column::Id.eq(resource_id))
            .filter(resource::Column::RevDeleted.is_null())
            .exec(txn)
            .await?;
        let ok = res.rows_affected > 0;
        if ok {
            ctx.app_state().notify_modified("graphql".to_string());
        }
        Ok(ok)
    }

    async fn booking_save(
        ctx: &Context,
        db_id: Option<i32>,
        task_id: i32,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
        resources: Vec<i32>,
        r#final: bool,
    ) -> anyhow::Result<booking::Model> {
        let txn = ctx.txn().await?;
        let revision_id = create_revision(txn, PlanState::NotCalculated).await?;

        if let Some(existing_id) = db_id {
            let existing = booking::Entity::find_by_id(existing_id).one(txn).await?;
            let Some(existing) = existing else {
                return Err(anyhow::anyhow!("Booking {existing_id} does not exist"));
            };
            if existing.rev_deleted.is_some() {
                return Err(anyhow::anyhow!("Booking {existing_id} has already been deleted"));
            }
            let soft_delete = booking::Entity::update_many()
                .col_expr(booking::Column::RevDeleted, Expr::value(Value::BigInt(Some(revision_id))))
                .filter(booking::Column::Id.eq(existing_id))
                .filter(booking::Column::RevDeleted.is_null())
                .exec(txn)
                .await?;
            if soft_delete.rows_affected == 0 {
                return Err(anyhow::anyhow!(
                    "Booking {existing_id} was modified concurrently and is no longer active"
                ));
            }
        }

        let db_booking = booking::ActiveModel {
            id: ActiveValue::NotSet,
            task_id: ActiveValue::Set(task_id),
            start: ActiveValue::Set(start),
            end: ActiveValue::Set(end),
            r#final: ActiveValue::Set(r#final),
            rev_created: ActiveValue::Set(revision_id),
            rev_deleted: ActiveValue::Set(None),
        }
        .insert(txn)
        .await?;

        for rid in resources {
            let arm = booking_resource::ActiveModel {
                id: ActiveValue::NotSet,
                booking_id: ActiveValue::Set(db_booking.id),
                resource_id: ActiveValue::Set(rid),
            };
            arm.insert(txn).await?;
        }

        ctx.app_state().notify_modified("graphql".to_string());
        Ok(db_booking)
    }

    async fn booking_delete(ctx: &Context, db_id: i32) -> anyhow::Result<bool> {
        let txn = ctx.txn().await?;
        let revision_id = create_revision(txn, PlanState::NotCalculated).await?;
        let res = booking::Entity::update_many()
            .col_expr(booking::Column::RevDeleted, Expr::value(Value::BigInt(Some(revision_id))))
            .filter(booking::Column::Id.eq(db_id))
            .filter(booking::Column::RevDeleted.is_null())
            .exec(txn)
            .await?;
        let ok = res.rows_affected > 0;
        if ok {
            ctx.app_state().notify_modified("graphql".to_string());
        }
        Ok(ok)
    }

    /// Trigger a manual recalculation now
    async fn recalculate_now(ctx: &Context) -> anyhow::Result<bool> {
        ctx.app_state().trigger_manual();
        Ok(true)
    }
}
