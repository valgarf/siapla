use crate::gql::common::resolve_many_to_many;
use crate::{
    entity::{booking, booking_resource, resource_iteration as resource, task_iteration as task},
    gql::context::Context,
};
use chrono::{DateTime, Utc};
use juniper::graphql_object;

use super::resource::GQLResource;
use super::task::GQLTask;

pub struct GQLBooking {
    pub model: booking::Model,
    pub revision: Option<i64>,
}

impl GQLBooking {
    pub fn at_revision(model: booking::Model, revision: Option<i64>) -> Self {
        Self { model, revision }
    }
}

impl From<booking::Model> for GQLBooking {
    fn from(model: booking::Model) -> Self {
        Self { model, revision: None }
    }
}

#[graphql_object]
#[graphql(name = "Booking")]
impl GQLBooking {
    fn db_id(&self) -> &i32 {
        &self.model.id
    }

    fn start(&self) -> &DateTime<Utc> {
        &self.model.start
    }

    fn end(&self) -> &DateTime<Utc> {
        &self.model.end
    }

    fn r#final(&self) -> bool {
        self.model.r#final
    }

    pub async fn resources(&self, ctx: &Context) -> anyhow::Result<Vec<GQLResource>> {
        let models: Vec<resource::Model> = resolve_many_to_many!(
            ctx,
            target_revision: self.revision,
            booking_resource::Entity,
            booking_resource::Column::BookingId,
            self.model.id,
            |l: booking_resource::Model| l.resource_id,
            resource::Entity,
            resource::Column::HeaderId
        )?;
        Ok(models.into_iter().map(|m| GQLResource::at_revision(m, self.revision)).collect())
    }

    pub async fn task(&self, ctx: &Context) -> anyhow::Result<GQLTask> {
        if let Some(model) = ctx
            .load_one_by_col_at_revision::<task::Entity>(
                task::Column::HeaderId,
                self.model.task_id,
                self.revision,
            )
            .await?
        {
            return Ok(GQLTask::at_revision(model, self.revision));
        }

        ctx.load_one_by_col::<task::Entity>(task::Column::Id, self.model.task_id)
            .await
            .map(|opt_t| GQLTask::at_revision(opt_t.expect("Task must exist."), self.revision))
    }
}
