<script lang="ts">
  // Throughput panel: title + mini ↓/↑ stats row up top, full-width sparkline
  // pinned at the bottom. tabular-nums keeps digit widths stable so the
  // layout doesn't jitter when rates change order of magnitude.

  import { stats } from "$lib/state.svelte";
  import { fmtRate } from "$lib/format";

  const MAX_BARS = 60;


  let down = $derived(fmtRate(stats.latest.down));
  let up = $derived(fmtRate(stats.latest.up));

  let bars = $derived.by(() => {
    const h = stats.history;
    if (h.length === 0) return [] as number[];
    const max = h.reduce((m, s) => Math.max(m, s.up + s.down), 1);
    return h.slice(-MAX_BARS).map((s) => ((s.up + s.down) / max) * 100);
  });
</script>

<div class="panel throughput-panel">
  <div class="p-head">
    <div class="p-title">Throughput</div>
    <div class="p-sub">live · {stats.connections.length} conns</div>
  </div>

  <div class="tp-stats">
    <div class="tp-stat">
      <span class="arrow">↓</span>
      <span class="num">{down.v}</span>
      <span class="unit">{down.u}</span>
    </div>
    <div class="tp-stat">
      <span class="arrow">↑</span>
      <span class="num">{up.v}</span>
      <span class="unit">{up.u}</span>
    </div>
  </div>

  <div class="spark">
    {#each bars as h}
      <i style="height: {Math.max(1, h)}%"></i>
    {/each}
  </div>
</div>

<style>
  .panel {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 12px 14px;
    display: flex; flex-direction: column;
  }
  .p-head {
    display: flex; align-items: center; justify-content: space-between;
    gap: 10px; margin-bottom: 6px; min-width: 0;
  }
  .p-title { font-size: 13px; font-weight: 600; letter-spacing: -.01em; white-space: nowrap }
  .p-sub {
    font-family: var(--sans); font-size: 12px; color: var(--text-3); white-space: nowrap;
  }

  .tp-stats {
    display: flex; align-items: baseline; gap: 14px;
    font-variant-numeric: tabular-nums;
  }
  .tp-stat {
    display: inline-flex; align-items: baseline; gap: 5px;
    white-space: nowrap;
  }
  .arrow { color: var(--text-3); font-size: 13px; font-weight: 600 }
  .num {
    font-size: 16px; font-weight: 600; letter-spacing: -.01em; line-height: 1;
    color: var(--text);
  }
  .unit {
    font-family: var(--sans); font-size: 12px; color: var(--text-3);
    text-transform: uppercase; letter-spacing: .3px;
  }

  /* Pinned to bottom, full width; flex:1 on the panel's children above keeps
     the sparkline at the last row regardless of how tall the panel grows. */
  .spark {
    margin-top: auto;
    height: 36px;
    display: flex; align-items: flex-end; gap: 2px;
    padding-top: 8px;
  }
  .spark i {
    flex: 1;
    background: color-mix(in oklab, var(--accent) 70%, transparent);
    border-radius: 1px;
    display: inline-block;
    min-height: 1px;
  }
</style>
