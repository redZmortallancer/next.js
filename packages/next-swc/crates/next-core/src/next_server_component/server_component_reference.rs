use anyhow::Result;
use turbo_tasks::{primitives::StringVc, ValueToString, ValueToStringVc};
use turbopack_binding::turbopack::core::{
    asset::{Asset, AssetVc},
    chunk::{
        ChunkableAssetReference, ChunkableAssetReferenceVc, ChunkingType, ChunkingTypeOptionVc,
    },
    reference::{AssetReference, AssetReferenceVc},
    resolve::{ResolveResult, ResolveResultVc},
};

#[turbo_tasks::value]
pub struct NextServerComponentAssetReference {
    asset: AssetVc,
}

#[turbo_tasks::value_impl]
impl NextServerComponentAssetReferenceVc {
    #[turbo_tasks::function]
    pub fn new(asset: AssetVc) -> Self {
        NextServerComponentAssetReference { asset }.cell()
    }
}

#[turbo_tasks::value_impl]
impl ValueToString for NextServerComponentAssetReference {
    #[turbo_tasks::function]
    async fn to_string(&self) -> Result<StringVc> {
        Ok(StringVc::cell(format!(
            "Next.js server component {}",
            self.asset.ident().to_string().await?
        )))
    }
}

#[turbo_tasks::value_impl]
impl AssetReference for NextServerComponentAssetReference {
    #[turbo_tasks::function]
    fn resolve_reference(&self) -> ResolveResultVc {
        ResolveResult::asset(self.asset).cell()
    }
}

#[turbo_tasks::value_impl]
impl ChunkableAssetReference for NextServerComponentAssetReference {
    #[turbo_tasks::function]
    fn chunking_type(&self) -> ChunkingTypeOptionVc {
        // TODO(alexkirsz) Instead of isolated parallel, have the server component
        // reference create a new chunk group entirely?
        ChunkingTypeOptionVc::cell(Some(ChunkingType::IsolatedParallel))
    }
}
