use anyhow::{bail, Result};
use turbopack_binding::turbopack::{
    core::asset::AssetVc,
    ecmascript::chunk::EcmascriptChunkPlaceableVc,
    turbopack::{
        transition::{Transition, TransitionVc},
        ModuleAssetContextVc,
    },
};

use super::server_component_asset::NextServerComponentAssetVc;

/// This transition wraps an asset into a marker [`NextServerComponentAssetVc`].
///
/// When walking the asset graph to build the client reference manifest, this is
/// used to determine under which server component CSS client references are
/// required. Ultimately, this tells Next.js what CSS to inject into the page.
#[turbo_tasks::value(shared)]
pub struct NextServerComponentTransition {}

#[turbo_tasks::value_impl]
impl NextServerComponentTransitionVc {
    /// Creates a new [`NextServerComponentTransitionVc`].
    #[turbo_tasks::function]
    pub fn new() -> Self {
        NextServerComponentTransition {}.cell()
    }
}

#[turbo_tasks::value_impl]
impl Transition for NextServerComponentTransition {
    #[turbo_tasks::function]
    async fn process_module(
        _self_vc: NextServerComponentTransitionVc,
        asset: AssetVc,
        _context: ModuleAssetContextVc,
    ) -> Result<AssetVc> {
        let Some(asset) = EcmascriptChunkPlaceableVc::resolve_from(asset).await? else {
            bail!("not an ecmascript module");
        };

        Ok(NextServerComponentAssetVc::new(asset).into())
    }
}
