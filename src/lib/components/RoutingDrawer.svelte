<script lang="ts">
  // Right-side drawer mirroring the design's Routing panel.
  //   - Warning banner when core_mode !== "tun" (rules only take effect in Tun).
  //   - Segmented Whitelist / Blacklist / Off.
  //   - Search box + Add menu (From running / Browse .exe…).
  //   - List of apps with name/path/toggle/delete.
  //
  // Route changes (add, toggle, delete) take effect on the NEXT core_start —
  // we don't hot-reload sing-box. Users disconnect/connect to apply.

  import { onMount } from "svelte";
  import * as api from "$lib/api";
  import { routing, toast } from "$lib/state.svelte";
  import Drawer from "$lib/components/Drawer.svelte";
  import WarningBanner from "$lib/components/WarningBanner.svelte";
  import ProcessPickerModal from "./ProcessPickerModal.svelte";
  import IconWarning from "$lib/components/icons/IconWarning.svelte";
  import IconLock from "$lib/components/icons/IconLock.svelte";
  import IconSearch from "$lib/components/icons/IconSearch.svelte";
  import IconTrash from "$lib/components/icons/IconTrash.svelte";
  import type { RoutingMode } from "$lib/types";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";

  let { open: isOpen, onClose }: { open: boolean; onClose: () => void } = $props();

  let search = $state("");
  let showAddMenu = $state(false);
  let showPicker = $state(false);

  onMount(() => {
    // Keep in sync so the drawer opens showing real state even on first mount.
    void routing.refresh();
  });

  $effect(() => {
    if (isOpen) void routing.refresh();
  });

  const filteredRules = $derived(
    search.trim() === ""
      ? routing.rules
      : routing.rules.filter((r) => {
          const q = search.toLowerCase();
          return (
            (r.app_name ?? "").toLowerCase().includes(q) ||
            r.app_path.toLowerCase().includes(q)
          );
        }),
  );

  const activeCount = $derived(routing.rules.filter((r) => r.enabled).length);
  const offCount = $derived(routing.rules.length - activeCount);

  async function browseExe() {
    showAddMenu = false;
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [{ name: "Executable", extensions: ["exe"] }],
      });
      if (!selected || typeof selected !== "string") return;
      const name = selected.split(/[\\/]/).pop() ?? null;
      await routing.add(selected, name);
      toast.show(`Added ${name ?? selected}`);
    } catch (e) {
      toast.show(String(e));
    }
  }

  async function setMode(m: RoutingMode) {
    try {
      await routing.setMode(m);
    } catch (e) {
      toast.show(String(e));
    }
  }

  const needsTun = $derived(routing.coreMode !== "tun" && routing.mode !== "none");
  const needsAdmin = $derived(routing.coreMode === "tun" && !routing.elevated);

  async function switchToTun() {
    try {
      await routing.setCoreMode("tun");
      toast.show("Tunnel mode set — reconnect to apply");
    } catch (e) {
      toast.show(String(e));
    }
  }

  async function relaunchElevated() {
    try {
      await api.relaunchAsAdmin();
    } catch (e) {
      toast.show(String(e));
    }
  }

</script>

