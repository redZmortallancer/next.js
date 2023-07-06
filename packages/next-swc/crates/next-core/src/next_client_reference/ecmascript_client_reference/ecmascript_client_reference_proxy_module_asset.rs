use std::{io::Write, iter::once};

use anyhow::{bail, Result};
use indoc::writedoc;
use turbo_tasks::{primitives::StringVc, Value, ValueToString};
use turbo_tasks_fs::File;
use turbopack_binding::turbopack::{
    core::{
        asset::{Asset, AssetContentVc, AssetVc},
        chunk::{
            availability_info::AvailabilityInfo, ChunkItem, ChunkItemVc, ChunkVc, ChunkableAsset,
            ChunkableAssetVc, ChunkingContextVc,
        },
        code_builder::CodeBuilder,
        context::{AssetContext, AssetContextVc},
        ident::AssetIdentVc,
        reference::{AssetReferencesVc, SingleAssetReferenceVc},
        reference_type::ReferenceType,
        virtual_asset::VirtualAssetVc,
    },
    ecmascript::{
        chunk::{
            EcmascriptChunkItem, EcmascriptChunkItemContentVc, EcmascriptChunkItemVc,
            EcmascriptChunkPlaceable, EcmascriptChunkPlaceableVc, EcmascriptChunkVc,
            EcmascriptChunkingContextVc, EcmascriptExports, EcmascriptExportsVc,
        },
        utils::StringifyJs,
        EcmascriptModuleAssetVc,
    },
};

use super::ecmascript_client_reference_asset::EcmascriptClientReferenceAssetVc;

/// A [`EcmascriptClientReferenceProxyModuleAsset`] is used in RSC to represent
/// a client or SSR asset.
#[turbo_tasks::value(transparent)]
pub struct EcmascriptClientReferenceProxyModuleAsset {
    server_module_ident: AssetIdentVc,
    server_asset_context: AssetContextVc,
    client_module: EcmascriptChunkPlaceableVc,
    ssr_module: EcmascriptChunkPlaceableVc,
}

#[turbo_tasks::value_impl]
impl EcmascriptClientReferenceProxyModuleAssetVc {
    /// Create a new [`EcmascriptClientReferenceProxyModuleAsset`].
    ///
    /// # Arguments
    ///
    /// * `server_module_ident` - The identifier of the server module.
    /// * `server_asset_context` - The context of the server module.
    /// * `client_module` - The client module.
    /// * `ssr_module` - The SSR module.
    #[turbo_tasks::function]
    pub fn new(
        server_module_ident: AssetIdentVc,
        server_asset_context: AssetContextVc,
        client_module: EcmascriptChunkPlaceableVc,
        ssr_module: EcmascriptChunkPlaceableVc,
    ) -> EcmascriptClientReferenceProxyModuleAssetVc {
        EcmascriptClientReferenceProxyModuleAsset {
            server_module_ident,
            server_asset_context,
            client_module,
            ssr_module,
        }
        .cell()
    }

    #[turbo_tasks::function]
    async fn proxy_module_asset(self) -> Result<EcmascriptModuleAssetVc> {
        let this = self.await?;
        let mut code = CodeBuilder::default();

        // Adapted from
        // next.js/packages/next/src/build/webpack/loaders/next-flight-loader/index.ts
        writedoc!(
            code,
            r#"
                import {{ createProxy }} from 'next/dist/build/webpack/loaders/next-flight-loader/module-proxy'

                const proxy = createProxy({server_module_path})

                // Accessing the __esModule property and exporting $$typeof are required here.
                // The __esModule getter forces the proxy target to create the default export
                // and the $$typeof value is for rendering logic to determine if the module
                // is a client boundary.
                const {{ __esModule, $$typeof }} = proxy;
                const __default__ = proxy.default;
            "#,
            server_module_path = StringifyJs(&this.server_module_ident.path().to_string().await?)
        )?;

        // See next-flight-loader
        match &*this.client_module.get_exports().await? {
            EcmascriptExports::EsmExports(esm_exports) => {
                let mut cnt = 0;

                let esm_exports = &*esm_exports.await?;
                for export_name in esm_exports.exports.keys() {
                    match export_name.as_str() {
                        "default" => {
                            writedoc!(
                                code,
                                r#"
                                    export {{ __esModule, $$typeof }};
                                    export default __default__;
                                "#
                            )?;
                        }
                        named => {
                            writedoc!(
                                code,
                                r#"
                                    const e{cnt} = proxy["{named}"];
                                    export {{ e{cnt} as {named} }};
                                "#,
                                cnt = cnt,
                                named = named
                            )?;
                            cnt += 1;
                        }
                    }
                }

                if !esm_exports.star_exports.is_empty() {
                    // TODO(alexkirsz) This should be an issue.
                    bail!(
                        r#"It's currently unsupported to use "export *" in a client boundary. Please use named exports instead."#
                    )
                }
            }
            EcmascriptExports::CommonJs => {
                // TODO(alexkirsz) We should also support CommonJS exports, but right now they
                // aren't statically analyzeable in Turbopack.
                writedoc!(
                    code,
                    r#"
                        export {{ __esModule, $$typeof }};
                        export default __default__;
                    "#
                )?;
            }
            _ => {
                // Invariant.
                bail!("unsupported exports type");
            }
        }

        let code = code.build();
        let proxy_module_asset_content =
            AssetContentVc::from(File::from(code.source_code().clone()));

        let proxy_asset = VirtualAssetVc::new(
            this.server_module_ident.path().join("proxy.ts"),
            proxy_module_asset_content,
        );

        let proxy_asset = this
            .server_asset_context
            .process(proxy_asset.into(), Value::new(ReferenceType::Undefined));

        let Some(proxy_module_asset) = EcmascriptModuleAssetVc::resolve_from(&proxy_asset).await? else {
            bail!("proxy asset is not an ecmascript module");
        };

        Ok(proxy_module_asset)
    }
}

