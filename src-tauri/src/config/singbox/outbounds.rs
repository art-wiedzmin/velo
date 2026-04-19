use serde_json::{json, Map, Value};

use super::types::{TAG_DIRECT, TAG_PROXY};
use crate::profile::*;

pub(super) fn build_outbounds(profile: &Profile) -> Value {
    json!([
        build_proxy_outbound(profile),
        { "type": "direct", "tag": TAG_DIRECT }
    ])
}

fn build_proxy_outbound(p: &Profile) -> Value {
    let mut o = Map::new();
    o.insert("tag".into(), TAG_PROXY.into());
    o.insert("server".into(), p.address.clone().into());
    o.insert("server_port".into(), p.port.into());

    match p.protocol {
        Protocol::Vless => {
            o.insert("type".into(), "vless".into());
            o.insert("uuid".into(), p.credential.clone().into());
            if let Some(Flow::XtlsRprxVision) = p.flow {
                o.insert("flow".into(), "xtls-rprx-vision".into());
            }
            if let Some(pe) = &p.packet_encoding {
                o.insert("packet_encoding".into(), pe.clone().into());
            }
        }
        Protocol::Vmess => {
            o.insert("type".into(), "vmess".into());
            o.insert("uuid".into(), p.credential.clone().into());
            o.insert(
                "security".into(),
                p.cipher.clone().unwrap_or_else(|| "auto".into()).into(),
            );
            o.insert("alter_id".into(), p.alter_id.unwrap_or(0).into());
        }
        Protocol::Trojan => {
            o.insert("type".into(), "trojan".into());
            o.insert("password".into(), p.credential.clone().into());
        }
        Protocol::Shadowsocks => {
            o.insert("type".into(), "shadowsocks".into());
            o.insert(
                "method".into(),
                p.ss_method.clone().unwrap_or_else(|| "chacha20-ietf-poly1305".into()).into(),
            );
            o.insert("password".into(), p.credential.clone().into());
        }
        Protocol::Hysteria2 | Protocol::Tuic => {
            // Parsers for these aren't wired yet; keep the arm explicit so a
            // future addition can't silently fall through to VLESS shape.
            o.insert("type".into(), "placeholder-unsupported".into());
        }
    }

    if let Some(tls) = build_tls(p) {
        o.insert("tls".into(), tls);
    }
    if let Some(tr) = build_transport(p) {
        o.insert("transport".into(), tr);
    }

    Value::Object(o)
}

fn build_tls(p: &Profile) -> Option<Value> {
    let tls = p.tls.as_ref()?;
    let mut m = Map::new();
    m.insert("enabled".into(), true.into());
    if let Some(sni) = &tls.sni {
        m.insert("server_name".into(), sni.clone().into());
    }
    if !tls.alpn.is_empty() {
        m.insert(
            "alpn".into(),
            Value::Array(tls.alpn.iter().cloned().map(Value::from).collect()),
        );
    }
    if tls.allow_insecure {
        m.insert("insecure".into(), true.into());
    }

    // uTLS: required for Reality, optional otherwise. Default to "chrome"
    // when Reality is on and the profile didn't specify — sing-box rejects
    // a Reality outbound without a uTLS fingerprint.
    let fingerprint = tls
        .fingerprint
        .as_ref()
        .map(|f| f.0.clone())
        .or_else(|| tls.reality.is_some().then(|| "chrome".into()));
    if let Some(fp) = fingerprint {
        m.insert("utls".into(), json!({ "enabled": true, "fingerprint": fp }));
    }

    if let Some(r) = &tls.reality {
        let mut reality = Map::new();
        reality.insert("enabled".into(), true.into());
        reality.insert("public_key".into(), r.public_key.clone().into());
        reality.insert("short_id".into(), r.short_id.clone().into());
        m.insert("reality".into(), Value::Object(reality));
    }

    Some(Value::Object(m))
}

fn build_transport(p: &Profile) -> Option<Value> {
    let tp = &p.transport_params;
    match p.transport {
        Transport::Tcp => None,
        Transport::Ws => {
            let mut m = Map::new();
            m.insert("type".into(), "ws".into());
            if let Some(path) = &tp.path {
                m.insert("path".into(), path.clone().into());
            }
            if let Some(host) = &tp.host {
                m.insert("headers".into(), json!({ "Host": host }));
            }
            Some(Value::Object(m))
        }
        Transport::Grpc => {
            let mut m = Map::new();
            m.insert("type".into(), "grpc".into());
            if let Some(sn) = &tp.service_name {
                m.insert("service_name".into(), sn.clone().into());
            }
            Some(Value::Object(m))
        }
        Transport::H2 => {
            let mut m = Map::new();
            m.insert("type".into(), "http".into());
            if let Some(path) = &tp.path {
                m.insert("path".into(), path.clone().into());
            }
            if let Some(host) = &tp.host {
                m.insert("host".into(), Value::Array(vec![host.clone().into()]));
            }
            Some(Value::Object(m))
        }
        Transport::Httpupgrade => {
            let mut m = Map::new();
            m.insert("type".into(), "httpupgrade".into());
            if let Some(path) = &tp.path {
                m.insert("path".into(), path.clone().into());
            }
            if let Some(host) = &tp.host {
                m.insert("host".into(), host.clone().into());
            }
            Some(Value::Object(m))
        }
        // sing-box doesn't implement xhttp; surface it as unsupported in the
        // config rather than silently mapping to ws.
        Transport::Xhttp => Some(json!({ "type": "xhttp-unsupported" })),
    }
}

