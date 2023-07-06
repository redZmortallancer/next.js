use anyhow::{bail, Result};
use turbo_tasks::primitives::StringVc;
use turbopack_binding::turbopack::{
    core::{
        asset::{Asset, AssetContentVc, AssetVc},
        ident::AssetIdentVc,
        reference::AssetReferencesVc,
    },
    ecmascript::chunk::EcmascriptChunkPlaceableVc,
};

/// An [`EcmascriptClientReferenceAsset`] is a marker asset, used by the
/// [`EcmascriptProxyModuleAsset`] to indicate which client reference should
/// appear in the client reference manifest.
#[turbo_tasks::value(transparent)]
pub struct EcmascriptClientReferenceAsset {
    pub server_ident: AssetIdentVc,
    pub client_asset: EcmascriptChunkPlaceableVc,
    pub ssr_asset: EcmascriptChunkPlaceableVc,
}

#[turbo_tasks::value_impl]
impl EcmascriptClientReferenceAssetVc {
    /// Create a new [`EcmascriptClientReferenceAsset`].
    ///
    /// # Arguments
    ///
    /// * `server_ident` - The identifier of the server asset, used to identify
    ///   the client reference.
    /// * `client_asset` - The client asset.
    /// * `ssr_asset` - The SSR asset.
    #[turbo_tasks::function]
    pub fn new(
        server_ident: AssetIdentVc,
        client_asset: EcmascriptChunkPlaceableVc,
        ssr_asset: EcmascriptChunkPlaceableVc,
    ) -> EcmascriptClientReferenceAssetVc {
        EcmascriptClientReferenceAsset {
            server_ident,
            client_asset,
            ssr_asset,
        }
        .cell()
    }
}

#[turbo_tasks::function]
fn ecmascript_client_reference_modifier() -> StringVc {
    StringVc::cell("ecmascript client reference".to_string())
}

#[turbo_tasks::value_impl]
impl Asset for EcmascriptClientReferenceAsset {
    #[turbo_tasks::function]
    fn ident(&self) -> AssetIdentVc {
        self.server_ident
            .with_modifier(ecmascript_client_reference_modifier())
    }

    #[turbo_tasks::function]
    fn content(&self) -> Result<AssetContentVc> {
        // The ES client reference asset only serves as a marker asset.
        bail!("EcmascriptClientReferenceAsset has no content")
    }

    #[turbo_tasks::function]
    fn references(_self_vc: EcmascriptClientReferenceAssetVc) -> AssetReferencesVc {
        AssetReferencesVc::empty()
    }
}
