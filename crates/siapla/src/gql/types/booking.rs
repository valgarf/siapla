use crate::gql::scalars::ExtendedScalarValue;
use crate::{entity::booking, gql::context::Context};
use chrono::{DateTime, Utc};
use juniper::graphql_object;

use super::resource::GQLResource;
use super::task::GQLTask;

pub struct GQLBooking {
    pub model: booking::Model,
    pub revision: i64,
}

impl GQLBooking {
    pub fn at_revision(model: booking::Model, revision: i64) -> Self {
        Self { model, revision }
    }
}

impl From<booking::Model> for GQLBooking {
    fn from(model: booking::Model) -> Self {
        let revision = model.rev_created;
        Self { model, revision }
    }
}

#[graphql_object]
#[graphql(name = "Booking", context = Context, scalar = ExtendedScalarValue)]
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
        let models = self.model.dataloader_resource_iterations(ctx.db(), self.revision).await?;
        Ok(models.into_iter().map(|m| GQLResource::at_revision(m, self.revision)).collect())
    }

    pub async fn task(&self, ctx: &Context) -> anyhow::Result<GQLTask> {
        let model = self
            .model
            .dataloader_task_iteration(ctx.db(), self.revision)
            .await?
            .expect("Task must exist.");
        Ok(GQLTask::at_revision(model, self.revision))
    }
}
