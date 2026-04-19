import { catalog, core, toast } from "$lib/state.svelte";

/** Single source of truth for the Connect/Cancel/Disconnect/Retry button
 * dispatch — used by both the StatusBar and the tray menu's toggle item.
 *
 * disconnected → connect selected; if no selection, toast.
 * connecting/connected/switching → disconnect.
 * error → retry.
 * disconnecting → noop. */
export async function dispatchConnectAction(): Promise<void> {
  const k = core.state.kind;
  try {
    if (k === "disconnected") {
      const id = catalog.selectedId;
      const sel = id == null ? null : catalog.profiles.find((p) => p.id === id) ?? null;
      if (!sel) {
        toast.show("Select a profile first");
        return;
      }
      await core.connect(sel);
    } else if (k === "connected" || k === "connecting" || k === "switching") {
      await core.disconnect();
    } else if (k === "error") {
      await core.retry();
    }
  } catch (e) {
    toast.show(String(e));
  }
}
