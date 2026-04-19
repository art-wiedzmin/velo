import type { ConnectionEntry, ConnectionsSnapshot, Traffic } from "$lib/types";

const TRAFFIC_HISTORY = 60;

export class StatsStore {
  latest = $state<Traffic>({ up: 0, down: 0 });
  history = $state<Traffic[]>([]);
  downloadTotal = $state(0);
  uploadTotal = $state(0);
  memory = $state<number | null>(null);
  connections = $state<ConnectionEntry[]>([]);

  pushTraffic(t: Traffic): void {
    this.latest = t;
    const next = this.history.length >= TRAFFIC_HISTORY
      ? [...this.history.slice(this.history.length - TRAFFIC_HISTORY + 1), t]
      : [...this.history, t];
    this.history = next;
  }

  applyConnections(s: ConnectionsSnapshot): void {
    this.downloadTotal = s.downloadTotal ?? 0;
    this.uploadTotal = s.uploadTotal ?? 0;
    this.memory = typeof s.memory === "number" ? s.memory : null;
    this.connections = s.connections ?? [];
  }

  reset(): void {
    this.latest = { up: 0, down: 0 };
    this.history = [];
    this.downloadTotal = 0;
    this.uploadTotal = 0;
    this.memory = null;
    this.connections = [];
  }
}

export const stats = new StatsStore();
