use std::collections::{HashMap, HashSet};

use anyhow::Result;
use next_core::{
    self,
    next_client_reference::{ClientReference, ClientReferenceType},
};
use turbo_tasks::{TryJoinIterExt, ValueToString};
use turbopack_binding::{
    turbo::tasks_fs::FileSystemPath,
    turbopack::{
        build::BuildChunkingContextVc,
        core::{
            asset::{Asset, AssetVc, AssetsVc},
            chunk::{ChunkableAsset, ChunkingContext, ModuleId as TurbopackModuleId},
        },
        ecmascript::chunk::{
            EcmascriptChunkPlaceable, EcmascriptChunkingContextVc, EcmascriptExports,
        },
    },
};

use crate::manifests::{ClientReferenceManifest, ManifestNode, ManifestNodeEntry, ModuleId};

/// Computes all client references chunks, and adds them to the relevant
/// manifests.
///
/// This returns a map from client reference type to the chunks that reference
/// type needs to load.
pub async fn compute_app_client_references_chunks(
    app_client_references: &HashSet<ClientReference>,
    app_client_reference_tys: &HashSet<ClientReferenceType>,
    client_root: &FileSystemPath,
    node_root: &FileSystemPath,
    client_chunking_context: EcmascriptChunkingContextVc,
    ssr_chunking_context: BuildChunkingContextVc,
    client_reference_manifest: &mut ClientReferenceManifest,
    all_chunks: &mut Vec<AssetVc>,
) -> Result<HashMap<ClientReferenceType, ClientReferenceChunks>> {
    let app_client_references_chunks: HashMap<_, _> = app_client_reference_tys
        .iter()
        .map(|client_reference_ty| async move {
            Ok((
                *client_reference_ty,
                match client_reference_ty {
                    ClientReferenceType::EcmascriptClientReference(ecmascript_client_reference) => {
                        let ecmascript_client_reference_ref = ecmascript_client_reference.await?;
                        let client_entry_chunk = ecmascript_client_reference_ref
                            .client_asset
                            .as_root_chunk(client_chunking_context.into());
                        let ssr_entry_chunk = ecmascript_client_reference_ref
                            .ssr_asset
                            .as_root_chunk(ssr_chunking_context.into());
                        ClientReferenceChunks {
                            client_chunks: client_chunking_context.chunk_group(client_entry_chunk),
                            ssr_chunks: ssr_chunking_context.chunk_group(ssr_entry_chunk),
                        }
                    }
                    ClientReferenceType::CssClientReference(css_client_reference) => {
                        let css_client_reference_ref = css_client_reference.await?;
                        let client_entry_chunk = css_client_reference_ref
                            .client_asset
                            .as_root_chunk(client_chunking_context.into());
                        ClientReferenceChunks {
                            client_chunks: client_chunking_context.chunk_group(client_entry_chunk),
                            ssr_chunks: AssetsVc::empty(),
                        }
                    }
                },
            ))
        })
        .try_join()
        .await?
        .into_iter()
        .collect();

    for (app_client_reference_ty, app_client_reference_chunks) in &app_client_references_chunks {
        match app_client_reference_ty {
            ClientReferenceType::EcmascriptClientReference(ecmascript_client_reference) => {
                let client_chunks = &app_client_reference_chunks.client_chunks.await?;
                let ssr_chunks = &app_client_reference_chunks.ssr_chunks.await?;
                all_chunks.extend(client_chunks.iter().copied());
                all_chunks.extend(ssr_chunks.iter().copied());

                let ecmascript_client_reference = ecmascript_client_reference.await?;

                let client_module_id = ecmascript_client_reference
                    .client_asset
                    .as_chunk_item(client_chunking_context)
                    .id()
                    .await?;
                let ssr_module_id = ecmascript_client_reference
                    .ssr_asset
                    .as_chunk_item(ssr_chunking_context.into())
                    .id()
                    .await?;

                let server_path = ecmascript_client_reference
                    .server_ident
                    .path()
                    .to_string()
                    .await?;

                let client_chunks_paths = client_chunks
                    .iter()
                    .map(|chunk| chunk.ident().path())
                    .try_join()
                    .await?;
                let client_chunks_paths: Vec<String> = client_chunks_paths
                    .iter()
                    .filter_map(|chunk_path| client_root.get_path_to(chunk_path))
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                let ssr_chunks_paths = ssr_chunks
                    .iter()
                    .map(|chunk| chunk.ident().path())
                    .try_join()
                    .await?;
                let ssr_chunks_paths = ssr_chunks_paths
                    .iter()
                    .filter_map(|chunk_path| node_root.get_path_to(chunk_path))
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                let mut ssr_manifest_node = ManifestNode::default();

                match &*ecmascript_client_reference
                    .client_asset
                    .get_exports()
                    .await?
                {
                    EcmascriptExports::EsmExports(exports) => {
                        let exports = exports.await?;
                        for (export_name, _) in &exports.exports {
                            client_reference_manifest
                                .client_modules
                                .module_exports
                                .insert(
                                    get_client_reference_module_key(&*server_path, export_name),
                                    ManifestNodeEntry {
                                        name: export_name.clone(),
                                        id: match client_module_id.clone_value() {
                                            TurbopackModuleId::String(string) => {
                                                ModuleId::String(string)
                                            }
                                            TurbopackModuleId::Number(number) => {
                                                ModuleId::Number(number as _)
                                            }
                                        },
                                        chunks: client_chunks_paths.clone(),
                                        // TODO(WEB-434)
                                        r#async: false,
                                    },
                                );
                            ssr_manifest_node.module_exports.insert(
                                export_name.clone(),
                                ManifestNodeEntry {
                                    name: export_name.clone(),
                                    id: convert_module_id(&ssr_module_id),
                                    chunks: ssr_chunks_paths.clone(),
                                    // TODO(WEB-434)
                                    r#async: false,
                                },
                            );
                        }
                    }
                    EcmascriptExports::CommonJs => {}
                    _ => {}
                }

                client_reference_manifest
                    .client_modules
                    .module_exports
                    .insert(
                        get_client_reference_module_key(&server_path, "*"),
                        ManifestNodeEntry {
                            name: "*".to_string(),
                            id: convert_module_id(&client_module_id),
                            chunks: client_chunks_paths.clone(),
                            // TODO(WEB-434)
                            r#async: false,
                        },
                    );

                ssr_manifest_node.module_exports.insert(
                    "*".to_string(),
                    ManifestNodeEntry {
                        name: "*".to_string(),
                        id: convert_module_id(&ssr_module_id),
                        chunks: ssr_chunks_paths.clone(),
                        // TODO(WEB-434)
                        r#async: false,
                    },
                );

                client_reference_manifest
                    .ssr_module_mapping
                    .insert(convert_module_id(&client_module_id), ssr_manifest_node);
            }
            ClientReferenceType::CssClientReference(_) => {
                let client_chunks = &app_client_reference_chunks.client_chunks.await?;
                all_chunks.extend(client_chunks.iter().copied());
            }
        }
    }

    for app_client_reference in app_client_references {
        if let Some(server_component) = app_client_reference.server_component() {
            let app_client_reference_ty = app_client_reference.ty();
            let client_reference_chunks = app_client_references_chunks
                .get(app_client_reference_ty)
                .expect("client reference chunks not found");
            let client_chunks = &client_reference_chunks.client_chunks.await?;

            let entry_name = server_component
                .server_path()
                .with_extension("")
                .to_string()
                .await?;

            let client_chunks_paths = client_chunks
                .iter()
                .map(|chunk| chunk.ident().path())
                .try_join()
                .await?;

            let entry_css_files = client_reference_manifest
                .entry_css_files
                .entry(entry_name.clone_value())
                .or_insert_with(Default::default);

            match app_client_reference_ty {
                ClientReferenceType::CssClientReference(_) => entry_css_files.extend(
                    client_chunks_paths
                        .iter()
                        .filter_map(|chunk_path| client_root.get_path_to(chunk_path))
                        .map(ToString::to_string),
                ),

                ClientReferenceType::EcmascriptClientReference(_) => entry_css_files.extend(
                    client_chunks_paths
                        .iter()
                        .filter_map(|chunk_path| {
                            if chunk_path.extension() == Some("css") {
                                client_root.get_path_to(chunk_path)
                            } else {
                                None
                            }
                        })
                        .map(ToString::to_string),
                ),
            }
        }
    }

    Ok(app_client_references_chunks)
}

pub struct ClientReferenceChunks {
    pub client_chunks: AssetsVc,
    pub ssr_chunks: AssetsVc,
}

/// See next.js/packages/next/src/lib/client-reference.ts
fn get_client_reference_module_key(server_path: &str, export_name: &str) -> String {
    if export_name == "*" {
        server_path.to_string()
    } else {
        format!("{}#{}", server_path, export_name)
    }
}

fn convert_module_id(module_id: &TurbopackModuleId) -> ModuleId {
    match module_id {
        TurbopackModuleId::String(string) => ModuleId::String(string.clone()),
        TurbopackModuleId::Number(number) => ModuleId::Number(*number as _),
    }
}
