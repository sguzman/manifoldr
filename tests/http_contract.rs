//! All transport-contract tests run against an ephemeral local TCP server.
//! No real credentials, orders or public network requests are used.
use manifoldr::api::client::{ManifoldClient, is_not_found};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

type Requests = Arc<Mutex<Vec<String>>>;

async fn receive_request(stream: &mut TcpStream) -> String {
    let mut data = Vec::new();
    let mut buffer = [0u8; 2048];
    loop {
        let n = stream.read(&mut buffer).await.expect("read HTTP request");
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buffer[..n]);
        if let Some(offset) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            let header = String::from_utf8_lossy(&data[..offset]);
            let length = header
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .and_then(|value| value.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            if data.len() >= offset + 4 + length {
                break;
            }
        }
        assert!(data.len() < 100_000, "request too large for test");
    }
    String::from_utf8(data).expect("UTF-8 fixture request")
}

async fn mock_server(responses: Vec<(&'static str, &'static str)>) -> (String, Requests) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind local server");
    let address = listener.local_addr().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let shared = Arc::clone(&requests);
    tokio::spawn(async move {
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let request = receive_request(&mut stream).await;
            shared.lock().unwrap().push(request);
            let reply = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(reply.as_bytes()).await.expect("write response");
            stream.shutdown().await.expect("close response");
        }
    });
    (format!("http://{address}/v0"), requests)
}

fn request_lines(log: &Requests) -> Vec<String> {
    log.lock()
        .unwrap()
        .iter()
        .map(|request| request.lines().next().unwrap_or_default().to_owned())
        .collect()
}

#[tokio::test]
async fn user_cursor_sends_expected_query() {
    let response = r#"[{"id":"u1","createdTime":10,"name":"Tester","username":"test",
        "url":"https://manifold.markets/test","balance":1,"totalDeposits":2}]"#;
    let (url, requests) = mock_server(vec![("200 OK", response)]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let users = client.list_users(1, Some("previous-id")).await.unwrap();
    assert_eq!(users[0].id, "u1");
    let lines = request_lines(&requests);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("GET /v0/users?"));
    assert!(lines[0].contains("limit=1"));
    assert!(lines[0].contains("before=previous-id"));
}

#[tokio::test]
async fn search_cursor_requires_newest_and_sends_before_time() {
    let (url, requests) = mock_server(vec![("200 OK", "[]")]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    assert!(
        client.search_markets_with_cursor("rust", Some(10), None, None, Some(123)).await.is_err()
    );
    assert!(request_lines(&requests).is_empty());
    let results = client
        .search_markets_with_cursor("rust", Some(10), Some("newest"), Some("open"), Some(123))
        .await
        .unwrap();
    assert!(results.is_empty());
    let line = &request_lines(&requests)[0];
    assert!(line.contains("term=rust"));
    assert!(line.contains("sort=newest"));
    assert!(line.contains("beforeTime=123"));
}

#[tokio::test]
async fn bet_cursor_sends_expected_query() {
    let (url, requests) = mock_server(vec![("200 OK", "[]")]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let results = client.list_bets_with_cursor(Some("user1"), None, Some(20), Some("bet1"), None)
        .await.unwrap();
    assert!(results.is_empty());
    let line = &request_lines(&requests)[0];
    assert!(line.contains("userId=user1"));
    assert!(line.contains("before=bet1"));
    assert!(line.contains("limit=20"));
}

#[tokio::test]
async fn single_market_probability_uses_documented_path() {
    let (url, requests) = mock_server(vec![("200 OK", r#"{"prob":0.61}"#)]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let response: Value = client.market_probability("market1").await.unwrap();
    assert_eq!(response["prob"], 0.61);
    assert_eq!(request_lines(&requests), ["GET /v0/market/market1/prob HTTP/1.1"]);
}

#[tokio::test]
async fn transient_get_uses_bounded_retry() {
    let (url, requests) = mock_server(vec![
        ("503 Service Unavailable", r#"{"error":"temporary"}"#),
        ("200 OK", "[]"),
    ])
    .await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let users = client.list_users(5, None).await.unwrap();
    assert!(users.is_empty());
    assert_eq!(request_lines(&requests).len(), 2);
}

#[tokio::test]
async fn dry_run_posts_once_even_on_server_error() {
    let (url, requests) =
        mock_server(vec![("502 Bad Gateway", r#"{"error":"gateway"}"#)]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let error = client.place_bet("market1", 5.0, "YES", false).await.unwrap_err();
    assert!(!is_not_found(&error));
    let all = requests.lock().unwrap();
    assert_eq!(all.len(), 1, "POST must never be retried");
    assert!(all[0].starts_with("POST /v0/bet HTTP/1.1"));
    let body = all[0].split_once("\r\n\r\n").unwrap().1;
    let parsed: Value = serde_json::from_str(body).unwrap();
    assert_eq!(parsed["dryRun"], true);
    assert_eq!(parsed["amount"], 5.0);
    assert_eq!(parsed["contractId"], "market1");
}

#[tokio::test]
async fn missing_user_reports_status_without_leaking_transport_details() {
    let (url, _) = mock_server(vec![("404 Not Found", "{}")]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let err = client.get_user_by_username("ghost").await.unwrap_err();
    assert!(is_not_found(&err));
}

#[test]
fn bad_api_key_returns_an_error_not_a_panic() {
    assert!(ManifoldClient::new(Some("invalid\nkey".to_owned())).is_err());
}
