use anyhow::{bail, Result};
use turbo_tasks::primitives::StringVc;
use turbopack_binding::turbopack::core::{
    asset::{Asset, AssetContentVc, AssetVc, AssetsVc},
    chunk::{ChunkableAsset, ChunkableAssetVc, ChunkingContext, ChunkingContextVc},
    ident::AssetIdentVc,
    reference::AssetReferencesVc,
};

/// A [`NextDynamicEntryAsset`] is a marker asset used to indicate which dynamic
/// assets should appear in the dynamic manifest.
#[turbo_tasks::value(transparent)]
pub struct NextDynamicEntryAsset {
    pub client_entry_asset: AssetVc,
}

#[turbo_tasks::value_impl]
impl NextDynamicEntryAssetVc {
    /// Create a new [`NextDynamicEntryAsset`] from the given source CSS
    /// asset.
    #[turbo_tasks::function]
    pub fn new(client_entry_asset: AssetVc) -> NextDynamicEntryAssetVc {
        NextDynamicEntryAsset { client_entry_asset }.cell()
    }

    #[turbo_tasks::function]
    pub async fn client_chunks(
        self,
        client_chunking_context: ChunkingContextVc,
    ) -> Result<AssetsVc> {
        let this = self.await?;

        let Some(client_entry_asset) = ChunkableAssetVc::resolve_from(this.client_entry_asset).await? else {
            bail!("dynamic client asset must be chunkable");
        };

        let client_entry_chunk = client_entry_asset.as_root_chunk(client_chunking_context.into());
        Ok(client_chunking_context.chunk_group(client_entry_chunk))
    }
}

#[turbo_tasks::function]
fn dynamic_modifier() -> StringVc {
    StringVc::cell("dynamic".to_string())
}

#[turbo_tasks::value_impl]
impl Asset for NextDynamicEntryAsset {
    #[turbo_tasks::function]
    fn ident(&self) -> AssetIdentVc {
        self.client_entry_asset
            .ident()
            .with_modifier(dynamic_modifier())
    }

    #[turbo_tasks::function]
    fn content(&self) -> Result<AssetContentVc> {
        // The client reference asset only serves as a marker asset.
        bail!("NextDynamicEntryAsset has no content")
    }

    #[turbo_tasks::function]
    fn references(_self_vc: NextDynamicEntryAssetVc) -> AssetReferencesVc {
        AssetReferencesVc::empty()
    }
}
