use crate::entity::vacation;
use chrono::{DateTime, Utc};
use juniper::graphql_object;
use sea_orm::ActiveValue;

pub struct GQLVacation {
    pub model: vacation::Model,
    pub revision: i64,
}

impl GQLVacation {
    pub fn at_revision(model: vacation::Model, revision: i64) -> Self {
        Self { model, revision }
    }
}

impl From<vacation::Model> for GQLVacation {
    fn from(model: vacation::Model) -> Self {
        let revision = model.rev_created;
        Self { model, revision }
    }
}

#[graphql_object]
#[graphql(name = "Vacation")]
impl GQLVacation {
    fn db_id(&self) -> &i32 {
        &self.model.id
    }
    fn from(&self) -> DateTime<Utc> {
        self.model.from
    }
    fn until(&self) -> DateTime<Utc> {
        self.model.until
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
            rev_created: ActiveValue::NotSet,
            rev_deleted: ActiveValue::NotSet,
        }
    }
}
