<script lang="ts">
  // Right-side drawer: subscription management (sync/remove) + About.
  // Routing lives in a separate drawer.

  import * as api from "$lib/api";
  import {
    loadAutostart,
    setAutoconnectMode,
    setAutoconnectProfileId,
    setEnabled,
    setMinimized,
    type AutoconnectMode,
  } from "$lib/autostart";
  import Drawer from "$lib/components/Drawer.svelte";
  import { catalog, toast } from "$lib/state.svelte";
  import { fmtTs } from "$lib/format";
  import { onMount } from "svelte";

  let { open, onClose }: { open: boolean; onClose: () => void } = $props();

  let busyId = $state<number | null>(null);

  // Autostart state. The toggle is backed by a Windows scheduled task with
  // `/RL HIGHEST` (velo's exe manifest forces admin, so an HKCU\Run entry
  // would UAC-prompt every logon — only the scheduler path is silent).
  // The rest we persist ourselves so Rust can read them at launch time.
  // `portable === null` means the probe is still in flight — the Startup
  // section doesn't render until we know, and never renders in portable
  // mode (the scheduled task's absolute exe path would go stale the first
  // time the user moves the folder).
  let portable = $state<boolean | null>(null);
  let autoEnabled = $state(false);
  let autoMinimized = $state(true);
  let autoMode = $state<AutoconnectMode>("off");
  let autoPinnedId = $state<number | null>(null);
  let autoBusy = $state(false);

  onMount(async () => {
    try {
      portable = await api.isPortable();
      if (portable) return;
      const s = await loadAutostart();
      autoEnabled = s.enabled;
      autoMinimized = s.minimized;
      autoMode = s.autoconnect;
      autoPinnedId = s.autoconnectProfileId;
    } catch (e) {
      toast.show(`Autostart: ${e}`);
    }
  });

  async function toggleAutoEnabled(next: boolean) {
    autoBusy = true;
    try {
      await setEnabled(next);
      autoEnabled = next;
    } catch (e) {
      toast.show(String(e));
    } finally {
      autoBusy = false;
    }
  }

  async function toggleAutoMinimized(next: boolean) {
    autoBusy = true;
    try {
      await setMinimized(next);
      autoMinimized = next;
    } catch (e) {
      toast.show(String(e));
    } finally {
      autoBusy = false;
    }
  }

  async function onAutoModeChange(next: AutoconnectMode) {
    autoBusy = true;
    try {
      await setAutoconnectMode(next);
      autoMode = next;
      // Pinned mode without a selection yet: auto-pick the first profile so
      // the dropdown below shows a valid choice instead of an empty slot.
      if (next === "pinned" && autoPinnedId == null && catalog.profiles.length > 0) {
        const id = catalog.profiles[0].id;
        await setAutoconnectProfileId(id);
        autoPinnedId = id;
      }
    } catch (e) {
      toast.show(String(e));
    } finally {
      autoBusy = false;
    }
  }

  async function onAutoPinnedChange(id: number) {
    autoBusy = true;
    try {
      await setAutoconnectProfileId(id);
      autoPinnedId = id;
    } catch (e) {
      toast.show(String(e));
    } finally {
      autoBusy = false;
    }
  }

  async function syncSub(id: number) {
    busyId = id;
    try {
      await api.subscriptionsSync(id);
      await catalog.refresh();
      toast.show("Subscription synced");
    } catch (e) {
      toast.show(String(e));
    } finally {
      busyId = null;
    }
  }

  async function deleteSub(id: number) {
    busyId = id;
    try {
      await api.subscriptionsDelete(id);
      await catalog.refresh();
      toast.show("Subscription removed");
    } catch (e) {
      toast.show(String(e));
    } finally {
      busyId = null;
    }
  }

</script>

