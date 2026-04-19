import * as api from "$lib/api";

export class SysProxyStore {
  enabled = $state(false);
  busy = $state(false);
  lastError = $state<string | null>(null);

  async refresh(): Promise<void> {
    try {
      this.enabled = await api.sysproxyStatus();
    } catch (e) {
      this.lastError = String(e);
    }
  }

  async enable(port: number): Promise<void> {
    this.busy = true;
    this.lastError = null;
    try {
      await api.sysproxyEnable("127.0.0.1", port);
      this.enabled = true;
    } catch (e) {
      this.lastError = String(e);
      throw e;
    } finally {
      this.busy = false;
    }
  }

  async disable(): Promise<void> {
    this.busy = true;
    this.lastError = null;
    try {
      await api.sysproxyDisable();
      this.enabled = false;
    } catch (e) {
      this.lastError = String(e);
      throw e;
    } finally {
      this.busy = false;
    }
  }
}

export const sysproxy = new SysProxyStore();
