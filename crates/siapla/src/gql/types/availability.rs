use std::{
    collections::{HashMap, HashSet},
    iter::zip,
};

use crate::{
    entity::{availability, resource},
    gql::{common::opt_to_av, context::Context},
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

#[graphql_object]
#[graphql(name = "Availability")]
impl availability::Model {
    fn db_id(&self) -> &i32 {
        &self.id
    }
    async fn resource(&self, ctx: &Context) -> anyhow::Result<resource::Model> {
        const CIDX: usize = resource::Column::Id as usize;
        let resource = ctx
            .load_one_by_col::<resource::Entity, CIDX>(self.resource_id)
            .await?;
        resource.ok_or(anyhow!("Failed to find resource for Availability"))
    }
    fn duration(&self) -> anyhow::Result<i32> {
        let mut secs = self.duration * Decimal::new(3600, 0);
        secs.rescale(0); // rounding to whole seconds
        Ok(secs.try_into()?)
    }
    fn weekday(&self) -> anyhow::Result<Weekday> {
        Ok(self.weekday.as_str().try_into()?)
    }
}

#[derive(juniper::GraphQLInputObject)]
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
            duration: ActiveValue::Set(value.duration.into()),
        }
    }
}

pub async fn update_availability(
    ctx: &Context,
    model: &resource::Model,
    availability: Vec<AvailabilityInput>,
) -> anyhow::Result<()> {
    let txn = ctx.txn().await?;
    let existing_availability: Vec<_> = model.availability(ctx).await?.into_iter().collect();
    let existing: HashSet<Weekday> = existing_availability
        .iter()
        .map(|el| el.weekday())
        .collect::<anyhow::Result<_>>()?;
    let target: HashSet<Weekday> = availability.iter().map(|a| a.weekday).collect();
    let remove: HashSet<Weekday> = existing.difference(&target).cloned().collect();
    let add: HashSet<Weekday> = target.difference(&existing).cloned().collect();
    let update: HashSet<Weekday> = target.difference(&existing).cloned().collect();
    trace!(
        "availability: existing={:?}, target={:?}, remove={:?}, add={:?}, update={:?}",
        existing, target, remove, add, update
    );
    if !remove.is_empty() {
        availability::Entity::delete_many()
            .filter(
                availability::Column::ResourceId.eq(model.id).and(
                    availability::Column::Weekday.is_in(
                        remove
                            .iter()
                            .map(|w| {
                                let wstr: &'static str = w.into();
                                wstr.to_owned()
                            })
                            .collect::<Vec<String>>(),
                    ),
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
                am.resource_id = ActiveValue::Set(model.id);
                am
            })
            .collect();
        availability::Entity::insert_many(add_models)
            .exec(txn)
            .await?;
    }
    if !update.is_empty() {
        let existing_models: Vec<&availability::Model> = existing_availability
            .iter()
            .filter(|a| {
                let wd = a.weekday();
                if let Ok(wd) = wd {
                    update.contains(&wd)
                } else {
                    false
                }
            })
            .collect();
        let update_models: Vec<&AvailabilityInput> = availability
            .iter()
            .filter(|a| update.contains(&a.weekday))
            .collect();
        if existing_models.len() != update_models.len() {
            return Err(anyhow!("Internal error trying to update the availability."));
        }
        let update_models: Vec<availability::ActiveModel> = zip(existing_models, update_models)
            .filter(|(e, u)| e.duration != u.duration.into())
            .map(|(e, u)| {
                let mut am: availability::ActiveModel = u.into();
                am.resource_id = ActiveValue::Set(model.id);
                am.id = ActiveValue::Set(e.id);
                am
            })
            .collect();
        for am in update_models {
            am.update(txn).await?;
        }
    }
    Ok(())
}
