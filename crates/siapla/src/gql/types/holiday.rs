use std::{collections::HashMap, sync::OnceLock};

use anyhow::anyhow;
use juniper::{FieldResult, graphql_object};
use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseTransaction, EntityTrait, Order, QueryFilter, QueryOrder,
};
use tokio::sync::OnceCell;

use crate::{
    entity::{holiday, holiday_entry},
    gql::context::Context,
};

use siapla_open_holidays_api::apis::{configuration::Configuration, holidays_api, regional_api};
//

#[derive(Debug, Clone)]
pub struct Country {
    pub isocode: String,
    pub name: String,
}

#[graphql_object]
#[graphql(name = "Country")]
impl Country {
    fn isocode(&self) -> &str {
        &self.isocode
    }
    fn name(&self) -> &str {
        &self.name
    }
    pub async fn regions(&self, _ctx: &Context) -> anyhow::Result<Vec<Region>> {
        let subdivisions = regional_api::subdivisions_get(
            &Configuration {
                base_path: "https://openholidaysapi.org".into(),
                ..Default::default()
            },
            &self.isocode,
            "EN".into(),
        )
        .await?;
        Ok(subdivisions
            .into_iter()
            .map(|sd| Region {
                isocode: sd
                    .iso_code
                    .flatten()
                    .expect("Subdivision should have an iso code"),
                country_name: self.name.clone(),
                region_name: sd
                    .name
                    .first()
                    .expect("API did not return region name")
                    .text
                    .clone(),
            })
            .collect())
    }
}

#[derive(Debug, Clone)]
pub struct Region {
    pub isocode: String,
    pub country_name: String,
    pub region_name: String,
}

#[graphql_object]
#[graphql(name = "Region")]
impl Region {
    fn isocode(&self) -> &str {
        &self.isocode
    }
    fn name(&self) -> String {
        self.country_name.clone() + " - " + self.region_name.as_str()
    }
    fn country_name(&self) -> &str {
        &self.country_name
    }
    fn region_name(&self) -> &str {
        &self.region_name
    }
    fn holiday(&self, _ctx: &Context) -> GQLHoliday {
        GQLHoliday {
            isocode: self.isocode.clone(),
            model: Default::default(),
        }
    }
}

pub struct GQLHoliday {
    isocode: String,
    model: tokio::sync::OnceCell<holiday::Model>,
}

#[graphql_object]
#[graphql(name = "Holiday")]
impl GQLHoliday {
    async fn db_id(&self, ctx: &Context) -> FieldResult<&i32> {
        Ok(&self.get_model(ctx).await?.id)
    }
    fn external_id(&self) -> &str {
        &self.isocode
    }
    async fn name(&self, ctx: &Context) -> FieldResult<&str> {
        Ok(&self.get_model(ctx).await?.name)
    }
    async fn entries(
        &self,
        ctx: &Context,
        from: chrono::NaiveDate,
        until: chrono::NaiveDate,
    ) -> anyhow::Result<Vec<holiday_entry::Model>> {
        let model = self.get_model(ctx).await?;
        let txn = ctx.txn().await?;
        model.ensure_entries(txn, from, until).await
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
    async fn holiday(&self, ctx: &Context) -> anyhow::Result<GQLHoliday> {
        const CIDX: usize = holiday::Column::Id as usize;
        let model = ctx
            .load_one_by_col::<holiday::Entity, CIDX>(self.holiday_id)
            .await
            .and_then(|r| {
                r.ok_or(anyhow!(
                    "Failed to find a holiday with id {}",
                    self.holiday_id
                ))
            })?;
        Ok(GQLHoliday::from_model(model))
    }
}

impl GQLHoliday {
    async fn get_model(&self, ctx: &Context) -> anyhow::Result<&holiday::Model> {
        let isocode = self.isocode.clone();
        let result = self
            .model
            .get_or_try_init(move || async {
                let txn = ctx.txn().await?;
                let result = holiday::Model::get_from_open_holidays(txn, isocode).await?;
                Ok::<_, anyhow::Error>(result)
            })
            .await?;
        Ok(result)
    }

