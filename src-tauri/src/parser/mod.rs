pub mod common;
pub mod error;
pub mod shadowsocks;
pub mod trojan;
pub mod vless;
pub mod vmess;

pub use error::ParseError;

use crate::profile::Profile;

/// Dispatch a proxy URL to the right protocol parser by scheme.
/// Schemes are case-insensitive (RFC 3986); copy-pasted URLs occasionally
/// arrive uppercased, so normalize before dispatch.
pub fn parse_any(url: &str) -> Result<Profile, ParseError> {
    let trimmed = url.trim();
    let (scheme, rest) = trimmed
        .split_once("://")
        .ok_or_else(|| ParseError::InvalidPayload("missing `://` in URL".into()))?;
    let scheme = scheme.to_ascii_lowercase();
    let normalized = format!("{scheme}://{rest}");

    match scheme.as_str() {
        "vless" => vless::parse(&normalized),
        "vmess" => vmess::parse(&normalized),
        "trojan" => trojan::parse(&normalized),
        "ss" => shadowsocks::parse(&normalized),
        other => Err(ParseError::UnsupportedScheme(other.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::Protocol;
    use pretty_assertions::assert_eq;

    #[test]
    fn dispatch_vless() {
        let p = parse_any("vless://00000000-0000-4000-8000-000000000000@h:443?type=tcp").unwrap();
        assert_eq!(p.protocol, Protocol::Vless);
    }

    #[test]
    fn dispatch_trojan() {
        let p = parse_any("trojan://pw@h:443?type=tcp").unwrap();
        assert_eq!(p.protocol, Protocol::Trojan);
    }

    #[test]
    fn unsupported() {
        assert!(matches!(
            parse_any("hysteria2://pw@h:443"),
            Err(ParseError::UnsupportedScheme(_))
        ));
    }

    #[test]
    fn scheme_is_case_insensitive() {
        let p = parse_any("VLESS://00000000-0000-4000-8000-000000000000@h:443?type=tcp").unwrap();
        assert_eq!(p.protocol, Protocol::Vless);
        let p = parse_any("SS://aes-256-gcm:pwd@1.2.3.4:8388#n").unwrap();
        assert_eq!(p.protocol, Protocol::Shadowsocks);
    }
}
