<script lang="ts">
  import { fmtBytes, daysFrom } from "$lib/format";
  import type { StoredSubscription } from "$lib/types";

  let { sub }: { sub: StoredSubscription } = $props();

  let subData = $derived.by(() => {
    const used = sub.used_bytes;
    const total = sub.total_bytes;
    const expires = sub.expires_at;
    const now = Date.now();
    const days = expires ? daysFrom(now, expires * 1000) : null;
    const pct = (used != null && total != null && total > 0) ? Math.min(100, (used / total) * 100) : null;
    const quotaTone = pct == null ? "" : pct >= 90 ? "bad" : pct >= 75 ? "warn" : "";
    const expTone = days == null ? "" : days < 0 ? "dead" : days <= 7 ? "soon" : "";
    return { used, total, expires, days, pct, quotaTone, expTone };
  });
</script>

{#if subData.total != null || subData.used != null || subData.expires != null}
  <div class="sub-meta">
    {#if subData.total != null || subData.used != null}
      <div class="sub-quota">
        <span class="k">quota</span>
        <span class="v">
          {#if subData.total == null}
            <span class="inf">∞</span>
          {:else}
            <b>{subData.used != null ? fmtBytes(subData.used) : "—"}</b> / {fmtBytes(subData.total)}
          {/if}
        </span>
      </div>
      <div class="sub-bar" class:warn={subData.quotaTone === "warn"} class:bad={subData.quotaTone === "bad"} class:inf={subData.total == null}>
        {#if subData.pct != null}<i style="width: {subData.pct}%"></i>{/if}
      </div>
    {/if}
    {#if subData.expires != null}
      <div class="sub-expiry">
        <span>active until</span>
        <span class="exp-v" class:soon={subData.expTone === "soon"} class:dead={subData.expTone === "dead"}>
          {#if subData.days !== null && subData.days < 0}expired{:else if subData.days !== null}{subData.days}d{:else}—{/if}
        </span>
      </div>
    {/if}
  </div>
{/if}

<style>
  .sub-meta {
    display: flex; flex-direction: column; gap: 4px;
    padding-top: 6px; margin-top: auto;
    border-top: 1px dashed var(--border);
  }
  .sub-quota {
    display: flex; justify-content: space-between; align-items: baseline;
    font-family: var(--sans); font-size: 12px; color: var(--text);
  }
  .sub-quota .k { color: var(--text-3); font-family: var(--sans); font-size: 12px }
  .sub-quota .v b { color: var(--text); font-weight: 600 }
  .sub-quota .v .inf {
    color: var(--text); font-size: 12px; line-height: 1;
    display: inline-block; transform: translateY(1px);
  }
  .sub-bar {
    height: 4px; border-radius: 999px; background: var(--surface-3); overflow: hidden;
    border: 1px solid var(--border);
  }
  .sub-bar > i {
    display: block; height: 100%; background: var(--accent);
    transition: width .4s;
  }
  .sub-bar.warn > i { background: var(--warn) }
  .sub-bar.bad > i { background: var(--bad) }
  .sub-bar.inf {
    background: repeating-linear-gradient(
      -45deg,
      transparent 0 6px,
      color-mix(in oklab, var(--accent) 25%, transparent) 6px 8px
    );
    border-color: color-mix(in oklab, var(--accent) 30%, var(--border));
  }
  .sub-bar.inf > i { display: none }
  .sub-expiry {
    display: flex; justify-content: space-between; align-items: center;
    font-family: var(--sans); font-size: 12px; color: var(--text-3);
  }
  .sub-expiry .exp-v { color: var(--text-2) }
  .sub-expiry .exp-v.soon { color: var(--warn) }
  .sub-expiry .exp-v.dead { color: var(--bad) }
</style>
