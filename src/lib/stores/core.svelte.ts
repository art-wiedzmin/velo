import * as api from "$lib/api";
import type { CoreStateUi, Profile, StoredProfile } from "$lib/types";
import { catalog } from "./catalog.svelte";
import { sysproxy } from "./sysproxy.svelte";

export class CoreStore {
  state = $state<CoreStateUi>({ kind: "disconnected" });
  /** Last port used for sysproxy (from the built sing-box config). Needed
   * because sysproxy is toggled independently of the runner. Defaults to
   * the mixed-inbound port baked into the config builder. */
  mixedPort = $state(10808);

  /** Convenience boolean the status-bar light reacts to. */
  get running(): boolean {
    const k = this.state.kind;
    return k === "connecting" || k === "connected" || k === "switching";
  }
  /** Profile the UI highlights as the "active" card. During switching we
   * highlight the target (the card the user just clicked). */
  get activeProfile(): StoredProfile | null {
    switch (this.state.kind) {
      case "connected": return this.state.active;
      case "connecting": return this.state.target;
      case "switching": return this.state.to;
      case "disconnecting": return this.state.wasActive;
      default: return null;
    }
  }

  async refresh(): Promise<void> {
    try {
      const running = await api.coreStatus();
      // We can't tell *which* profile sing-box is running without extra
      // IPC; if the runner is up but our state says disconnected, leave it
      // — the state-bar will catch up as soon as the user acts.
      if (!running && this.running) {
        this.state = { kind: "disconnected" };
      }
    } catch (e) {
      this.state = {
        kind: "error",
        reason: String(e),
        lastTarget: this.activeProfile,
      };
    }
  }

  async connect(sp: StoredProfile): Promise<void> {
    if (this.state.kind === "connected" || this.state.kind === "switching") {
      return this.switchTo(sp);
    }
    this.state = { kind: "connecting", target: sp };
    try {
      await api.coreStart(sp.profile, sp.id);
      // Auto-enable sysproxy so the browser actually routes through the
      // new proxy. Failure here is surfaced separately — core is up, UI
      // should reflect that and let the user retry the proxy toggle.
      try {
        await sysproxy.enable(this.mixedPort);
      } catch (proxyErr) {
        console.warn("sysproxy enable failed:", proxyErr);
      }
      this.state = { kind: "connected", active: sp, since: Date.now() };
      void catalog.refresh();
    } catch (e) {
      this.state = { kind: "error", reason: String(e), lastTarget: sp };
      throw e;
    }
  }

  async switchTo(sp: StoredProfile): Promise<void> {
    if (this.state.kind !== "connected") return this.connect(sp);
    const from = this.state.active;
    if (from.id === sp.id) return;
    this.state = { kind: "switching", from, to: sp };
    try {
      // core_stop also clears sysproxy backend-side. After restart we must
      // re-enable it — a switch should not silently leave traffic direct.
      await api.coreStop();
      await api.coreStart(sp.profile, sp.id);
      try {
        await sysproxy.enable(this.mixedPort);
      } catch (proxyErr) {
        console.warn("sysproxy re-enable after switch failed:", proxyErr);
      }
      this.state = { kind: "connected", active: sp, since: Date.now() };
      void catalog.refresh();
    } catch (e) {
      // Rollback: we're not connected to anything usable now. Degrade to
      // error with the new target recorded for Retry.
      this.state = { kind: "error", reason: String(e), lastTarget: sp };
      throw e;
    }
  }

  async disconnect(): Promise<void> {
    const active = this.activeProfile;
    if (active && this.state.kind === "connected") {
      this.state = { kind: "disconnecting", wasActive: active };
    }
    try {
      await api.coreStop();
      // Backend clears the registry; sync our local store so the toggle
      // displays correctly without a separate refresh call.
      sysproxy.enabled = false;
      this.state = { kind: "disconnected" };
    } catch (e) {
      this.state = { kind: "error", reason: String(e), lastTarget: active };
      throw e;
    }
  }

  async retry(): Promise<void> {
    if (this.state.kind !== "error") return;
    const target = this.state.lastTarget;
    if (!target) {
      this.state = { kind: "disconnected" };
      return;
    }
    await this.connect(target);
  }

  dismissError(): void {
    if (this.state.kind === "error") this.state = { kind: "disconnected" };
  }

  /** Called by the `core://state { running: false }` event when sing-box
   * exits without our involvement (crash, external kill). */
  onExternalExit(): void {
    const k = this.state.kind;
    if (k === "connecting") {
      this.state = {
        kind: "error",
        reason: "sing-box exited during handshake",
        lastTarget: this.state.target,
      };
    } else if (k === "connected" || k === "switching" || k === "disconnecting") {
      this.state = { kind: "disconnected" };
    }
  }

  /** Patch an in-flight profile snapshot after an external mutation (e.g.
   * rename). CoreStore holds its own copy of StoredProfile frozen at connect
   * time; without this the StatusBar headline would keep the stale name. */
  patchProfile(id: number, patch: Partial<Profile>): void {
    const merge = (sp: StoredProfile): StoredProfile =>
      sp.id === id ? { ...sp, profile: { ...sp.profile, ...patch } } : sp;
    const s = this.state;
    switch (s.kind) {
      case "connecting":
        this.state = { ...s, target: merge(s.target) };
        break;
      case "connected":
        this.state = { ...s, active: merge(s.active) };
        break;
      case "switching":
        this.state = { ...s, from: merge(s.from), to: merge(s.to) };
        break;
      case "disconnecting":
        this.state = { ...s, wasActive: merge(s.wasActive) };
        break;
      case "error":
        if (s.lastTarget) this.state = { ...s, lastTarget: merge(s.lastTarget) };
        break;
    }
  }
}

export const core = new CoreStore();
