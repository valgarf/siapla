use anyhow::anyhow;
use juniper::graphql_object;
use sea_orm::{
    ActiveValue, ColumnTrait as _, Condition, ConnectionTrait as _, EntityTrait as _,
    QueryFilter as _, prelude::*,
};

use crate::db::delete::delete_rev_by_pk;
use crate::db::r#impl::booking::BookingUpserter;
use crate::db::revisioning::LazyRevision;
use crate::db::revisioning::{PlanState, create_revision, resolve_revision};
use crate::db::upsert::upsert_rev_one;
use crate::entity::{
    allocated_resource, allocation, availability, dependency, issue, resource_constraint,
    resource_constraint_entry, resource_header, revision, task_header, vacation,
};
use crate::entity::{booking, booking_resource};
use crate::entity::{resource_iteration, task_iteration};
use crate::gql::scalars::ExtendedScalarValue;

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
        let revision = LazyRevision::new();
        let header_id = match resource_save(ctx.db(), &revision, resource).await {
            Ok(res) => res,
            Err(err) => {
                ctx.failed().await;
                Err(err)?
            }
        };
        let opt_revision = revision.take();

        if opt_revision.is_some() {
            ctx.app_state().notify_modified("graphql".to_string());
        }
        let revision_id =
            resolve_revision(ctx.txn().await?, opt_revision).await?.ok_or(anyhow!(
                "We wanted to save a resource and cannot find a revision. This should never happen"
            ))?;
        let model = resource_iteration::Model::dataloader_by_header_at_rev(
            ctx.db(),
            header_id,
            revision_id,
        )
        .await?;
        Ok(GQLResource::at_revision(model, revision_id))
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
        mut resources: Vec<i32>,
        r#final: bool,
    ) -> anyhow::Result<GQLBooking> {
        let revision = LazyRevision::new();
        let txn = ctx.txn().await?;

        if let Some(existing_id) = db_id {
            // TODO: is this check even necessary? We could just insert it, the returned booking is
            // new anyway if it changed anything.
            let existing = booking::Entity::find_by_id(existing_id).one(txn).await?;
            let Some(existing) = existing else {
                return Err(anyhow::anyhow!("Booking {existing_id} does not exist"));
            };
            if existing.rev_deleted.is_some() {
                return Err(anyhow::anyhow!("Booking {existing_id} has already been deleted"));
            }
        }

        let upserter = BookingUpserter::new(task_id, db_id);
        let model = booking::ActiveModel {
            id: ActiveValue::NotSet,
            task_id: ActiveValue::Set(task_id),
            start: ActiveValue::Set(start),
            end: ActiveValue::Set(end),
            r#final: ActiveValue::Set(r#final),
            rev_created: ActiveValue::NotSet,
            rev_deleted: ActiveValue::NotSet,
        };

        resources.sort();
        let (_, db_booking) =
            upsert_rev_one(ctx.db(), &revision, upserter, model, resources).await?;

        let opt_revision = revision.take();
        if opt_revision.is_some() {
            ctx.app_state().notify_modified("graphql".to_string());
        }
        let revision_id = resolve_revision(ctx.txn().await?, opt_revision)
            .await?
            .ok_or(anyhow!("Cannot find a revision for the booking save operation"))?;

        Ok(GQLBooking::at_revision(db_booking, revision_id))
    }

    async fn booking_delete(ctx: &Context, db_id: i32) -> anyhow::Result<bool> {
        let res = delete_rev_by_pk::<booking::Entity>(ctx.db(), &LazyRevision::new(), vec![db_id])
            .await?;

        let ok = res > 0;
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
