use chrono::NaiveDateTime;
use juniper::graphql_object;

use super::context::Context;
use crate::entity::task;

#[graphql_object]
#[graphql(name = "Task")]
impl task::Model {
    fn db_id(&self) -> &i32 {
        &self.id
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn earliest_start(&self) -> &Option<NaiveDateTime> {
        &self.earliest_start
    }
    fn schedule_target(&self) -> &Option<NaiveDateTime> {
        &self.schedule_target
    }
    fn effort(&self) -> Option<f64> {
        self.effort.map(Into::into)
    }
    async fn parent(&self, ctx: &Context) -> anyhow::Result<Option<Self>> {
        match self.parent_id {
            None => Ok(None),
            Some(parent_id) => {
                const CIDX: usize = task::Column::Id as usize;
                ctx.load_one_by_col::<task::Entity, CIDX>(parent_id).await
            }
        }
    }
}
