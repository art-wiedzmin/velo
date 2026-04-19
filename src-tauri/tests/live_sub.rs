//! Live subscription probe. Runs only when VELO_TEST_SUB_URL is set so CI stays offline.
//! Usage (Windows): `set VELO_TEST_SUB_URL=<url> && cargo test --test live_sub -- --nocapture`

use velo_lib::subscription;

#[tokio::test]
async fn fetch_real_subscription() {
    let Ok(url) = std::env::var("VELO_TEST_SUB_URL") else {
        eprintln!("skipped: set VELO_TEST_SUB_URL to a subscription URL");
        return;
    };
    let fetched = subscription::fetch(&url).await.expect("fetch");
    let r = &fetched.result;
    eprintln!("decoded_base64 = {}", r.decoded_base64);
    eprintln!("profiles = {}", r.profiles.len());
    eprintln!(
        "quota: used={:?} total={:?} expires={:?}",
        fetched.quota.used_bytes, fetched.quota.total_bytes, fetched.quota.expires_at,
    );
    for p in &r.profiles {
        eprintln!(
            "- {:?} {} {}:{} sec={:?} transport={:?} flow={:?}",
            p.protocol, p.name, p.address, p.port, p.security, p.transport, p.flow
        );
    }
    for e in &r.errors {
        eprintln!("! line {}: {} — {}", e.line, e.input, e.error);
    }
    assert!(!r.profiles.is_empty(), "subscription returned no profiles");
}
