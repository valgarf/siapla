use std::{
    env,
    marker::PhantomData,
    sync::{Arc, Weak},
};

use super::dataloader::{AvailabilityBatcher, AvailabilityLoader, ByColBatcher, ByColLoader};
use crate::scheduling::Intervals;
use chrono::NaiveDateTime;

type AvailabilityLoaderMap =
    std::collections::HashMap<(NaiveDateTime, NaiveDateTime), AvailabilityLoader>;
use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use futures::TryFutureExt;
use sea_orm::{
    Database, DatabaseTransaction, EntityTrait, TransactionTrait as _, strum::IntoEnumIterator,
};
use tokio::sync::{OnceCell, RwLock};

pub struct Context {
    txn: OnceCell<DatabaseTransaction>,
    by_col_loaders: Arc<RwLock<anymap::Map<dyn anymap::any::Any + Send + Sync>>>,
    availability_loaders: Arc<RwLock<AvailabilityLoaderMap>>,
    me: Weak<Self>,
}

impl juniper::Context for Context {}

async fn load_by_column_value<ET: EntityTrait, const CIDX: usize>(
    loader: &ByColLoader<ET, CIDX>,
    value: impl Into<sea_orm::Value>,
) -> anyhow::Result<Vec<ET::Model>>
where
    ET::Column: IntoEnumIterator,
{
    let id_value: sea_orm::Value = value.into();
    let res = loader.try_load(id_value).await;
    match res {
        Err(_) => Ok(vec![]), // id cannot be found
        Ok(value) => Ok(value.map_err(|err| anyhow::anyhow!("{err}"))?),
    }
}

impl Context {
    pub fn new() -> Arc<Self> {
        Arc::new_cyclic(|me| Self {
            txn: Default::default(),
            by_col_loaders: Arc::new(RwLock::new(
                anymap::Map::<dyn anymap::any::Any + Send + Sync>::new(),
            )),
            availability_loaders: Arc::new(RwLock::new(AvailabilityLoaderMap::new())),
            me: me.clone(),
        })
    }

    pub async fn txn(&self) -> anyhow::Result<&DatabaseTransaction> {
        self.txn
            .get_or_try_init::<anyhow::Error, _, _>(|| async {
                Ok(Database::connect(env::var("DATABASE_URL")?).await?.begin().await?)
            })
            .await
    }

    pub async fn commit(&mut self) -> anyhow::Result<()> {
        if let Some(txn) = self.txn.take() {
            txn.commit().await?;
        }
        Ok(())
    }

    pub async fn rollback(&mut self) -> anyhow::Result<()> {
        if let Some(txn) = self.txn.take() {
            txn.rollback().await?
        }
        Ok(())
    }

    /// Generic dataloader that loads values by column value
    ///
    /// Column selection is a little hackish, you have to provide the column as usize.
    /// Usage:
    /// ```ignore
    /// const CIDX: usize = task::Column::Id as usize;
    /// ctx.load_by_col::<task::Entity, CIDX>(parent_id).await
    /// ```
    pub fn load_by_col<ET: EntityTrait, const CIDX: usize>(
        &self,
        value: impl Into<sea_orm::Value>,
    ) -> impl Future<Output = anyhow::Result<Vec<ET::Model>>> + 'static
    where
        ET::Column: IntoEnumIterator,
    {
        let loaders = Arc::clone(&self.by_col_loaders);
        let me = self.me.clone();
        let value: sea_orm::Value = value.into();
        let fut = async move {
            // check that the column exists
            <ET::Column as IntoEnumIterator>::iter()
                .nth(CIDX)
                .ok_or(anyhow::anyhow!("Column index does not exist!"))?;
            loop {
                let read_loaders = loaders.read().await;
                let opt_loader = read_loaders.get::<ByColLoader<ET, CIDX>>();
                let loader = match opt_loader {
                    Some(loader) => loader,
                    None => {
                        drop(read_loaders);
                        let mut write_loaders = loaders.write().await;
                        let loader_entry = write_loaders.entry::<ByColLoader<ET, CIDX>>();
                        match loader_entry {
                            anymap::Entry::Occupied(_) => {
                                // retry with a read lock
                            }
                            anymap::Entry::Vacant(entry) => {
                                // insert new loader and then retry with read lock
                                let loader =
                                    ByColLoader::<ET, CIDX>::new(ByColBatcher::<ET, CIDX> {
                                        ctx: me.clone(),
                                        pd: PhantomData,
                                    })
                                    .with_yield_count(100);
                                entry.insert(loader);
                            }
                        }
                        drop(write_loaders);
                        continue;
                    }
                };
                return load_by_column_value(loader, value).await;
            }
        };
        fut
    }

    pub fn load_one_by_col<ET: EntityTrait, const CIDX: usize>(
        &self,
        value: impl Into<sea_orm::Value>,
    ) -> impl Future<Output = anyhow::Result<Option<ET::Model>>> + 'static
    where
        ET::Column: IntoEnumIterator,
    {
        let fut = self.load_by_col::<ET, CIDX>(value);
        let new_fut = fut.and_then(|mut res: Vec<ET::Model>| async move {
            if res.is_empty() {
                Ok(None)
            } else if res.len() == 1 {
                Ok(res.drain(..).next())
            } else {
                Err(anyhow::anyhow!("More than one entry found"))
            }
        });
        new_fut
        // let mut res = self.load_by_col::<ET, CIDX>(value).await?;
        // if res.is_empty() {
        //     Ok(None)
        // } else if res.len() == 1 {
        //     Ok(res.drain(..).next())
        // } else {
        //     Err(anyhow::anyhow!("More than one entry found"))
        // }
    }

    /// Load availability intervals for a single resource id, for the given start/end window.
    /// Loaders are cached per (start,end) pair in an AvailabilityLoaderMap stored in the same anymap
    /// used for the `by_col_loaders`.
    pub fn load_combined_availability(
        &self,
        resource_id: i32,
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> impl std::future::Future<Output = anyhow::Result<Intervals<NaiveDateTime>>> + 'static {
        let loaders = Arc::clone(&self.availability_loaders);
        let me = self.me.clone();
        let fut = async move {
            loop {
                let read_map = loaders.read().await;
                let opt_loader = read_map.get(&(start, end)).cloned();
                if let Some(loader) = opt_loader {
                    let res = loader.try_load(resource_id).await;
                    match res {
                        Err(_) => return Ok(Intervals::new()), // id cannot be found
                        Ok(v) => return Ok(v.map_err(|err| anyhow::anyhow!("{err}"))?),
                    }
                }
                drop(read_map);
                let mut write_map = loaders.write().await;
                if !write_map.contains_key(&(start, end)) {
                    let loader = AvailabilityLoader::new(AvailabilityBatcher {
                        ctx: me.clone(),
                        start,
                        end,
                    })
                    .with_yield_count(100);
                    write_map.insert((start, end), loader);
                }
                drop(write_map);
                continue;
            }
        };
        fut
    }
}

pub async fn add_context(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let ctx = Context::new();
    req.extensions_mut().insert(Arc::clone(&ctx));
    let res = next.run(req).await;
    let mut ctx = Arc::into_inner(ctx).expect("All other references should have been destroyed");
    ctx.commit().await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(res)
}
