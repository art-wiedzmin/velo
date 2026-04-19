use serde_json::{json, Value};

use super::types::{TAG_DIRECT, TAG_PROXY};

pub(super) fn build_dns() -> Value {
    // Minimal DNS: proxy goes through proxy, direct through system resolver.
    json!({
        "servers": [
            { "tag": "remote", "address": "tls://1.1.1.1",  "detour": TAG_PROXY },
            { "tag": "local",  "address": "local",           "detour": TAG_DIRECT }
        ],
        "rules": [
            { "outbound": "any", "server": "local" }
        ],
        "final": "remote",
        "strategy": "prefer_ipv4"
    })
}
