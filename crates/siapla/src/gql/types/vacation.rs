use crate::entity::vacation;
use chrono::{DateTime, Utc};
use juniper::graphql_object;
use sea_orm::ActiveValue;

#[graphql_object]
#[graphql(name = "Vacation")]
impl vacation::Model {
    fn db_id(&self) -> &i32 {
        &self.id
    }
    fn from(&self) -> DateTime<Utc> {
        self.from
    }
    fn until(&self) -> DateTime<Utc> {
        self.until
    }
}

#[derive(juniper::GraphQLInputObject)]
pub struct VacationInput {
    from: DateTime<Utc>,
    until: DateTime<Utc>,
}

impl From<VacationInput> for crate::entity::vacation::ActiveModel {
    fn from(value: VacationInput) -> Self {
        crate::entity::vacation::ActiveModel {
            id: ActiveValue::NotSet,
            resource_id: ActiveValue::NotSet,
            from: ActiveValue::Set(value.from),
            until: ActiveValue::Set(value.until),
        }
    }
}
