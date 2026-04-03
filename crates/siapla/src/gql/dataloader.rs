use std::{
    any::{Any, TypeId},
    cmp::{max, min},
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

use tokio::sync::RwLock;

use dataloader::cached::Loader;
use itertools::Itertools as _;
use sea_orm::{ColumnTrait, EntityTrait, ModelTrait, Order, QueryFilter, QueryOrder};

use super::context::Context;
use crate::{ColumnIntoUsize, RevModeEntity};

use crate::entity::{availability, resource_iteration as resource, vacation};
use crate::scheduling::{Interval, Intervals};
use chrono::{DateTime, Datelike, NaiveDateTime, NaiveTime, TimeDelta, Utc, Weekday};
use chrono_tz::Tz;
use sea_orm::prelude::Decimal;

use std::fmt::Debug;

pub trait Batcher: Clone + Send + Sync + 'static {
    type Key: Eq + Hash + Clone + Debug + Send + Sync;
    type Value: Clone + Send + Sync;

    fn load(
        &self,
        ctx: &Context,
        keys: &[Self::Key],
    ) -> impl Future<Output = Result<HashMap<Self::Key, Self::Value>, anyhow::Error>> + Send;
}

pub trait BatcherToKey: Batcher {
    type MapKey: Eq + Hash + Clone + Debug + Send + Sync;
    fn loader_map_key(&self) -> Self::MapKey;

    fn loader(
        &self,
        ctx: &Weak<Context>,
    ) -> impl Future<
        Output = Result<
            Arc<Loader<Self::Key, Result<Self::Value, Arc<anyhow::Error>>, BatcherWrapper<Self>>>,
            anyhow::Error,
        >,
    > + Send {
        let ctx = ctx.clone();
        let batcher = self.clone();
        async move {
            match ctx.upgrade() {
                None => {
                    Err(anyhow::anyhow!("Weak ref not upgradable in dataloader loader creation"))
                }
                Some(ctx) => Ok(ctx.loader(batcher).await),
            }
        }
    }
}

impl<B: Batcher> BatcherToKey for B
where
    B: Hash + Eq + Debug,
{
    type MapKey = Self;
    fn loader_map_key(&self) -> Self::MapKey {
        self.clone()
    }
}

pub struct BatcherLoaderKey {
    type_id: TypeId,
    batcher_key: Arc<dyn Any + Send + Sync>,
    eq_fn: fn(&(dyn Any + Send + Sync), &(dyn Any + Send + Sync)) -> bool,
    hash_fn: fn(&(dyn Any + Send + Sync), &mut dyn Hasher),
}

impl BatcherLoaderKey {
    pub(crate) fn new<B: BatcherToKey>(batcher_key: B::MapKey) -> Self {
        Self {
            type_id: TypeId::of::<B>(),
            batcher_key: Arc::new(batcher_key),
            eq_fn: |left, right| {
                right
                    .downcast_ref::<B::MapKey>()
                    .is_some_and(|right| left.downcast_ref::<B::MapKey>() == Some(right))
            },
            hash_fn: |batcher_key, state| {
                if let Some(batcher_key) = batcher_key.downcast_ref::<B::MapKey>() {
                    let mut inner = std::collections::hash_map::DefaultHasher::new();
                    batcher_key.hash(&mut inner);
                    state.write_u64(inner.finish());
                }
            },
        }
    }
}

impl Clone for BatcherLoaderKey {
    fn clone(&self) -> Self {
        Self {
            type_id: self.type_id,
            batcher_key: Arc::clone(&self.batcher_key),
            eq_fn: self.eq_fn,
            hash_fn: self.hash_fn,
        }
    }
}

impl PartialEq for BatcherLoaderKey {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
            && (self.eq_fn)(self.batcher_key.as_ref(), other.batcher_key.as_ref())
    }
}

impl Eq for BatcherLoaderKey {}

impl std::hash::Hash for BatcherLoaderKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
        (self.hash_fn)(self.batcher_key.as_ref(), state);
    }
}

pub type GenericBatchLoaderMap = RwLock<HashMap<BatcherLoaderKey, Arc<dyn Any + Send + Sync>>>;

pub struct BatcherWrapper<B: Batcher> {
    ctx: Weak<Context>,
    batcher: B,
}

impl<B: Batcher> BatcherWrapper<B> {
    pub(crate) fn new(ctx: Weak<Context>, batcher: B) -> Self {
        Self { ctx, batcher }
    }
}

