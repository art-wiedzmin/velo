use serde::Serialize;

/// Decouples the runner from the UI framework. Anything that can receive
/// log lines, a running/stopped flag, and live stats is a valid sink.
///
/// Stats methods default to no-op so tests and non-GUI callers can ignore
/// them. Production `TauriSink` overrides them to emit events to the frontend.
pub trait EventSink: Send + Sync + 'static {
    fn log(&self, line: LogLine);
    fn state(&self, running: bool);
    fn traffic(&self, _t: &crate::core::stats::Traffic) {}
    fn connections(&self, _snapshot: &serde_json::Value) {}
}

/// No-op sink for tests and headless callers.
#[derive(Default, Clone, Copy)]
pub struct NoopSink;
impl EventSink for NoopSink {
    fn log(&self, _: LogLine) {}
    fn state(&self, _: bool) {}
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub stream: Stream,
    pub line: String,
}
