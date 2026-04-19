//! Runs `sing-box` as a child process and streams its logs.
//!
//! Lifecycle:
//!   - `start` writes the config to a temp file, spawns the binary, assigns
//!     it to a Windows Job Object (KILL_ON_JOB_CLOSE), and kicks off two
//!     tasks that forward stdout+stderr to a Tauri event.
//!   - `stop` kills the child, awaits readers, removes the temp file.
//!   - `Drop` is a best-effort fallback; the Job Object is our real safety
//!     net so a panic or hard kill can't leave an orphan.
//!
//! The runner owns its temp path and guarantees cleanup, so callers cannot
//! forget to delete the config — the type system makes the resource lifetime
//! explicit.

use super::sidecar::{self, ResolveError};
use super::sink::{EventSink, LogLine, Stream};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;

pub const LOG_EVENT: &str = "core://log";
pub const STATE_EVENT: &str = "core://state";
pub const TRAFFIC_EVENT: &str = "stats://traffic";
pub const CONNECTIONS_EVENT: &str = "stats://connections";

#[derive(Debug, Error)]
pub enum RunError {
    #[error("resolve sing-box: {0}")]
    Resolve(#[from] ResolveError),
    #[error("write config: {0}")]
    WriteConfig(#[source] std::io::Error),
    #[error("spawn sing-box: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("serialize config: {0}")]
    Serialize(#[from] serde_json::Error),
    #[cfg(windows)]
    #[error("job object: {0}")]
    Job(String),
}


pub struct Runner {
    child: Child,
    config_path: PathBuf,
    // Keep the Job Object alive: dropping it triggers KILL_ON_JOB_CLOSE and
    // takes the child with it even if we never call `stop`.
    #[cfg(windows)]
    _job: win32job::Job,
    stdout_task: Option<JoinHandle<()>>,
    stderr_task: Option<JoinHandle<()>>,
    /// Stats subscribers (traffic + connections). Aborted on `stop` / `drop`.
    stats_tasks: Vec<JoinHandle<()>>,
}

impl Runner {
    pub async fn start(
        cfg: &serde_json::Value,
        sink: Arc<dyn EventSink>,
    ) -> Result<Self, RunError> {
        let binary = sidecar::resolve()?;
        let config_path = write_temp_config(cfg)?;

        let mut cmd = Command::new(&binary);
        cmd.arg("run").arg("-c").arg(&config_path);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());

        // Windows: don't pop a console window when the GUI launches us.
        #[cfg(windows)]
        {
            // tokio::process::Command exposes `creation_flags` directly on Windows,
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = cmd.spawn().map_err(RunError::Spawn)?;

        #[cfg(windows)]
        let job = super::job::attach_to_job(&child)?;

        let stdout = child.stdout.take().map(BufReader::new);
        let stderr = child.stderr.take().map(BufReader::new);
        let stdout_task = stdout.map(|r| spawn_reader(sink.clone(), r, Stream::Stdout));
        let stderr_task = stderr.map(|r| spawn_reader(sink.clone(), r, Stream::Stderr));
        sink.state(true);

        // Best-effort: start WS subscribers to the Clash API. If the config
        // disabled it, or the port isn't reachable, the subscriber task just
        // keeps retrying until abort — no error propagation needed here.
        let stats_tasks = super::stats::spawn_from_config(cfg, sink.clone());

        Ok(Self {
            child,
            config_path,
            #[cfg(windows)]
            _job: job,
            stdout_task,
            stderr_task,
            stats_tasks,
        })
    }

    pub async fn stop(mut self, sink: Arc<dyn EventSink>) {
        use tokio::time::{timeout, Duration};
        const STOP_BUDGET: Duration = Duration::from_secs(3);

        // Abort stats WS tasks first so they don't spam reconnect attempts
        // while the child is being torn down.
        for h in self.stats_tasks.drain(..) {
            h.abort();
        }

        // TerminateProcess is abrupt but safe for a stateless proxy. Graceful
        // shutdown would require signals sing-box doesn't consume portably on
        // Windows. TUN mode can make the process hang briefly while WinTUN
        // unwinds routes — hence the timeout fences.
        let _ = self.child.kill().await;
        let _ = timeout(STOP_BUDGET, self.child.wait()).await;

        // Reader tasks normally exit when the pipe closes (EOF). If they
        // don't (hung I/O, buffered data), abort — the Windows Job Object
        // will reap any stragglers when we drop.
        if let Some(h) = self.stdout_task.take() {
            let _ = timeout(Duration::from_millis(500), h).await;
        }
        if let Some(h) = self.stderr_task.take() {
            let _ = timeout(Duration::from_millis(500), h).await;
        }

        let _ = std::fs::remove_file(&self.config_path);
        sink.state(false);
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        // Best-effort only. Real safety comes from the Windows Job Object,
        // which kills the child when our process (or the job handle) goes
        // away, so a panicking drop can't orphan sing-box.
        for h in self.stats_tasks.drain(..) {
            h.abort();
        }
        let _ = self.child.start_kill();
        let _ = std::fs::remove_file(&self.config_path);
    }
}

fn spawn_reader<Io>(sink: Arc<dyn EventSink>, mut reader: BufReader<Io>, stream: Stream) -> JoinHandle<()>
where
    Io: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut buf = String::new();
        loop {
            buf.clear();
            match reader.read_line(&mut buf).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let line = buf.trim_end_matches(['\r', '\n']).to_owned();
                    if line.is_empty() {
                        continue;
                    }
                    sink.log(LogLine {
                        stream: stream.clone(),
                        line,
                    });
                }
                Err(_) => break,
            }
        }
    })
}

fn write_temp_config(cfg: &serde_json::Value) -> Result<PathBuf, RunError> {
    use std::io::Write;
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut p = std::env::temp_dir();
    p.push(format!("velo-singbox-{pid}-{nanos}.json"));
    let mut f = std::fs::File::create(&p).map_err(RunError::WriteConfig)?;
    serde_json::to_writer(&mut f, cfg)?;
    f.flush().map_err(RunError::WriteConfig)?;
    Ok(p)
}

