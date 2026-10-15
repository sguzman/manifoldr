//! All transport-contract tests run against an ephemeral local TCP server.
//! No real credentials, orders or public network requests are used.
use manifoldr::api::client::{ApiError, ManifoldClient, is_not_found};
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
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local server");
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
            stream
                .write_all(reply.as_bytes())
                .await
                .expect("write response");
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
        client
            .search_markets_with_cursor("rust", Some(10), None, None, Some(123))
            .await
            .is_err()
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
    let results = client
        .list_bets_with_cursor(Some("user1"), None, Some(20), Some("bet1"), None)
        .await
        .unwrap();
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
    assert_eq!(
        request_lines(&requests),
        ["GET /v0/market/market1/prob HTTP/1.1"]
    );
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
    let (url, requests) = mock_server(vec![("502 Bad Gateway", r#"{"error":"gateway"}"#)]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let error = client
        .place_bet("market1", 5.0, "YES", false)
        .await
        .unwrap_err();
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

#[tokio::test]
async fn rate_limit_error_is_typed_and_never_retried() {
    let (url, requests) =
        mock_server(vec![("429 Too Many Requests", r#"{"error":"slow down"}"#)]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let err = client.list_users(1, None).await.unwrap_err();
    assert!(matches!(
        err,
        ApiError::Http {
            status: reqwest::StatusCode::TOO_MANY_REQUESTS,
            ..
        }
    ));
    assert_eq!(request_lines(&requests).len(), 1);
}

#[tokio::test]
async fn validation_error_is_typed_and_never_sends_request() {
    let (url, requests) = mock_server(vec![]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let err = client.list_users(0, None).await.unwrap_err();
    assert!(matches!(err, ApiError::InvalidInput(_)));
    assert!(request_lines(&requests).is_empty());
}

#[tokio::test]
async fn position_offset_order_and_per_answer_are_sent() {
    let (url, requests) = mock_server(vec![(
        "200 OK",
        r#"{"metricsByContract":{},"contracts":[]}"#,
    )])
    .await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let page = client
        .get_user_contract_metrics_page("user1", 25, 50, Some("profit"), true)
        .await
        .unwrap();
    assert!(page.contracts.is_empty());
    let line = &request_lines(&requests)[0];
    for part in [
        "userId=user1",
        "limit=25",
        "offset=50",
        "order=profit",
        "perAnswer=true",
    ] {
        assert!(line.contains(part), "missing {part} from {line}");
    }
}

#[tokio::test]
async fn market_creator_topic_filters_are_sent() {
    let (url, requests) = mock_server(vec![("200 OK", "[]")]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let page = client
        .list_markets_filtered(
            Some(5),
            None,
            None,
            Some("last"),
            Some("user1"),
            Some("topic1"),
        )
        .await
        .unwrap();
    assert!(page.is_empty());
    let line = &request_lines(&requests)[0];
    for part in ["limit=5", "before=last", "userId=user1", "groupId=topic1"] {
        assert!(line.contains(part), "missing {part} from {line}");
    }
}

#[tokio::test]
async fn invalid_page_options_never_contact_api() {
    let (url, requests) = mock_server(vec![]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let err = client
        .get_user_contract_metrics_page("user1", 10, 0, Some("unknown"), false)
        .await
        .unwrap_err();
    assert!(matches!(err, ApiError::InvalidInput(_)));
    assert!(
        client
            .list_markets(Some(1001), None, None, None)
            .await
            .is_err()
    );
    assert!(
        client
            .get_user_portfolio_history("user1", "yearly")
            .await
            .is_err()
    );
    assert!(request_lines(&requests).is_empty());
}

#[tokio::test]
async fn open_limit_filters_emit_query_keys() {
    use manifoldr::api::client::BetFilters;
    let (url, requests) = mock_server(vec![("200 OK", "[]")]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let result = client
        .list_bets_filtered(BetFilters {
            username: Some("sample_trader"),
            contract_slug: Some("market-slug"),
            limit: Some(12),
            kinds: Some("open-limit"),
            before_time: Some(1706000000000),
            order: Some("asc"),
            ..BetFilters::default()
        })
        .await
        .unwrap();
    assert!(result.is_empty());
    let line = &request_lines(&requests)[0];
    for term in [
        "username=sample_trader",
        "contractSlug=market-slug",
        "kinds=open-limit",
        "limit=12",
        "beforeTime=1706000000000",
        "order=asc",
    ] {
        assert!(line.contains(term), "missing {term} in {line}");
    }
}

#[tokio::test]
async fn cancel_is_local_preview_until_execute() {
    let (url, requests) = mock_server(vec![("200 OK", "{}")]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let preview = client.cancel_limit_order("bet_123", false).await.unwrap();
    assert_eq!(preview["executed"], false);
    assert!(request_lines(&requests).is_empty());
    assert!(
        client
            .cancel_limit_order("../injection", true)
            .await
            .is_err()
    );
    assert!(request_lines(&requests).is_empty());
    let result = client.cancel_limit_order("bet_123", true).await.unwrap();
    assert!(result.is_object());
    assert_eq!(
        request_lines(&requests),
        ["POST /v0/bet/cancel/bet_123 HTTP/1.1"]
    );
}

#[tokio::test]
async fn cancel_does_not_retry_failed_post() {
    let (url, requests) = mock_server(vec![(
        "503 Service Unavailable",
        r#"{"error":"unavailable"}"#,
    )])
    .await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let err = client.cancel_limit_order("123abc", true).await.unwrap_err();
    assert_eq!(err.status(), Some(reqwest::StatusCode::SERVICE_UNAVAILABLE));
    assert_eq!(
        request_lines(&requests),
        ["POST /v0/bet/cancel/123abc HTTP/1.1"]
    );
}

#[tokio::test]
async fn bad_bet_filters_make_no_requests() {
    use manifoldr::api::client::BetFilters;
    let (url, requests) = mock_server(vec![("200 OK", "[]")]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    for filters in [
        BetFilters {
            before: Some("a"),
            after: Some("b"),
            ..BetFilters::default()
        },
        BetFilters {
            kinds: Some("unknown"),
            ..BetFilters::default()
        },
        BetFilters {
            order: Some("random"),
            ..BetFilters::default()
        },
        BetFilters {
            limit: Some(1001),
            ..BetFilters::default()
        },
    ] {
        assert!(client.list_bets_filtered(filters).await.is_err());
    }
    assert!(request_lines(&requests).is_empty());
}

#[tokio::test]
async fn sell_shares_previews_without_network_and_posts_once_when_explicit() {
    let (url, requests) = mock_server(vec![("200 OK", r#"{"sharesSold":4.0}"#)]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let preview = client
        .sell_shares("market123", "YES", Some(4.0), Some("answer456"), false)
        .await
        .unwrap();
    assert_eq!(preview["executed"], false);
    assert_eq!(preview["shares"], 4.0);
    assert!(
        request_lines(&requests).is_empty(),
        "preview sent a request"
    );

    let result = client
        .sell_shares("market123", "YES", Some(4.0), Some("answer456"), true)
        .await
        .unwrap();
    assert_eq!(result["sharesSold"], 4.0);
    let all = requests.lock().unwrap();
    assert_eq!(all.len(), 1);
    assert!(all[0].starts_with("POST /v0/market/market123/sell HTTP/1.1"));
    let body: serde_json::Value =
        serde_json::from_str(all[0].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(body["outcome"], "YES");
    assert_eq!(body["shares"], 4.0);
    assert_eq!(body["answerId"], "answer456");
}

#[tokio::test]
async fn all_shares_omits_shares_property_and_never_retries() {
    let (url, requests) =
        mock_server(vec![("503 Service Unavailable", r#"{"error":"down"}"#)]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let error = client
        .sell_shares("m1", "NO", None, None, true)
        .await
        .unwrap_err();
    assert_eq!(
        error.status(),
        Some(reqwest::StatusCode::SERVICE_UNAVAILABLE)
    );
    let all = requests.lock().unwrap();
    assert_eq!(all.len(), 1, "sale POST must not retry");
    let body: serde_json::Value =
        serde_json::from_str(all[0].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(body["outcome"], "NO");
    assert!(body.get("shares").is_none());
    assert!(body.get("answerId").is_none());
}

#[tokio::test]
async fn invalid_sale_inputs_fail_before_post() {
    let (url, requests) = mock_server(vec![("200 OK", "{}")]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    for (market, outcome, shares, answer) in [
        ("../bad", "YES", Some(1.0), None),
        ("m1", "MAYBE", Some(1.0), None),
        ("m1", "YES", Some(0.0), None),
        ("m1", "YES", Some(f64::NAN), None),
        ("m1", "NO", Some(-1.0), None),
        ("m1", "NO", None, Some("../../wrong")),
    ] {
        assert!(
            client
                .sell_shares(market, outcome, shares, answer, true)
                .await
                .is_err()
        );
    }
    assert!(request_lines(&requests).is_empty());
}

#[tokio::test]
async fn search_filters_encode_topics_creator_type_and_offset() {
    use manifoldr::api::client::SearchFilters;
    let (url, requests) = mock_server(vec![("200 OK", "[]")]).await;
    let client = ManifoldClient::with_base_url(None, &url).unwrap();
    let markets = client
        .search_markets_advanced(
            "rust",
            SearchFilters {
                limit: Some(10),
                offset: 20,
                contract_type: Some("BINARY"),
                topic_slug: Some("economics"),
                creator_id: Some("user1"),
                ..SearchFilters::default()
            },
        )
        .await
        .unwrap();
    assert!(markets.is_empty());
    let line = &request_lines(&requests)[0];
    for key in [
        "offset=20",
        "contractType=BINARY",
        "topicSlug=economics",
        "creatorId=user1",
    ] {
        assert!(line.contains(key), "{key} missing from {line}");
    }
    assert!(
        client
            .search_markets_advanced(
                "",
                SearchFilters {
                    sort: Some("newest"),
                    before_time: Some(123),
                    offset: 10,
                    ..SearchFilters::default()
                }
            )
            .await
            .is_err()
    );
    assert_eq!(request_lines(&requests).len(), 1);
}


fn unique_cache_dir() -> std::path::PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("manifoldr-test-{}-{now}", std::process::id()))
}

#[tokio::test]
async fn public_cache_serves_second_request_without_network() {
    let user = r#"[{"id":"u1","createdTime":10,"name":"Tester","username":"test",
        "url":"https://manifold.markets/test","balance":1,"totalDeposits":2}]"#;
    let (url, requests) = mock_server(vec![("200 OK", user)]).await;
    let dir = unique_cache_dir();
    let mut client = ManifoldClient::with_base_url(None, &url).unwrap();
    client.configure_public_cache(dir.clone(), std::time::Duration::from_secs(60), false);
    let result = client.list_users(1, None).await.unwrap();
    assert_eq!(result[0].id, "u1");
    let second = tokio::time::timeout(std::time::Duration::from_millis(600),
        client.list_users(1, None)).await.unwrap().unwrap();
    assert_eq!(second[0].id, "u1");
    assert_eq!(request_lines(&requests).len(), 1, "unexpected second HTTP fetch");
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn offline_cache_reads_expired_data_and_rejects_misses_or_writes() {
    let (url, requests) = mock_server(vec![("200 OK", "[]")]).await;
    let dir = unique_cache_dir();
    let mut online = ManifoldClient::with_base_url(None, &url).unwrap();
    online.configure_public_cache(dir.clone(), std::time::Duration::from_secs(0), false);
    online.list_users(2, None).await.unwrap();
    let mut offline = ManifoldClient::with_base_url(None, &url).unwrap();
    offline.configure_public_cache(dir.clone(), std::time::Duration::from_secs(0), true);
    assert!(offline.list_users(2, None).await.unwrap().is_empty());
    assert!(matches!(offline.list_users(3, None).await.unwrap_err(), ApiError::OfflineMiss { .. }));
    assert!(matches!(offline.get_me().await.unwrap_err(), ApiError::OfflineMiss { .. }));
    assert!(matches!(offline.place_bet("m1", 5.0, "YES", false).await.unwrap_err(),
        ApiError::OfflineWrite { .. }));
    assert_eq!(request_lines(&requests).len(), 1, "offline mode contacted network");
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn short_cache_ttl_forces_network_refresh() {
    let (url, requests) = mock_server(vec![
        ("200 OK", "[]"),
        ("200 OK", "[]")
    ]).await;
    let dir = unique_cache_dir();
    let mut client = ManifoldClient::with_base_url(None, &url).unwrap();
    client.configure_public_cache(dir.clone(), std::time::Duration::from_secs(0), false);
    client.list_users(1, None).await.unwrap();
    client.list_users(1, None).await.unwrap();
    assert_eq!(request_lines(&requests).len(), 2);
    std::fs::remove_dir_all(dir).unwrap();
}
