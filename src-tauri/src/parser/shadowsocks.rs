//! Shadowsocks URL parser (SIP002 + legacy).
//!
//! Two encodings in the wild:
//!   1. SIP002: ss://<base64url(method:password)>@host:port[/?plugin=…]#name
//!   2. Legacy: ss://<base64(method:password@host:port)>#name
//!
//! We detect by structure: if `@` is present after stripping the scheme
//! prefix, it's SIP002; otherwise we assume legacy and decode the whole
//! payload. Plugin parameters are preserved in `transport_params` by key.

use super::common::*;
use super::error::ParseError;
use crate::profile::*;
use percent_encoding::percent_decode_str;
use url::Url;

const PREFIX: &str = "ss://";

pub fn parse(input: &str) -> Result<Profile, ParseError> {
    let input = input.trim();
    let payload = input.strip_prefix(PREFIX).ok_or_else(|| ParseError::SchemeMismatch {
        expected: "ss",
        got: input.split("://").next().unwrap_or("").to_owned(),
    })?;

    // SIP002: userinfo is `base64(method:password)`, then @host:port and
    // optional query/fragment just like a regular URL.
    if let Some(at_idx) = payload.find('@') {
        let (userinfo_raw, rest) = payload.split_at(at_idx);
        let userinfo = percent_decode_str(userinfo_raw)
            .decode_utf8_lossy()
            .into_owned();
        let (method, password) = split_method_password(&userinfo)?;

        let reconstructed = format!("ss://ignore@{}", &rest[1..]);
        let url = Url::parse(&reconstructed)?;
        let host = url.host_str().ok_or(ParseError::MissingHost)?.to_owned();
        let port = url.port().ok_or(ParseError::MissingPort)?;

        return Ok(Profile {
            name: extract_name(&url, &host, port),
            protocol: Protocol::Shadowsocks,
            address: host,
            port,
            credential: password,
            flow: None,
            packet_encoding: None,
            alter_id: None,
            cipher: None,
            ss_method: Some(method),
            transport: Transport::Tcp,
            transport_params: TransportParams::default(),
            security: Security::None,
            tls: None,
        });
    }

    // Legacy: base64 of `method:password@host:port`. Fragment with `#name`
    // may ride outside the base64 blob.
    let (b64_part, name_opt) = match payload.split_once('#') {
        Some((b, n)) => (b, Some(percent_decode_str(n).decode_utf8_lossy().into_owned())),
        None => (payload, None),
    };
    let decoded = base64_decode_loose(b64_part)?;
    let decoded = String::from_utf8(decoded)
        .map_err(|e| ParseError::InvalidPayload(format!("ss legacy utf8: {e}")))?;

    let at = decoded.rfind('@').ok_or_else(|| {
        ParseError::InvalidPayload("legacy ss: expected `method:password@host:port`".into())
    })?;
    let (creds, hostport) = decoded.split_at(at);
    let hostport = &hostport[1..];
    let (method, password) = split_method_password(creds)?;

    let colon = hostport.rfind(':').ok_or(ParseError::MissingPort)?;
    let host = hostport[..colon].trim_matches(&['[', ']'][..]).to_owned();
    if host.is_empty() {
        return Err(ParseError::MissingHost);
    }
    let port: u16 = hostport[colon + 1..]
        .parse()
        .map_err(|e: std::num::ParseIntError| ParseError::InvalidInt(e.to_string()))?;

    Ok(Profile {
        name: name_opt.unwrap_or_else(|| format!("{host}:{port}")),
        protocol: Protocol::Shadowsocks,
        address: host,
        port,
        credential: password,
        flow: None,
        packet_encoding: None,
        alter_id: None,
        cipher: None,
        ss_method: Some(method),
        transport: Transport::Tcp,
        transport_params: TransportParams::default(),
        security: Security::None,
        tls: None,
    })
}

fn split_method_password(raw: &str) -> Result<(String, String), ParseError> {
    // Prefer literal `method:password` when present; fall back to base64.
    if let Some((m, p)) = raw.split_once(':') {
        if !m.is_empty() && !p.is_empty() {
            return Ok((m.to_owned(), p.to_owned()));
        }
    }
    let decoded = base64_decode_loose(raw)?;
    let s = String::from_utf8(decoded)
        .map_err(|e| ParseError::InvalidPayload(format!("ss userinfo utf8: {e}")))?;
    s.split_once(':')
        .map(|(m, p)| (m.to_owned(), p.to_owned()))
        .ok_or_else(|| ParseError::InvalidPayload("ss userinfo: expected `method:password`".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use pretty_assertions::assert_eq;

    #[test]
    fn sip002_plain_userinfo() {
        // Some servers send method:password raw in userinfo (not base64).
        let p = parse("ss://aes-256-gcm:pwd@1.2.3.4:8388#name").unwrap();
        assert_eq!(p.protocol, Protocol::Shadowsocks);
        assert_eq!(p.ss_method.as_deref(), Some("aes-256-gcm"));
        assert_eq!(p.credential, "pwd");
        assert_eq!(p.port, 8388);
        assert_eq!(p.name, "name");
    }

    #[test]
    fn sip002_base64_userinfo() {
        let userinfo = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode("chacha20-ietf-poly1305:p@ss".as_bytes());
        let url = format!("ss://{userinfo}@host.example:443#tag");
        let p = parse(&url).unwrap();
        assert_eq!(p.ss_method.as_deref(), Some("chacha20-ietf-poly1305"));
        assert_eq!(p.credential, "p@ss");
        assert_eq!(p.address, "host.example");
    }

    #[test]
    fn legacy_whole_blob() {
        let payload = base64::engine::general_purpose::STANDARD
            .encode("aes-128-gcm:secret@10.0.0.1:8388".as_bytes());
        let url = format!("ss://{payload}#legacy");
        let p = parse(&url).unwrap();
        assert_eq!(p.ss_method.as_deref(), Some("aes-128-gcm"));
        assert_eq!(p.credential, "secret");
        assert_eq!(p.address, "10.0.0.1");
        assert_eq!(p.port, 8388);
        assert_eq!(p.name, "legacy");
    }
}
