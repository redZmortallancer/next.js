use anyhow::Result;
use turbo_tasks::{Value, Vc};
use turbopack_binding::{
    turbo::tasks_fs::FileSystemPath,
    turbopack::{
        core::{
            compile_time_defines,
            compile_time_info::{
                CompileTimeDefines, CompileTimeInfo, FreeVarReference, FreeVarReferences,
            },
            environment::{
                EdgeWorkerEnvironment, Environment, EnvironmentIntention, ExecutionEnvironment,
                ServerAddr,
            },
            free_var_references,
        },
        node::execution_context::ExecutionContext,
        turbopack::resolve_options_context::ResolveOptionsContext,
    },
};

use crate::{
    next_config::NextConfig, next_import_map::get_next_edge_import_map,
    next_server::context::ServerContextType, next_shared::resolve::UnsupportedModulesResolvePlugin,
    util::foreign_code_context_condition,
};

fn defines() -> CompileTimeDefines {
    compile_time_defines!(
        process.turbopack = true,
        process.env.NODE_ENV = "development",
        process.env.__NEXT_CLIENT_ROUTER_FILTER_ENABLED = false,
        process.env.NEXT_RUNTIME = "edge"
    )
    // TODO(WEB-937) there are more defines needed, see
    // packages/next/src/build/webpack-config.ts
}

#[turbo_tasks::function]
fn next_edge_defines() -> Vc<CompileTimeDefines> {
    defines().cell()
}

#[turbo_tasks::function]
fn next_edge_free_vars(project_path: Vc<FileSystemPath>) -> Vc<FreeVarReferences> {
    free_var_references!(
        ..defines().into_iter(),
        Buffer = FreeVarReference::EcmaScriptModule {
            request: "next/dist/compiled/buffer".to_string(),
            context: Some(project_path),
            export: Some("Buffer".to_string()),
        },
        process = FreeVarReference::EcmaScriptModule {
            request: "next/dist/build/polyfills/process".to_string(),
            context: Some(project_path),
            export: Some("default".to_string()),
        },
    )
    .cell()
}

#[turbo_tasks::function]
pub fn get_edge_compile_time_info(
    project_path: Vc<FileSystemPath>,
    server_addr: Vc<ServerAddr>,
    intention: Value<EnvironmentIntention>,
) -> Vc<CompileTimeInfo> {
    CompileTimeInfo::builder(Environment::new(
        Value::new(ExecutionEnvironment::EdgeWorker(
            EdgeWorkerEnvironment { server_addr }.into(),
        )),
        intention,
    ))
    .defines(next_edge_defines())
    .free_var_references(next_edge_free_vars(project_path))
    .cell()
}

#[turbo_tasks::function]
pub async fn get_edge_resolve_options_context(
    project_path: Vc<FileSystemPath>,
    ty: Value<ServerContextType>,
    next_config: Vc<NextConfig>,
    execution_context: Vc<ExecutionContext>,
) -> Result<Vc<ResolveOptionsContext>> {
    let next_edge_import_map =
        get_next_edge_import_map(project_path, ty, next_config, execution_context);

    let resolve_options_context = ResolveOptionsContext {
        enable_node_modules: Some(project_path.root().resolve().await?),
        // https://github.com/vercel/next.js/blob/bf52c254973d99fed9d71507a2e818af80b8ade7/packages/next/src/build/webpack-config.ts#L96-L102
        custom_conditions: vec![
            "edge-light".to_string(),
            "worker".to_string(),
            "development".to_string(),
        ],
        import_map: Some(next_edge_import_map),
        module: true,
        browser: true,
        plugins: vec![Vc::upcast(UnsupportedModulesResolvePlugin::new(
            project_path,
        ))],
        ..Default::default()
    };

    Ok(ResolveOptionsContext {
        enable_typescript: true,
        enable_react: true,
        rules: vec![(
            foreign_code_context_condition(next_config).await?,
            resolve_options_context.clone().cell(),
        )],
        ..resolve_options_context
    }
    .cell())
}
