use std::{
    env,
    marker::PhantomData,
    sync::{Arc, Weak},
};

use super::dataloader::{ByColBatcher, ByColLoader};
use sea_orm::{Database, DatabaseConnection, EntityTrait, strum::IntoEnumIterator};
use tokio::sync::{OnceCell, RwLock};
#[derive(Clone)]
pub struct Context {
    db: OnceCell<DatabaseConnection>,
    by_col_loaders: Arc<RwLock<anymap::Map<dyn anymap::any::Any + Send + Sync>>>,
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
            db: Default::default(),
            by_col_loaders: Arc::new(RwLock::new(
                anymap::Map::<dyn anymap::any::Any + Send + Sync>::new(),
            )),
            me: me.clone(),
        })
    }

    pub async fn db(&self) -> anyhow::Result<&DatabaseConnection> {
        self.db
            .get_or_try_init::<anyhow::Error, _, _>(|| async {
                Ok(Database::connect(env::var("DATABASE_URL")?).await?)
            })
            .await
    }

    /// Generic dataloader that loads values by column value
    ///
    /// Column selection is a little hackish, you have to provide the column as usize.
    /// Usage:
    /// ```
    /// const CIDX: usize = task::Column::Id as usize;
    /// ctx.load_by_col::<task::Entity, CIDX>(parent_id).await
    /// ```
    pub async fn load_by_col<ET: EntityTrait, const CIDX: usize>(
        &self,
        value: impl Into<sea_orm::Value>,
    ) -> anyhow::Result<Vec<ET::Model>>
    where
        ET::Column: IntoEnumIterator,
    {
        // check that the column exists
        <ET::Column as IntoEnumIterator>::iter()
            .nth(CIDX)
            .ok_or(anyhow::anyhow!("Column index does not exist!"))?;
        loop {
            let read_loaders = self.by_col_loaders.read().await;
            let opt_loader = read_loaders.get::<ByColLoader<ET, CIDX>>();
            let loader = match opt_loader {
                Some(loader) => loader,
                None => {
                    drop(read_loaders);
                    let mut write_loaders = self.by_col_loaders.write().await;
                    let loader_entry = write_loaders.entry::<ByColLoader<ET, CIDX>>();
                    match loader_entry {
                        anymap::Entry::Occupied(_) => {
                            // retry with a read lock
                        }
                        anymap::Entry::Vacant(entry) => {
                            // insert new loader and then retry with read lock
                            let loader = ByColLoader::<ET, CIDX>::new(ByColBatcher::<ET, CIDX> {
                                ctx: self.me.clone(),
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
    }

    pub async fn load_one_by_col<ET: EntityTrait, const CIDX: usize>(
        &self,
        value: impl Into<sea_orm::Value>,
    ) -> anyhow::Result<Option<ET::Model>>
    where
        ET::Column: IntoEnumIterator,
    {
        let mut res = self.load_by_col::<ET, CIDX>(value).await?;
        if res.is_empty() {
            Ok(None)
        } else if res.len() == 1 {
            Ok(res.drain(..).next())
        } else {
            Err(anyhow::anyhow!("More than one entry found"))
        }
    }
}
