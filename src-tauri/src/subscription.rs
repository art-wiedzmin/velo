//! Subscription loader.
//!
//! Subscription endpoints return one of:
//!   1. base64-encoded text body, decoded to newline-separated proxy URLs
//!   2. raw newline-separated proxy URLs (plain text)
//!
//! We detect base64 by trying to decode the whole body; if it succeeds *and*
//! the result looks like URLs, we use it. Otherwise we treat the body as raw.
//!
//! Quota/expiry metadata arrives as a `Subscription-Userinfo` HTTP header on
//! the response. Format (widely followed in the V2Ray/VLESS ecosystem):
//!   `upload=N; download=N; total=N; expire=UNIX_SECS`
//! Any field may be absent; we tolerate missing or malformed pieces.

use crate::parser::{self, ParseError};
use crate::profile::Profile;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Upper bound on a subscription body. Real subscriptions are well under
/// 1 MB; the cap exists so a malicious or broken server can't balloon the
/// (elevated) velo process with an unbounded download.
const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("empty response body")]
    Empty,
    #[error("response body exceeds {MAX_BODY_BYTES} bytes")]
    TooLarge,
}

#[derive(Debug, Serialize, Clone)]
pub struct LineError {
    pub line: usize,
    pub input: String,
    pub error: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct SubscriptionResult {
    pub profiles: Vec<Profile>,
    pub errors: Vec<LineError>,
    /// True if the body was recognized and decoded as base64.
    pub decoded_base64: bool,
}

/// Subscription quota info parsed from the provider's `Subscription-Userinfo`
/// HTTP header. Any field may be missing depending on the provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubscriptionQuota {
    pub used_bytes: Option<i64>,
    pub total_bytes: Option<i64>,
    pub expires_at: Option<i64>,
}

/// Full result returned by [`fetch`]: parsed profiles + provider-supplied
/// quota/expiry metadata (when the `Subscription-Userinfo` header is present).
#[derive(Debug, Clone)]
pub struct FetchedSubscription {
    pub result: SubscriptionResult,
    pub quota: SubscriptionQuota,
}

/// Fetch a subscription URL and return parsed profiles + per-line errors +
/// quota metadata.
pub async fn fetch(url: &str) -> Result<FetchedSubscription, FetchError> {
    let client = reqwest::Client::builder()
        .user_agent("velo/0.1 (subscription)")
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        // Subscription URLs embed the user's secret token; following an
        // https→http redirect would replay that token in cleartext.
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let downgrade = attempt.url().scheme() == "http"
                && attempt
                    .previous()
                    .last()
                    .is_some_and(|u| u.scheme() == "https");
            if downgrade {
                attempt.error("refusing https→http redirect downgrade")
            } else if attempt.previous().len() > 10 {
                attempt.error("too many redirects")
            } else {
                attempt.follow()
            }
        }))
        .build()?;
    let mut resp = client.get(url).send().await?.error_for_status()?;
    if resp.content_length().is_some_and(|l| l > MAX_BODY_BYTES as u64) {
        return Err(FetchError::TooLarge);
    }

    // Extract the header before consuming the body — reading chunks
    // takes ownership and would drop headers with the response.
    let quota = resp
        .headers()
        .get("subscription-userinfo")
        .and_then(|v| v.to_str().ok())
        .map(parse_userinfo)
        .unwrap_or_default();

    // Chunked read with a running cap: Content-Length is advisory (absent
    // on chunked transfer), so the limit must be enforced on actual bytes.
    let mut raw: Vec<u8> = Vec::new();
    while let Some(chunk) = resp.chunk().await? {
        if raw.len() + chunk.len() > MAX_BODY_BYTES {
            return Err(FetchError::TooLarge);
        }
        raw.extend_from_slice(&chunk);
    }
    let body = String::from_utf8_lossy(&raw);
    if body.trim().is_empty() {
        return Err(FetchError::Empty);
    }
    Ok(FetchedSubscription {
        result: parse_body(&body),
        quota,
    })
}

/// Parse a subscription body. Public for testing; production callers use
/// [`fetch`] which wraps HTTP + this.
pub fn parse_body(body: &str) -> SubscriptionResult {
    let (text, decoded_base64) = match try_decode_base64(body) {
        Some(decoded) => (decoded, true),
        None => (body.to_owned(), false),
    };

    let mut profiles = Vec::new();
    let mut errors = Vec::new();
    for (idx, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match parser::parse_any(line) {
            Ok(p) => profiles.push(p),
            Err(e) => errors.push(LineError {
                line: idx + 1,
                input: truncate(line, 80),
                error: classify(&e),
            }),
        }
    }

    SubscriptionResult {
        profiles,
        errors,
        decoded_base64,
    }
}

