pub mod runner;
pub mod sidecar;
pub mod stats;
mod sink;
#[cfg(windows)]
mod job;

use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::Mutex;

pub use runner::{Runner, CONNECTIONS_EVENT, LOG_EVENT, STATE_EVENT, TRAFFIC_EVENT};
pub use sink::{EventSink, LogLine, NoopSink, Stream};

#[derive(Default)]
pub struct CoreState {
    pub inner: Mutex<Option<Runner>>,
}

/// Bridges the runtime-agnostic `EventSink` to a real Tauri AppHandle. Kept
/// generic over `Runtime` so test code can use `MockRuntime` and the GUI
/// uses `Wry` — the runner doesn't care which.
pub struct TauriSink<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> EventSink for TauriSink<R> {
    fn log(&self, line: LogLine) {
        let _ = self.0.emit(LOG_EVENT, line);
    }
    fn state(&self, running: bool) {
        let _ = self.0.emit(STATE_EVENT, serde_json::json!({ "running": running }));
    }
    fn traffic(&self, t: &stats::Traffic) {
        let _ = self.0.emit(TRAFFIC_EVENT, t);
    }
    fn connections(&self, snapshot: &serde_json::Value) {
        let _ = self.0.emit(CONNECTIONS_EVENT, snapshot);
    }
}

pub fn tauri_sink<R: Runtime>(app: AppHandle<R>) -> Arc<dyn EventSink> {
    Arc::new(TauriSink(app))
}

/// Like [`tauri_sink`] but also tees every log line to a file. Used in
/// production so we have a post-hoc record of sing-box output — the UI
/// buffer is capped at 1000 lines and drops anything older. On file-open
/// failure we silently fall back to the UI-only sink; missing logs are
/// less harmful than a crash on app start when app_data_dir is locked.
pub struct FileTeeSink<R: Runtime> {
    inner: TauriSink<R>,
    file: std::sync::Mutex<Option<std::fs::File>>,
}

impl<R: Runtime> EventSink for FileTeeSink<R> {
    fn log(&self, line: LogLine) {
        use std::io::Write;
        if let Ok(mut guard) = self.file.lock() {
            if let Some(f) = guard.as_mut() {
                let prefix = match line.stream {
                    Stream::Stdout => "stdout",
                    Stream::Stderr => "stderr",
                };
                let _ = writeln!(f, "{prefix}: {}", line.line);
                let _ = f.flush();
            }
        }
        self.inner.log(line);
    }
    fn state(&self, running: bool) {
        self.inner.state(running);
    }
    fn traffic(&self, t: &stats::Traffic) {
        self.inner.traffic(t);
    }
    fn connections(&self, snapshot: &serde_json::Value) {
        self.inner.connections(snapshot);
    }
}

pub fn tauri_sink_with_file<R: Runtime>(
    app: AppHandle<R>,
    log_path: Option<std::path::PathBuf>,
) -> Arc<dyn EventSink> {
    let file = log_path.and_then(|p| {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Truncate on each run so we see only the current session's log.
        // Historical sessions rarely help and the file would grow unbounded.
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&p)
            .ok()
    });
    Arc::new(FileTeeSink {
        inner: TauriSink(app),
        file: std::sync::Mutex::new(file),
    })
}
