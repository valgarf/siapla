use sea_orm::ActiveModelTrait;

use crate::db::{
    context::DbContext, entity::resource_header, insert::insert_rev, upsert::LazyRevision,
};
use anyhow;

impl resource_header::Model {
    pub async fn ensure_header_id(
        db: &DbContext,
        revision: &LazyRevision,
        header_id: Option<i32>,
    ) -> anyhow::Result<i32> {
        if let Some(header_id) = header_id {
            Ok(header_id)
        } else {
            // If no header ID is provided, we are creating a new resource.
            let am = vec![<resource_header::ActiveModel as ActiveModelTrait>::default()];
            let res = insert_rev::<resource_header::Entity>(db, revision, am).await?;
            res.ok_or(anyhow::anyhow!("Failed to insert resource header"))
        }
    }
}
