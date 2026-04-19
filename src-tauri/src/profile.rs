//! Profile model: a single outbound endpoint the user can connect to.
//!
//! The struct is flat and protocol-wide: fields that don't apply to the
//! current protocol stay `None`. This mirrors how sing-box / v2rayN represent
//! profiles internally and makes round-tripping imported URLs lossless.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Vless,
    Vmess,
    Trojan,
    Shadowsocks,
    Hysteria2,
    Tuic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    Tcp,
    Ws,
    Grpc,
    H2,
    Httpupgrade,
    Xhttp,
}

/// TLS/Reality layer. `None` = plain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Security {
    None,
    Tls,
    Reality,
}

/// VLESS flow. Only Vision is live in 2024+.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Flow {
    XtlsRprxVision,
}

/// uTLS fingerprint identifier. Mirrors sing-box `tls.utls.fingerprint`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TlsParams {
    pub sni: Option<String>,
    pub alpn: Vec<String>,
    pub fingerprint: Option<Fingerprint>,
    pub allow_insecure: bool,
    pub reality: Option<RealityParams>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealityParams {
    pub public_key: String,
    pub short_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TransportParams {
    pub path: Option<String>,
    pub host: Option<String>,
    pub service_name: Option<String>, // gRPC
    pub mode: Option<String>,         // gRPC / xhttp mode
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub protocol: Protocol,
    pub address: String,
    pub port: u16,

    /// UUID for vless/vmess; password for trojan/shadowsocks.
    pub credential: String,

    // --- VLESS-specific ---
    pub flow: Option<Flow>,
    pub packet_encoding: Option<String>,

    // --- VMess-specific ---
    pub alter_id: Option<u32>,
    /// VMess cipher ("auto", "aes-128-gcm", "chacha20-poly1305", "none", "zero").
    pub cipher: Option<String>,

    // --- Shadowsocks-specific ---
    pub ss_method: Option<String>,

    // --- Common transport + security ---
    pub transport: Transport,
    pub transport_params: TransportParams,
    pub security: Security,
    pub tls: Option<TlsParams>,
}
