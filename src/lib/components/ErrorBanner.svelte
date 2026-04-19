<script lang="ts">
  import { core } from "$lib/state.svelte";
  import IconInfo from "$lib/components/icons/IconInfo.svelte";
  import IconClose from "$lib/components/icons/IconClose.svelte";

  let visible = $derived(core.state.kind === "error");
  let reason = $derived(core.state.kind === "error" ? core.state.reason : "");

  async function onRetry() {
    try { await core.retry(); } catch { /* state already reflects error */ }
  }
</script>

{#if visible}
  <div class="err-banner">
    <IconInfo stroke="var(--bad)" />
    <div class="err-text">
      <div class="err-title">Connection failed</div>
      <div class="err-detail">{reason}</div>
    </div>
    <button class="err-retry" onclick={onRetry}>Retry</button>
    <button class="err-close" aria-label="Dismiss" onclick={() => core.dismissError()}>
      <IconClose />
    </button>
  </div>
{/if}

<style>
  .err-banner {
    display: flex; align-items: center; gap: 12px;
    padding: 10px 14px; border-radius: var(--radius);
    background: color-mix(in oklab, var(--bad) 12%, var(--surface-2));
    border: 1px solid color-mix(in oklab, var(--bad) 40%, var(--border));
    flex-shrink: 0;
  }
  .err-text { flex: 1; min-width: 0 }
  .err-title { font-size: 13px; font-weight: 600; color: var(--text); letter-spacing: -.01em }
  .err-detail {
    font-family: var(--sans); font-size: 12px; color: var(--text-2);
    margin-top: 2px;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .err-retry {
    font-family: var(--sans); font-size: 12px; font-weight: 600;
    background: transparent; color: var(--bad);
    border: 1px solid color-mix(in oklab, var(--bad) 50%, transparent);
    border-radius: 6px; padding: 6px 12px; cursor: pointer; height: 30px;
  }
  .err-retry:hover { background: color-mix(in oklab, var(--bad) 15%, transparent) }
  .err-close {
    background: transparent; border: 0; color: var(--text-3); cursor: pointer;
    width: 26px; height: 26px; display: grid; place-items: center; border-radius: 5px;
  }
  .err-close:hover { background: var(--surface-3); color: var(--text) }
</style>
