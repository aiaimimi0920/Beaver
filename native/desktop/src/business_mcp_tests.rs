use super::*;
use axum::{
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use beaver_core::{store::Store, task_callback};
use std::sync::Mutex;
use tokio::io::{AsyncBufReadExt, BufReader};

const TOKEN: &str = "callback-fixture-token-not-a-real-secret";

async fn callback_api(
    State((store, root)): State<(Arc<Mutex<Store>>, std::path::PathBuf)>,
    headers: HeaderMap,
    Json(input): Json<Value>,
) -> Json<Value> {
    assert_eq!(headers["authorization"], format!("Bearer {TOKEN}"));
    assert_eq!(headers["x-beaver-client"], "mcp");
    let method = input["method"].as_str().unwrap();
    let args = &input["input"];
    if let Err((code, message)) = crate::business_catalog::validate(method, args) {
        return Json(json!({"ok":false,"error":{"code":code,"message":message}}));
    }
    let result = if method == "task.framework" {
        beaver_core::framework::call(
            store.clone(),
            Arc::new(beaver_core::files::Files::new(root)),
            args["id"].as_str().unwrap().into(),
            None,
            args["request"].clone(),
        )
        .await
    } else {
        task_callback::business(&mut store.lock().unwrap(), method, args)
    };
    Json(match result {
        Ok(value) => json!({"apiVersion":"1","ok":true,"result":value}),
        Err(error) => {
            json!({"apiVersion":"1","ok":false,"error":{"code":"CALL_FAILED","message":error.to_string()}})
        }
    })
}

#[tokio::test]
async fn callback_mcp_stream_http_core_roundtrip() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path())?;
    store.put(
        "task",
        "task",
        &json!({"id":"task","projectId":"project","status":"running",
        "threadId":"thread","turnId":"turn"}),
    )?;
    let store = Arc::new(Mutex::new(store));
    let router = Router::new()
        .route(
            "/v1/capabilities",
            get(|| async { Json(json!({"tools":crate::business_catalog::tools()})) }),
        )
        .route("/v1/call", post(callback_api))
        .with_state((store.clone(), temp.path().to_path_buf()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let bridge = Arc::new(Bridge {
        client: reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()?,
        base: format!("http://{}", listener.local_addr()?),
        token: TOKEN.into(),
        initialized: AtomicBool::new(false),
    });
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let (client, host) = tokio::io::duplex(2 * 1024 * 1024);
    let (reader, writer) = tokio::io::split(host);
    let protocol = tokio::spawn(serve(bridge, reader, writer));
    let (client_read, mut client_write) = tokio::io::split(client);
    let mut lines = BufReader::new(client_read).lines();
    let request = json!({"operation":"report","requestId":"draft","expectedRevision":0,
        "report":{"kind":"result","summary":"Design draft saved","inputs":{"prompt":"character"},"outputs":{"path":"design.md"}}});
    let mut conflict = request.clone();
    conflict["report"]["summary"] = json!("Changed draft");
    let callback = |request: Value, turn: &str| {
        json!({"name":"task.callback","arguments":{
        "id":"task","threadId":"thread","turnId":turn,"request":request}})
    };
    let messages = [
        ("tools/list", json!({})),
        ("initialize", json!({"protocolVersion":"2025-11-25"})),
        ("tools/list", json!({})),
        ("tools/call", callback(request.clone(), "turn")),
        ("tools/call", callback(request.clone(), "turn")),
        ("tools/call", callback(conflict, "turn")),
        ("tools/call", callback(request, "old-turn")),
        (
            "tools/call",
            json!({"name":"task.callbackState","arguments":{"id":"task","requestId":"draft"}}),
        ),
        (
            "tools/call",
            json!({"name":"task.framework","arguments":{"id":"task","request":{"operation":"configure","configuration":{"revision":0,"plugins":[],"rules":[],"semanticRequired":true}}}}),
        ),
        (
            "tools/call",
            json!({"name":"task.framework","arguments":{"id":"task","request":{"operation":"state"}}}),
        ),
    ];
    let mut responses = Vec::new();
    for (index, (method, params)) in messages.into_iter().enumerate() {
        let mut bytes = serde_json::to_vec(
            &json!({"jsonrpc":"2.0","id":index,"method":method,"params":params}),
        )?;
        bytes.push(b'\n');
        client_write.write_all(&bytes).await?;
        let line = tokio::time::timeout(Duration::from_secs(10), lines.next_line())
            .await??
            .unwrap();
        let response: Value = serde_json::from_str(&line)?;
        assert_eq!(response["id"], index);
        responses.push(response);
    }
    assert_eq!(responses[0]["error"]["code"], -32600);
    assert_eq!(
        responses[1]["result"]["serverInfo"]["name"],
        "beaver-business"
    );
    let tools = responses[2]["result"]["tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t["name"] == "task.callback"));
    assert!(tools.iter().any(|t| t["name"] == "task.callbackState"));
    assert!(tools.iter().any(|t| t["name"] == "task.framework"));
    assert_eq!(responses[3]["result"], responses[4]["result"]);
    assert_eq!(responses[3]["result"]["isError"], false);
    assert_eq!(responses[5]["result"]["isError"], true);
    assert_eq!(responses[6]["result"]["isError"], true);
    let receipt = &responses[7]["result"]["structuredContent"]["result"]["receipt"];
    assert_eq!(
        receipt["response"],
        responses[3]["result"]["structuredContent"]["result"]
    );
    assert_eq!(receipt["request"]["report"]["outputs"]["path"], "design.md");
    assert_eq!(responses[8]["result"]["isError"], false, "{}", responses[8]);
    assert_eq!(
        responses[8]["result"]["structuredContent"]["result"]["revision"],
        1
    );
    assert_eq!(
        responses[9]["result"]["structuredContent"]["result"]["configuration"]["semanticRequired"],
        true
    );
    assert_eq!(store.lock().unwrap().events("task")?.len(), 1);
    drop(client_write);
    protocol.abort();
    let _ = protocol.await;
    server.abort();
    let _ = server.await;
    Ok(())
}
