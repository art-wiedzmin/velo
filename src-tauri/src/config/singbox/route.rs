use serde_json::{json, Value};

use super::types::{Mode, Options, RoutingMode, TAG_DIRECT, TAG_PROXY};

pub(super) fn build_route(opts: &Options) -> Value {
    // Baseline: hijack DNS, keep private IPs local.
    let mut rules: Vec<Value> = vec![
        // hijack-dns is the sing-box 1.11 replacement for the deprecated
        // `dns` outbound. It MUST be filtered — an unfiltered hijack-dns rule
        // matches every packet and funnels all traffic through the DNS hijack,
        // which fails parsing and tears down every connection.
        json!({ "protocol": "dns", "action": "hijack-dns" }),
        json!({ "ip_is_private": true, "outbound": TAG_DIRECT }),
    ];

    // Process-level routing only makes sense when the TUN interface is
    // actually capturing system traffic; in Sysproxy mode only apps that
    // opted into the HTTP proxy reach us at all.
    let mut final_tag = TAG_PROXY;
    if opts.mode == Mode::Tun && !opts.routing_apps.is_empty() {
        let apps: Vec<Value> = opts
            .routing_apps
            .iter()
            .map(|p| Value::from(p.clone()))
            .collect();
        match opts.routing_mode {
            RoutingMode::Whitelist => {
                // Only listed apps traverse the proxy; everything else bypasses.
                rules.push(json!({ "process_path": apps, "outbound": TAG_PROXY }));
                final_tag = TAG_DIRECT;
            }
            RoutingMode::Blacklist => {
                // Listed apps bypass, everyone else still flows through.
                rules.push(json!({ "process_path": apps, "outbound": TAG_DIRECT }));
                final_tag = TAG_PROXY;
            }
            RoutingMode::None => {}
        }
    }

    json!({
        "rules": rules,
        "final": final_tag,
        "auto_detect_interface": true
    })
}

#[cfg(test)]
mod tests {
    use crate::config::singbox::{build, Mode, Options, RoutingMode};
    use crate::parser;
    use pretty_assertions::assert_eq;

    #[test]
    fn whitelist_routes_listed_apps_to_proxy_and_final_direct() {
        let p = parser::parse_any("trojan://pw@ex.com:443?type=tcp").unwrap();
        let cfg = build(
            &p,
            &Options {
                mode: Mode::Tun,
                routing_mode: RoutingMode::Whitelist,
                routing_apps: vec![r"C:\Apps\browser.exe".into()],
                ..Options::default()
            },
        );
        let rules = cfg["route"]["rules"].as_array().unwrap();
        let app_rule = rules
            .iter()
            .find(|r| r.get("process_path").is_some())
            .unwrap();
        assert_eq!(app_rule["outbound"], "proxy");
        assert_eq!(cfg["route"]["final"], "direct");
    }

    #[test]
    fn blacklist_routes_listed_apps_to_direct_and_final_proxy() {
        let p = parser::parse_any("trojan://pw@ex.com:443?type=tcp").unwrap();
        let cfg = build(
            &p,
            &Options {
                mode: Mode::Tun,
                routing_mode: RoutingMode::Blacklist,
                routing_apps: vec![r"C:\Apps\torrent.exe".into()],
                ..Options::default()
            },
        );
        let rules = cfg["route"]["rules"].as_array().unwrap();
        let app_rule = rules
            .iter()
            .find(|r| r.get("process_path").is_some())
            .unwrap();
        assert_eq!(app_rule["outbound"], "direct");
        assert_eq!(cfg["route"]["final"], "proxy");
    }

    #[test]
    fn routing_rules_ignored_in_sysproxy_mode() {
        let p = parser::parse_any("trojan://pw@ex.com:443?type=tcp").unwrap();
        let cfg = build(
            &p,
            &Options {
                // Mode::Sysproxy is default; routing config should be dropped.
                routing_mode: RoutingMode::Whitelist,
                routing_apps: vec![r"C:\Apps\browser.exe".into()],
                ..Options::default()
            },
        );
        let rules = cfg["route"]["rules"].as_array().unwrap();
        assert!(rules.iter().all(|r| r.get("process_path").is_none()));
    }
}
