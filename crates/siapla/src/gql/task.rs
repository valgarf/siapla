use chrono::NaiveDateTime;
use juniper::graphql_object;

use super::context::Context;

#[graphql_object]
#[graphql(name = "Task")]
impl crate::entity::task::Model {
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
                let res = ctx
                    .task_loader()
                    .await
                    .try_load(parent_id)
                    .await
                    .map_err(|_| anyhow::anyhow!("No task exists for ID `{}`", parent_id))?
                    .map_err(|err| anyhow::anyhow!("{err}"))?;
                Ok(Some(res))
            }
        }
    }
}
