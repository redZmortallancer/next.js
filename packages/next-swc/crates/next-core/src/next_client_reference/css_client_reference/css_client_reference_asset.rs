use anyhow::{bail, Result};
use turbo_tasks::primitives::StringVc;
use turbopack_binding::turbopack::{
    core::{
        asset::{Asset, AssetContentVc, AssetVc},
        ident::AssetIdentVc,
        reference::AssetReferencesVc,
    },
    turbopack::css::{chunk::CssChunkPlaceableVc, ParseCss, ParseCssResultVc, ParseCssVc},
};

/// A [`CssClientReferenceAsset`] is a marker asset used to indicate which
/// client reference should appear in the client reference manifest.
#[turbo_tasks::value(transparent)]
pub struct CssClientReferenceAsset {
    pub client_asset: CssChunkPlaceableVc,
}

#[turbo_tasks::value_impl]
impl CssClientReferenceAssetVc {
    /// Create a new [`CssClientReferenceAsset`] from the given source CSS
    /// asset.
    #[turbo_tasks::function]
    pub fn new(client_asset: CssChunkPlaceableVc) -> CssClientReferenceAssetVc {
        CssClientReferenceAsset { client_asset }.cell()
    }
}

#[turbo_tasks::function]
fn css_client_reference_modifier() -> StringVc {
    StringVc::cell("css client reference".to_string())
}

#[turbo_tasks::value_impl]
impl Asset for CssClientReferenceAsset {
    #[turbo_tasks::function]
    fn ident(&self) -> AssetIdentVc {
        self.client_asset
            .ident()
            .with_modifier(css_client_reference_modifier())
    }

    #[turbo_tasks::function]
    fn content(&self) -> Result<AssetContentVc> {
        // The client reference asset only serves as a marker asset.
        bail!("CssClientReferenceAsset has no content")
    }

    #[turbo_tasks::function]
    fn references(_self_vc: CssClientReferenceAssetVc) -> AssetReferencesVc {
        AssetReferencesVc::empty()
    }
}

#[turbo_tasks::value_impl]
impl ParseCss for CssClientReferenceAsset {
    #[turbo_tasks::function]
    async fn parse_css(&self) -> Result<ParseCssResultVc> {
        let Some(parse_css) = ParseCssVc::resolve_from(self.client_asset).await? else {
            bail!("CSS client reference client asset must be CSS parseable");
        };

        Ok(parse_css.parse_css())
    }
}
