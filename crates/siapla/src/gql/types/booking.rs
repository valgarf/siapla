use crate::gql::common::resolve_many_to_many;
use crate::{
    entity::{booking, booking_resource, resource_iteration as resource, task_iteration as task},
    gql::context::Context,
};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use juniper::graphql_object;

#[graphql_object]
#[graphql(name = "Booking")]
impl booking::Model {
    fn db_id(&self) -> &i32 {
        &self.id
    }

    fn start(&self) -> &DateTime<Utc> {
        &self.start
    }

    fn end(&self) -> &DateTime<Utc> {
        &self.end
    }

    fn r#final(&self) -> bool {
        self.r#final
    }

    pub async fn resources(&self, ctx: &Context) -> anyhow::Result<Vec<resource::Model>> {
        resolve_many_to_many!(
            ctx,
            booking_resource::Entity,
            booking_resource::Column::BookingId,
            self.id,
            |l: booking_resource::Model| l.resource_id,
            resource::Entity,
            resource::Column::Id
        )
    }

    pub async fn task(&self, ctx: &Context) -> anyhow::Result<task::Model> {
        const CIDX: usize = task::Column::Id as usize;
        ctx.load_one_by_col::<task::Entity, CIDX>(self.task_id)
            .await
            .map(|opt_t| opt_t.expect("Task must exist."))
    }
}
