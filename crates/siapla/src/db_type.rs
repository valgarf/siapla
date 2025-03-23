use sea_orm::{ColumnTrait, EntityTrait, ModelTrait, PrimaryKeyTrait, sea_query::SimpleExpr};

pub trait DbType {
    type Entity: EntityTrait
        + DbType<
            Entity = Self::Entity,
            Model = Self::Model,
            Column = Self::Column,
            PK = Self::PK,
            PKValueType = Self::PKValueType,
        >;
    type Model: ModelTrait
        + DbType<
            Entity = Self::Entity,
            Model = Self::Model,
            Column = Self::Column,
            PK = Self::PK,
            PKValueType = Self::PKValueType,
        >;
    type Column: ColumnTrait;
    type PK: PrimaryKeyTrait;
    type PKValueType: Copy + Clone;
    fn pk_is_in(values: Vec<Self::PKValueType>) -> SimpleExpr;
    fn get_pk(model: &Self::Model) -> &Self::PKValueType;
}

impl DbType for crate::entity::task::Model {
    type Column = crate::entity::task::Column;
    type Model = crate::entity::task::Model;
    type Entity = crate::entity::task::Entity;
    type PK = crate::entity::task::PrimaryKey;
    type PKValueType = <Self::PK as PrimaryKeyTrait>::ValueType;

    fn pk_is_in(values: Vec<Self::PKValueType>) -> SimpleExpr {
        Self::Column::Id.is_in(values)
    }
    fn get_pk(model: &Self::Model) -> &Self::PKValueType {
        &model.id
    }
}

impl DbType for crate::entity::task::Entity {
    type Column = crate::entity::task::Column;
    type Model = crate::entity::task::Model;
    type Entity = crate::entity::task::Entity;
    type PK = crate::entity::task::PrimaryKey;
    type PKValueType = <Self::PK as PrimaryKeyTrait>::ValueType;

    fn pk_is_in(values: Vec<Self::PKValueType>) -> SimpleExpr {
        Self::Column::Id.is_in(values)
    }
    fn get_pk(model: &Self::Model) -> &Self::PKValueType {
        &model.id
    }
}
