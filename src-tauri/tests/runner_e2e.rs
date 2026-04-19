//! End-to-end: spawn sing-box via the Runner on the user's real profile,
//! then make an HTTP request through the local SOCKS5 inbound.
//!
//! Skips unless VELO_TEST_VLESS_URL is set (keeps offline dev runs fast and avoids hitting
//! the user's VPS on every `cargo test`).
//!
//! Success criteria:
//!   1. sing-box starts, binds 127.0.0.1:10808 within 3s.
//!   2. HTTP GET through the SOCKS5 inbound succeeds.
//!
//! Uses NoopSink — event delivery is a GUI concern, not runner behavior.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use velo_lib::config::singbox::{self, Options};
use velo_lib::core::{EventSink, NoopSink, Runner};
use velo_lib::parser;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn connect_and_fetch_ip() {
    let Ok(url) = std::env::var("VELO_TEST_VLESS_URL") else {
        eprintln!("skipped: set VELO_TEST_VLESS_URL to a VLESS profile URL");
        return;
    };

    let profile = parser::parse_any(&url).expect("parse profile");
    let cfg = singbox::build(&profile, &Options::default());

    let sink: Arc<dyn EventSink> = Arc::new(NoopSink);
    let runner = Runner::start(&cfg, sink.clone()).await.expect("runner start");

    let mut bound = false;
    for _ in 0..30 {
        if std::net::TcpStream::connect_timeout(
            &"127.0.0.1:10808".parse().unwrap(),
            Duration::from_millis(200),
        )
        .is_ok()
        {
            bound = true;
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }
    assert!(bound, "sing-box did not open 127.0.0.1:10808 within 3s");

    let proxy = reqwest::Proxy::all("socks5h://127.0.0.1:10808").expect("proxy url");
    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(20))
        .build()
        .expect("http client");

    let resp = client
        .get("https://api.ipify.org?format=json")
        .send()
        .await
        .expect("send through proxy");
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    eprintln!("status = {status}\nbody = {body}");
    assert!(status.is_success(), "upstream responded with {status}");

    runner.stop(sink).await;
}
