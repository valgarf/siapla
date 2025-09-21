use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Weak},
};

use dataloader::cached::Loader;
use itertools::Itertools as _;
use sea_orm::{
    ColumnTrait, EntityTrait, ModelTrait, Order, QueryFilter, QueryOrder, strum::IntoEnumIterator,
};

use super::context::Context;
use crate::SiaplaError;

use crate::entity::{availability, resource, vacation};
use crate::scheduling::{Interval, Intervals};
use chrono::{DateTime, Datelike, NaiveDateTime, NaiveTime, TimeDelta, Utc, Weekday};
use chrono_tz::Tz;
use sea_orm::prelude::Decimal;

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
                secs.rescale(0); // rounding to whole seconds
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
                    .and_local_timezone(self.timezone.clone())
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
                    .and_local_timezone(self.timezone.clone())
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

/// Query combined availability for a list of resource ids.
/// Returns a vector of `Intervals<NaiveDateTime>` in the same order as `resource_ids`.
pub async fn query_combined_availability(
    ctx: &Context,
    resource_ids: &Vec<i32>,
    start: NaiveDateTime,
    end: NaiveDateTime,
) -> anyhow::Result<Vec<Intervals<NaiveDateTime>>> {
    let id_set = resource_ids.iter().cloned().collect::<HashSet<_>>();
    let db = ctx.txn().await?;
    let db_availabilities = availability::Entity::find()
        .filter(availability::Column::ResourceId.is_in(id_set.clone()))
        .all(db)
        .await?;
    let db_vacations = vacation::Entity::find()
        .filter(vacation::Column::ResourceId.is_in(id_set.clone()))
        .filter(vacation::Column::From.lt(end))
        .filter(vacation::Column::Until.gt(start))
        .order_by(vacation::Column::From, Order::Asc)
        .all(db)
        .await?;
    let db_resources = resource::Entity::find()
        .filter(resource::Column::Id.is_in(resource_ids.clone()))
        .all(db)
        .await?;
    let res_map = db_resources.into_iter().map(|r| (r.id, r)).collect::<HashMap<i32, _>>();

    let mut results: Vec<Intervals<NaiveDateTime>> = Vec::with_capacity(resource_ids.len());
    for &rid in resource_ids.iter() {
        let db_res = res_map.get(&rid).expect("Resource must exist");
        let availability_iter = _AvailabilityIterator::new(
            &db_res.timezone,
            start,
            end,
            db_availabilities.iter().filter(|a| a.resource_id == rid).collect(),
        )?;

        let holiday_intervals = match db_res.holiday(ctx).await? {
            Some(h) => h
                .entries(
                    ctx,
                    availability_iter.start.date_naive(),
                    availability_iter.end.date_naive(),
                )
                .await?
                .into_iter()
                .map(|he| {
                    let start = NaiveDateTime::new(
                        he.date,
                        NaiveTime::from_hms_opt(0, 0, 0).expect("Must be a valid time"),
                    )
                    .and_local_timezone(availability_iter.timezone)
                    .earliest()
                    .expect("Cannot determine holidays datetime.")
                    .to_utc()
                    .naive_local();
                    let end = start + TimeDelta::hours(24);
                    Interval::new_lcro(start, end)
                })
                .collect(),
            None => Intervals::new(),
        };

        let vacation_intervals: Intervals<NaiveDateTime> = db_vacations
            .iter()
            .filter(|v| v.resource_id == rid)
            .map(|v| Interval::new_lcro(v.from.naive_local(), v.until.naive_local()))
            .collect();

        let availability_intervals: Intervals<NaiveDateTime> = availability_iter.collect();
        let intervals =
            availability_intervals.difference(&vacation_intervals).difference(&holiday_intervals);
        results.push(intervals);
    }
    Ok(results)
}

pub struct AvailabilityBatcher {
    pub ctx: Weak<Context>,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

impl dataloader::BatchFn<i32, Result<Intervals<NaiveDateTime>, Arc<anyhow::Error>>>
    for AvailabilityBatcher
{
    async fn load(
        &mut self,
        values: &[i32],
    ) -> HashMap<i32, Result<Intervals<NaiveDateTime>, Arc<anyhow::Error>>> {
        let ids = values.to_vec();
        let ctx = self.ctx.upgrade();
        if ctx.is_none() {
            let a = Arc::new(anyhow::anyhow!("Weak ref not upgradable in dataloader."));
            return values.iter().map(|&k| (k, Err(a.clone()))).collect();
        }
        let ctx = ctx.unwrap();
        match query_combined_availability(&ctx, &ids, self.start, self.end).await {
            Ok(vec) => {
                let mut map = HashMap::new();
                for (id, iv) in ids.into_iter().zip(vec.into_iter()) {
                    map.insert(id, Ok(iv));
                }
                map
            }
            Err(err) => {
                let a = Arc::new(err);
                values.iter().map(|&k| (k, Err(a.clone()))).collect()
            }
        }
    }
}

pub type AvailabilityLoader =
    Loader<i32, Result<Intervals<NaiveDateTime>, Arc<anyhow::Error>>, AvailabilityBatcher>;

pub struct ByColBatcher<ET: EntityTrait, const CIDX: usize>
where
    ET::Column: IntoEnumIterator,
{
    pub ctx: Weak<Context>,
    pub pd: std::marker::PhantomData<ET>,
}

async fn fallible_load<ET: EntityTrait, const CIDX: usize>(
    ctx: &Weak<Context>,
    values: &[sea_orm::Value],
) -> Result<HashMap<sea_orm::Value, Result<Vec<ET::Model>, Arc<anyhow::Error>>>, anyhow::Error>
where
    ET::Column: IntoEnumIterator,
{
    let col: ET::Column = ET::Column::iter().nth(CIDX).expect("Loader with invalid column index");
    let ctx = ctx.upgrade().ok_or(SiaplaError::new("Weak ref not upgradable in dataloader."))?;
    let txn = ctx.txn().await?;
    let tasks: Vec<ET::Model> =
        ET::find().filter(col.is_in(values.to_vec())).order_by_asc(col).all(txn).await?;
    Ok(tasks
        .into_iter()
        .chunk_by(|task| task.get(col))
        .into_iter()
        .map(|(key, tasks)| (key, Ok(tasks.collect())))
        .collect())
}

impl<ET: EntityTrait, const CIDX: usize>
    dataloader::BatchFn<sea_orm::Value, Result<Vec<ET::Model>, Arc<anyhow::Error>>>
    for ByColBatcher<ET, CIDX>
where
    ET::Column: IntoEnumIterator,
{
    async fn load(
        &mut self,
        values: &[sea_orm::Value],
    ) -> HashMap<sea_orm::Value, Result<Vec<ET::Model>, Arc<anyhow::Error>>> {
        match fallible_load::<ET, CIDX>(&self.ctx, values).await {
            Ok(data) => data,
            Err(err) => {
                let clonable_err = Arc::new(err);
                values.iter().map(|k| (k.clone(), Err(clonable_err.clone()))).collect()
            }
        }
    }
}

pub type ByColLoader<ET, const CIDX: usize> = Loader<
    sea_orm::Value,
    Result<Vec<<ET as EntityTrait>::Model>, Arc<anyhow::Error>>,
    ByColBatcher<ET, CIDX>,
>;
