use crate::{business_catalog, Backend};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

pub const MAX_CALLS: u32 = 16;

#[derive(Clone)]
struct Api {
    app: tauri::AppHandle,
    backend: Arc<Backend>,
    token: String,
}

pub fn token() -> anyhow::Result<Option<String>> {
    match std::env::var("BEAVER_API_TOKEN") {
        Ok(token)
            if token.len() >= 32
                && token.len() <= 256
                && token
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b)) =>
        {
            Ok(Some(token))
        }
        Err(std::env::VarError::NotPresent) => Ok(None),
        _ => {
            anyhow::bail!("BEAVER_API_TOKEN must contain 32..256 ASCII letters, digits, '-' or '_'")
        }
    }
}

pub fn port() -> anyhow::Result<u16> {
    let port = std::env::var("BEAVER_API_PORT")
        .unwrap_or_else(|_| "4319".into())
        .parse::<u16>()?;
    anyhow::ensure!(port != 0, "BEAVER_API_PORT must be nonzero");
    Ok(port)
}

pub fn start(app: tauri::AppHandle, backend: Arc<Backend>) -> anyhow::Result<()> {
    let Some(token) = token()? else {
        return Ok(());
    };
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port()?))?;
    listener.set_nonblocking(true)?;
    let state = Api {
        app: app.clone(),
        backend: backend.clone(),
        token,
    };
    let router = Router::new()
        .route("/v1/capabilities", get(capabilities))
        .route("/v1/call", post(call))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), authorize))
        .with_state(state);
    tauri::async_runtime::spawn(async move {
        let result = async {
            let listener = tokio::net::TcpListener::from_std(listener)?;
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    while !backend.closing.load(Ordering::SeqCst) {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                })
                .await
        }
        .await;
        if result.is_err() {
            use tauri::Emitter;
            let _ = app.emit("beaver:api-error", "Business API listener stopped");
        }
    });
    Ok(())
}

fn error(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(json!({"apiVersion":"1","ok":false,"error":{"code":code,"message":message}})),
    )
        .into_response()
}

fn authorized(headers: &HeaderMap, token: &str) -> bool {
    if headers.contains_key("origin") {
        return false;
    }
    let expected = format!("Bearer {token}");
    let supplied = headers
        .get("authorization")
        .map(|h| h.as_bytes())
        .unwrap_or_default();
    supplied.len() == expected.len()
        && supplied
            .iter()
            .zip(expected.bytes())
            .fold(0u8, |diff, (a, b)| diff | (a ^ b))
            == 0
}

async fn authorize(
    State(api): State<Api>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    if !authorized(request.headers(), &api.token) {
        return error(
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED",
            "A valid local API bearer token is required; browser origins are not accepted",
        );
    }
    if api.backend.closing.load(Ordering::SeqCst) {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "SHUTTING_DOWN",
            "Beaver is shutting down",
        );
    }
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    response
}

async fn capabilities() -> Json<Value> {
    Json(json!({"apiVersion":"1","tools":business_catalog::tools(),
        "catalogs":{
            "design":serde_json::from_str::<Value>(include_str!("../../../dist-native/design-catalog.json")).unwrap(),
            "blueprint":serde_json::from_str::<Value>(include_str!("../../../dist-native/blueprint-catalog.json")).unwrap()
        },
        "execution":{"taskProgress":"state and task.events","automaticRetries":false,"maxConcurrentCalls":MAX_CALLS},
        "scope":"full local owner access; not a sandbox"}))
}

async fn call(State(api): State<Api>, headers: axum::http::HeaderMap, body: Bytes) -> Response {
    let source = if headers.get("x-beaver-client").and_then(|v| v.to_str().ok()) == Some("mcp") {
        "mcp"
    } else {
        "api"
    };
    let request: Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                "INVALID_JSON",
                "Expected a JSON object",
            )
        }
    };
    if !request
        .as_object()
        .is_some_and(|v| v.keys().all(|k| k == "method" || k == "input"))
    {
        return error(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            "Expected method and optional input",
        );
    }
    let method = request["method"].as_str().unwrap_or("");
    let input = request.get("input").cloned().unwrap_or_else(|| json!({}));
    if let Err((code, message)) = business_catalog::validate(method, &input) {
        return error(StatusCode::BAD_REQUEST, code, &message);
    }
    let permit = match api.backend.api_calls.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return error(
                StatusCode::TOO_MANY_REQUESTS,
                "BUSY",
                "Too many active API calls",
            )
        }
    };
    let method = method.to_owned();
    // A disconnected HTTP client must not drop a business transition midway.
    let result = tauri::async_runtime::spawn(async move {
        let _permit = permit;
        crate::business_call(api.app, api.backend, method, Some(input), source).await
    })
    .await;
    match result {
        Ok(Ok(value)) => Json(json!({"apiVersion":"1","ok":true,"result":value})).into_response(),
        Ok(Err(message)) => error(StatusCode::CONFLICT, "BUSINESS_ERROR", &message),
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "Business operation failed unexpectedly",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_origins_and_missing_or_wrong_tokens_are_rejected() {
        let mut headers = HeaderMap::new();
        assert!(!authorized(&headers, "test"));
        headers.insert("authorization", "Bearer wrong".parse().unwrap());
        assert!(!authorized(&headers, "test"));
        headers.insert("authorization", "Bearer test".parse().unwrap());
        assert!(authorized(&headers, "test"));
        headers.insert("origin", "http://localhost".parse().unwrap());
        assert!(!authorized(&headers, "test"));
    }
}
