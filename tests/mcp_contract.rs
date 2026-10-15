//! MCP HTTP integration tests, all backed by a local mock Manifold API.
//! They never make authenticated requests or touch live trades.
use manifoldr::{api::client::ManifoldClient, mcp::router};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

async fn fake_manifold() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v0", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut conn, _) = listener.accept().await.unwrap();
        let mut buf = vec![0_u8; 2048];
        let read = conn.read(&mut buf).await.unwrap();
        let request = String::from_utf8_lossy(&buf[..read]);
        assert!(request.starts_with("GET /v0/search-markets?"), "{request}");
        assert!(request.contains("sort=most-popular"), "{request}");
        let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]";
        conn.write_all(response.as_bytes()).await.unwrap();
    });
    (url, task)
}

#[tokio::test]
async fn streamable_http_lists_read_only_tools_and_calls_market_browse() {
    let (api_url, fake) = fake_manifold().await;
    let client = ManifoldClient::with_base_url(None, &api_url).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, router(client, vec![])).await.unwrap();
    });
    let http = reqwest::Client::new();
    assert_eq!(
        http.get(format!("{base}/healthz"))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::OK
    );
    let protocol = "2025-11-25";
    let initialize = json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "protocolVersion": protocol,
            "clientInfo": {"name":"manifoldr-contract-test","version":"1.0"},
            "capabilities": {}
        }
    });
    let response = http
        .post(format!("{base}/mcp"))
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", protocol)
        .json(&initialize)
        .send()
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "initialize returned {}",
        response.status()
    );
    let init_json: Value = response.json().await.unwrap();
    assert!(init_json["result"]["capabilities"]["tools"].is_object());
    let list = http
        .post(format!("{base}/mcp"))
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", protocol)
        .json(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
        .send()
        .await
        .unwrap();
    assert!(
        list.status().is_success(),
        "tools/list returned {}",
        list.status()
    );
    let tools: Value = list.json().await.unwrap();
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    for expected in [
        "popular_markets",
        "trending_markets",
        "suggested_markets",
        "search_markets",
        "list_categories",
        "get_market",
    ] {
        assert!(names.contains(&expected), "missing {expected}: {names:?}");
    }
    assert!(!names.contains(&"place_bet"));
    assert!(!names.contains(&"cancel_limit_order"));
    let response = http
        .post(format!("{base}/mcp"))
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", protocol)
        .json(
            &json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
                "name":"popular_markets","arguments":{"limit":1}
            }}),
        )
        .send()
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "tools/call returned {}",
        response.status()
    );
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["result"]["isError"], Value::Bool(false));
    assert_eq!(body["result"]["structuredContent"]["data"], json!([]));
    let denied = http
        .post(format!("{base}/mcp"))
        .header("origin", "http://malicious.example")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", protocol)
        .json(&initialize)
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), reqwest::StatusCode::FORBIDDEN);
    fake.await.unwrap();
    server.abort();
}
