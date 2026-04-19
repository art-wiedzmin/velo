<script lang="ts">
  // Live log pane. Parsing + batching live in LogsStore so render is pure
  // reconciliation over pre-parsed lines keyed by a stable id.
  // Auto-scroll only if the user is already at the bottom — otherwise they're
  // reading history and a sudden jump would steal their place.
  // Raw lines still go to `%APPDATA%\com.velo.app\sing-box.log` untouched.

  import { tick } from "svelte";
  import { logs } from "$lib/state.svelte";

  let container: HTMLDivElement | undefined = $state();
  let stickBottom = $state(true);

  function onScroll() {
    if (!container) return;
    const atBottom =
      container.scrollTop + container.clientHeight >= container.scrollHeight - 4;
    stickBottom = atBottom;
  }

  $effect(() => {
    void logs.lines;
    if (!stickBottom || !container) return;
    void tick().then(() => {
      if (container) container.scrollTop = container.scrollHeight;
    });
  });
</script>

<div class="panel">
  <div class="p-head">
    <div class="p-title">Logs</div>
    <div class="p-sub">{logs.lines.length} lines {stickBottom ? "· autoscroll" : ""}</div>
    <button class="p-clear" aria-label="Clear logs" onclick={() => logs.clear()}>Clear</button>
  </div>
  <div class="log-list" bind:this={container} onscroll={onScroll}>
    {#each logs.lines as l (l.id)}
      <div class="log-line">
        {#if l.level}<span class="lvl {l.level}">{l.level}</span>{/if}
        <span class="msg">{l.text}</span>
      </div>
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
    min-height: 0;
    min-width: 0;
  }
  .p-head {
    display: flex; align-items: center; justify-content: space-between;
    gap: 10px; margin-bottom: 8px; flex-shrink: 0;
  }
  .p-title { font-size: 13px; font-weight: 600; letter-spacing: -.01em; margin-right: auto }
  .p-clear {
    font: inherit; font-size: 12px; color: var(--text-3);
    background: transparent; border: 1px solid var(--border);
    padding: 3px 10px; border-radius: 6px; cursor: pointer;
  }
  .p-clear:hover { color: var(--text); border-color: var(--border-strong) }
  .p-sub {
    font-family: var(--sans); font-size: 12px; color: var(--text-3);
  }
  .log-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .log-line {
    font-family: var(--sans); font-size: 12px; color: var(--text-2);
    line-height: 1.5;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    margin-bottom: 2px;
    flex-shrink: 0;
  }
  .lvl {
    display: inline-block;
    font-weight: 600;
    margin-right: 8px;
    text-transform: uppercase;
    letter-spacing: .4px;
    font-size: 12px;
    min-width: 44px;
  }
  .lvl.info  { color: var(--good) }
  .lvl.debug { color: var(--text-3) }
  .lvl.trace { color: var(--text-3) }
  .lvl.warn  { color: var(--warn) }
  .lvl.error { color: var(--bad) }
  .lvl.fatal { color: var(--bad) }
</style>
