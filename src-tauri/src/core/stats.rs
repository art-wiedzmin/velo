//! Clash-compatible stats client.
//!
//! Subscribes to the sing-box RESTful API exposed as WebSockets on two
//! endpoints:
//!   * `/traffic`     — one `{"up": N, "down": N}` object per second (bytes/sec)
//!   * `/connections` — snapshot with active connections + cumulative totals
//!
//! Both endpoints push from the server; we never send anything back. A
//! subscriber task retries with bounded exponential backoff so that slow
//! sing-box startup doesn't race us, and a single hiccup doesn't kill the
//! feed. Tasks are aborted by `Runner::stop` / `Drop` — no graceful shutdown
//! needed since the server is going away anyway.
//!
//! The payload shape of `/connections` varies by sing-box version and carries
//! metadata we don't parse (per-connection chains, rule hits, interface info).
//! We forward the raw `serde_json::Value` so the frontend can render whatever
//! fields are present without forcing the backend to track schema drift.

use super::sink::EventSink;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Traffic {
    pub up: u64,
    pub down: u64,
}

/// Extract the clash_api listen address from a generated sing-box config.
/// Returns `None` when clash_api is disabled or missing.
fn clash_api_addr(cfg: &serde_json::Value) -> Option<String> {
    cfg.get("experimental")?
        .get("clash_api")?
        .get("external_controller")?
        .as_str()
        .map(|s| s.to_owned())
}

/// Spawn subscribers for `/traffic` and `/connections` given a generated config.
/// Returns empty if clash_api is disabled.
pub fn spawn_from_config(
    cfg: &serde_json::Value,
    sink: Arc<dyn EventSink>,
) -> Vec<JoinHandle<()>> {
    let Some(addr) = clash_api_addr(cfg) else {
        return Vec::new();
    };
    vec![
        tokio::spawn(run_traffic(addr.clone(), sink.clone())),
        tokio::spawn(run_connections(addr, sink)),
    ]
}

async fn run_traffic(addr: String, sink: Arc<dyn EventSink>) {
    run_subscriber(&addr, "traffic", sink, |value, sink| {
        if let Ok(t) = serde_json::from_value::<Traffic>(value) {
            sink.traffic(&t);
        }
    })
    .await;
}

async fn run_connections(addr: String, sink: Arc<dyn EventSink>) {
    run_subscriber(&addr, "connections", sink, |value, sink| {
        sink.connections(&value);
    })
    .await;
}

/// Shared connect/read/reconnect loop. `handler` fires per decoded JSON frame.
async fn run_subscriber<F>(
    addr: &str,
    endpoint: &str,
    sink: Arc<dyn EventSink>,
    handler: F,
) where
    F: Fn(serde_json::Value, &Arc<dyn EventSink>),
{
    let url = format!("ws://{addr}/{endpoint}");
    // Backoff strategy: start at 200 ms, cap at ~5 s. Reset on any successful
    // frame so a long-running session recovers quickly after a blip.
    let mut delay = Duration::from_millis(200);

    loop {
        match connect_async(&url).await {
            Ok((mut ws, _)) => {
                delay = Duration::from_millis(200);
                while let Some(msg) = ws.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                                handler(v, &sink);
                            }
                        }
                        Ok(Message::Binary(bytes)) => {
                            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                                handler(v, &sink);
                            }
                        }
                        Ok(Message::Close(_)) | Err(_) => break,
                        _ => {}
                    }
                }
            }
            Err(_) => {
                // Not connected yet (sing-box still starting) or the listener
                // vanished. Back off and try again.
            }
        }

        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(Duration::from_secs(5));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn addr_extracted_when_present() {
        let cfg = json!({
            "experimental": { "clash_api": { "external_controller": "127.0.0.1:9090" } }
        });
        assert_eq!(clash_api_addr(&cfg).as_deref(), Some("127.0.0.1:9090"));
    }

    #[test]
    fn addr_none_when_disabled() {
        let cfg = json!({ "inbounds": [] });
        assert_eq!(clash_api_addr(&cfg), None);
    }

    #[test]
    fn traffic_deserializes_clash_shape() {
        let t: Traffic = serde_json::from_str(r#"{"up": 1024, "down": 4096}"#).unwrap();
        assert_eq!(t.up, 1024);
        assert_eq!(t.down, 4096);
    }
}
