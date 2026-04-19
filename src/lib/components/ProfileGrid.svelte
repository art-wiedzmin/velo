<script lang="ts">
  // Auto-fill grid: at least 240px columns, expanding to fill width.
  // Cards live inside a scrollable wrapper so the bottom strip stays
  // pinned. EmptyState replaces the grid contents (and the +Add card)
  // when filtered to zero so the user gets one clear CTA.

  import { catalog } from "$lib/state.svelte";
  import type { StoredProfile } from "$lib/types";
  import ProfileCard from "./ProfileCard.svelte";
  import AddProfileCard from "./AddProfileCard.svelte";
  import EmptyState from "./EmptyState.svelte";

  let {
    onAdd,
    onImport,
    onContextMenu,
  }: {
    onAdd: () => void;
    onImport: () => void;
    onContextMenu: (e: MouseEvent, p: StoredProfile) => void;
  } = $props();

  let visible = $derived(catalog.filtered);
  let isEmpty = $derived(visible.length === 0);

  function subscriptionFor(p: StoredProfile) {
    if (p.subscription_id == null) return null;
    return catalog.subscriptions.find((s) => s.id === p.subscription_id) ?? null;
  }
</script>

<div class="scroll">
  <div class="grid" class:empty-grid={isEmpty}>
    {#if isEmpty}
      <EmptyState onAdd={onAdd} onImport={onImport} />
    {:else}
      {#each visible as p (p.id)}
        <ProfileCard
          profile={p}
          subscription={subscriptionFor(p)}
          onContextMenu={onContextMenu}
        />
      {/each}
      <AddProfileCard onClick={onAdd} />
    {/if}
  </div>
</div>

<style>
  .scroll {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    margin-right: -6px;
    padding-right: 6px;
    scrollbar-gutter: stable;
  }
  .grid {
    display: grid;
    gap: 12px;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    /* 2px breathing room so the 1px hover lift doesn't clip the top-row
       card outline against the scroll viewport edge. */
    padding: 2px 0;
    /* stretch (not start) so row-siblings match the tallest card. Mixed
       content — a subscription row with quota+expiry vs a bare profile vs
       the add-card — would otherwise render with slightly different heights. */
    align-items: stretch;
    align-content: start;
  }
  .grid > :global(*) { min-width: 0 }
  .grid.empty-grid {
    min-height: 100%;
    grid-auto-rows: 1fr;
  }
</style>
