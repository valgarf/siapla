use juniper::graphql_object;
use sea_orm::ActiveModelTrait;
use sea_orm::{
    ActiveValue, ColumnTrait as _, Condition, ConnectionTrait as _, EntityTrait as _,
    QueryFilter as _, prelude::*,
};

use crate::entity::{
    allocated_resource, allocation, availability, dependency, issue, resource_constraint,
    resource_constraint_entry, resource_header, revision, task_header, vacation,
};
use crate::entity::{booking, booking_resource};
use crate::entity::{resource_iteration, task_iteration};
use crate::gql::scalars::ExtendedScalarValue;
use crate::revisioning::{PlanState, create_revision};

use super::{
    booking::GQLBooking,
    context::Context,
    resource::{GQLResource, ResourceSaveInput, resource_save},
    task::{GQLTask, TaskSaveInput, task_save},
};

#[derive(Default)]
pub struct Mutation {}

#[graphql_object]
#[graphql(context = Context, scalar = ExtendedScalarValue)]
impl Mutation {
    pub fn new() -> Self {
        Default::default()
    }

    async fn task_save(ctx: &Context, task: TaskSaveInput) -> anyhow::Result<GQLTask> {
        let (res, revision) = match task_save(ctx, task).await {
            Ok(res) => res,
            Err(err) => {
                ctx.failed().await;
                Err(err)?
            }
        };
        ctx.app_state().notify_modified("graphql".to_string());
        Ok(GQLTask::at_revision(res, revision))
    }

    /// Delete a task by its **header id** (stable identity).
    async fn task_delete(ctx: &Context, task_id: i32) -> anyhow::Result<bool> {
        let txn = ctx.txn().await?;
        let revision_id = create_revision(txn, PlanState::NotCalculated).await?;
        let res = task_iteration::Entity::update_many()
            .col_expr(
                task_iteration::Column::RevDeleted,
                Expr::value(Value::BigInt(Some(revision_id))),
            )
            .filter(task_iteration::Column::HeaderId.eq(task_id))
            .filter(task_iteration::Column::RevDeleted.is_null())
            .exec(txn)
            .await?;
        let ok = res.rows_affected > 0;
        if ok {
            dependency::Entity::update_many()
                .col_expr(
                    dependency::Column::RevDeleted,
                    Expr::value(Value::BigInt(Some(revision_id))),
                )
                .filter(
                    Condition::any()
                        .add(dependency::Column::PredecessorId.eq(task_id))
                        .add(dependency::Column::SuccessorId.eq(task_id)),
                )
                .filter(dependency::Column::RevDeleted.is_null())
                .exec(txn)
                .await?;
            resource_constraint::Entity::update_many()
                .col_expr(
                    resource_constraint::Column::RevDeleted,
                    Expr::value(Value::BigInt(Some(revision_id))),
                )
                .filter(resource_constraint::Column::TaskId.eq(task_id))
                .filter(resource_constraint::Column::RevDeleted.is_null())
                .exec(txn)
                .await?;
            ctx.app_state().notify_modified("graphql".to_string());
        }
        Ok(ok)
    }

    async fn resource_save(
        ctx: &Context,
        resource: ResourceSaveInput,
    ) -> anyhow::Result<GQLResource> {
        let (res, revision_id) = match resource_save(ctx, resource).await {
            Ok(res) => res,
            Err(err) => {
                ctx.failed().await;
                Err(err)?
            }
        };
        ctx.app_state().notify_modified("graphql".to_string());
        Ok(GQLResource::at_revision(res, revision_id))
    }

