import * as events from "$lib/events";
import { dispatchConnectAction } from "./actions/connect";
import { core } from "./stores/core.svelte";
import { logs } from "./stores/logs.svelte";
import { stats } from "./stores/stats.svelte";

/** Wire up Tauri event listeners. Call once from the root component's onMount. */
export async function attachEventListeners(): Promise<() => void> {
  const offLog = await events.onCoreLog((l) => logs.push(l));
  const offState = await events.onCoreState((s) => {
    if (!s.running) {
      core.onExternalExit();
      stats.reset();
    }
  });
  const offTraffic = await events.onTraffic((t) => stats.pushTraffic(t));
  const offConnections = await events.onConnections((s) =>
    stats.applyConnections(s),
  );
  const offTray = await events.onTrayAction(() => void dispatchConnectAction());
  return () => {
    offLog();
    offState();
    offTraffic();
    offConnections();
    offTray();
  };
}
