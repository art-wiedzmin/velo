#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Local mixed (SOCKS5+HTTP) only. App-level routing happens via the
    /// Windows system proxy.
    Sysproxy,
    /// TUN interface (+ the mixed inbound for compatibility). Process-level
    /// whitelist/blacklist routing becomes available in this mode.
    Tun,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Sysproxy
    }
}

/// Whitelist: only listed apps go via proxy, everything else is direct.
/// Blacklist: everything goes via proxy, listed apps bypass to direct.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RoutingMode {
    #[default]
    None,
    Whitelist,
    Blacklist,
}

#[derive(Debug, Clone)]
pub struct Options {
    /// Mixed inbound (SOCKS5 + HTTP on one port). 0 = disabled.
    pub mixed_port: u16,
    /// Bind address for local inbounds. `127.0.0.1` unless LAN sharing is on.
    pub listen: String,
    /// sing-box log level.
    pub log_level: String,
    /// Port for the Clash-compatible RESTful/WS stats API. 0 = disabled.
    /// Bound to `listen`; loopback keeps the API unauthenticated safely.
    pub clash_api_port: u16,
    /// Traffic mode — selects whether we emit a TUN inbound.
    pub mode: Mode,
    /// Routing rules. Ignored unless `mode == Mode::Tun`.
    pub routing_mode: RoutingMode,
    /// Absolute `.exe` paths that the routing rules apply to.
    pub routing_apps: Vec<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            mixed_port: 10808,
            listen: "127.0.0.1".into(),
            log_level: "debug".into(),
            clash_api_port: 9090,
            mode: Mode::default(),
            routing_mode: RoutingMode::default(),
            routing_apps: Vec::new(),
        }
    }
}

pub(super) const TAG_PROXY: &str = "proxy";
pub(super) const TAG_DIRECT: &str = "direct";
