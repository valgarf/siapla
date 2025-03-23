use std::{collections::HashMap, sync::Arc, sync::Weak};

use dataloader::cached::Loader;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::{SiaplaError, entity::task};

use super::context::Context;

#[derive(Clone)]
pub struct IdBatcher {
    pub ctx: Weak<Context>,
}

impl dataloader::BatchFn<i32, Result<task::Model, Arc<anyhow::Error>>> for IdBatcher {
    async fn load(
        &mut self,
        task_ids: &[i32],
    ) -> HashMap<i32, Result<task::Model, Arc<anyhow::Error>>> {
        async fn inner_load(
            ctx: &Weak<Context>,
            task_ids: &[i32],
        ) -> Result<HashMap<i32, Result<task::Model, Arc<anyhow::Error>>>, anyhow::Error> {
            let ctx = ctx
                .upgrade()
                .ok_or(SiaplaError::new("Weak ref not upgradable in dataloader."))?;
            let db = ctx.db().await?;
            let tasks = task::Entity::find()
                .filter(task::Column::Id.is_in(task_ids.to_vec()))
                .all(db)
                .await?;
            Ok(tasks.into_iter().map(|task| (task.id, Ok(task))).collect())
        }
        match inner_load(&self.ctx, task_ids).await {
            Ok(data) => data,
            Err(err) => {
                let clonable_err = Arc::new(err);
                task_ids
                    .iter()
                    .map(|k| (*k, Err(clonable_err.clone())))
                    .collect()
            }
        }
    }
}

// #[derive(Clone)]
// pub struct Batcher {
//     pub ctx: Weak<Context>,
// }

// impl dataloader::BatchFn<i32, Result<task::Model, Arc<anyhow::Error>>> for Batcher {
//     async fn load(
//         &mut self,
//         task_ids: &[i32],
//     ) -> HashMap<i32, Result<task::Model, Arc<anyhow::Error>>> {
//         async fn inner_load(
//             ctx: &Weak<Context>,
//             task_ids: &[i32],
//         ) -> Result<HashMap<i32, Result<task::Model, Arc<anyhow::Error>>>, anyhow::Error> {
//             let ctx = ctx
//                 .upgrade()
//                 .ok_or(SiaplaError::new("Weak ref not upgradable in dataloader."))?;
//             let db = ctx.db().await?;
//             let tasks = task::Entity::find()
//                 .filter(task::Column::Id.is_in(task_ids.to_vec()))
//                 .all(db)
//                 .await?;
//             Ok(tasks.into_iter().map(|task| (task.id, Ok(task))).collect())
//         }
//         match inner_load(&self.ctx, task_ids).await {
//             Ok(data) => data,
//             Err(err) => {
//                 let clonable_err = Arc::new(err);
//                 task_ids
//                     .iter()
//                     .map(|k| (*k, Err(clonable_err.clone())))
//                     .collect()
//             }
//         }
//     }
// }

pub type TaskLoader = Loader<i32, Result<task::Model, Arc<anyhow::Error>>, IdBatcher>;
