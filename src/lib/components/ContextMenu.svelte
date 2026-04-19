<script lang="ts">
  // Floating context menu anchored at click coordinates. Closed by
  // outside click, Escape, scroll, or after any item runs.
  //
  // Edit / Copy URL are intentionally absent: we do not currently store
  // the original profile URL on the backend, and reconstructing it from
  // parsed fields is non-trivial. Add when a `profile_to_url` Rust helper
  // lands.

  import { onMount } from "svelte";
  import IconPlay from "$lib/components/icons/IconPlay.svelte";
  import IconStar from "$lib/components/icons/IconStar.svelte";
  import IconPencil from "$lib/components/icons/IconPencil.svelte";
  import IconCopy from "$lib/components/icons/IconCopy.svelte";
  import IconTrash from "$lib/components/icons/IconTrash.svelte";
  import * as api from "$lib/api";
  import { catalog, core, toast } from "$lib/state.svelte";
  import type { StoredProfile } from "$lib/types";

  let target = $state<{ profile: StoredProfile; x: number; y: number } | null>(null);

  export function open(e: MouseEvent, profile: StoredProfile) {
    // Clamp to viewport (menu width ~ 220px, height grows with items).
    const W = 220;
    const H = 240;
    const x = Math.min(e.clientX, window.innerWidth - W - 8);
    const y = Math.min(e.clientY, window.innerHeight - H - 8);
    target = { profile, x, y };
  }

  function close() { target = null }

  onMount(() => {
    // Don't close on scroll: the app has scrollable inner panels (logs,
    // profile grid) whose scroll bubbles via capture and would dismiss the
    // menu mid-click. The menu is position:fixed, so internal scroll doesn't
    // misalign it. Viewport itself never scrolls (body is overflow:hidden).
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") close() };
    const onResize = () => close();
    window.addEventListener("keydown", onKey);
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("resize", onResize);
    };
  });

  async function doConnect() {
    if (!target) return;
    const p = target.profile;
    close();
    try {
      if (core.state.kind === "connected" && core.state.active.id === p.id) return;
      if (core.state.kind === "connected") await core.switchTo(p);
      else await core.connect(p);
    } catch (e) {
      toast.show(String(e));
    }
  }

  function doFavorite() {
    if (!target) return;
    const p = target.profile;
    const wasFav = catalog.isFavorite(p.id);
    catalog.toggleFavorite(p.id);
    toast.show(wasFav ? "Removed from favorites" : "Added to favorites");
    close();
  }

  function doRename() {
    if (!target) return;
    catalog.startRename(target.profile.id);
    close();
  }

  async function doDuplicate() {
    if (!target) return;
    const p = target.profile;
    close();
    try {
      await api.profilesDuplicate(p.id);
      await catalog.refresh();
      toast.show("Profile duplicated");
    } catch (e) {
      toast.show(String(e));
    }
  }

  async function doDelete() {
    if (!target) return;
    const p = target.profile;
    if (core.state.kind === "connected" && core.state.active.id === p.id) {
      toast.show("Disconnect before deleting the active profile");
      close();
      return;
    }
    close();
    try {
      await api.profilesDelete(p.id);
      await catalog.refresh();
      toast.show("Profile deleted");
    } catch (e) {
      toast.show(String(e));
    }
  }

  let isFav = $derived(target ? catalog.isFavorite(target.profile.id) : false);
  let isActive = $derived(
    !!target && core.state.kind === "connected" && core.state.active.id === target.profile.id,
  );
</script>

<svelte:window onclick={(e) => { if (target && !(e.target as Element).closest(".ctxmenu")) close() }} />

{#if target}
  <div
    class="ctxmenu"
    role="menu"
    style="left: {target.x}px; top: {target.y}px"
  >
    <div class="hd">
      <b>{target.profile.profile.name}</b>
    </div>
    <button onclick={doConnect} disabled={isActive}>
      <IconPlay />
      <span>Connect</span>
    </button>
    <button onclick={doFavorite}>
      <IconStar filled={isFav} />
      <span>{isFav ? "Remove favorite" : "Add favorite"}</span>
    </button>
    <div class="sep"></div>
    <button onclick={doRename}>
      <IconPencil />
      <span>Rename</span>
    </button>
    <button onclick={doDuplicate}>
      <IconCopy />
      <span>Duplicate</span>
    </button>
    <button class="danger" onclick={doDelete}>
      <IconTrash />
      <span>Delete</span>
    </button>
  </div>
{/if}

<style>
  .ctxmenu {
    position: fixed; z-index: 60; min-width: 220px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 9px;
    padding: 4px;
    box-shadow: 0 24px 48px -12px rgba(0,0,0,.55), 0 0 0 1px rgba(255,255,255,.02);
    font-family: var(--sans);
  }
  .hd {
    font-family: var(--sans); font-size: 12px; color: var(--text-3);
    text-transform: uppercase; letter-spacing: .5px;
    padding: 8px 10px 6px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 4px;
    display: flex; align-items: center; gap: 8px;
  }
  .hd b {
    color: var(--text); font-family: var(--sans); font-size: 12px;
    font-weight: 600; text-transform: none; letter-spacing: 0;
  }
  button {
    display: flex; align-items: center; gap: 10px; width: 100%;
    font-family: var(--sans); font-size: 13px; color: var(--text);
    background: transparent; border: 0; padding: 8px 10px;
    border-radius: 6px; cursor: pointer; text-align: left;
  }
  button:hover:not(:disabled) { background: var(--surface-2) }
  button:disabled { opacity: .4; cursor: not-allowed }
  button :global(svg) { color: var(--text-3); flex-shrink: 0; width: 14px; height: 14px }
  .danger { color: var(--bad) }
  .danger :global(svg) { color: var(--bad) }
  .danger:hover { background: color-mix(in oklab, var(--bad) 12%, transparent) }
  .sep { height: 1px; background: var(--border); margin: 4px 2px }
</style>
