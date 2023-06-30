use anyhow::{bail, Result};
use indoc::formatdoc;
use turbo_tasks::{primitives::StringVc, Value};
use turbo_tasks_fs::FileSystemPathVc;
use turbopack_binding::turbopack::{
    core::{
        asset::{Asset, AssetContentVc, AssetVc},
        chunk::{
            availability_info::AvailabilityInfo, ChunkItem, ChunkItemVc, ChunkVc, ChunkableAsset,
            ChunkableAssetVc, ChunkingContextVc,
        },
        ident::AssetIdentVc,
        reference::{AssetReferenceVc, AssetReferencesVc},
    },
    turbopack::ecmascript::{
        chunk::{
            EcmascriptChunkItem, EcmascriptChunkItemContent, EcmascriptChunkItemContentVc,
            EcmascriptChunkItemVc, EcmascriptChunkPlaceable, EcmascriptChunkPlaceableVc,
            EcmascriptChunkVc, EcmascriptChunkingContextVc, EcmascriptExports, EcmascriptExportsVc,
        },
        utils::StringifyJs,
    },
};

use super::server_component_reference::NextServerComponentAssetReferenceVc;

#[turbo_tasks::function]
fn modifier() -> StringVc {
    StringVc::cell("Next.js server component".to_string())
}

#[turbo_tasks::value(shared)]
pub struct NextServerComponentAsset {
    asset: EcmascriptChunkPlaceableVc,
}

#[turbo_tasks::value_impl]
impl NextServerComponentAssetVc {
    #[turbo_tasks::function]
    pub fn new(asset: EcmascriptChunkPlaceableVc) -> Self {
        NextServerComponentAsset { asset }.cell()
    }

    #[turbo_tasks::function]
    pub async fn server_path(self_vc: NextServerComponentAssetVc) -> Result<FileSystemPathVc> {
        let this = self_vc.await?;
        Ok(this.asset.ident().path())
    }
}

#[turbo_tasks::value_impl]
impl Asset for NextServerComponentAsset {
    #[turbo_tasks::function]
    fn ident(&self) -> AssetIdentVc {
        self.asset.ident().with_modifier(modifier())
    }

    #[turbo_tasks::function]
    fn content(&self) -> Result<AssetContentVc> {
        bail!("Next.js server component asset has no content")
    }

    #[turbo_tasks::function]
    fn references(&self) -> AssetReferencesVc {
        let references: Vec<AssetReferenceVc> =
            vec![NextServerComponentAssetReferenceVc::new(self.asset.into()).into()];
        AssetReferencesVc::cell(references)
    }
}

#[turbo_tasks::value_impl]
impl ChunkableAsset for NextServerComponentAsset {
    #[turbo_tasks::function]
    fn as_chunk(
        self_vc: NextServerComponentAssetVc,
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
impl EcmascriptChunkPlaceable for NextServerComponentAsset {
    #[turbo_tasks::function]
    async fn as_chunk_item(
        self_vc: NextServerComponentAssetVc,
        context: EcmascriptChunkingContextVc,
    ) -> Result<EcmascriptChunkItemVc> {
        Ok(BuildServerComponentChunkItem {
            context,
            inner: self_vc,
        }
        .cell()
        .into())
    }

    #[turbo_tasks::function]
    fn get_exports(&self) -> EcmascriptExportsVc {
        // TODO This should be EsmExports
        EcmascriptExports::Value.cell()
    }
}

#[turbo_tasks::value]
struct BuildServerComponentChunkItem {
    context: EcmascriptChunkingContextVc,
    inner: NextServerComponentAssetVc,
}

#[turbo_tasks::value_impl]
impl EcmascriptChunkItem for BuildServerComponentChunkItem {
    #[turbo_tasks::function]
    fn chunking_context(&self) -> EcmascriptChunkingContextVc {
        self.context
    }

    #[turbo_tasks::function]
    async fn content(
        self_vc: BuildServerComponentChunkItemVc,
    ) -> Result<EcmascriptChunkItemContentVc> {
        let this = self_vc.await?;
        let inner = this.inner.await?;

        let module_id = inner.asset.as_chunk_item(this.context).id().await?;
        Ok(EcmascriptChunkItemContent {
            inner_code: formatdoc!(
                r#"
                    __turbopack_esm__({{
                        default: () => __turbopack_import__({}),
                    }});
                "#,
                StringifyJs(&module_id),
            )
            .into(),
            ..Default::default()
        }
        .cell())
    }
}

#[turbo_tasks::value_impl]
impl ChunkItem for BuildServerComponentChunkItem {
    #[turbo_tasks::function]
    fn asset_ident(&self) -> AssetIdentVc {
        self.inner.ident()
    }

    #[turbo_tasks::function]
    fn references(&self) -> AssetReferencesVc {
        self.inner.references()
    }
}
