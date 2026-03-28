use std::{
    cmp::{max, min},
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
use crate::revisioning::{active_for_revision, resolve_revision};

use crate::entity::{availability, resource_iteration as resource, vacation};
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

/// Query combined availability for a list of resource header ids.
/// Returns a vector of `Intervals<NaiveDateTime>` in the same order as `resource_ids`.
pub async fn query_combined_availability(
    ctx: &Context,
    resource_ids: &Vec<i32>,
    start: NaiveDateTime,
    end: NaiveDateTime,
    revision: i64,
) -> anyhow::Result<Vec<Intervals<NaiveDateTime>>> {
    let id_set = resource_ids.iter().cloned().collect::<HashSet<_>>();
    let db = ctx.txn().await?;

    let db_resources = resource::Entity::find()
        .filter(resource::Column::HeaderId.is_in(resource_ids.clone()))
        .filter(active_for_revision(
            resource::Column::RevCreated,
            resource::Column::RevDeleted,
            Some(revision),
        )?)
        .all(db)
        .await?;
    let res_map =
        db_resources.into_iter().filter_map(|r| r.header_id.map(|hid| (hid, r))).collect::<HashMap<i32, _>>();

    let db_availabilities = availability::Entity::find()
        .filter(availability::Column::ResourceId.is_in(id_set.clone()))
        .filter(active_for_revision(
            availability::Column::RevCreated,
            availability::Column::RevDeleted,
            Some(revision),
        )?)
        .all(db)
        .await?;
    let db_vacations = vacation::Entity::find()
        .filter(vacation::Column::ResourceId.is_in(id_set.clone()))
        .filter(active_for_revision(
            vacation::Column::RevCreated,
            vacation::Column::RevDeleted,
            Some(revision),
        )?)
        .filter(vacation::Column::From.lt(end))
        .filter(vacation::Column::Until.gt(start))
        .order_by(vacation::Column::From, Order::Asc)
        .all(db)
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

pub struct AvailabilityBatcher {
    pub ctx: Weak<Context>,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub revision: i64,
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
        match query_combined_availability(&ctx, &ids, self.start, self.end, self.revision).await {
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

pub struct ByFixedRevisionColBatcher<ET: EntityTrait, const KEY_CIDX: usize, const REV_CIDX: usize>
where
    ET::Column: IntoEnumIterator,
{
    pub ctx: Weak<Context>,
    pub revision: i64,
    pub pd: std::marker::PhantomData<ET>,
}

async fn fallible_load_fixed_revision<ET: EntityTrait, const KEY_CIDX: usize, const REV_CIDX: usize>(
    ctx: &Weak<Context>,
    values: &[sea_orm::Value],
    revision: i64,
) -> Result<HashMap<sea_orm::Value, Result<Vec<ET::Model>, Arc<anyhow::Error>>>, anyhow::Error>
where
    ET::Column: IntoEnumIterator,
{
    let key_col: ET::Column =
        ET::Column::iter().nth(KEY_CIDX).expect("Loader with invalid key column index");
    let rev_col: ET::Column =
        ET::Column::iter().nth(REV_CIDX).expect("Loader with invalid revision column index");
    let ctx = ctx.upgrade().ok_or(SiaplaError::new("Weak ref not upgradable in dataloader."))?;
    let txn = ctx.txn().await?;
    let rows: Vec<ET::Model> = ET::find()
        .filter(key_col.is_in(values.to_vec()))
        .filter(rev_col.eq(revision))
        .order_by_asc(key_col)
        .all(txn)
        .await?;
    Ok(rows
        .into_iter()
        .chunk_by(|row| row.get(key_col))
        .into_iter()
        .map(|(key, rows)| (key, Ok(rows.collect())))
        .collect())
}

impl<ET: EntityTrait, const KEY_CIDX: usize, const REV_CIDX: usize>
    dataloader::BatchFn<sea_orm::Value, Result<Vec<ET::Model>, Arc<anyhow::Error>>>
    for ByFixedRevisionColBatcher<ET, KEY_CIDX, REV_CIDX>
where
    ET::Column: IntoEnumIterator,
{
    async fn load(
        &mut self,
        values: &[sea_orm::Value],
    ) -> HashMap<sea_orm::Value, Result<Vec<ET::Model>, Arc<anyhow::Error>>> {
        match fallible_load_fixed_revision::<ET, KEY_CIDX, REV_CIDX>(&self.ctx, values, self.revision)
            .await
        {
            Ok(data) => data,
            Err(err) => {
                let clonable_err = Arc::new(err);
                values.iter().map(|k| (k.clone(), Err(clonable_err.clone()))).collect()
            }
        }
    }
}

pub type ByFixedRevisionColLoader<ET, const KEY_CIDX: usize, const REV_CIDX: usize> = Loader<
    sea_orm::Value,
    Result<Vec<<ET as EntityTrait>::Model>, Arc<anyhow::Error>>,
    ByFixedRevisionColBatcher<ET, KEY_CIDX, REV_CIDX>,
>;

pub struct ByColBatcher<ET: EntityTrait, const CIDX: usize>
where
    ET::Column: IntoEnumIterator,
{
    pub ctx: Weak<Context>,
    pub revision: Option<i64>,
    pub rev_created_idx: Option<usize>,
    pub rev_deleted_idx: Option<usize>,
    pub pd: std::marker::PhantomData<ET>,
}

async fn fallible_load<ET: EntityTrait, const CIDX: usize>(
    ctx: &Weak<Context>,
    values: &[sea_orm::Value],
    revision: Option<i64>,
    rev_created_idx: Option<usize>,
    rev_deleted_idx: Option<usize>,
) -> Result<HashMap<sea_orm::Value, Result<Vec<ET::Model>, Arc<anyhow::Error>>>, anyhow::Error>
where
    ET::Column: IntoEnumIterator,
{
    let col: ET::Column = ET::Column::iter().nth(CIDX).expect("Loader with invalid column index");
    let ctx = ctx.upgrade().ok_or(SiaplaError::new("Weak ref not upgradable in dataloader."))?;
    let txn = ctx.txn().await?;
    let resolved_revision = resolve_revision(txn, revision).await?;
    let mut query = ET::find().filter(col.is_in(values.to_vec()));
    if let (Some(revision), Some(rev_created_idx), Some(rev_deleted_idx)) =
        (resolved_revision, rev_created_idx, rev_deleted_idx)
    {
        let rev_created_col: ET::Column = ET::Column::iter()
            .nth(rev_created_idx)
            .expect("Loader with invalid rev_created column index");
        let rev_deleted_col: ET::Column = ET::Column::iter()
            .nth(rev_deleted_idx)
            .expect("Loader with invalid rev_deleted column index");
        query = query.filter(active_for_revision(rev_created_col, rev_deleted_col, Some(revision))?);
    }
    let tasks: Vec<ET::Model> = query.order_by_asc(col).all(txn).await?;
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
        match fallible_load::<ET, CIDX>(
            &self.ctx,
            values,
            self.revision,
            self.rev_created_idx,
            self.rev_deleted_idx,
        )
        .await
        {
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
