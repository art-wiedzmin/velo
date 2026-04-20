use serde_json::{json, Value};

use super::types::{TAG_DIRECT, TAG_PROXY};
use crate::profile::Profile;

pub(super) fn build_dns(profile: &Profile) -> Value {
    // Both servers use DoT over TCP/853. We deliberately avoid plain UDP/53
    // (`address: "1.1.1.1"`) here: many ISPs — especially in regions where
    // users actually need a proxy — block or intercept UDP/53, which silently
    // breaks every lookup the moment the proxy server is identified by
    // hostname. TCP/853 survives that filtering. We also avoid `address:
    // "local"` (system resolver) because its syscalls get re-captured by TUN
    // auto_route on Windows, deadlocking lookups in Tunnel mode.
    let servers = json!([
        { "tag": "remote", "address": "tls://1.1.1.1", "detour": TAG_PROXY },
        { "tag": "direct", "address": "tls://1.1.1.1", "detour": TAG_DIRECT }
    ]);

    let mut rules: Vec<Value> = Vec::new();

    // Bootstrap: resolving the proxy server's own hostname via the proxy is a
    // chicken-and-egg deadlock (we need its IP to connect to it, but the
    // lookup is routed through it). Pin it to the direct resolver. No-op when
    // the profile already carries a raw IP.
    if profile.address.parse::<std::net::IpAddr>().is_err() {
        rules.push(json!({
            "domain": [profile.address.clone()],
            "server": "direct"
        }));
    }

    // Anything sing-box resolves for its own internal plumbing (identified by
    // `outbound: "any"`) must never go through the proxy itself. Direct-DNS
    // handles those without the proxy-server-domain loop.
    rules.push(json!({ "outbound": "any", "server": "direct" }));

    json!({
        "servers": servers,
        "rules": rules,
        "final": "remote",
        "strategy": "prefer_ipv4"
    })
}

#[cfg(test)]
mod tests {
    use crate::config::singbox::{build, Options};
    use crate::parser;
    use pretty_assertions::assert_eq;

    #[test]
    fn domain_server_is_bootstrapped_via_direct() {
        let p = parser::parse_any(
            "vless://00000000-0000-4000-8000-000000000000@srv.example.org:443?type=tcp",
        )
        .unwrap();
        let cfg = build(&p, &Options::default());
        let rules = cfg["dns"]["rules"].as_array().unwrap();
        let bootstrap = rules
            .iter()
            .find(|r| r.get("domain").is_some())
            .expect("bootstrap rule present for domain server");
        assert_eq!(bootstrap["domain"][0], "srv.example.org");
        assert_eq!(bootstrap["server"], "direct");
    }

    #[test]
    fn ip_server_gets_no_bootstrap_rule() {
        let p =
            parser::parse_any("vless://00000000-0000-4000-8000-000000000000@1.2.3.4:443?type=tcp")
                .unwrap();
        let cfg = build(&p, &Options::default());
        let rules = cfg["dns"]["rules"].as_array().unwrap();
        assert!(
            rules.iter().all(|r| r.get("domain").is_none()),
            "IP servers don't need a domain bootstrap rule",
        );
    }

    #[test]
    fn direct_server_is_dot_via_direct_outbound() {
        // The old `address: "local"` deadlocks in TUN mode because the system
        // resolver's syscalls get re-captured by auto_route. Plain UDP/53 is
        // also unreliable — commonly filtered by ISPs. DoT over TCP/853
        // through the direct outbound survives both.
        let p = parser::parse_any("trojan://pw@example.invalid:443?type=tcp").unwrap();
        let cfg = build(&p, &Options::default());
        let servers = cfg["dns"]["servers"].as_array().unwrap();
        let direct = servers
            .iter()
            .find(|s| s["tag"] == "direct")
            .expect("direct server tagged");
        assert_eq!(direct["address"], "tls://1.1.1.1");
        assert_eq!(direct["detour"], "direct");
    }
}