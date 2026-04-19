use serde_json::{json, Value};

use super::types::{Mode, Options};

pub(super) fn build_inbounds(opts: &Options) -> Value {
    let mut arr = Vec::new();
    if opts.mixed_port != 0 {
        arr.push(json!({
            "type": "mixed",
            "tag": "mixed-in",
            "listen": opts.listen,
            "listen_port": opts.mixed_port,
            "sniff": true,
            "sniff_override_destination": false
        }));
    }
    if opts.mode == Mode::Tun {
        arr.push(json!({
            "type": "tun",
            "tag": "tun-in",
            "interface_name": "velo-tun",
            "address": ["172.19.0.1/30", "fdfe:dcba:9876::1/126"],
            "auto_route": true,
            "strict_route": true,
            "stack": "mixed",
            "endpoint_independent_nat": true,
            "sniff": true
        }));
    }
    Value::Array(arr)
}

#[cfg(test)]
mod tests {
    use crate::config::singbox::{build, Mode, Options};
    use crate::parser;
    use pretty_assertions::assert_eq;

    #[test]
    fn tun_mode_emits_tun_inbound_alongside_mixed() {
        let p = parser::parse_any("trojan://pw@ex.com:443?type=tcp").unwrap();
        let cfg = build(
            &p,
            &Options { mode: Mode::Tun, ..Options::default() },
        );
        let inbounds = cfg["inbounds"].as_array().unwrap();
        assert_eq!(inbounds.len(), 2);
        assert!(inbounds.iter().any(|i| i["type"] == "mixed"));
        let tun = inbounds.iter().find(|i| i["type"] == "tun").unwrap();
        assert_eq!(tun["auto_route"], true);
        assert_eq!(tun["strict_route"], true);
    }

    #[test]
    fn sysproxy_mode_omits_tun_inbound() {
        let p = parser::parse_any("trojan://pw@ex.com:443?type=tcp").unwrap();
        let cfg = build(&p, &Options::default());
        let inbounds = cfg["inbounds"].as_array().unwrap();
        assert!(inbounds.iter().all(|i| i["type"] != "tun"));
    }
}
