use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("unsupported scheme: {0}")]
    UnsupportedScheme(String),
    #[error("scheme mismatch: expected `{expected}`, got `{got}`")]
    SchemeMismatch { expected: &'static str, got: String },
    #[error("missing credential (userinfo)")]
    MissingCredential,
    #[error("missing host")]
    MissingHost,
    #[error("missing port")]
    MissingPort,
    #[error("invalid UUID: {0}")]
    InvalidUuid(String),
    #[error("unsupported transport: {0}")]
    UnsupportedTransport(String),
    #[error("unsupported security: {0}")]
    UnsupportedSecurity(String),
    #[error("reality requires `pbk` and `sid`")]
    RealityMissingFields,
    #[error("unsupported flow: {0}")]
    UnsupportedFlow(String),
    #[error("invalid base64: {0}")]
    InvalidBase64(String),
    #[error("invalid payload: {0}")]
    InvalidPayload(String),
    #[error("invalid integer: {0}")]
    InvalidInt(String),
}