#[cfg(test)]
mod tests {
    use crate::config::singbox::{build, Options};
    use crate::parser;
    use pretty_assertions::assert_eq;
    use serde_json::Value;

    fn build_from_url(url: &str) -> Value {
        let p = parser::parse_any(url).expect("parse");
        build(&p, &Options::default())
    }

    #[test]
    fn reality_vision_tcp() {
        let url = "vless://00000000-0000-4000-8000-000000000000@example.invalid:443\
                   ?security=reality&sni=example.invalid&fp=random\
                   &pbk=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\
                   &sid=0123456789abcdef&type=tcp&flow=xtls-rprx-vision\
                   &packetEncoding=xudp&encryption=none#node";

        let cfg = build_from_url(url);
        let outs = cfg["outbounds"].as_array().unwrap();
        let proxy = outs.iter().find(|o| o["tag"] == "proxy").unwrap();

        assert_eq!(proxy["type"], "vless");
        assert_eq!(proxy["server"], "example.invalid");
        assert_eq!(proxy["server_port"], 443);
        assert_eq!(proxy["uuid"], "00000000-0000-4000-8000-000000000000");
        assert_eq!(proxy["flow"], "xtls-rprx-vision");
        assert_eq!(proxy["packet_encoding"], "xudp");
        assert!(proxy.get("transport").is_none(), "tcp must not emit transport");

        let tls = &proxy["tls"];
        assert_eq!(tls["enabled"], true);
        assert_eq!(tls["server_name"], "example.invalid");
        assert_eq!(tls["utls"]["enabled"], true);
        assert_eq!(tls["utls"]["fingerprint"], "random");
        assert_eq!(tls["reality"]["enabled"], true);
        assert_eq!(tls["reality"]["public_key"], "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        assert_eq!(tls["reality"]["short_id"], "0123456789abcdef");
    }

    #[test]
    fn vmess_ws_tls() {
        use base64::Engine;
        let inner = r#"{"add":"ex.com","port":"443","id":"00000000-0000-4000-8000-000000000000","aid":"0","net":"ws","tls":"tls","path":"/ray","host":"front.ex","sni":"ex.com","scy":"auto","ps":"n"}"#;
        let url = format!("vmess://{}", base64::engine::general_purpose::STANDARD.encode(inner));

        let cfg = build_from_url(&url);
        let proxy = &cfg["outbounds"].as_array().unwrap()[0];
        assert_eq!(proxy["type"], "vmess");
        assert_eq!(proxy["alter_id"], 0);
        assert_eq!(proxy["security"], "auto");
        assert_eq!(proxy["transport"]["type"], "ws");
        assert_eq!(proxy["transport"]["path"], "/ray");
        assert_eq!(proxy["transport"]["headers"]["Host"], "front.ex");
        assert_eq!(proxy["tls"]["server_name"], "ex.com");
    }

    #[test]
    fn trojan_tcp_tls() {
        let cfg = build_from_url("trojan://pw@ex.com:443?type=tcp#n");
        let proxy = &cfg["outbounds"].as_array().unwrap()[0];
        assert_eq!(proxy["type"], "trojan");
        assert_eq!(proxy["password"], "pw");
        assert_eq!(proxy["tls"]["enabled"], true);
        assert!(proxy["tls"].get("reality").is_none());
    }

    #[test]
    fn reality_default_fingerprint_when_absent() {
        let url = "vless://00000000-0000-4000-8000-000000000000@ex.com:443\
                   ?security=reality&pbk=AAA&sid=BBB&type=tcp";
        let cfg = build_from_url(url);
        let proxy = &cfg["outbounds"].as_array().unwrap()[0];
        // Reality without explicit fp must still carry uTLS — enforced default.
        assert_eq!(proxy["tls"]["utls"]["enabled"], true);
        assert_eq!(proxy["tls"]["utls"]["fingerprint"], "chrome");
    }
}