    pub fn from_model(model: holiday::Model) -> Self {
        GQLHoliday {
            isocode: model.external_id.clone(),
            model: OnceCell::const_new_with(model),
        }
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
            .filter(holiday_entry::Column::HolidayId.eq(self.id))
            .filter(holiday_entry::Column::Date.gte(from))
            .filter(holiday_entry::Column::Date.lte(until))
            .order_by(holiday_entry::Column::Date, Order::Asc)
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
    ) -> anyhow::Result<holiday::Model> {
        let existing = holiday::Entity::find()
            .filter(holiday::Column::ExternalId.eq(isocode.clone()))
            .one(txn)
            .await?;
        let holiday = match existing {
            Some(holiday) => holiday,
            None => {
                let country_name = countries()
                    .get(&isocode[0..2])
                    .ok_or(anyhow!("Unknown country code: {}", &isocode[0..2]))?;
                let mut name = country_name.clone();
                if isocode.len() > 2 {
                    let subdivisions = regional_api::subdivisions_get(
                        &Configuration {
                            base_path: "https://openholidaysapi.org".into(),
                            ..Default::default()
                        },
                        &isocode[0..2],
                        "EN".into(),
                    )
                    .await?;
                    let subdividion_name = subdivisions
                        .into_iter()
                        .filter_map(|sub| {
                            if sub.iso_code.unwrap_or(None).unwrap_or("".into()) == isocode {
                                sub.name.first().map(|n| n.text.clone())
                            } else {
                                None
                            }
                        })
                        .next();
                    let sub_name = match subdividion_name {
                        None => Err(anyhow!("Isocode for country/region '{}' unknown", isocode))?,
                        Some(sub_name) => sub_name,
                    };
                    name = name + " - " + sub_name.as_str();
                }
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
        };

        Ok(holiday)
    }
}

static COUNTRIES: OnceLock<HashMap<String, String>> = OnceLock::new();
pub fn countries() -> &'static HashMap<String, String> {
    COUNTRIES.get_or_init(|| {
        let data = r#"[
            {
                "isoCode": "AD",
                "name": [
                {
                    "language": "EN",
                    "text": "Andorra"
                }
                ],
                "officialLanguages": [
                "CA"
                ]
            },
            {
                "isoCode": "AL",
                "name": [
                {
                    "language": "EN",
                    "text": "Albania"
                }
                ],
                "officialLanguages": [
                "SQ"
                ]
            },
            {
                "isoCode": "AT",
                "name": [
                {
                    "language": "EN",
                    "text": "Austria"
                }
                ],
                "officialLanguages": [
                "DE"
                ]
            },
            {
                "isoCode": "BE",
                "name": [
                {
                    "language": "EN",
                    "text": "Belgium"
                }
                ],
                "officialLanguages": [
                "NL",
                "FR",
                "DE"
                ]
            },
            {
                "isoCode": "BG",
                "name": [
                {
                    "language": "EN",
                    "text": "Bulgaria"
                }
                ],
                "officialLanguages": [
                "BG"
                ]
            },
            {
                "isoCode": "BR",
                "name": [
                {
                    "language": "EN",
                    "text": "Brazil"
                }
                ],
                "officialLanguages": [
                "PT"
                ]
            },
            {
                "isoCode": "BY",
                "name": [
                {
                    "language": "EN",
                    "text": "Belarus"
                }
                ],
                "officialLanguages": [
                "BE",
                "RU"
                ]
            },
            {
                "isoCode": "CH",
                "name": [
                {
                    "language": "EN",
                    "text": "Switzerland"
                }
                ],
                "officialLanguages": [
                "DE",
                "FR",
                "IT",
                "RM"
                ]
            },
            {
                "isoCode": "CZ",
                "name": [
                {
                    "language": "EN",
                    "text": "Czechia"
                }
                ],
                "officialLanguages": [
                "CS"
                ]
            },
            {
                "isoCode": "DE",
                "name": [
                {
                    "language": "EN",
                    "text": "Germany"
                }
                ],
                "officialLanguages": [
                "DE"
                ]
            },
            {
                "isoCode": "EE",
                "name": [
                {
                    "language": "EN",
                    "text": "Estonia"
                }
                ],
                "officialLanguages": [
                "ET"
                ]
            },
            {
                "isoCode": "ES",
                "name": [
                {
                    "language": "EN",
                    "text": "Spain"
                }
                ],
                "officialLanguages": [
                "ES",
                "CA",
                "EU",
                "GL"
                ]
            },
            {
                "isoCode": "FR",
                "name": [
                {
                    "language": "EN",
                    "text": "France"
                }
                ],
                "officialLanguages": [
                "FR"
                ]
            },
            {
                "isoCode": "HR",
                "name": [
                {
                    "language": "EN",
                    "text": "Croatia"
                }
                ],
                "officialLanguages": [
                "HR"
                ]
            },
            {
                "isoCode": "HU",
                "name": [
                {
                    "language": "EN",
                    "text": "Hungary"
                }
                ],
                "officialLanguages": [
                "HU"
                ]
            },
            {
                "isoCode": "IE",
                "name": [
                {
                    "language": "EN",
                    "text": "Ireland"
                }
                ],
                "officialLanguages": [
                "EN",
                "GA"
                ]
            },
            {
                "isoCode": "IT",
                "name": [
                {
                    "language": "EN",
                    "text": "Italy"
                }
                ],
                "officialLanguages": [
                "IT"
                ]
            },
            {
                "isoCode": "LI",
                "name": [
                {
                    "language": "EN",
                    "text": "Liechtenstein"
                }
                ],
                "officialLanguages": [
                "DE"
                ]
            },
            {
                "isoCode": "LT",
                "name": [
                {
                    "language": "EN",
                    "text": "Lithuania"
                }
                ],
                "officialLanguages": [
                "LT"
                ]
            },
            {
                "isoCode": "LU",
                "name": [
                {
                    "language": "EN",
                    "text": "Luxembourg"
                }
                ],
                "officialLanguages": [
                "LB",
                "FR",
                "DE"
                ]
            },
            {
                "isoCode": "LV",
                "name": [
                {
                    "language": "EN",
                    "text": "Latvia"
                }
                ],
                "officialLanguages": [
                "LV"
                ]
            },
            {
                "isoCode": "MC",
                "name": [
                {
                    "language": "EN",
                    "text": "Monaco"
                }
                ],
                "officialLanguages": [
                "FR"
                ]
            },
            {
                "isoCode": "MD",
                "name": [
                {
                    "language": "EN",
                    "text": "Moldova"
                }
                ],
                "officialLanguages": [
                "RO"
                ]
            },
            {
                "isoCode": "MT",
                "name": [
                {
                    "language": "EN",
                    "text": "Malta"
                }
                ],
                "officialLanguages": [
                "MT",
                "EN"
                ]
            },
            {
                "isoCode": "MX",
                "name": [
                {
                    "language": "EN",
                    "text": "Mexico"
                }
                ],
                "officialLanguages": [
                "ES"
                ]
            },
            {
                "isoCode": "NL",
                "name": [
                {
                    "language": "EN",
                    "text": "Netherlands (the)"
                }
                ],
                "officialLanguages": [
                "NL"
                ]
            },
            {
                "isoCode": "PL",
                "name": [
                {
                    "language": "EN",
                    "text": "Poland"
                }
                ],
                "officialLanguages": [
                "PL"
                ]
            },
            {
                "isoCode": "PT",
                "name": [
                {
                    "language": "EN",
                    "text": "Portugal"
                }
                ],
                "officialLanguages": [
                "PT"
                ]
            },
            {
                "isoCode": "RO",
                "name": [
                {
                    "language": "EN",
                    "text": "Romania"
                }
                ],
                "officialLanguages": [
                "RO"
                ]
            },
            {
                "isoCode": "RS",
                "name": [
                {
                    "language": "EN",
                    "text": "Serbia"
                }
                ],
                "officialLanguages": [
                "SR"
                ]
            },
            {
                "isoCode": "SE",
                "name": [
                {
                    "language": "EN",
                    "text": "Sweden"
                }
                ],
                "officialLanguages": [
                "SV"
                ]
            },
            {
                "isoCode": "SI",
                "name": [
                {
                    "language": "EN",
                    "text": "Slovenia"
                }
                ],
                "officialLanguages": [
                "SL"
                ]
            },
            {
                "isoCode": "SK",
                "name": [
                {
                    "language": "EN",
                    "text": "Slovakia"
                }
                ],
                "officialLanguages": [
                "SK"
                ]
            },
            {
                "isoCode": "SM",
                "name": [
                {
                    "language": "EN",
                    "text": "San Marino"
                }
                ],
                "officialLanguages": [
                "IT"
                ]
            },
            {
                "isoCode": "VA",
                "name": [
                {
                    "language": "EN",
                    "text": "Vatican City"
                }
                ],
                "officialLanguages": [
                "IT"
                ]
            }
            ]"#;

        let countries: Vec<siapla_open_holidays_api::models::CountryResponse> =
            serde_json::from_str(data).expect("Fixed data, parsing errors should be impossible.");
        countries
            .into_iter()
            .map(|c| {
                (
                    c.iso_code,
                    c.name.first().expect("field exists in data.").text.clone(),
                )
            })
            .collect()
    })
}
