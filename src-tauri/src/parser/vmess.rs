//! VMess URL parser.
//!
//! Format: `vmess://<base64-of-JSON>` — invented by v2rayN, adopted everywhere.
//! The JSON schema is ad-hoc: numeric fields may come as strings or numbers,
//! many keys are optional, and `tls` is a string acting as a tagged union.

use super::common::*;
use super::error::ParseError;
use crate::profile::*;
use serde_json::Value;
use std::collections::HashMap;

const PREFIX: &str = "vmess://";

pub fn parse(input: &str) -> Result<Profile, ParseError> {
    let input = input.trim();
    let payload = input.strip_prefix(PREFIX).ok_or_else(|| ParseError::SchemeMismatch {
        expected: "vmess",
        got: input.split("://").next().unwrap_or("").to_owned(),
    })?;

    let bytes = base64_decode_loose(payload)?;
    let json: Value = serde_json::from_slice(&bytes)
        .map_err(|e| ParseError::InvalidPayload(format!("vmess json: {e}")))?;
    let obj = json
        .as_object()
        .ok_or_else(|| ParseError::InvalidPayload("vmess: expected JSON object".into()))?;

    let uuid = obj
        .get("id")
        .and_then(Value::as_str)
        .ok_or(ParseError::MissingCredential)?
        .to_owned();
    uuid::Uuid::parse_str(&uuid).map_err(|_| ParseError::InvalidUuid(uuid.clone()))?;

    let address = obj
        .get("add")
        .and_then(Value::as_str)
        .ok_or(ParseError::MissingHost)?
        .to_owned();
    let port = get_int(obj.get("port")).ok_or(ParseError::MissingPort)? as u16;
    let alter_id = get_int(obj.get("aid")).map(|v| v as u32);
    let cipher = obj
        .get("scy")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);

    let net = obj.get("net").and_then(Value::as_str).unwrap_or("tcp");
    let transport = parse_transport(net)?;

    let tls_field = obj.get("tls").and_then(Value::as_str).unwrap_or("");
    let security = parse_security(tls_field)?;

    // Remap VMess JSON keys onto the same shape our shared helpers expect.
    let mut qs: HashMap<String, String> = HashMap::new();
    if let Some(v) = obj.get("sni").and_then(Value::as_str) {
        if !v.is_empty() { qs.insert("sni".into(), v.into()); }
    }
    if let Some(v) = obj.get("alpn").and_then(Value::as_str) {
        if !v.is_empty() { qs.insert("alpn".into(), v.into()); }
    }
    if let Some(v) = obj.get("fp").and_then(Value::as_str) {
        if !v.is_empty() { qs.insert("fp".into(), v.into()); }
    }
    if let Some(v) = obj.get("path").and_then(Value::as_str) {
        if !v.is_empty() { qs.insert("path".into(), v.into()); }
    }
    if let Some(v) = obj.get("host").and_then(Value::as_str) {
        if !v.is_empty() { qs.insert("host".into(), v.into()); }
    }
    // gRPC: v2rayN stores the service name in `path`.
    if transport == Transport::Grpc {
        if let Some(sn) = qs.get("path").cloned() {
            qs.insert("serviceName".into(), sn);
        }
    }

    let tls = build_tls(&qs, security, &address)?;
    let name = obj
        .get("ps")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{address}:{port}"));

    Ok(Profile {
        name,
        protocol: Protocol::Vmess,
        address,
        port,
        credential: uuid,
        flow: None,
        packet_encoding: None,
        alter_id,
        cipher,
        ss_method: None,
        transport,
        transport_params: transport_params_from_query(&qs),
        security,
        tls,
    })
}

fn get_int(v: Option<&Value>) -> Option<i64> {
    match v? {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use pretty_assertions::assert_eq;

    fn encode(json: &str) -> String {
        let b = base64::engine::general_purpose::STANDARD.encode(json.as_bytes());
        format!("vmess://{b}")
    }

    #[test]
    fn basic_ws_tls() {
        let json = r#"{
            "v":"2","ps":"demo","add":"example.com","port":"443",
            "id":"00000000-0000-4000-8000-000000000000","aid":"0",
            "scy":"auto","net":"ws","type":"none","host":"front.example",
            "path":"/ray","tls":"tls","sni":"example.com"
        }"#;
        let p = parse(&encode(json)).unwrap();
        assert_eq!(p.protocol, Protocol::Vmess);
        assert_eq!(p.address, "example.com");
        assert_eq!(p.port, 443);
        assert_eq!(p.name, "demo");
        assert_eq!(p.transport, Transport::Ws);
        assert_eq!(p.security, Security::Tls);
        assert_eq!(p.alter_id, Some(0));
        assert_eq!(p.cipher.as_deref(), Some("auto"));
        assert_eq!(p.transport_params.path.as_deref(), Some("/ray"));
        assert_eq!(p.transport_params.host.as_deref(), Some("front.example"));
        assert_eq!(p.tls.unwrap().sni.as_deref(), Some("example.com"));
    }

    #[test]
    fn port_as_number() {
        let json = r#"{"add":"x.com","port":8443,"id":"00000000-0000-4000-8000-000000000000","net":"tcp","tls":""}"#;
        let p = parse(&encode(json)).unwrap();
        assert_eq!(p.port, 8443);
        assert_eq!(p.security, Security::None);
    }

    #[test]
    fn grpc_service_name_from_path() {
        let json = r#"{"add":"x.com","port":"443","id":"00000000-0000-4000-8000-000000000000","net":"grpc","path":"mygrpc","tls":"tls"}"#;
        let p = parse(&encode(json)).unwrap();
        assert_eq!(p.transport, Transport::Grpc);
        assert_eq!(p.transport_params.service_name.as_deref(), Some("mygrpc"));
    }

    #[test]
    fn rejects_non_vmess() {
        assert!(matches!(
            parse("trojan://x@y:1"),
            Err(ParseError::SchemeMismatch { .. })
        ));
    }

    #[test]
    fn rejects_bad_base64() {
        assert!(matches!(parse("vmess://!!!"), Err(ParseError::InvalidBase64(_))));
    }
}
