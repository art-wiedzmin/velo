pub mod common;
pub mod error;
pub mod shadowsocks;
pub mod trojan;
pub mod vless;
pub mod vmess;

pub use error::ParseError;

use crate::profile::Profile;

/// Dispatch a proxy URL to the right protocol parser by scheme.
pub fn parse_any(url: &str) -> Result<Profile, ParseError> {
    let trimmed = url.trim();
    let scheme = trimmed
        .split_once("://")
        .map(|(s, _)| s)
        .ok_or_else(|| ParseError::InvalidPayload("missing `://` in URL".into()))?;

    match scheme {
        "vless" => vless::parse(trimmed),
        "vmess" => vmess::parse(trimmed),
        "trojan" => trojan::parse(trimmed),
        "ss" => shadowsocks::parse(trimmed),
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
}
