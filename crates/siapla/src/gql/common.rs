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
    // Original form: dataloader-based, no filtering
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

    // Revision-aware dataloader form
    ($ctx:ident, $revision:expr, $link_ent:ty, $link_from_col:expr, $from_id:expr, $target_id_field:expr, $target_ent:ty, $target_col:expr) => {{
        const CIDX: usize = $link_from_col as usize;
        match $ctx.load_by_col_at_revision::<$link_ent, CIDX>($from_id, $revision).await {
            Err(err) => Err(err),
            Ok(links) => {
                let mut joins = tokio::task::JoinSet::new();
                for link in links {
                    const CIDX: usize = $target_col as usize;
                    joins.spawn($ctx.load_one_by_col_at_revision::<$target_ent, CIDX>(
                        $target_id_field(link),
                        $revision,
                    ));
                }
                let results = joins.join_all().await;
                let (values, mut errors): (Vec<_>, Vec<_>) =
                    ::itertools::Itertools::partition_map(results.into_iter(), |v| match v {
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

    // Filtered form: direct queries with additional filter conditions on link and target entities.
    // Use this when you need other custom filtering that dataloaders don't support.
    ($ctx:ident, $link_ent:ty, $link_from_col:expr, $from_id:expr, $target_id_field:expr, $target_ent:ty, $target_col:expr,
     link_filter: $link_filter:expr, target_filter: $target_filter:expr) => {{
        let txn = $ctx.txn().await?;
        let _lf_col = $link_from_col;
        let links =
            <$link_ent>::find().filter(_lf_col.eq($from_id)).filter($link_filter).all(txn).await?;
        let target_ids: ::std::vec::Vec<_> = links.into_iter().map($target_id_field).collect();
        if target_ids.is_empty() {
            Ok::<::std::vec::Vec<<$target_ent as ::sea_orm::EntityTrait>::Model>, ::anyhow::Error>(
                ::std::vec::Vec::new(),
            )
        } else {
            let _tf_col = $target_col;
            Ok::<_, ::anyhow::Error>(
                <$target_ent>::find()
                    .filter(_tf_col.is_in(target_ids))
                    .filter($target_filter)
                    .all(txn)
                    .await?,
            )
        }
    }};
}

pub(crate) use resolve_many_to_many;
