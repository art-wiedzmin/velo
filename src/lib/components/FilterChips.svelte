<script lang="ts">
  // Single-select filter chips. Clicking the active chip is a no-op — to
  // clear the filter the user selects "All" explicitly. This matches the
  // design's decision to fold "clear" into the All chip rather than a
  // hover/close affordance.

  import { catalog } from "$lib/state.svelte";
  import type { FilterKey } from "$lib/types";

  const CHIPS: { key: FilterKey; label: string; icon?: string }[] = [
    { key: "all", label: "All" },
    { key: "favorites", label: "Favorites", icon: "★" },
    { key: "recent", label: "Recent" },
  ];

  let counts = $derived(catalog.counts);

  function select(key: FilterKey) {
    if (counts[key] === 0 && key !== "all") return;
    catalog.filter = key;
  }
</script>

<div class="filter-row">
  {#each CHIPS as chip, i (chip.key)}
    <button
      class="chip"
      class:on={catalog.filter === chip.key}
      class:off={counts[chip.key] === 0 && chip.key !== "all"}
      disabled={counts[chip.key] === 0 && chip.key !== "all"}
      onclick={() => select(chip.key)}
    >
      {#if chip.icon}<span class="ic">{chip.icon}</span>{/if}
      <span>{chip.label}</span>
      <span class="cnt">{counts[chip.key]}</span>
    </button>
  {/each}
</div>

<style>
  .filter-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .chip {
    font-family: var(--sans); font-size: 12px; font-weight: 500;
    background: var(--surface-2); border: 1px solid var(--border);
    color: var(--text-2);
    padding: 5px 11px; border-radius: 999px; cursor: pointer;
    display: inline-flex; align-items: center; gap: 6px;
    transition: all .12s;
  }
  .chip:hover:not(:disabled) { color: var(--text); border-color: var(--border-strong) }
  .chip.on {
    background: var(--accent-soft);
    border-color: color-mix(in oklab, var(--accent) 50%, var(--border));
    color: var(--accent);
  }
  .chip:disabled {
    opacity: .35;
    cursor: not-allowed;
  }
  .cnt { font-family: var(--sans); font-size: 12px; color: var(--text-3) }
  .chip.on .cnt { color: color-mix(in oklab, var(--accent) 80%, var(--text-2)) }
  .ic { color: var(--accent); font-size: 12px }
</style>
