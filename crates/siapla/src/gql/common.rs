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
        let link_loader = $ctx
            .loader(crate::gql::dataloader::ByColBatcher::<$link_ent> { col: $link_from_col })
            .await;
        match link_loader.load(($from_id).into()).await {
            Err(err) => Ok(err),
            Ok(links) => {
                let target_loader = $ctx
                    .loader(crate::gql::dataloader::ByColBatcher::<$target_ent> {
                        col: $target_col,
                    })
                    .await;
                let target_ids: ::std::vec::Vec<::sea_orm::Value> =
                    links.iter().map(|link| $target_id_field(link.clone()).into()).collect();
                let targets = target_loader.load_many_one(target_ids.clone()).await?;
                target_ids
                    .into_iter()
                    .map(|target_id| {
                        targets.get(&target_id).cloned().flatten().ok_or_else(|| {
                            ::anyhow::anyhow!(
                                "Could not resolve link between {} and {}",
                                ::std::any::type_name::<$link_ent>(),
                                ::std::any::type_name::<$target_ent>()
                            )
                        })
                    })
                    .collect::<::anyhow::Result<::std::vec::Vec<_>>>()
            }
        }
    }};

    // Revision-aware dataloader form for revisioned link and target entities
    ($ctx:ident, $revision:expr, $link_ent:ty, $link_from_col:expr, $from_id:expr, $target_id_field:expr, $target_ent:ty, $target_col:expr) => {{
        let revision = $revision;
        let txn = $ctx.txn().await?;
        let revision = crate::revisioning::resolve_revision(txn, revision)
            .await?
            .ok_or(::anyhow::anyhow!("No revision found in database"))?;
        let link_loader = $ctx
            .loader(crate::gql::dataloader::ByColRevBatcher::<$link_ent> {
                revision,
                col: $link_from_col,
            })
            .await;
        match link_loader.load(($from_id).into()).await {
            Err(err) => Err(err),
            Ok(links) => {
                let target_loader = $ctx
                    .loader(crate::gql::dataloader::ByColRevBatcher::<$target_ent> {
                        revision,
                        col: $target_col,
                    })
                    .await;
                let target_ids: ::std::vec::Vec<::sea_orm::Value> =
                    links.iter().map(|link| $target_id_field(link.clone()).into()).collect();
                let targets = target_loader.load_many_one(target_ids.clone()).await?;
                target_ids
                    .into_iter()
                    .map(|target_id| {
                        targets.get(&target_id).cloned().flatten().ok_or_else(|| {
                            ::anyhow::anyhow!(
                                "Could not resolve link between {} and {}",
                                ::std::any::type_name::<$link_ent>(),
                                ::std::any::type_name::<$target_ent>()
                            )
                        })
                    })
                    .collect::<::anyhow::Result<::std::vec::Vec<_>>>()
            }
        }
    }};

    // Revision-aware target form for non-revisioned link entities
    ($ctx:ident, target_revision: $revision:expr, $link_ent:ty, $link_from_col:expr, $from_id:expr, $target_id_field:expr, $target_ent:ty, $target_col:expr) => {{
        let link_loader = $ctx
            .loader(crate::gql::dataloader::ByColBatcher::<$link_ent> { col: $link_from_col })
            .await;
        match link_loader.load(($from_id).into()).await {
            Err(err) => Err(err),
            Ok(links) => {
                let revision = $revision;
                let txn = $ctx.txn().await?;
                let revision = crate::revisioning::resolve_revision(txn, revision)
                    .await?
                    .ok_or(::anyhow::anyhow!("No revision found in database"))?;
                let target_loader = $ctx
                    .loader(crate::gql::dataloader::ByColRevBatcher::<$target_ent> {
                        revision,
                        col: $target_col,
                    })
                    .await;
                let target_ids: ::std::vec::Vec<::sea_orm::Value> =
                    links.iter().map(|link| $target_id_field(link.clone()).into()).collect();
                let targets = target_loader.load_many_one(target_ids.clone()).await?;
                target_ids
                    .into_iter()
                    .map(|target_id| {
                        targets.get(&target_id).cloned().flatten().ok_or_else(|| {
                            ::anyhow::anyhow!(
                                "Could not resolve link between {} and {}",
                                ::std::any::type_name::<$link_ent>(),
                                ::std::any::type_name::<$target_ent>()
                            )
                        })
                    })
                    .collect::<::anyhow::Result<::std::vec::Vec<_>>>()
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
