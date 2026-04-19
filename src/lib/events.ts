// Thin helpers around Tauri's event system. Centralized so component code
// never touches the string event names directly — if an event is renamed on
// the Rust side we change it here once.

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ConnectionsSnapshot, CoreStateEvent, LogLine, Traffic } from "./types";

export const CORE_LOG = "core://log";
export const CORE_STATE = "core://state";

export const onCoreLog = (cb: (line: LogLine) => void): Promise<UnlistenFn> =>
  listen<LogLine>(CORE_LOG, (e) => cb(e.payload));

export const onCoreState = (
  cb: (state: CoreStateEvent) => void,
): Promise<UnlistenFn> =>
  listen<CoreStateEvent>(CORE_STATE, (e) => cb(e.payload));


// Stats WS events emitted by the Runner's spawned subscribers.
export const STATS_TRAFFIC = "stats://traffic";
export const STATS_CONNECTIONS = "stats://connections";

export const onTraffic = (cb: (t: Traffic) => void): Promise<UnlistenFn> =>
  listen<Traffic>(STATS_TRAFFIC, (e) => cb(e.payload));

export const onConnections = (
  cb: (s: ConnectionsSnapshot) => void,
): Promise<UnlistenFn> =>
  listen<ConnectionsSnapshot>(STATS_CONNECTIONS, (e) => cb(e.payload));

// Tray menu: a single toggle emits `toggle`. Frontend dispatches by
// current core state (connect / cancel / disconnect / retry).
export const TRAY_ACTION = "tray://action";

export const onTrayAction = (cb: () => void): Promise<UnlistenFn> =>
  listen<string>(TRAY_ACTION, () => cb());