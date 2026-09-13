use crate::Backend;
use anyhow::Result;
use std::sync::Arc;

pub(crate) fn serve(
    backend: Arc<Backend>,
    window: String,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    tauri::async_runtime::spawn(async move {
        let origin = request
            .headers()
            .get("origin")
            .and_then(|s| s.to_str().ok())
            .unwrap_or("http://tauri.localhost");
        if ![
            "http://tauri.localhost",
            "https://tauri.localhost",
            "tauri://localhost",
        ]
        .contains(&origin)
        {
            responder.respond(
                tauri::http::Response::builder()
                    .status(403)
                    .body(Vec::new())
                    .expect("static response"),
            );
            return;
        }
        let result = async {
            if request.method() != "GET" && request.method() != "HEAD" {
                return Ok(tauri::http::Response::builder()
                    .status(405)
                    .body(Vec::new())?);
            }
            crate::asset_task_windows::authorize_image(&backend, &window, request.uri().path())
                .map_err(anyhow::Error::msg)?;
            let asset = read(backend, &request).await?;
            let mut response = tauri::http::Response::builder()
                .status(asset.status)
                .header("Content-Type", asset.content_type)
                .header("Content-Length", asset.length)
                .header("Accept-Ranges", "bytes")
                .header("Cache-Control", "no-store")
                .header("X-Content-Type-Options", "nosniff")
                .header("Content-Security-Policy", "default-src 'none'");
            if let Some(range) = asset.content_range {
                response = response.header("Content-Range", range);
            }
            Ok::<_, anyhow::Error>(
                response
                    .header("Access-Control-Allow-Origin", origin)
                    .header(
                        "Access-Control-Expose-Headers",
                        "Content-Range, Accept-Ranges",
                    )
                    .body(asset.bytes)?,
            )
        }
        .await;
        responder.respond(result.unwrap_or_else(|_| {
            tauri::http::Response::builder()
                .status(404)
                .header("Access-Control-Allow-Origin", origin)
                .body(Vec::new())
                .expect("static response")
        }));
    });
}

async fn read(
    backend: Arc<Backend>,
    request: &tauri::http::Request<Vec<u8>>,
) -> Result<beaver_core::assets::AssetResponse> {
    let uri = request.uri().path().to_owned();
    let head = request.method() == "HEAD";
    if uri.starts_with("/asset-task/") {
        let bytes = crate::asset_task_preview::image(backend, &uri).await?;
        return Ok(beaver_core::assets::AssetResponse {
            status: 200,
            content_type: "image/png".into(),
            length: bytes.len() as u64,
            content_range: None,
            bytes: if head { Vec::new() } else { bytes },
        });
    }
    let range = request
        .headers()
        .get("range")
        .and_then(|s| s.to_str().ok())
        .map(str::to_owned);
    tauri::async_runtime::spawn_blocking(move || {
        let path = if uri.starts_with("/validation/") {
            crate::validation_runtime::media(&backend, &uri)?
        } else {
            let store = backend
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
            beaver_core::assets::resolve_asset(&store, &uri)?
        };
        beaver_core::assets::read_asset(&path, range.as_deref(), head)
    })
    .await?
}
