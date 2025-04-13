use anyhow::anyhow;
use juniper::{FieldResult, graphql_object};
use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, TransactionTrait,
};

use super::context::Context;
use crate::entity::{holiday, holiday_entry};

use siapla_open_holidays_api::apis::{configuration::Configuration, holidays_api, regional_api};
//
#[graphql_object]
#[graphql(name = "Holiday")]
impl holiday::Model {
    fn db_id(&self) -> &i32 {
        &self.id
    }
    fn external_id(&self) -> &str {
        &self.external_id
    }
    fn name(&self) -> &str {
        &self.name
    }
    async fn entries(
        &self,
        ctx: &Context,
        from: chrono::NaiveDate,
        until: chrono::NaiveDate,
    ) -> anyhow::Result<Vec<holiday_entry::Model>> {
        let db = ctx.db().await?;
        let txn = db.begin().await?;
        let result = self.ensure_entries(&txn, from, until).await;
        txn.commit().await?;
        result
    }
}

#[graphql_object]
#[graphql(name = "HolidayEntry")]
impl holiday_entry::Model {
    fn db_id(&self) -> &i32 {
        &self.id
    }
    fn date(&self) -> &chrono::NaiveDate {
        &self.date
    }
    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    async fn holiday(&self, ctx: &Context) -> anyhow::Result<holiday::Model> {
        const CIDX: usize = holiday::Column::Id as usize;
        ctx.load_one_by_col::<holiday::Entity, CIDX>(self.holiday_id)
            .await
            .and_then(|r| {
                r.ok_or(anyhow!(
                    "Failed to find a holiday with id {}",
                    self.holiday_id
                ))
            })
    }
}

impl holiday::Model {
    pub async fn ensure_entries(
        &self,
        txn: &DatabaseTransaction,
        from: chrono::NaiveDate,
        until: chrono::NaiveDate,
    ) -> anyhow::Result<Vec<holiday_entry::Model>> {
        match (self.start, self.end) {
            (Some(start), Some(end)) => {
                if start > from {
                    self.download_entries(
                        txn,
                        from,
                        start
                            .pred_opt()
                            .ok_or(anyhow!("Not a representable date"))?,
                    )
                    .await?;
                }

                if end < until {
                    self.download_entries(
                        txn,
                        end.succ_opt().ok_or(anyhow!("Not a representable date"))?,
                        until,
                    )
                    .await?;
                }
                holiday::Entity::update(holiday::ActiveModel {
                    id: ActiveValue::Set(self.id),
                    start: ActiveValue::Set(Some(start.min(from))),
                    end: ActiveValue::Set(Some(end.max(until))),
                    ..Default::default()
                })
                .exec(txn)
                .await?;
            }
            _ => {
                self.download_entries(txn, from, until).await?;
                holiday::Entity::update(holiday::ActiveModel {
                    id: ActiveValue::Set(self.id),
                    start: ActiveValue::Set(Some(from)),
                    end: ActiveValue::Set(Some(until)),
                    ..Default::default()
                })
                .exec(txn)
                .await?;
            }
        }

        Ok(holiday_entry::Entity::find()
            .filter(holiday_entry::Column::Date.gte(from))
            .filter(holiday_entry::Column::Date.lte(until))
            .all(txn)
            .await?)
    }

    async fn download_entries(
        &self,
        txn: &DatabaseTransaction,
        from: chrono::NaiveDate,
        until: chrono::NaiveDate,
    ) -> anyhow::Result<()> {
        // https://openholidaysapi.org/PublicHolidays?countryIsoCode=DE&languageIsoCode=EN&validFrom=2025-01-01&validTo=2025-12-31
        let config = Configuration {
            base_path: "https://openholidaysapi.org".into(),
            ..Default::default()
        };
        let entries = holidays_api::public_holidays_get(
            &config,
            &self.external_id[0..2],
            from,
            until,
            "EN".into(),
            Some(&self.external_id),
        )
        .await?;

        let new_entries = entries.iter().flat_map(|e| {
            let mut result = vec![];
            if e.nationwide
                || e.subdivisions
                    .iter()
                    .flatten()
                    .any(|d| d.code == self.external_id)
            {
                let mut start = e.start_date;
                while start <= e.end_date {
                    result.push(holiday_entry::ActiveModel {
                        date: ActiveValue::Set(start),
                        holiday_id: ActiveValue::Set(self.id),
                        name: ActiveValue::Set(Some(
                            e.name.first().map(|n| n.text.clone()).unwrap_or("".into()),
                        )),
                        ..Default::default()
                    });
                    start = start.succ_opt().expect("Should always be representable");
                }
            }
            result.into_iter()
        });

        holiday_entry::Entity::insert_many(new_entries)
            .exec(txn)
            .await?;
        Ok(())
    }

    pub async fn get_from_open_holidays(
        txn: &DatabaseTransaction,
        isocode: String,
    ) -> FieldResult<holiday::Model> {
        let existing = holiday::Entity::find()
            .filter(holiday::Column::ExternalId.eq(isocode.clone()))
            .one(txn)
            .await?;
        let holiday = match existing {
            Some(holiday) => holiday,
            None => {
                let subdivisions = regional_api::subdivisions_get(
                    &Configuration {
                        base_path: "https://openholidaysapi.org".into(),
                        ..Default::default()
                    },
                    &isocode[0..2],
                    "EN".into(),
                )
                .await?;
                let name = subdivisions
                    .into_iter()
                    .filter_map(|sub| {
                        if sub.iso_code.unwrap_or(None).unwrap_or("".into()) == isocode {
                            sub.name.first().map(|n| n.text.clone())
                        } else {
                            None
                        }
                    })
                    .next();
                match name {
                    None => Err(anyhow!("Isocode for country/region '{}' unknown", isocode))?,
                    Some(name) => {
                        let am = holiday::ActiveModel {
                            name: ActiveValue::Set(name),
                            external_id: ActiveValue::Set(isocode.clone()),
                            ..Default::default()
                        };
                        let id = holiday::Entity::insert(am).exec(txn).await?.last_insert_id;
                        holiday::Entity::find_by_id(id)
                            .one(txn)
                            .await?
                            .ok_or(anyhow!(
                                "We just created a holiday with id {}! Where is it?",
                                id
                            ))?
                    }
                }
            }
        };

        Ok(holiday)
    }
}