#[turbo_tasks::value_impl]
impl Asset for EcmascriptClientReferenceProxyModuleAsset {
    #[turbo_tasks::function]
    fn ident(&self) -> AssetIdentVc {
        self.server_module_ident
            .with_modifier(client_proxy_modifier())
    }

    #[turbo_tasks::function]
    fn content(&self) -> Result<AssetContentVc> {
        bail!("proxy module asset has no content")
    }

    #[turbo_tasks::function]
    async fn references(
        self_vc: EcmascriptClientReferenceProxyModuleAssetVc,
    ) -> Result<AssetReferencesVc> {
        let EcmascriptClientReferenceProxyModuleAsset {
            server_module_ident,
            server_asset_context: _,
            client_module,
            ssr_module,
        } = &*self_vc.await?;

        let references: Vec<_> = self_vc
            .proxy_module_asset()
            .references()
            .await?
            .iter()
            .copied()
            .chain(once(
                SingleAssetReferenceVc::new(
                    EcmascriptClientReferenceAssetVc::new(
                        *server_module_ident,
                        *client_module,
                        *ssr_module,
                    )
                    .into(),
                    client_reference_description(),
                )
                .into(),
            ))
            .collect();

        Ok(AssetReferencesVc::cell(references))
    }
}

#[turbo_tasks::value_impl]
impl ChunkableAsset for EcmascriptClientReferenceProxyModuleAsset {
    #[turbo_tasks::function]
    fn as_chunk(
        self_vc: EcmascriptClientReferenceProxyModuleAssetVc,
        context: ChunkingContextVc,
        availability_info: Value<AvailabilityInfo>,
    ) -> ChunkVc {
        EcmascriptChunkVc::new(
            context,
            self_vc.as_ecmascript_chunk_placeable(),
            availability_info,
        )
        .into()
    }
}

#[turbo_tasks::value_impl]
impl EcmascriptChunkPlaceable for EcmascriptClientReferenceProxyModuleAsset {
    #[turbo_tasks::function]
    fn as_chunk_item(
        self_vc: EcmascriptClientReferenceProxyModuleAssetVc,
        chunking_context: EcmascriptChunkingContextVc,
    ) -> EcmascriptChunkItemVc {
        ProxyModuleChunkItem {
            client_proxy_asset: self_vc,
            inner_proxy_module_chunk_item: self_vc
                .proxy_module_asset()
                .as_chunk_item(chunking_context),
            chunking_context,
        }
        .cell()
        .into()
    }

    #[turbo_tasks::function]
    fn get_exports(self_vc: EcmascriptClientReferenceProxyModuleAssetVc) -> EcmascriptExportsVc {
        self_vc.proxy_module_asset().get_exports()
    }
}

/// This wrapper only exists to overwrite the `asset_ident` method of the
/// wrapped [`EcmascriptChunkItemVc`]. Otherwise, the asset ident of the
/// chunk item would not be the same as the asset ident of the
/// [`EcmascriptClientReferenceProxyModuleAssetVc`].
#[turbo_tasks::value]
struct ProxyModuleChunkItem {
    client_proxy_asset: EcmascriptClientReferenceProxyModuleAssetVc,
    inner_proxy_module_chunk_item: EcmascriptChunkItemVc,
    chunking_context: EcmascriptChunkingContextVc,
}

#[turbo_tasks::function]
fn client_proxy_modifier() -> StringVc {
    StringVc::cell("client proxy".to_string())
}

#[turbo_tasks::function]
fn client_reference_description() -> StringVc {
    StringVc::cell("client references".to_string())
}

#[turbo_tasks::value_impl]
impl ChunkItem for ProxyModuleChunkItem {
    #[turbo_tasks::function]
    async fn asset_ident(&self) -> AssetIdentVc {
        self.client_proxy_asset.ident()
    }

    #[turbo_tasks::function]
    fn references(&self) -> AssetReferencesVc {
        self.client_proxy_asset.references()
    }
}

#[turbo_tasks::value_impl]
impl EcmascriptChunkItem for ProxyModuleChunkItem {
    #[turbo_tasks::function]
    fn content(&self) -> EcmascriptChunkItemContentVc {
        self.inner_proxy_module_chunk_item.content()
    }

    #[turbo_tasks::function]
    fn content_with_availability_info(
        &self,
        availability_info: Value<AvailabilityInfo>,
    ) -> EcmascriptChunkItemContentVc {
        self.inner_proxy_module_chunk_item
            .content_with_availability_info(availability_info)
    }

    #[turbo_tasks::function]
    fn chunking_context(&self) -> EcmascriptChunkingContextVc {
        self.inner_proxy_module_chunk_item.chunking_context()
    }
}
