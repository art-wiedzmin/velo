use serde_json::{json, Value};

use super::types::{TAG_DIRECT, TAG_PROXY};
use crate::profile::Profile;

pub(super) fn build_dns(profile: &Profile) -> Value {
    // Both servers use DoT over TCP/853 (sing-box 1.12+ server format:
    // `type` + `server` instead of the legacy `address` URL, removed in
    // 1.14). We deliberately avoid plain UDP/53: many ISPs — especially in
    // regions where users actually need a proxy — block or intercept UDP/53,
    // which silently breaks every lookup the moment the proxy server is
    // identified by hostname. TCP/853 survives that filtering. We also avoid
    // `type: "local"` (system resolver) because its syscalls get re-captured
    // by TUN auto_route on Windows, deadlocking lookups in Tunnel mode.
    let servers = json!([
        { "type": "tls", "tag": "remote", "server": "1.1.1.1", "detour": TAG_PROXY },
        { "type": "tls", "tag": "direct", "server": "1.1.1.1", "detour": TAG_DIRECT }
    ]);

    let mut rules: Vec<Value> = Vec::new();

    // Bootstrap: resolving the proxy server's own hostname via the proxy is a
    // chicken-and-egg deadlock (we need its IP to connect to it, but the
    // lookup is routed through it). Pin it to the direct resolver. No-op when
    // the profile already carries a raw IP. Outbound dials are covered by
    // `route.default_domain_resolver` (the 1.12+ replacement for the legacy
    // `outbound: "any"` DNS rule); this rule additionally pins client
    // queries for the same name.
    if profile.address.parse::<std::net::IpAddr>().is_err() {
        rules.push(json!({
            "domain": [profile.address.clone()],
            "server": "direct"
        }));
    }

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
        // `type: "local"` deadlocks in TUN mode because the system resolver's
        // syscalls get re-captured by auto_route. Plain UDP/53 is also
        // unreliable — commonly filtered by ISPs. DoT over TCP/853 through
        // the direct outbound survives both.
        let p = parser::parse_any("trojan://pw@example.invalid:443?type=tcp").unwrap();
        let cfg = build(&p, &Options::default());
        let servers = cfg["dns"]["servers"].as_array().unwrap();
        let direct = servers
            .iter()
            .find(|s| s["tag"] == "direct")
            .expect("direct server tagged");
        assert_eq!(direct["type"], "tls");
        assert_eq!(direct["server"], "1.1.1.1");
        assert_eq!(direct["detour"], "direct");
    }

    #[test]
    fn outbound_dials_resolve_via_direct_resolver() {
        // The 1.12+ replacement for the legacy `outbound: "any"` DNS rule —
        // without it, dialing a domain-addressed proxy server either fails
        // ("missing domain resolver") or loops through the proxy itself.
        let p = parser::parse_any("trojan://pw@example.invalid:443?type=tcp").unwrap();
        let cfg = build(&p, &Options::default());
        assert_eq!(cfg["route"]["default_domain_resolver"], "direct");
    }
}