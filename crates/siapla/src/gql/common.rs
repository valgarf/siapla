macro_rules! opt_to_av {
    ($opt: expr) => {
        match $opt {
            Some(v) => ::sea_orm::ActiveValue::Set(v),
            None => ::sea_orm::ActiveValue::NotSet,
        }
    };
}

macro_rules! nullable_to_av {
    ($opt: expr) => {
        match $opt {
            ::juniper::Nullable::Some(v) => ::sea_orm::ActiveValue::Set(Some(v)),
            ::juniper::Nullable::ExplicitNull => ::sea_orm::ActiveValue::Set(None),
            ::juniper::Nullable::ImplicitNull => ::sea_orm::ActiveValue::NotSet,
        }
    };
}

pub(crate) use nullable_to_av;
pub(crate) use opt_to_av;
