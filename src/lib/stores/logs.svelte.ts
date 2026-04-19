import type { LogLevel, LogLine, ParsedLogLine } from "$lib/types";

const LOG_BUFFER_CAP = 1000;

// Log parsing: cheap regex strip applied once on ingest so the render loop
// doesn't reparse 1000 lines every frame. Formats come straight from
// sing-box stdout/stderr — see docs on LogsPanel for the shape.
const ANSI_RE = /\u001b\[[0-9;]*m/g;
const TIMESTAMP_RE = /^\+\d+\s+\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\s+/;
const CONN_ID_RE = /^\[\d+\s+\d+(?:\.\d+)?(?:ms|s)\]\s+/;
const LEVEL_RE = /^(INFO|DEBUG|WARN|WARNING|ERROR|FATAL|TRACE)\s+/;

function parseLogLine(raw: string, id: number): ParsedLogLine {
  let s = raw.replace(ANSI_RE, "").replace(TIMESTAMP_RE, "");
  let level: LogLevel = "";
  const m = s.match(LEVEL_RE);
  if (m) {
    s = s.slice(m[0].length);
    const norm = m[1].toLowerCase();
    level = norm === "warning" ? "warn" : (norm as LogLevel);
  }
  s = s.replace(CONN_ID_RE, "");
  return { id, level, text: s };
}

export class LogsStore {
  lines = $state<ParsedLogLine[]>([]);
  // Burst bursts of log events arrive faster than Svelte can reconcile the
  // #each list — without batching, the main thread stalls and IPC replies
  // (Disconnect, profile delete) get starved behind render work. We coalesce
  // per animation frame: the backend can emit as fast as it wants, the UI
  // updates at display refresh rate.
  #pending: LogLine[] = [];
  #scheduled = false;
  #nextId = 0;

  push(line: LogLine): void {
    this.#pending.push(line);
    if (this.#scheduled) return;
    this.#scheduled = true;
    requestAnimationFrame(() => this.#flush());
  }

  #flush(): void {
    this.#scheduled = false;
    if (this.#pending.length === 0) return;
    const batch = this.#pending;
    this.#pending = [];
    const parsed = batch.map((l) => parseLogLine(l.line, this.#nextId++));
    const merged = this.lines.concat(parsed);
    const overflow = merged.length - LOG_BUFFER_CAP;
    this.lines = overflow > 0 ? merged.slice(overflow) : merged;
  }

  clear(): void {
    this.lines = [];
    this.#pending = [];
  }
}

export const logs = new LogsStore();
