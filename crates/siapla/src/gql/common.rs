macro_rules! opt_to_av {
    ($opt: expr) => {
        match $opt {
            Some(v) => ::sea_orm::ActiveValue::Set(v),
            None => ::sea_orm::ActiveValue::NotSet,
        }
    };
}
pub(crate) use opt_to_av;

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

macro_rules! resolve_many_to_many {
    ($ctx: ident, $link_ent: ty,  $link_from_col: expr, $from_id: expr, $target_id_field: expr, $target_ent: ty, $target_col: expr) => {{
        const CIDX: usize = $link_from_col as usize;
        match $ctx.load_by_col::<$link_ent, CIDX>($from_id).await {
            Err(err) => Err(err),
            Ok(links) => {
                let mut joins = tokio::task::JoinSet::new();
                for link in links {
                    const CIDX: usize = $target_col as usize;
                    joins.spawn($ctx.load_one_by_col::<$target_ent, CIDX>($target_id_field(link)));
                }
                let results = joins.join_all().await;
                let (values, mut errors): (Vec<_>, Vec<_>) =
                    results.into_iter().partition_map(|v| match v {
                        Ok(Some(v)) => ::itertools::Either::Left(v),
                        Ok(None) => ::itertools::Either::Right(::anyhow::anyhow!(
                            "Could not resolve link between {} and {}",
                            ::std::any::type_name::<$link_ent>(),
                            ::std::any::type_name::<$target_ent>()
                        )),
                        Err(e) => ::itertools::Either::Right(e),
                    });
                let first_error = errors.drain(..).next();
                if let Some(err) = first_error { Err(err) } else { Ok(values) }
            }
        }
    }};
}

pub(crate) use resolve_many_to_many;