<Drawer {open} title="Settings" tag="v0.1.0" {onClose}>
  {#if portable === false}
    <div class="st-sec">
      <div class="st-sec-h">Startup</div>
      <div class="st-card">
        <label class="st-row">
          <div class="st-txt">
            <div class="st-lbl">Launch velo at login</div>
            <div class="st-sub">Creates a scheduled task so velo starts elevated at logon without a UAC prompt.</div>
          </div>
          <input
            type="checkbox"
            class="st-switch"
            checked={autoEnabled}
            disabled={autoBusy}
            onchange={(e) => toggleAutoEnabled(e.currentTarget.checked)}
          />
        </label>
        <label class="st-row" class:st-row-dim={!autoEnabled}>
          <div class="st-txt">
            <div class="st-lbl">Start minimized to tray</div>
            <div class="st-sub">When launched at login, stay hidden — the tray icon opens the window.</div>
          </div>
          <input
            type="checkbox"
            class="st-switch"
            checked={autoMinimized}
            disabled={autoBusy || !autoEnabled}
            onchange={(e) => toggleAutoMinimized(e.currentTarget.checked)}
          />
        </label>
        <div class="st-row" class:st-row-dim={!autoEnabled}>
          <div class="st-txt">
            <div class="st-lbl">Auto-connect on startup</div>
            <div class="st-sub">Only triggers when velo is launched at login.</div>
          </div>
          <select
            class="st-select"
            value={autoMode}
            disabled={autoBusy || !autoEnabled}
            onchange={(e) => onAutoModeChange(e.currentTarget.value as AutoconnectMode)}
          >
            <option value="off">Off</option>
            <option value="last">Last used</option>
            <option value="pinned">Specific profile…</option>
          </select>
        </div>
        {#if autoMode === "pinned"}
          <div class="st-row st-row-indent" class:st-row-dim={!autoEnabled}>
            <div class="st-txt">
              <div class="st-sub">Profile</div>
            </div>
            <select
              class="st-select"
              value={autoPinnedId ?? ""}
              disabled={autoBusy || !autoEnabled || catalog.profiles.length === 0}
              onchange={(e) => onAutoPinnedChange(Number(e.currentTarget.value))}
            >
              {#if catalog.profiles.length === 0}
                <option value="">No profiles</option>
              {:else}
                {#each catalog.profiles as p (p.id)}
                  <option value={p.id}>{p.profile.name}</option>
                {/each}
              {/if}
            </select>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <div class="st-sec">
    <div class="st-sec-h">Subscriptions</div>
    <div class="st-card">
      {#if catalog.subscriptions.length === 0}
        <div class="st-row">
          <div class="st-txt">
            <div class="st-lbl">No subscriptions</div>
            <div class="st-sub">Import one from the <b>+ Add profile</b> card.</div>
          </div>
        </div>
      {:else}
        {#each catalog.subscriptions as s (s.id)}
          <div class="st-row">
            <div class="st-txt">
              <div class="st-lbl">{s.name}</div>
              <div class="st-sub">
                last fetch: <b>{fmtTs(s.last_fetched_at)}</b>
                {#if s.last_error} · <span class="err">{s.last_error}</span>{/if}
              </div>
            </div>
            <button class="st-btn" disabled={busyId === s.id} onclick={() => syncSub(s.id)}>
              {busyId === s.id ? "Syncing…" : "Sync"}
            </button>
            <button class="st-btn-danger" disabled={busyId === s.id} onclick={() => deleteSub(s.id)}>
              Remove
            </button>
          </div>
        {/each}
      {/if}
    </div>
  </div>

  <div class="st-sec">
    <div class="st-sec-h">About</div>
    <div class="st-card">
      <div class="st-row">
        <div class="st-txt">
          <div class="st-lbl">velo</div>
          <div class="st-sub">VLESS desktop client · Tauri + sing-box</div>
        </div>
      </div>
    </div>
  </div>
</Drawer>

<style>
  .st-sec { margin-top: 18px }
  .st-sec:first-child { margin-top: 4px }
  .st-sec-h {
    font-family: var(--sans); font-size: 9.5px;
    text-transform: uppercase; letter-spacing: .9px;
    color: var(--text-3);
    padding: 0 2px 8px;
    display: flex; align-items: center; gap: 8px;
  }
  .st-sec-h::after { content: ""; flex: 1; height: 1px; background: var(--border) }
  .st-card {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 10px;
    display: flex; flex-direction: column; overflow: hidden;
  }
  .st-row {
    display: flex; align-items: center; gap: 10px;
    padding: 11px 14px;
    border-top: 1px solid var(--border);
    min-height: 46px;
  }
  .st-row:first-child { border-top: 0 }
  .st-txt { flex: 1; min-width: 0 }
  .st-lbl { font-size: 12.5px; color: var(--text); font-weight: 500; letter-spacing: -.005em }
  .st-sub {
    font-size: 12px; color: var(--text-3); margin-top: 2px; line-height: 1.35;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .st-sub b { color: var(--text-2); font-weight: 500 }
  .st-sub .err { color: var(--bad); font-family: var(--sans); font-size: 12px }

  .st-btn {
    font: inherit; font-size: 11.5px; color: var(--text-2);
    background: var(--surface); border: 1px solid var(--border);
    border-radius: 6px; padding: 5px 12px; cursor: pointer;
  }
  .st-btn:hover:not(:disabled) { border-color: var(--border-strong); color: var(--text) }
  .st-btn:disabled { opacity: .5; cursor: wait }
  .st-btn-danger {
    font: inherit; font-size: 11.5px; color: #ef9a84;
    background: transparent;
    border: 1px solid color-mix(in oklab, #ef9a84 35%, var(--border));
    border-radius: 6px; padding: 5px 12px; cursor: pointer;
  }
  .st-btn-danger:hover:not(:disabled) {
    background: color-mix(in oklab, #ef9a84 12%, transparent);
    color: #f2b5a2;
  }
  .st-btn-danger:disabled { opacity: .5; cursor: wait }

  /* iOS-style switch, sized to match `.st-btn` height so rows with a switch
   * line up with rows that have a button. */
  .st-switch {
    appearance: none;
    width: 34px; height: 20px;
    border-radius: 999px;
    background: var(--surface);
    border: 1px solid var(--border);
    position: relative; cursor: pointer;
    transition: background 120ms ease, border-color 120ms ease;
    flex-shrink: 0;
  }
  .st-switch::after {
    content: "";
    position: absolute; top: 2px; left: 2px;
    width: 14px; height: 14px; border-radius: 50%;
    background: var(--text-3);
    transition: left 140ms cubic-bezier(.4,.0,.2,1), background 120ms ease;
  }
  .st-switch:checked {
    background: color-mix(in oklab, var(--accent) 65%, var(--surface));
    border-color: color-mix(in oklab, var(--accent) 55%, var(--border));
  }
  .st-switch:checked::after { left: 16px; background: #fff }
  .st-switch:disabled { opacity: .45; cursor: not-allowed }

  label.st-row { cursor: pointer }
  .st-row-dim .st-lbl, .st-row-dim .st-sub { opacity: .55 }
  .st-row-indent { padding-left: 28px }

  .st-select {
    font: inherit; font-size: 11.5px; color: var(--text-2);
    background: var(--surface); border: 1px solid var(--border);
    border-radius: 6px; padding: 5px 10px; cursor: pointer;
    min-width: 130px;
  }
  .st-select:hover:not(:disabled) { border-color: var(--border-strong); color: var(--text) }
  .st-select:focus { outline: none; border-color: var(--accent) }
  .st-select:disabled { opacity: .5; cursor: not-allowed }

</style>
