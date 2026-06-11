//! Validates generated config by running the real `sing-box check`.
//! Finds the binary via VELO_SINGBOX env var; skips if missing. This
//! guarantees the config we hand the core is accepted by the current
//! release, not just what our unit tests *think* is correct.

use std::io::Write;
use std::process::Command;
use velo_lib::config::singbox::{self, Options};
use velo_lib::parser;

fn singbox_path() -> Option<String> {
    // Prefer an explicit env override; fall back to repo-local tools/ dir.
    if let Ok(p) = std::env::var("VELO_SINGBOX") {
        if !p.is_empty() {
            return Some(p);
        }
    }
    let candidate = concat!(env!("CARGO_MANIFEST_DIR"), "/../tools/sing-box.exe");
    std::path::Path::new(candidate)
        .exists()
        .then(|| candidate.to_string())
}

fn validate(label: &str, url: &str) {
    validate_with(label, url, &Options::default());
}

fn validate_with(label: &str, url: &str, opts: &Options) {
    let Some(sb) = singbox_path() else {
        eprintln!("[{label}] skipped: no sing-box binary (set VELO_SINGBOX or place at ../tools/)");
        return;
    };
    let profile = parser::parse_any(url).expect("parse");
    let cfg = singbox::build(&profile, opts);
    let json = serde_json::to_string_pretty(&cfg).unwrap();

    let mut tmp = tempfile_like(label);
    tmp.write_all(json.as_bytes()).unwrap();
    let path = tmp.path().clone();
    drop(tmp);

    let out = Command::new(&sb)
        .args(["check", "-c"])
        .arg(&path)
        .output()
        .expect("run sing-box");
    let _ = std::fs::remove_file(&path);

    if !out.status.success() {
        panic!(
            "[{label}] sing-box check failed (status {:?}):\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- config ---\n{}",
            out.status.code(),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
            json
        );
    }
    eprintln!("[{label}] sing-box check: OK");
}

// Minimal hand-rolled temp file — avoids pulling tempfile as a dep.
struct TmpFile {
    path: std::path::PathBuf,
    file: std::fs::File,
}
impl TmpFile {
    fn path(&self) -> std::path::PathBuf {
        self.path.clone()
    }
}
impl Write for TmpFile {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.file.write(b)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}
fn tempfile_like(tag: &str) -> TmpFile {
    let mut p = std::env::temp_dir();
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    p.push(format!("velo-sb-{tag}-{pid}-{nanos}.json"));
    let file = std::fs::File::create(&p).unwrap();
    TmpFile { path: p, file }
}

#[test]
fn reality_vision_tcp() {
    let url = "vless://00000000-0000-4000-8000-000000000000@example.invalid:443\
               ?security=reality&sni=example.invalid&fp=chrome\
               &pbk=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\
               &sid=0123456789abcdef&type=tcp&flow=xtls-rprx-vision\
               &packetEncoding=xudp&encryption=none#node";
    validate("reality_vision", url);
}

#[test]
fn vmess_ws_tls() {
    use base64::Engine;
    let inner = r#"{"add":"ex.com","port":"443","id":"00000000-0000-4000-8000-000000000000","aid":"0","net":"ws","tls":"tls","path":"/ray","host":"front.ex","sni":"ex.com","scy":"auto","ps":"n"}"#;
    let url = format!("vmess://{}", base64::engine::general_purpose::STANDARD.encode(inner));
    validate("vmess_ws", &url);
}

#[test]
fn trojan_tcp_tls() {
    validate("trojan", "trojan://pw@ex.com:443?type=tcp#n");
}

// TUN-mode config with process routing exercises the tun inbound and the
// route-rule shapes (sniff action, process_name, default_domain_resolver) —
// none of which appear in the sysproxy-mode cases above. `check` only
// parses; it does not create the interface, so no admin needed.
#[test]
fn tun_whitelist_routing() {
    use velo_lib::config::singbox::{Mode, RoutingMode};
    validate_with(
        "tun_whitelist",
        "trojan://pw@ex.com:443?type=tcp#n",
        &Options {
            mode: Mode::Tun,
            routing_mode: RoutingMode::Whitelist,
            routing_apps: vec![r"C:\Apps\browser.exe".into()],
            ..Options::default()
        },
    );
}

// `sing-box check` only parses; service wiring runs a second class of
// validations at START (e.g. 1.13 fatals on a DNS server detouring to a
// bare direct outbound — which check accepted and v1.1.0 shipped). Actually
// run the sidecar on an ephemeral port and require the "started" banner.
// Sysproxy-mode only: TUN needs admin.
#[test]
fn run_smoke_sysproxy() {
    use std::io::Read;
    let Some(sb) = singbox_path() else {
        eprintln!("[run_smoke] skipped: no sing-box binary");
        return;
    };
    let port = free_port();
    let profile = parser::parse_any(
        "vless://00000000-0000-4000-8000-000000000000@example.invalid:443?security=tls&type=tcp#n",
    )
    .expect("parse");
    let cfg = singbox::build(
        &profile,
        &Options { mixed_port: port, clash_api_port: 0, ..Options::default() },
    );
    let json = serde_json::to_string_pretty(&cfg).unwrap();
    let mut tmp = tempfile_like("run_smoke");
    tmp.write_all(json.as_bytes()).unwrap();
    let path = tmp.path().clone();
    drop(tmp);

    let mut child = Command::new(&sb)
        .args(["run", "-c"])
        .arg(&path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn sing-box run");

    // sing-box prints the banner within milliseconds; FATAL exits even
    // faster. Poll up to 5s: exit before the deadline = startup failure.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut exited = None;
    while std::time::Instant::now() < deadline {
        if let Some(status) = child.try_wait().expect("try_wait") {
            exited = Some(status);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let verdict = match exited {
        None => {
            let _ = child.kill();
            let _ = child.wait();
            Ok(())
        }
        Some(status) => {
            let mut err = String::new();
            if let Some(mut s) = child.stderr.take() {
                let _ = s.read_to_string(&mut err);
            }
            let mut out = String::new();
            if let Some(mut s) = child.stdout.take() {
                let _ = s.read_to_string(&mut out);
            }
            Err(format!(
                "sing-box run exited at startup (status {:?})\n--- stdout ---\n{out}\n--- stderr ---\n{err}\n--- config ---\n{json}",
                status.code(),
            ))
        }
    };
    let _ = std::fs::remove_file(&path);
    if let Err(msg) = verdict {
        panic!("[run_smoke] {msg}");
    }
    eprintln!("[run_smoke] sing-box run: started and stayed up, OK");
}

fn free_port() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    l.local_addr().expect("local_addr").port()
}
