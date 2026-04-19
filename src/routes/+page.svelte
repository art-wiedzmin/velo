<script lang="ts">
  import { onMount } from "svelte";
  import AddModal from "$lib/components/AddModal.svelte";
  import ContextMenu from "$lib/components/ContextMenu.svelte";
  import ErrorBanner from "$lib/components/ErrorBanner.svelte";
  import FilterChips from "$lib/components/FilterChips.svelte";
  import LogsPanel from "$lib/components/LogsPanel.svelte";
  import ProfileGrid from "$lib/components/ProfileGrid.svelte";
  import RoutingDrawer from "$lib/components/RoutingDrawer.svelte";
  import SettingsDrawer from "$lib/components/SettingsDrawer.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";
  import ThroughputPanel from "$lib/components/ThroughputPanel.svelte";
  import Toast from "$lib/components/Toast.svelte";
  import {
    attachEventListeners,
    catalog,
    core,
    routing,
    sysproxy,
  } from "$lib/state.svelte";
  import { runAutostartConnect } from "$lib/autostart";
  import type { StoredProfile } from "$lib/types";

  let showAdd = $state<false | "url" | "sub">(false);
  let showSettings = $state(false);
  let showRouting = $state(false);
  let ctxMenu: ContextMenu | undefined = $state();

  onMount(() => {
    let unlisten: (() => void) | null = null;
    // Sequential boot: fetch catalog first, then wire event listeners, then
    // attempt auto-connect. `runAutostartConnect` bails out unless Rust
    // reports this is an autostart launch AND the user opted in.
    const ready = (async () => {
      await Promise.all([
        catalog.refresh(),
        core.refresh(),
        sysproxy.refresh(),
        routing.refresh(),
      ]);
      unlisten = await attachEventListeners();
      await runAutostartConnect();
    })();

    // Kill the webview's default context menu ("Reload, Save as, Print…").
    // Our own menu handlers call preventDefault locally; this catches the
    // empty-space and non-card cases.
    const onCtx = (e: MouseEvent) => e.preventDefault();
    window.addEventListener("contextmenu", onCtx);

    return () => {
      window.removeEventListener("contextmenu", onCtx);
      ready.finally(() => unlisten?.());
    };
  });

  function openContextMenu(e: MouseEvent, p: StoredProfile) {
    ctxMenu?.open(e, p);
  }
</script>

<div class="app">
  <main class="body">
    <StatusBar
      onOpenSettings={() => (showSettings = true)}
      onOpenRouting={() => (showRouting = true)}
    />
    <ErrorBanner />
    <FilterChips />
    <ProfileGrid
      onAdd={() => (showAdd = "url")}
      onImport={() => (showAdd = "sub")}
      onContextMenu={openContextMenu}
    />
    <section class="strip">
      <ThroughputPanel />
      <LogsPanel />
    </section>
  </main>

  <Toast />
  <ContextMenu bind:this={ctxMenu} />

  {#if showAdd}
    <AddModal initialTab={showAdd} onClose={() => (showAdd = false)} />
  {/if}

  <SettingsDrawer open={showSettings} onClose={() => (showSettings = false)} />
  <RoutingDrawer open={showRouting} onClose={() => (showRouting = false)} />
</div>

<style>
  .app {
    height: 100vh;
    width: 100vw;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .body {
    flex: 1;
    min-height: 0;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .strip {
    display: grid;
    grid-template-columns: minmax(240px, 1fr) minmax(0, 2fr);
    gap: 12px;
    flex-shrink: 0;
    margin-top: auto;
    /* Fixed height: the strip must not grow/shrink with log volume,
       otherwise Clear (or startup with empty logs) visibly resizes the
       whole bottom row. */
    height: 200px;
  }
</style>
