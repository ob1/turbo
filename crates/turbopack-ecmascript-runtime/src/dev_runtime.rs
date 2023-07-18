use std::io::Write;

use anyhow::Result;
use indoc::writedoc;
use turbo_tasks::Vc;
use turbopack_core::{
    code_builder::{Code, CodeBuilder},
    context::AssetContext,
    environment::{ChunkLoading, Environment},
};
use turbopack_ecmascript::{utils::StringifyJs, StaticEcmascriptCode};

use crate::{asset_context::get_runtime_asset_context, embed_file_path};

/// Returns the code for the development ECMAScript runtime.
#[turbo_tasks::function]
pub async fn get_dev_runtime_code(
    environment: Vc<Environment>,
    chunk_base_path: Vc<Option<String>>,
) -> Result<Vc<Code>> {
    let asset_context = get_runtime_asset_context(environment);

    let shared_runtime_utils_code = StaticEcmascriptCode::new(
        asset_context,
        embed_file_path("shared/runtime-utils.ts".to_string()),
    )
    .code();

    let runtime_base_code = StaticEcmascriptCode::new(
        asset_context,
        embed_file_path("dev/runtime/base/runtime-base.ts".to_string()),
    )
    .code();

    let runtime_backend_code = StaticEcmascriptCode::new(
        asset_context,
        match &*asset_context
            .compile_time_info()
            .environment()
            .chunk_loading()
            .await?
        {
            ChunkLoading::None => {
                embed_file_path("dev/runtime/none/runtime-backend-none.ts".to_string())
            }
            ChunkLoading::NodeJs => {
                embed_file_path("dev/runtime/nodejs/runtime-backend-nodejs.ts".to_string())
            }
            ChunkLoading::Dom => {
                embed_file_path("dev/runtime/dom/runtime-backend-dom.ts".to_string())
            }
        },
    )
    .code();

    let mut code: CodeBuilder = CodeBuilder::default();

    writedoc!(
        code,
        r#"
            (() => {{
            if (!Array.isArray(globalThis.TURBOPACK)) {{
                return;
            }}

            const CHUNK_BASE_PATH = {};
        "#,
        StringifyJs(if let Some(chunk_base_path) = &*chunk_base_path.await? {
            chunk_base_path.as_str()
        } else {
            ""
        })
    )?;

    code.push_code(&*shared_runtime_utils_code.await?);
    code.push_code(&*runtime_base_code.await?);
    code.push_code(&*runtime_backend_code.await?);

    // Registering chunks depends on the BACKEND variable, which is set by the
    // specific runtime code, hence it must be appended after it.
    writedoc!(
        code,
        r#"
            const chunksToRegister = globalThis.TURBOPACK;
            globalThis.TURBOPACK = {{ push: registerChunk }};
            chunksToRegister.forEach(registerChunk);
            }})();
        "#
    )?;

    Ok(Code::cell(code.build()))
}