use sea_orm::EntityTrait;

pub struct ModelWrapper<ET: EntityTrait> {
    pub model: ET::Model,
    pub revision: i64,
}

impl<ET: EntityTrait> ModelWrapper<ET> {
    pub fn at_revision(model: ET::Model, revision: i64) -> Self {
        Self { model, revision }
    }
}
/// ---------------------------------------------------------------------------------
/// Conversion traits for typical models
/// ---------------------------------------------------------------------------------

pub trait ResultOptionToWrapper<ET: EntityTrait> {
    fn into_wrapper(self, revision: i64) -> anyhow::Result<Option<ModelWrapper<ET>>>;
}

impl<ET: EntityTrait> ResultOptionToWrapper<ET> for anyhow::Result<Option<ET::Model>> {
    fn into_wrapper(self, revision: i64) -> anyhow::Result<Option<ModelWrapper<ET>>> {
        self.map(|opt_model| opt_model.map(|m| ModelWrapper::at_revision(m, revision)))
    }
}

pub trait ResultVecToWrapper<ET: EntityTrait> {
    fn into_wrapper(self, revision: i64) -> anyhow::Result<Vec<ModelWrapper<ET>>>;
}

impl<ET: EntityTrait> ResultVecToWrapper<ET> for anyhow::Result<Vec<ET::Model>> {
    fn into_wrapper(self, revision: i64) -> anyhow::Result<Vec<ModelWrapper<ET>>> {
        self.map(|models| {
            models.into_iter().map(|m| ModelWrapper::at_revision(m, revision)).collect()
        })
    }
}
