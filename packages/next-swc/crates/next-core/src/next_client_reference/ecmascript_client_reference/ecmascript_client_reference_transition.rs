use anyhow::{bail, Result};
use turbo_tasks::Value;
use turbopack_binding::turbopack::{
    core::{
        asset::{Asset, AssetVc},
        reference_type::{EntryReferenceSubType, ReferenceType},
    },
    ecmascript::chunk::EcmascriptChunkPlaceableVc,
    turbopack::{
        transition::{ContextTransitionVc, Transition, TransitionVc},
        ModuleAssetContextVc,
    },
};

use super::ecmascript_client_reference_proxy_module_asset::EcmascriptClientReferenceProxyModuleAssetVc;

#[turbo_tasks::value(shared)]
pub struct NextEcmascriptClientReferenceTransition {
    client_transition: ContextTransitionVc,
    ssr_transition: ContextTransitionVc,
}

#[turbo_tasks::value_impl]
impl NextEcmascriptClientReferenceTransitionVc {
    #[turbo_tasks::function]
    pub fn new(
        client_transition: ContextTransitionVc,
        ssr_transition: ContextTransitionVc,
    ) -> Self {
        NextEcmascriptClientReferenceTransition {
            client_transition,
            ssr_transition,
        }
        .cell()
    }
}

#[turbo_tasks::value_impl]
impl Transition for NextEcmascriptClientReferenceTransition {
    #[turbo_tasks::function]
    async fn process(
        &self,
        asset: AssetVc,
        context: ModuleAssetContextVc,
        _reference_type: Value<ReferenceType>,
    ) -> Result<AssetVc> {
        let client_asset = self.client_transition.process(
            asset,
            context,
            Value::new(ReferenceType::Entry(
                EntryReferenceSubType::AppClientComponent,
            )),
        );

        let ssr_asset = self.ssr_transition.process(
            asset,
            context,
            Value::new(ReferenceType::Entry(
                EntryReferenceSubType::AppClientComponent,
            )),
        );

        let Some(client_module) = EcmascriptChunkPlaceableVc::resolve_from(&client_asset).await? else {
            bail!("client asset is not ecmascript chunk placeable");
        };

        let Some(ssr_module) = EcmascriptChunkPlaceableVc::resolve_from(&ssr_asset).await? else {
            bail!("SSR asset is not ecmascript chunk placeable");
        };

        // TODO(alexkirsz) This is necessary to remove the transition currently set on
        // the context.
        let context = context.await?;
        let server_context = ModuleAssetContextVc::new(
            context.transitions,
            context.compile_time_info,
            context.module_options_context,
            context.resolve_options_context,
        );

        Ok(EcmascriptClientReferenceProxyModuleAssetVc::new(
            asset.ident(),
            server_context.into(),
            client_module,
            ssr_module,
        )
        .into())
    }
}
