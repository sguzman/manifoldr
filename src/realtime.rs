//! Resilient public Manifold WebSocket subscriptions.
//!
//! Uses the documented subscription and ping protocol. Does not send any
//! authentication headers or support account writes.
use anyhow::{Context, Result, bail};
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const MANIFOLD_WS: &str = "wss://api.manifold.markets/ws";

/// Validate subscriptions before opening any network connection.
pub fn validate_topics(topics: &[String]) -> Result<()> {
    if topics.is_empty() || topics.len() > 25 {
        bail!("Provide 1 to 25 subscription topics");
    }
    for topic in topics {
        if topic.len() > 256
            || topic.is_empty()
            || topic
                .bytes()
                .any(|ch| !(ch.is_ascii_alphanumeric() || b"/-_".contains(&ch)))
            || topic.starts_with('/')
            || topic.ends_with('/')
        {
            bail!("Invalid WebSocket topic: {topic}");
        }
    }
    Ok(())
}

/// Interpret a Manifold event without assuming a stable event payload schema.
pub fn parse_broadcast(message: &str) -> Result<Option<Value>> {
    let value: Value = serde_json::from_str(message).context("Invalid WebSocket JSON")?;
    if value.get("type").and_then(Value::as_str) == Some("broadcast") {
        Ok(Some(value))
    } else {
        Ok(None)
    }
}

/// Subscribe to public events and write one JSON object per line to stdout.
/// Reconnects and resubscribes after interruptions. Ctrl+C stops cleanly.
pub async fn run(topics: Vec<String>, count: usize) -> Result<()> {
    run_at(MANIFOLD_WS, topics, count).await
}

/// Override the WebSocket endpoint for local integration tests. No credentials
/// or authenticated protocol messages are ever transmitted.
pub async fn run_at(endpoint: &str, topics: Vec<String>, count: usize) -> Result<()> {
    validate_topics(&topics)?;
    let mut received = 0usize;
    let mut backoff = 1u64;
    loop {
        let session = tokio::select! {
            result = connect_async(endpoint) => result,
            _ = tokio::signal::ctrl_c() => return Ok(()),
        };
        match session {
            Ok((socket, _)) => {
                backoff = 1;
                let (mut writer, mut reader) = socket.split();
                let subscribe = json!({
                    "type": "subscribe", "txid": 0, "topics": topics
                });
                writer
                    .send(Message::Text(subscribe.to_string().into()))
                    .await
                    .context("WebSocket subscribe failed")?;
                let mut ping = tokio::time::interval(Duration::from_secs(30));
                ping.tick().await;
                let mut txid = 1u64;
                loop {
                    tokio::select! {
                        _ = tokio::signal::ctrl_c() => {
                            let _ = writer.send(Message::Close(None)).await;
                            return Ok(());
                        }
                        _ = ping.tick() => {
                            let command = json!({"type": "ping", "txid": txid});
                            txid = txid.wrapping_add(1);
                            if writer.send(Message::Text(command.to_string().into())).await.is_err() {
                                break;
                            }
                        }
                        message = reader.next() => {
                            match message {
                                Some(Ok(Message::Text(body))) => {
                                    let value: Value = match serde_json::from_str(&body) {
                                        Ok(value) => value,
                                        Err(error) => {
                                            eprintln!("Skipping invalid WebSocket JSON: {error}");
                                            continue;
                                        }
                                    };
                                    if value.get("type").and_then(Value::as_str) == Some("ack")
                                        && value.get("success") == Some(&Value::Bool(false)) {
                                        bail!("Manifold refused the WebSocket subscription: {value}");
                                    }
                                    if value.get("type").and_then(Value::as_str) == Some("broadcast") {
                                        println!("{}", serde_json::to_string(&value)?);
                                        received += 1;
                                        if count > 0 && received >= count {
                                            let _ = writer.send(Message::Close(None)).await;
                                            return Ok(());
                                        }
                                    }
                                }
                                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                                Some(Ok(_)) => {}
                            }
                        }
                    }
                }
            }
            Err(err) => eprintln!("Manifold stream unavailable: {err}"),
        }
        eprintln!("Reconnecting Manifold stream in {backoff}s");
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(backoff)) => {},
            _ = tokio::signal::ctrl_c() => return Ok(()),
        }
        backoff = (backoff * 2).min(32);
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_broadcast, validate_topics};

    #[test]
    fn topics_must_be_well_formed() {
        assert!(
            validate_topics(&["global/new-bet".into(), "contract/abc-123/new-bet".into()]).is_ok()
        );
        assert!(validate_topics(&[]).is_err());
        assert!(validate_topics(&["../bad".into()]).is_err());
        assert!(validate_topics(&["".into()]).is_err());
        assert!(validate_topics(&["contract/bad?query".into()]).is_err());
    }

    #[test]
    fn broadcasts_are_distinguished_from_acknowledgements() {
        assert!(
            parse_broadcast(r#"{"type":"ack","txid":1,"success":true}"#)
                .unwrap()
                .is_none()
        );
        let msg = parse_broadcast(
            r#"{"type":"broadcast","topic":"global/new-bet","data":{"id":"bet1"}}"#,
        )
        .unwrap()
        .unwrap();
        assert_eq!(msg["data"]["id"], "bet1");
    }
}


#[cfg(test)]
mod integration_tests {
    use super::run_at;
    use futures::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::{accept_async, tungstenite::Message};

    #[tokio::test]
    async fn websocket_reconnects_and_resubscribes_after_disconnect() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("ws://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for attempt in 0..2 {
                let (stream, _) = listener.accept().await.unwrap();
                let mut socket = accept_async(stream).await.unwrap();
                let first = socket.next().await.unwrap().unwrap();
                let Message::Text(body) = first else { panic!("expected subscribe") };
                let command: serde_json::Value = serde_json::from_str(&body).unwrap();
                assert_eq!(command["type"], "subscribe");
                assert_eq!(command["topics"][0], "global/new-bet");
                if attempt == 0 {
                    socket.close(None).await.unwrap();
                } else {
                    socket.send(Message::Text(
                        r#"{"type":"broadcast","topic":"global/new-bet","data":{"id":"test"}}"#.into()
                    )).await.unwrap();
                    let _ = socket.next().await;
                }
            }
        });
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(6),
            run_at(&endpoint, vec!["global/new-bet".into()], 1)
        ).await;
        assert!(result.is_ok(), "reconnection timed out");
        result.unwrap().unwrap();
        server.await.unwrap();
    }
}
