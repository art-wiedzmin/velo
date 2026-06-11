//! Helpers shared by vless/vmess/trojan/ss URL parsers.
//!
//! These encode the *de facto* encoding conventions used by Xray / sing-box /
//! v2rayN — not an RFC. When servers disagree, we side with what the major
//! clients accept.

use super::error::ParseError;
use crate::profile::*;
use percent_encoding::percent_decode_str;
use std::collections::HashMap;
use url::Url;

/// Decode URL fragment as the human-readable profile name. Falls back to
/// `host:port` so every profile has a stable label — including the trailing
/// `#` case where the fragment is present but empty.
pub fn extract_name(url: &Url, host: &str, port: u16) -> String {
    url.fragment()
        .map(|f| percent_decode_str(f).decode_utf8_lossy().into_owned())
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| format!("{host}:{port}"))
}

/// Host with IPv6 brackets stripped. `Url::host_str` keeps the brackets
/// (`[2001:db8::1]`), which sing-box rejects as a `server` value and which
/// `IpAddr::parse` doesn't recognize — so the SNI fallback would treat the
/// address as a hostname.
pub fn extract_host(url: &Url) -> Result<String, ParseError> {
    match url.host().ok_or(ParseError::MissingHost)? {
        url::Host::Ipv6(a) => Ok(a.to_string()),
        _ => Ok(url.host_str().ok_or(ParseError::MissingHost)?.to_owned()),
    }
}

/// Shared `scheme://credential@host:port` skeleton for vless/trojan.
/// Returns the parsed URL plus percent-decoded credential and normalized
/// host/port.
pub fn parse_authority(
    input: &str,
    scheme: &'static str,
) -> Result<(Url, String, String, u16), ParseError> {
    let url = Url::parse(input.trim())?;
    if url.scheme() != scheme {
        return Err(ParseError::SchemeMismatch {
            expected: scheme,
            got: url.scheme().to_owned(),
        });
    }
    let raw = url.username();
    if raw.is_empty() {
        return Err(ParseError::MissingCredential);
    }
    let credential = percent_decode_str(raw).decode_utf8_lossy().into_owned();
    let host = extract_host(&url)?;
    let port = url.port().ok_or(ParseError::MissingPort)?;
    Ok((url, credential, host, port))
}

pub fn query_map(url: &Url) -> HashMap<String, String> {
    url.query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}

pub fn parse_transport(s: &str) -> Result<Transport, ParseError> {
    // `xhttp`/`splithttp` deliberately rejected: sing-box has no xhttp
    // transport, so accepting the URL would defer the failure to connect
    // time with an opaque sidecar error instead of a per-line import error.
    Ok(match s {
        "" | "tcp" | "raw" => Transport::Tcp,
        "ws" => Transport::Ws,
        "grpc" => Transport::Grpc,
        "h2" | "http" => Transport::H2,
        "httpupgrade" => Transport::Httpupgrade,
        other => return Err(ParseError::UnsupportedTransport(other.to_owned())),
    })
}

pub fn parse_security(s: &str) -> Result<Security, ParseError> {
    Ok(match s {
        "" | "none" => Security::None,
        "tls" => Security::Tls,
        "reality" => Security::Reality,
        other => return Err(ParseError::UnsupportedSecurity(other.to_owned())),
    })
}

pub fn transport_params_from_query(qs: &HashMap<String, String>) -> TransportParams {
    TransportParams {
        path: qs.get("path").cloned(),
        host: qs.get("host").cloned(),
        service_name: qs.get("serviceName").cloned(),
        mode: qs.get("mode").cloned(),
    }
}

/// Build `TlsParams` from a query map. `host` is the outbound server host,
/// used *only* as SNI fallback for hostname hosts (never for raw IPs — that
/// would be a misconfiguration we'd rather surface than mask).
pub fn build_tls(
    qs: &HashMap<String, String>,
    security: Security,
    host: &str,
) -> Result<Option<TlsParams>, ParseError> {
    if security == Security::None {
        return Ok(None);
    }

    let sni = qs
        .get("sni")
        .filter(|s| !s.is_empty())
        .cloned()
        .or_else(|| {
            if host.parse::<std::net::IpAddr>().is_err() {
                Some(host.to_owned())
            } else {
                None
            }
        });

    let alpn = qs
        .get("alpn")
        .map(|s| {
            s.split(',')
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();

    let fingerprint = qs
        .get("fp")
        .filter(|s| !s.is_empty())
        .cloned()
        .map(Fingerprint);

    let allow_insecure = qs
        .get("allowInsecure")
        .map(|s| matches!(s.as_str(), "1" | "true"))
        .unwrap_or(false);

    let reality = if security == Security::Reality {
        let public_key = qs
            .get("pbk")
            .filter(|s| !s.is_empty())
            .cloned()
            .ok_or(ParseError::RealityMissingFields)?;
        // `sid` is optional in the wild (sing-box accepts an empty short_id);
        // requiring it rejected real provider URLs.
        let short_id = qs.get("sid").cloned().unwrap_or_default();
        Some(RealityParams {
            public_key,
            short_id,
        })
    } else {
        None
    };

    Ok(Some(TlsParams {
        sni,
        alpn,
        fingerprint,
        allow_insecure,
        reality,
    }))
}

/// Permissive base64 decode. Accepts both standard and URL-safe alphabets,
/// with or without padding. Subscription bodies and vmess:// payloads are
/// sloppy about this in the wild.
pub fn base64_decode_loose(input: &str) -> Result<Vec<u8>, ParseError> {
    use base64::Engine;
    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    let engines = [
        base64::engine::general_purpose::STANDARD,
        base64::engine::general_purpose::STANDARD_NO_PAD,
        base64::engine::general_purpose::URL_SAFE,
        base64::engine::general_purpose::URL_SAFE_NO_PAD,
    ];
    for eng in engines {
        if let Ok(v) = eng.decode(&cleaned) {
            return Ok(v);
        }
    }
    Err(ParseError::InvalidBase64(
        cleaned.chars().take(32).collect::<String>() + "…",
    ))
}
