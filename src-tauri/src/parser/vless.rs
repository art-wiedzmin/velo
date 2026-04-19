//! VLESS URL parser.
//!
//! Format (de facto, used by Xray/sing-box/v2rayN):
//!   vless://<uuid>@<host>:<port>?<k=v&...>#<name>

use super::common::*;
use super::error::ParseError;
use crate::profile::*;
use percent_encoding::percent_decode_str;
use url::Url;

pub fn parse(input: &str) -> Result<Profile, ParseError> {
    let url = Url::parse(input.trim())?;
    if url.scheme() != "vless" {
        return Err(ParseError::SchemeMismatch {
            expected: "vless",
            got: url.scheme().to_owned(),
        });
    }

    let uuid_raw = url.username();
    if uuid_raw.is_empty() {
        return Err(ParseError::MissingCredential);
    }
    let uuid = percent_decode_str(uuid_raw).decode_utf8_lossy().into_owned();
    uuid::Uuid::parse_str(&uuid).map_err(|_| ParseError::InvalidUuid(uuid.clone()))?;

    let host = url.host_str().ok_or(ParseError::MissingHost)?.to_owned();
    let port = url.port().ok_or(ParseError::MissingPort)?;

    let qs = query_map(&url);
    let transport = parse_transport(qs.get("type").map(String::as_str).unwrap_or("tcp"))?;
    let security = parse_security(qs.get("security").map(String::as_str).unwrap_or("none"))?;
    let flow = qs
        .get("flow")
        .filter(|s| !s.is_empty())
        .map(|s| parse_flow(s))
        .transpose()?;
    let tls = build_tls(&qs, security, &host)?;

    Ok(Profile {
        name: extract_name(&url, &host, port),
        protocol: Protocol::Vless,
        address: host,
        port,
        credential: uuid,
        flow,
        packet_encoding: qs.get("packetEncoding").cloned(),
        alter_id: None,
        cipher: None,
        ss_method: None,
        transport,
        transport_params: transport_params_from_query(&qs),
        security,
        tls,
    })
}

fn parse_flow(s: &str) -> Result<Flow, ParseError> {
    match s {
        "xtls-rprx-vision" | "xtls-rprx-vision-udp443" => Ok(Flow::XtlsRprxVision),
        other => Err(ParseError::UnsupportedFlow(other.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn reality_vision_tcp() {
        let url = "vless://00000000-0000-4000-8000-000000000000@example.invalid:443\
                   ?security=reality&sni=example.invalid&fp=random\
                   &pbk=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\
                   &sid=0123456789abcdef&type=tcp&flow=xtls-rprx-vision\
                   &packetEncoding=xudp&encryption=none\
                   #%F0%9F%87%B1%F0%9F%87%BB%20reality-first";

        let p = parse(url).expect("parse");
        assert_eq!(p.protocol, Protocol::Vless);
        assert_eq!(p.address, "example.invalid");
        assert_eq!(p.port, 443);
        assert_eq!(p.credential, "00000000-0000-4000-8000-000000000000");
        assert_eq!(p.transport, Transport::Tcp);
        assert_eq!(p.security, Security::Reality);
        assert_eq!(p.flow, Some(Flow::XtlsRprxVision));
        assert_eq!(p.packet_encoding.as_deref(), Some("xudp"));
        assert_eq!(p.name, "\u{1F1F1}\u{1F1FB} reality-first");

        let tls = p.tls.as_ref().expect("tls present");
        assert_eq!(tls.sni.as_deref(), Some("example.invalid"));
        assert_eq!(tls.fingerprint.as_ref().unwrap().0, "random");
        let r = tls.reality.as_ref().unwrap();
        assert_eq!(r.public_key, "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        assert_eq!(r.short_id, "0123456789abcdef");
    }

    #[test]
    fn reality_without_pbk_errors() {
        let url = "vless://00000000-0000-4000-8000-000000000000@x.example:443\
                   ?security=reality&sid=abcd&type=tcp";
        assert!(matches!(parse(url), Err(ParseError::RealityMissingFields)));
    }

    #[test]
    fn plain_tls_ws() {
        let url = "vless://00000000-0000-4000-8000-000000000000@ex.com:443\
                   ?security=tls&type=ws&path=%2Fws&host=front.example#node1";
        let p = parse(url).unwrap();
        assert_eq!(p.transport, Transport::Ws);
        assert_eq!(p.security, Security::Tls);
        assert_eq!(p.transport_params.path.as_deref(), Some("/ws"));
        assert_eq!(p.transport_params.host.as_deref(), Some("front.example"));
        assert!(p.tls.as_ref().unwrap().reality.is_none());
        assert_eq!(p.tls.as_ref().unwrap().sni.as_deref(), Some("ex.com"));
    }

    #[test]
    fn rejects_non_vless() {
        assert!(matches!(
            parse("vmess://aaaa@x:1"),
            Err(ParseError::SchemeMismatch { .. })
        ));
    }

    #[test]
    fn rejects_bad_uuid() {
        let url = "vless://not-a-uuid@x:443?type=tcp";
        assert!(matches!(parse(url), Err(ParseError::InvalidUuid(_))));
    }

    #[test]
    fn ip_address_no_default_sni() {
        let url = "vless://00000000-0000-4000-8000-000000000000@1.2.3.4:443?security=tls&type=tcp";
        let p = parse(url).unwrap();
        assert_eq!(p.tls.unwrap().sni, None);
    }
}