    async fn resource_delete(ctx: &Context, resource_id: i32) -> anyhow::Result<bool> {
        let txn = ctx.txn().await?;
        let revision_id = create_revision(txn, PlanState::NotCalculated).await?;
        let res = resource_iteration::Entity::update_many()
            .col_expr(
                resource_iteration::Column::RevDeleted,
                Expr::value(Value::BigInt(Some(revision_id))),
            )
            .filter(resource_iteration::Column::HeaderId.eq(resource_id))
            .filter(resource_iteration::Column::RevDeleted.is_null())
            .exec(txn)
            .await?;
        let ok = res.rows_affected > 0;
        if ok {
            availability::Entity::update_many()
                .col_expr(
                    availability::Column::RevDeleted,
                    Expr::value(Value::BigInt(Some(revision_id))),
                )
                .filter(availability::Column::ResourceId.eq(resource_id))
                .filter(availability::Column::RevDeleted.is_null())
                .exec(txn)
                .await?;
            vacation::Entity::update_many()
                .col_expr(
                    vacation::Column::RevDeleted,
                    Expr::value(Value::BigInt(Some(revision_id))),
                )
                .filter(vacation::Column::ResourceId.eq(resource_id))
                .filter(vacation::Column::RevDeleted.is_null())
                .exec(txn)
                .await?;
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
    ) -> anyhow::Result<GQLBooking> {
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
                .col_expr(
                    booking::Column::RevDeleted,
                    Expr::value(Value::BigInt(Some(revision_id))),
                )
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

        // Store booking.task_id as the stable task header id. Accept both
        // header ids and iteration ids at the API boundary for compatibility.
        let resolved_header_id = if let Some(active_task) = task_iteration::Entity::find()
            .filter(task_iteration::Column::HeaderId.eq(task_id))
            .filter(task_iteration::Column::RevDeleted.is_null())
            .one(txn)
            .await?
        {
            active_task.header_id
        } else if let Some(active_task) = task_iteration::Entity::find_by_id(task_id)
            .filter(task_iteration::Column::RevDeleted.is_null())
            .one(txn)
            .await?
        {
            active_task.header_id
        } else {
            return Err(anyhow::anyhow!(
                "No active task iteration found for task header {task_id}"
            ));
        };

        let db_booking = booking::ActiveModel {
            id: ActiveValue::NotSet,
            task_id: ActiveValue::Set(resolved_header_id),
            start: ActiveValue::Set(start),
            end: ActiveValue::Set(end),
            r#final: ActiveValue::Set(r#final),
            rev_created: ActiveValue::Set(revision_id),
            rev_deleted: ActiveValue::Set(None),
        }
        .insert(txn)
        .await?;

        for rid in resources {
            let resolved_resource_header_id = if let Some(active_resource) =
                resource_iteration::Entity::find()
                    .filter(resource_iteration::Column::HeaderId.eq(rid))
                    .filter(resource_iteration::Column::RevDeleted.is_null())
                    .one(txn)
                    .await?
            {
                active_resource.header_id
            } else if let Some(active_resource) = resource_iteration::Entity::find_by_id(rid)
                .filter(resource_iteration::Column::RevDeleted.is_null())
                .one(txn)
                .await?
            {
                active_resource.header_id
            } else {
                return Err(anyhow::anyhow!(
                    "No active resource iteration found for resource header {rid}"
                ));
            };

            let arm = booking_resource::ActiveModel {
                id: ActiveValue::NotSet,
                booking_id: ActiveValue::Set(db_booking.id),
                resource_id: ActiveValue::Set(resolved_resource_header_id),
            };
            arm.insert(txn).await?;
        }

        ctx.app_state().notify_modified("graphql".to_string());
        Ok(GQLBooking::at_revision(db_booking, revision_id))
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

    /// Reset the database by hard-deleting all data and creating a fresh revision.
    /// Only available when the server is started with `--allow-reset`.
    async fn reset_database(ctx: &Context) -> anyhow::Result<bool> {
        if !ctx.app_state().allow_reset {
            return Err(anyhow::anyhow!(
                "resetDatabase is disabled. Start the server with --allow-reset to enable it."
            ));
        }
        let txn = ctx.txn().await?;
        // Delete in order respecting foreign key constraints
        allocated_resource::Entity::delete_many().exec(txn).await?;
        allocation::Entity::delete_many().exec(txn).await?;
        issue::Entity::delete_many().exec(txn).await?;
        resource_constraint_entry::Entity::delete_many().exec(txn).await?;
        resource_constraint::Entity::delete_many().exec(txn).await?;
        booking_resource::Entity::delete_many().exec(txn).await?;
        booking::Entity::delete_many().exec(txn).await?;
        dependency::Entity::delete_many().exec(txn).await?;
        vacation::Entity::delete_many().exec(txn).await?;
        availability::Entity::delete_many().exec(txn).await?;
        task_iteration::Entity::delete_many().exec(txn).await?;
        resource_iteration::Entity::delete_many().exec(txn).await?;
        task_header::Entity::delete_many().exec(txn).await?;
        resource_header::Entity::delete_many().exec(txn).await?;
        revision::Entity::delete_many().exec(txn).await?;
        // Reset AUTOINCREMENT counters so header and iteration IDs stay aligned
        txn.execute_unprepared("DELETE FROM sqlite_sequence").await?;
        // Create a fresh initial revision
        create_revision(txn, PlanState::NotCalculated).await?;
        Ok(true)
    }
}
