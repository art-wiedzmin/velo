// Mirrors Rust types in src-tauri/src/profile.rs, store.rs, subscription.rs,
// core/runner.rs. Kept as a plain interface surface — no classes, no runtime
// deps — so Svelte components can import them freely without pulling logic.
//
// `Option<T>` in Rust serializes to `T | null` over serde_json, so optional
// fields are typed as nullable rather than optional. Fields with `#[serde(default)]`
// or pure `Option` both arrive as `null` when unset, never `undefined`.

export type Protocol =
  | "vless"
  | "vmess"
  | "trojan"
  | "shadowsocks"
  | "hysteria2"
  | "tuic";

export type Transport =
  | "tcp"
  | "ws"
  | "grpc"
  | "h2"
  | "httpupgrade"
  | "xhttp";

export type Security = "none" | "tls" | "reality";

export type Flow = "xtls-rprx-vision";

export interface RealityParams {
  public_key: string;
  short_id: string;
}

export interface TlsParams {
  sni: string | null;
  alpn: string[];
  // `Fingerprint(pub String)` is a newtype; serde renders it as the inner string.
  fingerprint: string | null;
  allow_insecure: boolean;
  reality: RealityParams | null;
}

export interface TransportParams {
  path: string | null;
  host: string | null;
  service_name: string | null;
  mode: string | null;
}

export interface Profile {
  name: string;
  protocol: Protocol;
  address: string;
  port: number;
  credential: string;

  flow: Flow | null;
  packet_encoding: string | null;

  alter_id: number | null;
  cipher: string | null;

  ss_method: string | null;

  transport: Transport;
  transport_params: TransportParams;
  security: Security;
  tls: TlsParams | null;
}

export interface StoredProfile {
  id: number;
  profile: Profile;
  subscription_id: number | null;
  favorite: boolean;
  last_connected_at: number | null;
  region: string | null;
  created_at: number;
  updated_at: number;
}

export interface StoredSubscription {
  id: number;
  name: string;
  url: string;
  last_fetched_at: number | null;
  last_error: string | null;
  used_bytes: number | null;
  total_bytes: number | null;
  expires_at: number | null;
  created_at: number;
}

export interface SyncApplied {
  subscription_id: number;
  profiles_inserted: number;
}

export interface LineError {
  line: number;
  input: string;
  error: string;
}

export interface SubscriptionResult {
  profiles: Profile[];
  errors: LineError[];
  decoded_base64: boolean;
}

export type LogStream = "stdout" | "stderr";

export interface LogLine {
  stream: LogStream;
  line: string;
}

export interface CoreStateEvent {
  running: boolean;
}


// --- Clash API stats ---

export interface Traffic {
  up: number;
  down: number;
}

/** Per-connection entry from sing-box's `/connections` snapshot. Loose shape:
  * sing-box versions ship different metadata fields; we never require any
  * specific field, so the frontend tolerates schema drift. */
export interface ConnectionEntry {
  id: string;
  upload?: number;
  download?: number;
  start?: string;
  chains?: string[];
  rule?: string;
  metadata?: Record<string, unknown>;
  [k: string]: unknown;
}

export interface ConnectionsSnapshot {
  downloadTotal?: number;
  uploadTotal?: number;
  memory?: number;
  connections?: ConnectionEntry[];
  [k: string]: unknown;
}

export interface RoutingRule {
  id: number;
  app_path: string;
  app_name: string | null;
  enabled: boolean;
  created_at: number;
}

export interface ProcessInfo {
  name: string;
  path: string;
}
