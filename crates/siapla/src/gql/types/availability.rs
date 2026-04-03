use std::{collections::HashSet, iter::zip};

use super::resource::GQLResource;
use crate::{
    entity::{availability, resource_iteration as resource},
    gql::context::Context,
};
use anyhow::anyhow;
use juniper::{GraphQLEnum, graphql_object};
use sea_orm::{ActiveValue, prelude::*};
use strum::{EnumString, IntoStaticStr};
use tracing::trace;

#[derive(GraphQLEnum, IntoStaticStr, EnumString, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl From<Weekday> for String {
    fn from(value: Weekday) -> Self {
        let s: &'static str = value.into();
        s.into()
    }
}

pub struct GQLAvailability {
    pub model: availability::Model,
    pub revision: Option<i64>,
}

impl GQLAvailability {
    pub fn at_revision(model: availability::Model, revision: Option<i64>) -> Self {
        Self { model, revision }
    }
}

impl From<availability::Model> for GQLAvailability {
    fn from(model: availability::Model) -> Self {
        Self { model, revision: None }
    }
}

#[graphql_object]
#[graphql(name = "Availability")]
impl GQLAvailability {
    fn db_id(&self) -> &i32 {
        &self.model.id
    }
    async fn resource(&self, ctx: &Context) -> anyhow::Result<GQLResource> {
        let model = ctx
            .load_one_by_col_at_revision::<resource::Entity>(
                resource::Column::HeaderId,
                self.model.resource_id,
                self.revision,
            )
            .await?;
        let model = model.ok_or(anyhow!("Failed to find resource for Availability"))?;
        Ok(GQLResource::at_revision(model, self.revision))
    }
    fn duration(&self) -> anyhow::Result<i32> {
        let mut secs = self.model.duration * Decimal::new(3600, 0);
        secs.rescale(0); // rounding to whole seconds
        Ok(secs.try_into()?)
    }
    fn weekday(&self) -> anyhow::Result<Weekday> {
        Ok(self.model.weekday.as_str().try_into()?)
    }
}

#[derive(juniper::GraphQLInputObject, Debug)]
pub struct AvailabilityInput {
    weekday: Weekday,
    duration: i32,
}

impl From<&AvailabilityInput> for crate::entity::availability::ActiveModel {
    fn from(value: &AvailabilityInput) -> Self {
        crate::entity::availability::ActiveModel {
            id: ActiveValue::NotSet,
            resource_id: ActiveValue::NotSet,
            weekday: ActiveValue::Set(value.weekday.into()),
            duration: ActiveValue::Set(Decimal::from(value.duration) / Decimal::from(3600)),
            rev_created: ActiveValue::NotSet,
            rev_deleted: ActiveValue::NotSet,
        }
    }
}

pub async fn update_availability(
    ctx: &Context,
    model: &resource::Model,
    availability: Vec<AvailabilityInput>,
    revision_id: i64,
) -> anyhow::Result<()> {
    let txn = ctx.txn().await?;
    let existing_availability: Vec<_> = model.availability_latest(ctx).await?.into_iter().collect();
    let existing: HashSet<Weekday> = existing_availability
        .iter()
        .map(|el| el.weekday.as_str().try_into().map_err(anyhow::Error::from))
        .collect::<anyhow::Result<_>>()?;
    let target: HashSet<Weekday> = availability.iter().map(|a| a.weekday).collect();
    let remove: HashSet<Weekday> = existing.difference(&target).cloned().collect();
    let add: HashSet<Weekday> = target.difference(&existing).cloned().collect();
    let update: HashSet<Weekday> = target.intersection(&existing).cloned().collect();
    trace!(
        "availability: existing={:?}, target={:?}, remove={:?}, add={:?}, update={:?}",
        existing, target, remove, add, update
    );
    if !remove.is_empty() {
        availability::Entity::update_many()
            .col_expr(
                availability::Column::RevDeleted,
                Expr::value(Value::BigInt(Some(revision_id))),
            )
            .filter(availability::Column::ResourceId.eq(model.id))
            .filter(availability::Column::RevDeleted.is_null())
            .filter(
                availability::Column::Weekday.is_in(
                    remove
                        .iter()
                        .map(|w| {
                            let wstr: &'static str = w.into();
                            wstr.to_owned()
                        })
                        .collect::<Vec<String>>(),
                ),
            )
            .exec(txn)
            .await?;
    }
    if !add.is_empty() {
        let add_models: Vec<availability::ActiveModel> = availability
            .iter()
            .filter(|a| add.contains(&a.weekday))
            .map(|a| {
                let mut am: availability::ActiveModel = a.into();
                am.resource_id = ActiveValue::Set(model.header_id.unwrap_or(model.id));
                am.rev_created = ActiveValue::Set(revision_id);
                am.rev_deleted = ActiveValue::Set(None);
                am
            })
            .collect();
        availability::Entity::insert_many(add_models).exec(txn).await?;
    }
    if !update.is_empty() {
        let existing_models: Vec<&availability::Model> = existing_availability
            .iter()
            .filter(|a| {
                let wd: anyhow::Result<Weekday> =
                    a.weekday.as_str().try_into().map_err(anyhow::Error::from);
                if let Ok(wd) = wd { update.contains(&wd) } else { false }
            })
            .collect();
        let update_models: Vec<&AvailabilityInput> =
            availability.iter().filter(|a| update.contains(&a.weekday)).collect();
        if existing_models.len() != update_models.len() {
            return Err(anyhow!("Internal error trying to update the availability."));
        }
        let update_models: Vec<(&availability::Model, &AvailabilityInput)> =
            zip(existing_models, update_models)
                .filter(|(e, u)| e.duration != Decimal::from(u.duration) / Decimal::from(3600))
                .collect();
        for (existing, input) in update_models {
            availability::Entity::update_many()
                .col_expr(
                    availability::Column::RevDeleted,
                    Expr::value(Value::BigInt(Some(revision_id))),
                )
                .filter(availability::Column::Id.eq(existing.id))
                .filter(availability::Column::RevDeleted.is_null())
                .exec(txn)
                .await?;

            let mut am: availability::ActiveModel = input.into();
            am.resource_id = ActiveValue::Set(model.header_id.unwrap_or(model.id));
            am.rev_created = ActiveValue::Set(revision_id);
            am.rev_deleted = ActiveValue::Set(None);
            am.insert(txn).await?;
        }
    }
    Ok(())
}
