use std::{any::Any, collections::HashMap, sync::Arc};
use super::batcher::BatcherLoaderKey;
use tokio::sync::RwLock;

pub type GenericBatchLoaderMap = RwLock<HashMap<BatcherLoaderKey, Arc<dyn Any + Send + Sync>>>;
