use crate::{anyhow_result, Backend};
use std::sync::Arc;

pub(crate) fn serve(
    backend: Arc<Backend>,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    tauri::async_runtime::spawn_blocking(move || {
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
        let result = (|| -> anyhow_result::Result<tauri::http::Response<Vec<u8>>> {
            if request.method() != "GET" && request.method() != "HEAD" {
                return Ok(tauri::http::Response::builder()
                    .status(405)
                    .body(Vec::new())?);
            }
            let path = {
                let store = backend.store.lock().map_err(|_| "数据库锁不可用")?;
                beaver_core::assets::resolve_asset(&store, request.uri().path())?
            };
            let asset = beaver_core::assets::read_asset(
                &path,
                request.headers().get("range").and_then(|s| s.to_str().ok()),
                request.method() == "HEAD",
            )?;
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
            Ok(response
                .header("Access-Control-Allow-Origin", origin)
                .header(
                    "Access-Control-Expose-Headers",
                    "Content-Range, Accept-Ranges",
                )
                .body(asset.bytes)?)
        })();
        responder.respond(result.unwrap_or_else(|_| {
            tauri::http::Response::builder()
                .status(404)
                .header("Access-Control-Allow-Origin", origin)
                .body(Vec::new())
                .expect("static response")
        }));
    });
}
