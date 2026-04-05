use std::cmp::{max, min};
use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Datelike, NaiveDateTime, NaiveTime, TimeDelta, Utc, Weekday};
use chrono_tz::Tz;
use sea_orm::prelude::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder};

use super::base::Batcher;
use crate::db::RangeRevColumns;
use crate::db::context::DbContext;
use crate::db::entity::{availability, resource_iteration, vacation};
use crate::scheduling::{Interval, Intervals};

pub fn string_to_weekday(s: &str) -> anyhow::Result<Weekday> {
    match s {
        "Monday" => Ok(Weekday::Mon),
        "Tuesday" => Ok(Weekday::Tue),
        "Wednesday" => Ok(Weekday::Wed),
        "Thursday" => Ok(Weekday::Thu),
        "Friday" => Ok(Weekday::Fri),
        "Saturday" => Ok(Weekday::Sat),
        "Sunday" => Ok(Weekday::Sun),
        _ => Err(anyhow::anyhow!("Unknown weekday: {}", s)),
    }
}

pub struct _AvailabilityIterator {
    pub timezone: Tz,
    pub start: DateTime<Tz>,
    pub end: DateTime<Tz>,
    pub durations: HashMap<Weekday, TimeDelta>,
    pub last_end: Option<DateTime<Tz>>,
}

impl _AvailabilityIterator {
    pub fn new(
        timezone: &str,
        start: NaiveDateTime,
        end: NaiveDateTime,
        availabilities: Vec<&availability::Model>,
    ) -> anyhow::Result<Self> {
        let tz: Tz = timezone.parse()?;
        let start_dt = DateTime::<Utc>::from_naive_utc_and_offset(start, Utc).with_timezone(&tz);
        let end_dt = DateTime::<Utc>::from_naive_utc_and_offset(end, Utc).with_timezone(&tz);
        let durations = availabilities
            .into_iter()
            .map(|a| -> anyhow::Result<(Weekday, TimeDelta)> {
                let mut secs = a.duration * Decimal::new(3600, 0);
                secs.rescale(0);
                Ok((string_to_weekday(&a.weekday)?, TimeDelta::seconds(secs.try_into()?)))
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(Self { timezone: tz, start: start_dt, end: end_dt, durations, last_end: None })
    }
}

impl Iterator for _AvailabilityIterator {
    type Item = Interval<NaiveDateTime>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut date =
            self.last_end.map(|e| e + TimeDelta::days(1)).unwrap_or(self.start).date_naive();
        loop {
            if date > self.end.date_naive() {
                self.last_end = Some(self.end);
                return None;
            }
            if let Some(dur) = self.durations.get(&date.weekday()) {
                let secs = std::cmp::min(dur.num_seconds() / 2, 12 * 3600);
                if secs <= 0 {
                    date += TimeDelta::days(1);
                    continue;
                }
                let i_start = std::cmp::max(
                    NaiveDateTime::new(
                        date,
                        NaiveTime::from_num_seconds_from_midnight_opt(12 * 3600 - secs as u32, 0)
                            .unwrap(),
                    )
                    .and_local_timezone(self.timezone)
                    .latest()
                    .expect("Cannot determine availability start"),
                    self.start,
                );
                let i_end = std::cmp::min(
                    NaiveDateTime::new(
                        date,
                        NaiveTime::from_num_seconds_from_midnight_opt(12 * 3600 + secs as u32, 0)
                            .unwrap(),
                    )
                    .and_local_timezone(self.timezone)
                    .earliest()
                    .expect("Cannot determine availability end"),
                    self.end,
                );
                self.last_end = Some(i_end);
                if i_end <= i_start {
                    date += TimeDelta::days(1);
                    continue;
                }
                return Some(Interval::new_lcro(
                    i_start.to_utc().naive_local(),
                    i_end.to_utc().naive_local(),
                ));
            } else {
                date += TimeDelta::days(1);
            }
        }
    }
}

pub async fn query_combined_availability(
    db: &DbContext,
    resource_ids: &[i32],
    start: NaiveDateTime,
    end: NaiveDateTime,
    revision: i64,
) -> anyhow::Result<Vec<Intervals<NaiveDateTime>>> {
    let id_set = resource_ids.iter().cloned().collect::<HashSet<_>>();
    let txn = db.txn().await?;

    let db_resources = resource_iteration::Entity::find()
        .filter(resource_iteration::Column::HeaderId.is_in(resource_ids.to_vec()))
        .filter(resource_iteration::Entity::condition(revision))
        .all(txn)
        .await?;
    let res_map = db_resources.into_iter().map(|r| (r.header_id, r)).collect::<HashMap<i32, _>>();

    let db_availabilities = availability::Entity::find()
        .filter(availability::Column::ResourceId.is_in(id_set.clone()))
        .filter(availability::Entity::condition(revision))
        .all(txn)
        .await?;
    let db_vacations = vacation::Entity::find()
        .filter(vacation::Column::ResourceId.is_in(id_set.clone()))
        .filter(vacation::Entity::condition(revision))
        .filter(vacation::Column::From.lt(end))
        .filter(vacation::Column::Until.gt(start))
        .order_by(vacation::Column::From, Order::Asc)
        .all(txn)
        .await?;

    let mut results: Vec<Intervals<NaiveDateTime>> = Vec::with_capacity(resource_ids.len());
    for &rid in resource_ids.iter() {
        let db_res = res_map.get(&rid).expect("Resource must exist");
        let res_start = max(start, db_res.added.naive_utc());
        let res_end = match db_res.removed {
            Some(removed) => min(end, removed.naive_utc()),
            None => end,
        };
        let availability_iter = _AvailabilityIterator::new(
            &db_res.timezone,
            res_start,
            res_end,
            db_availabilities.iter().filter(|a| a.resource_id == rid).collect(),
        )?;

        let holiday_intervals = match db_res.dataloader_holiday(db).await? {
            Some(h) => h
                .ensure_entries(
                    txn,
                    availability_iter.start.date_naive(),
                    availability_iter.end.date_naive(),
                )
                .await?
                .into_iter()
                .map(|he| {
                    let start = NaiveDateTime::new(
                        he.date,
                        chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                    );
                    let end = start + chrono::Duration::days(1);
                    Interval::new_lcro(start, end)
                })
                .collect::<Vec<_>>(),
            None => vec![],
        };

        let vacation_intervals = db_vacations
            .iter()
            .filter(|v| v.resource_id == rid)
            .map(|v| Interval::new_lcro(v.from.naive_utc(), v.until.naive_utc()))
            .collect::<Vec<_>>();

        let mut all_intervals = Intervals::new();
        for iv in availability_iter {
            all_intervals.insert(iv);
        }
        for iv in holiday_intervals.into_iter().chain(vacation_intervals.into_iter()) {
            all_intervals.remove(iv);
        }
        results.push(all_intervals);
    }

    Ok(results)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AvailabilityBatcher {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub revision: i64,
}

impl Batcher for AvailabilityBatcher {
    type Key = i32;
    type Value = Intervals<NaiveDateTime>;
    async fn load(
        &self,
        db: &DbContext,
        values: &[Self::Key],
    ) -> Result<HashMap<Self::Key, Self::Value>, anyhow::Error> {
        let ids = values.to_vec();
        match query_combined_availability(db, &ids, self.start, self.end, self.revision).await {
            Ok(vec) => Ok(ids.into_iter().zip(vec.into_iter()).collect()),
            Err(err) => Err(err),
        }
    }
}
