//! Trojan URL parser.
//!
//! Format: `trojan://<password>@<host>:<port>?<k=v&...>#<name>`.
//! Trojan implies TLS by default; servers that disable it use `security=none`
//! which we accept literally rather than silently upgrading.

use super::common::*;
use super::error::ParseError;
use crate::profile::*;
use percent_encoding::percent_decode_str;
use url::Url;

pub fn parse(input: &str) -> Result<Profile, ParseError> {
    let url = Url::parse(input.trim())?;
    if url.scheme() != "trojan" {
        return Err(ParseError::SchemeMismatch {
            expected: "trojan",
            got: url.scheme().to_owned(),
        });
    }

    let pwd_raw = url.username();
    if pwd_raw.is_empty() {
        return Err(ParseError::MissingCredential);
    }
    let password = percent_decode_str(pwd_raw).decode_utf8_lossy().into_owned();

    let host = url.host_str().ok_or(ParseError::MissingHost)?.to_owned();
    let port = url.port().ok_or(ParseError::MissingPort)?;

    let qs = query_map(&url);
    let transport = parse_transport(qs.get("type").map(String::as_str).unwrap_or("tcp"))?;
    // Trojan defaults to TLS; only an explicit `security=none` downgrades.
    let security_str = qs.get("security").map(String::as_str).unwrap_or("tls");
    let security = parse_security(security_str)?;
    let tls = build_tls(&qs, security, &host)?;

    Ok(Profile {
        name: extract_name(&url, &host, port),
        protocol: Protocol::Trojan,
        address: host,
        port,
        credential: password,
        flow: None,
        packet_encoding: None,
        alter_id: None,
        cipher: None,
        ss_method: None,
        transport,
        transport_params: transport_params_from_query(&qs),
        security,
        tls,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn implicit_tls() {
        let p = parse("trojan://hunter2@ex.com:443?type=tcp#node").unwrap();
        assert_eq!(p.protocol, Protocol::Trojan);
        assert_eq!(p.credential, "hunter2");
        assert_eq!(p.port, 443);
        assert_eq!(p.security, Security::Tls);
        assert_eq!(p.tls.unwrap().sni.as_deref(), Some("ex.com"));
    }

    #[test]
    fn ws_with_path() {
        let p = parse("trojan://p%40ss@ex.com:443?type=ws&path=%2Ftrojan&sni=front.ex#n").unwrap();
        assert_eq!(p.credential, "p@ss");
        assert_eq!(p.transport, Transport::Ws);
        assert_eq!(p.transport_params.path.as_deref(), Some("/trojan"));
        assert_eq!(p.tls.unwrap().sni.as_deref(), Some("front.ex"));
    }

    #[test]
    fn explicit_security_none() {
        let p = parse("trojan://pw@ex.com:1080?security=none&type=tcp").unwrap();
        assert_eq!(p.security, Security::None);
        assert!(p.tls.is_none());
    }
}
