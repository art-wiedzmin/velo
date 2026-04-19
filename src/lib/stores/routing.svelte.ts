import * as api from "$lib/api";
import type { CoreMode, RoutingMode, RoutingRule } from "$lib/types";

function upsertRule(list: RoutingRule[], r: RoutingRule): RoutingRule[] {
  const i = list.findIndex((x) => x.id === r.id);
  if (i === -1) return [...list, r];
  const copy = [...list];
  copy[i] = r;
  return copy;
}

export class RoutingStore {
  rules = $state<RoutingRule[]>([]);
  coreMode = $state<CoreMode>("sysproxy");
  mode = $state<RoutingMode>("none");
  elevated = $state(true);
  loading = $state(false);
  lastError = $state<string | null>(null);

  async refresh(): Promise<void> {
    this.loading = true;
    this.lastError = null;
    try {
      const [rules, cm, rm, el] = await Promise.all([
        api.routingList(),
        api.settingsGet("core_mode"),
        api.settingsGet("routing_mode"),
        api.isElevated().catch(() => true),
      ]);
      this.rules = rules;
      this.coreMode = cm === "tun" ? "tun" : "sysproxy";
      this.mode = rm === "whitelist" || rm === "blacklist" ? rm : "none";
      this.elevated = el;
    } catch (e) {
      this.lastError = String(e);
    } finally {
      this.loading = false;
    }
  }

  async setCoreMode(m: CoreMode): Promise<void> {
    await api.settingsSet("core_mode", m);
    this.coreMode = m;
  }

  async setMode(m: RoutingMode): Promise<void> {
    await api.settingsSet("routing_mode", m);
    this.mode = m;
  }

  async add(appPath: string, appName: string | null): Promise<void> {
    const created = await api.routingAdd(appPath, appName);
    this.rules = upsertRule(this.rules, created);
  }

  async remove(id: number): Promise<void> {
    await api.routingDelete(id);
    this.rules = this.rules.filter((r) => r.id !== id);
  }

  async toggle(id: number, enabled: boolean): Promise<void> {
    await api.routingSetEnabled(id, enabled);
    this.rules = this.rules.map((r) => (r.id === id ? { ...r, enabled } : r));
  }
}

export const routing = new RoutingStore();