<Drawer open={isOpen} title="Routing" tag={`${activeCount} active · ${offCount} off`} {onClose}>
  {#if needsTun}
    <WarningBanner
      title="Rules apply only in Tunnel mode"
      message="Sysproxy mode can't see per-app traffic. Switch to Tunnel on next connect."
      action="Switch to Tunnel"
      onAction={switchToTun}
    >
      {#snippet icon()}<IconWarning />{/snippet}
    </WarningBanner>
  {/if}

  {#if needsAdmin}
    <WarningBanner
      title="Administrator privileges required"
      message="Tunnel mode creates a virtual NIC — Windows demands admin rights. Relaunch to elevate."
      action="Relaunch as admin"
      onAction={relaunchElevated}
    >
      {#snippet icon()}<IconLock />{/snippet}
    </WarningBanner>
  {/if}

  <div class="dr-seg" role="tablist">
    <button
      class:on={routing.mode === "whitelist"}
      onclick={() => setMode("whitelist")}
    >
      <b>Whitelist</b><small>only these via VPN</small>
    </button>
    <button
      class:on={routing.mode === "blacklist"}
      onclick={() => setMode("blacklist")}
    >
      <b>Blacklist</b><small>all except these</small>
    </button>
  </div>
  <div class="off-row">
    <button class="off-btn" class:on={routing.mode === "none"} onclick={() => setMode("none")}>
      Disable app-level routing
    </button>
  </div>

  <div class="dr-tools">
    <div class="dr-search">
      <IconSearch />
      <input placeholder="Filter apps…" bind:value={search} />
    </div>
    <div style="position:relative">
      <button class="dr-add" onclick={() => (showAddMenu = !showAddMenu)}>
        + Add
      </button>
      <div class="dr-add-menu" class:on={showAddMenu}>
        <button onclick={() => { showAddMenu = false; showPicker = true }}>
          <span>From running…</span>
          <span class="m-sub">pick a live process</span>
        </button>
        <button onclick={browseExe}>
          <span>Browse .exe…</span>
          <span class="m-sub">file picker</span>
        </button>
      </div>
    </div>
  </div>

  <div class="app-list">
    {#each filteredRules as r (r.id)}
      <div class="app" class:off={!r.enabled}>
        <div class="app-ico">{(r.app_name ?? "?").slice(0, 1)}</div>
        <div class="app-info">
          <div class="app-name">{r.app_name ?? "unknown"}</div>
          <div class="app-path" title={r.app_path}>{r.app_path}</div>
        </div>
        <button class="app-del" aria-label="Remove" onclick={() => routing.remove(r.id)}>
          <IconTrash size={13} variant="short" />
        </button>
        <button
          class="tgl"
          class:on={r.enabled}
          aria-pressed={r.enabled}
          aria-label={r.enabled ? "Disable" : "Enable"}
          onclick={() => routing.toggle(r.id, !r.enabled)}
        ></button>
      </div>
    {:else}
      <div class="empty-hint">
        {search ? "No matches." : "No apps yet. Use + Add to pick one."}
      </div>
    {/each}
  </div>
</Drawer>

<ProcessPickerModal
  open={showPicker}
  onClose={() => (showPicker = false)}
  onPick={(path, name) => routing.add(path, name)}
/>

<style>
  .dr-seg {
    display: grid; grid-template-columns: 1fr 1fr;
    background: var(--surface-3);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 3px; gap: 3px;
    margin-bottom: 8px;
  }
  .dr-seg button {
    font-family: var(--sans); font-size: 13px; font-weight: 500;
    background: transparent; border: 0; color: var(--text-2);
    cursor: pointer; padding: 8px 12px; border-radius: 6px;
    text-align: center; line-height: 1.1;
  }
  .dr-seg button b { display: block; font-size: 13px; font-weight: 600; color: inherit }
  .dr-seg button small { display: block; font-size: 12px; font-family: var(--sans); color: var(--text-3); margin-top: 3px; text-transform: uppercase; letter-spacing: .4px }
  .dr-seg button:hover { color: var(--text) }
  .dr-seg button.on { background: var(--surface); color: var(--text); box-shadow: 0 1px 0 rgba(255,255,255,.04), 0 1px 2px rgba(0,0,0,.3) }
  .dr-seg button.on small { color: var(--accent) }

  .off-row { display: flex; justify-content: flex-end; margin-bottom: 12px }
  .off-btn {
    font-family: var(--sans); font-size: 12px; color: var(--text-3);
    background: transparent; border: 0;
    padding: 4px 0; cursor: pointer;
    text-transform: uppercase; letter-spacing: .5px;
  }
  .off-btn:hover { color: var(--text-2) }
  .off-btn.on { color: var(--accent) }

  .dr-tools { display: flex; gap: 8px; margin-bottom: 12px }
  .dr-search { flex: 1; min-width: 0; position: relative }
  .dr-search :global(svg) {
    position: absolute; left: 9px; top: 50%; transform: translateY(-50%);
    color: var(--text-3); pointer-events: none;
  }
  .dr-search input {
    width: 100%;
    background: var(--surface-2); border: 1px solid var(--border);
    border-radius: 7px;
    color: var(--text); font-family: var(--sans); font-size: 13px;
    padding: 7px 10px 7px 30px; outline: none; height: 32px;
  }
  .dr-search input:focus { border-color: var(--border-strong) }
  .dr-search input::placeholder { color: var(--text-3) }

  .dr-add {
    position: relative;
    font-family: var(--sans); font-size: 12px; font-weight: 600;
    background: var(--accent); color: var(--accent-ink);
    border: 0; border-radius: 7px;
    padding: 0 12px; height: 32px; cursor: pointer;
    display: inline-flex; align-items: center; gap: 5px;
  }
  .dr-add:hover { filter: brightness(1.08) }

  .dr-add-menu {
    position: absolute; top: calc(100% + 6px); right: 0; min-width: 200px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 4px;
    box-shadow: 0 20px 40px -15px rgba(0,0,0,.5);
    z-index: 5;
    display: none;
  }
  .dr-add-menu.on { display: block }
  .dr-add-menu button {
    display: flex; flex-direction: column; align-items: flex-start;
    gap: 2px; width: 100%;
    font-family: var(--sans); font-size: 13px; color: var(--text);
    background: transparent; border: 0; padding: 8px 10px;
    border-radius: 5px; cursor: pointer; text-align: left;
  }
  .dr-add-menu button:hover { background: var(--surface-2) }
  .m-sub { font-family: var(--sans); font-size: 12px; color: var(--text-3) }

  .app-list { display: flex; flex-direction: column; gap: 2px }
  .app {
    display: flex; align-items: center; gap: 11px;
    padding: 9px 10px; border-radius: 7px;
    border: 1px solid transparent;
  }
  .app:hover { background: var(--surface-2) }
  .app-ico {
    width: 30px; height: 30px; border-radius: 7px; flex-shrink: 0;
    background: linear-gradient(135deg, var(--surface-3), var(--surface-2));
    border: 1px solid var(--border);
    display: grid; place-items: center;
    font-weight: 600; font-size: 12px; color: var(--text-2);
    text-transform: uppercase; letter-spacing: -.02em;
  }
  .app-info { flex: 1; min-width: 0 }
  .app-name { font-size: 13px; font-weight: 500; letter-spacing: -.01em; color: var(--text) }
  .app-path {
    font-family: var(--sans); font-size: 12px; color: var(--text-3);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    margin-top: 2px;
  }
  .app.off .app-name, .app.off .app-ico { color: var(--text-3); opacity: .65 }

  .tgl {
    width: 32px; height: 19px; border-radius: 999px;
    background: var(--surface-3); border: 1px solid var(--border);
    position: relative; cursor: pointer; flex-shrink: 0;
    transition: all .15s;
  }
  .tgl::after {
    content: ""; position: absolute; top: 1px; left: 1px;
    width: 15px; height: 15px; border-radius: 50%;
    background: var(--text-3); transition: all .15s;
  }
  .tgl.on { background: color-mix(in oklab, var(--accent) 30%, var(--surface-3)); border-color: color-mix(in oklab, var(--accent) 50%, var(--border)) }
  .tgl.on::after { left: 14px; background: var(--accent) }

  .app-del {
    width: 26px; height: 26px; border-radius: 5px;
    border: 0; background: transparent; color: var(--text-3);
    cursor: pointer; display: grid; place-items: center;
    opacity: 0; transition: opacity .12s;
  }
  .app:hover .app-del { opacity: 1 }
  .app-del:hover { background: var(--bad); color: #fff }

  .empty-hint {
    text-align: center; color: var(--text-3);
    font-family: var(--sans); font-size: 12px;
    padding: 24px 12px;
  }

</style>
