import type { StoredProfile } from "./wire";

export type LogLevel = "info" | "debug" | "warn" | "error" | "fatal" | "trace" | "";

export interface ParsedLogLine {
  id: number;
  level: LogLevel;
  text: string;
}

// --- UI state machine ---------------------------------------------------

export type CoreStateKind =
  | "disconnected"
  | "connecting"
  | "connected"
  | "switching"
  | "disconnecting"
  | "error";

export type CoreStateUi =
  | { kind: "disconnected" }
  | { kind: "connecting"; target: StoredProfile }
  | { kind: "connected"; active: StoredProfile; since: number }
  | { kind: "switching"; from: StoredProfile; to: StoredProfile }
  | { kind: "disconnecting"; wasActive: StoredProfile }
  | { kind: "error"; reason: string; lastTarget: StoredProfile | null };

export type FilterKey = "all" | "favorites" | "recent";

// --- Routing ------------------------------------------------------------

export type CoreMode = "sysproxy" | "tun";
export type RoutingMode = "none" | "whitelist" | "blacklist";
