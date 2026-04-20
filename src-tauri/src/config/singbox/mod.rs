//! sing-box JSON config generator.
//!
//! Produces the schema for sing-box 1.11 — the current stable branch. The
//! generator is intentionally a pure function of `Profile + Options`: no I/O,
//! no defaults pulled from environment, so the same input always produces
//! byte-identical output. This makes reviewing what we *actually* hand to
//! sing-box trivial.
//! Only one outbound profile is active at a time — UI-level selection happens
//! before we get here.

mod types;
pub use types::*;

mod dns;
mod inbounds;
mod outbounds;
mod route;

use serde_json::{json, Value};

use crate::profile::*;

pub fn build(profile: &Profile, opts: &Options) -> Value {
    let mut cfg = json!({
        "log": { "level": opts.log_level, "timestamp": true },
        "dns": dns::build_dns(profile),
        "inbounds": inbounds::build_inbounds(opts),
        "outbounds": outbounds::build_outbounds(profile),
        "route": route::build_route(opts),
    });
    if opts.clash_api_port != 0 {
        // Clash API is emitted under `experimental` per sing-box 1.11 schema.
        // No `secret` because we only listen on `opts.listen` (loopback).
        cfg["experimental"] = json!({
            "clash_api": {
                "external_controller": format!("{}:{}", opts.listen, opts.clash_api_port),
            }
        });
    }
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use pretty_assertions::assert_eq;

    fn build_from_url(url: &str) -> Value {
        let p = parser::parse_any(url).expect("parse");
        build(&p, &Options::default())
    }

    #[test]
    fn inbound_and_route_shape() {
        let cfg = build_from_url("trojan://pw@ex.com:443?type=tcp");
        assert_eq!(cfg["inbounds"][0]["type"], "mixed");
        assert_eq!(cfg["inbounds"][0]["listen_port"], 10808);
        assert_eq!(cfg["route"]["final"], "proxy");
    }

    #[test]
    fn clash_api_default_enabled() {
        let cfg = build_from_url("trojan://pw@ex.com:443?type=tcp");
        assert_eq!(
            cfg["experimental"]["clash_api"]["external_controller"],
            "127.0.0.1:9090"
        );
    }

    #[test]
    fn clash_api_disabled_when_port_zero() {
        let p = parser::parse_any("trojan://pw@ex.com:443?type=tcp").unwrap();
        let cfg = build(
            &p,
            &Options { clash_api_port: 0, ..Options::default() },
        );
        assert!(cfg.get("experimental").is_none());
    }
}
