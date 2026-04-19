<script lang="ts">
  // Subscription tab: insert the row, then fire-and-surface a sync.
  // Sync failure is non-fatal — the row persists with last_error and the
  // Settings → Subscriptions pane offers a retry.

  import * as api from "$lib/api";
  import { catalog, toast } from "$lib/state.svelte";

  let { onSaved }: { onSaved: () => void } = $props();

  let subName = $state("");
  let subUrl = $state("");
  let subBusy = $state(false);
  let subError = $state<string | null>(null);

  async function saveSub() {
    subError = null;
    const name = subName.trim();
    const url = subUrl.trim();
    if (!name || !url) {
      subError = "Name and URL are required";
      return;
    }
    subBusy = true;
    try {
      const created = await api.subscriptionsAdd(name, url);
      try {
        await api.subscriptionsSync(created.id);
        toast.show("Subscription imported and synced");
      } catch (e) {
        toast.show(`Subscription saved but sync failed: ${e}`);
      }
      await catalog.refresh();
      onSaved();
    } catch (e) {
      const msg = String(e);
      if (msg.includes("already exists")) {
        subError = "This subscription URL is already imported. Open Settings → Subscriptions to sync or remove it.";
      } else {
        subError = msg;
      }
    } finally {
      subBusy = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) saveSub();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="field">
  <label for="sub-name">Name</label>
  <input id="sub-name" class="input" bind:value={subName} placeholder="Provider name" />
</div>
<div class="field">
  <label for="sub-url">Subscription URL</label>
  <textarea
    id="sub-url"
    class="input"
    bind:value={subUrl}
    placeholder="https://example.org/sub/abc123"
    spellcheck="false"
    autocomplete="off"
  ></textarea>
</div>
{#if subError}<div class="err">{subError}</div>{/if}

<div class="actions">
  <span class="hint">⌘/Ctrl + Enter to submit · Esc to close</span>
  <button class="btn-primary" onclick={saveSub} disabled={subBusy}>
    {subBusy ? "Importing…" : "Import"}
  </button>
</div>

<style>
  .field { display: flex; flex-direction: column; gap: 6px; margin-bottom: 14px }
  .field label {
    font-family: var(--sans); font-size: 12px;
    text-transform: uppercase; letter-spacing: .6px;
    color: var(--text-3);
  }
  .input {
    font-family: var(--sans); font-size: 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 7px;
    color: var(--text);
    padding: 9px 11px;
    outline: 0;
    transition: border-color .12s;
  }
  .input:focus { border-color: var(--accent) }
  textarea.input { min-height: 80px; resize: vertical; font-family: var(--sans); line-height: 1.5 }

  .err {
    font-family: var(--sans); font-size: 12px; color: var(--bad);
    background: color-mix(in oklab, var(--bad) 10%, transparent);
    border: 1px solid color-mix(in oklab, var(--bad) 40%, transparent);
    border-radius: 6px; padding: 8px 10px;
    white-space: pre-wrap; word-break: break-word;
  }

  .hint { font-family: var(--sans); font-size: 12px; color: var(--text-3); margin-right: auto }
  .actions { display: flex; gap: 8px; align-items: center; margin-top: 16px }
  .btn-primary {
    font-family: var(--sans); font-size: 13px; font-weight: 600;
    background: var(--accent); color: var(--accent-ink);
    border: 0; border-radius: 7px;
    padding: 8px 16px; cursor: pointer;
  }
  .btn-primary:hover:not(:disabled) { filter: brightness(1.08) }
  .btn-primary:disabled { opacity: .5; cursor: not-allowed }
</style>