impl<B: Batcher> dataloader::BatchFn<B::Key, Result<B::Value, Arc<anyhow::Error>>>
    for BatcherWrapper<B>
{
    async fn load(
        &mut self,
        values: &[B::Key],
    ) -> HashMap<B::Key, Result<B::Value, Arc<anyhow::Error>>> {
        let ctx = self.ctx.upgrade();
        match ctx {
            None => {
                let a = Arc::new(anyhow::anyhow!("Weak ref not upgradable in dataloader."));
                values.iter().map(|k| (k.clone(), Err(a.clone()))).collect()
            }
            Some(ctx) => match self.batcher.load(&ctx, values).await {
                Ok(data) => data.into_iter().map(|(k, v)| (k, Ok(v))).collect(),
                Err(err) => {
                    let clonable_err = Arc::new(err);
                    values.iter().map(|k| (k.clone(), Err(clonable_err.clone()))).collect()
                }
            },
        }
    }
}

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
        .filter(resource::Entity::rev_condition(revision))
        .all(db)
        .await?;
    let res_map = db_resources
        .into_iter()
        .filter_map(|r| r.header_id.map(|hid| (hid, r)))
        .collect::<HashMap<i32, _>>();

    let db_availabilities = availability::Entity::find()
        .filter(availability::Column::ResourceId.is_in(id_set.clone()))
        .filter(availability::Entity::rev_condition(revision))
        .all(db)
        .await?;
    let db_vacations = vacation::Entity::find()
        .filter(vacation::Column::ResourceId.is_in(id_set.clone()))
        .filter(vacation::Entity::rev_condition(revision))
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
        ctx: &Context,
        values: &[Self::Key],
    ) -> Result<HashMap<Self::Key, Self::Value>, anyhow::Error> {
        let ids = values.to_vec();
        match query_combined_availability(&ctx, &ids, self.start, self.end, self.revision).await {
            Ok(vec) => Ok(ids.into_iter().zip(vec.into_iter()).collect()),
            Err(err) => Err(err),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ByColBatcher<ET: EntityTrait>
where
    ET::Model: Send + Sync,
{
    pub col: ET::Column,
}

impl<ET: EntityTrait> Batcher for ByColBatcher<ET>
where
    ET::Model: Send + Sync,
{
    type Key = sea_orm::Value;
    type Value = Vec<ET::Model>;

    async fn load(
        &self,
        ctx: &Context,
        values: &[Self::Key],
    ) -> Result<HashMap<Self::Key, Self::Value>, anyhow::Error> {
        let txn = ctx.txn().await?;
        let query = ET::find().filter(self.col.is_in(values.to_vec()));

        let models: Vec<ET::Model> = query.order_by_asc(self.col).all(txn).await?;
        Ok(models
            .into_iter()
            .chunk_by(|model| model.get(self.col))
            .into_iter()
            .map(|(key, models)| (key, models.collect()))
            .collect())
    }
}

impl<ET: EntityTrait> BatcherToKey for ByColBatcher<ET>
where
    ET::Column: ColumnIntoUsize,
    ET::Model: Send + Sync,
{
    type MapKey = usize;
    fn loader_map_key(&self) -> Self::MapKey {
        self.col.to_column_index()
    }
}

#[derive(Debug, Clone)]
pub struct ByColRevBatcher<ET: EntityTrait>
where
    ET::Model: Send + Sync,
{
    pub revision: i64,
    pub col: ET::Column,
}

impl<ET> Batcher for ByColRevBatcher<ET>
where
    ET: RevModeEntity,
    ET::Model: Send + Sync,
{
    type Key = sea_orm::Value;
    type Value = Vec<ET::Model>;

    async fn load(
        &self,
        ctx: &Context,
        values: &[Self::Key],
    ) -> Result<HashMap<Self::Key, Self::Value>, anyhow::Error> {
        let txn = ctx.txn().await?;
        let query = ET::find()
            .filter(self.col.is_in(values.to_vec()))
            .filter(ET::rev_condition(self.revision));

        let models: Vec<ET::Model> = query.order_by_asc(self.col).all(txn).await?;
        Ok(models
            .into_iter()
            .chunk_by(|model| model.get(self.col))
            .into_iter()
            .map(|(key, models)| (key, models.collect()))
            .collect())
    }
}

impl<ET> BatcherToKey for ByColRevBatcher<ET>
where
    ET: RevModeEntity,
    ET::Column: ColumnIntoUsize,
    ET::Model: Send + Sync,
{
    type MapKey = (i64, usize);
    fn loader_map_key(&self) -> Self::MapKey {
        (self.revision, self.col.to_column_index())
    }
}