/// Parse the provider's `Subscription-Userinfo` header. Returns a fully-
/// populated [`SubscriptionQuota`] — missing or malformed fields remain
/// `None` so upstream can fall back to its cached value.
///
/// Spec by convention: `upload=…; download=…; total=…; expire=…` with
/// sizes in bytes and expire in unix seconds. Some providers emit
/// commas instead of semicolons — we accept both.
pub fn parse_userinfo(raw: &str) -> SubscriptionQuota {
    let mut upload: Option<i64> = None;
    let mut download: Option<i64> = None;
    let mut total: Option<i64> = None;
    let mut expire: Option<i64> = None;

    for piece in raw.split([';', ',']) {
        let piece = piece.trim();
        let Some((k, v)) = piece.split_once('=') else { continue };
        let v = v.trim().parse::<i64>().ok();
        match k.trim().to_ascii_lowercase().as_str() {
            "upload" => upload = v,
            "download" => download = v,
            "total" => total = v,
            "expire" => expire = v,
            _ => {}
        }
    }

    let used = match (upload, download) {
        (Some(u), Some(d)) => Some(u.saturating_add(d)),
        (Some(u), None) => Some(u),
        (None, Some(d)) => Some(d),
        (None, None) => None,
    };

    SubscriptionQuota {
        used_bytes: used,
        // `total = 0` is the convention for "unlimited"; treat it as None
        // so the UI shows ∞ instead of a completed progress bar.
        total_bytes: total.filter(|&t| t > 0),
        expires_at: expire.filter(|&e| e > 0),
    }
}

fn try_decode_base64(body: &str) -> Option<String> {
    let decoded = parser::common::base64_decode_loose(body).ok()?;
    let text = String::from_utf8(decoded).ok()?;
    // Guard: require at least one recognizable scheme so we don't misread
    // plain text that just happens to be valid base64 by accident.
    if text
        .lines()
        .any(|l| matches!(l.trim_start().split("://").next(), Some("vless" | "vmess" | "trojan" | "ss")))
    {
        Some(text)
    } else {
        None
    }
}

fn classify(e: &ParseError) -> String {
    e.to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use pretty_assertions::assert_eq;

    #[test]
    fn raw_plaintext_body() {
        let body = "\
vless://00000000-0000-4000-8000-000000000000@h.example:443?type=tcp#one
trojan://pw@t.example:443?type=tcp#two
# comment line
not-a-url
";
        let r = parse_body(body);
        assert_eq!(r.decoded_base64, false);
        assert_eq!(r.profiles.len(), 2);
        assert_eq!(r.profiles[0].name, "one");
        assert_eq!(r.profiles[1].name, "two");
        assert_eq!(r.errors.len(), 1);
    }

    #[test]
    fn base64_encoded_body() {
        let inner = "vless://00000000-0000-4000-8000-000000000000@h.example:443?type=tcp#one\n";
        let encoded = base64::engine::general_purpose::STANDARD.encode(inner.as_bytes());
        let r = parse_body(&encoded);
        assert_eq!(r.decoded_base64, true);
        assert_eq!(r.profiles.len(), 1);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn base64_that_doesnt_decode_to_urls_stays_raw() {
        // Valid base64 but inner is not proxy URLs — must be treated as raw.
        let inner = base64::engine::general_purpose::STANDARD.encode(b"hello\nworld\n");
        let r = parse_body(&inner);
        assert_eq!(r.decoded_base64, false);
        assert!(r.profiles.is_empty());
    }

    #[test]
    fn userinfo_full_standard_form() {
        let q = parse_userinfo("upload=1000; download=2000; total=10000; expire=1735689600");
        assert_eq!(q.used_bytes, Some(3000));
        assert_eq!(q.total_bytes, Some(10000));
        assert_eq!(q.expires_at, Some(1735689600));
    }

    #[test]
    fn userinfo_total_zero_means_unlimited() {
        let q = parse_userinfo("upload=5; download=10; total=0");
        assert_eq!(q.used_bytes, Some(15));
        assert_eq!(q.total_bytes, None, "total=0 must be treated as unlimited");
    }

    #[test]
    fn userinfo_tolerates_missing_and_malformed_fields() {
        let q = parse_userinfo("download=42; total=abc; garbage");
        assert_eq!(q.used_bytes, Some(42));
        assert_eq!(q.total_bytes, None);
        assert_eq!(q.expires_at, None);
    }

    #[test]
    fn userinfo_comma_separator() {
        let q = parse_userinfo("upload=1, download=2, total=100");
        assert_eq!(q.used_bytes, Some(3));
        assert_eq!(q.total_bytes, Some(100));
    }

    #[test]
    fn userinfo_empty_returns_all_none() {
        let q = parse_userinfo("");
        assert!(q.used_bytes.is_none());
        assert!(q.total_bytes.is_none());
        assert!(q.expires_at.is_none());
    }
}
