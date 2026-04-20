import * as api from "$lib/api";

// Read-only mirror of the backend's sysproxy state. The toggle is driven
// by `core_start`/`core_stop` on the Rust side, gated by mode — this store
// exists so the status indicator can reflect reality without coupling the
// UI to the runner's lifecycle.
export class SysProxyStore {
  enabled = $state(false);
  lastError = $state<string | null>(null);

  async refresh(): Promise<void> {
    try {
      this.enabled = await api.sysproxyStatus();
    } catch (e) {
      this.lastError = String(e);
    }
  }
}

export const sysproxy = new SysProxyStore();