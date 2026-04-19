<script lang="ts">
  // Profile URL tab: paste a single vless://, vmess://, trojan://, ss://, hy2://, tuic:// URL.
  // parseAny runs through the Rust parser so the preview reflects exactly what would be saved.
  // Parse errors are surfaced inline because they are user-input problems, not system failures.

  import * as api from "$lib/api";
  import { catalog, toast } from "$lib/state.svelte";
  import type { Profile } from "$lib/types";

  let { onSaved }: { onSaved: () => void } = $props();

  let urlInput = $state("");
  let urlBusy = $state(false);
  let urlError = $state<string | null>(null);
  let urlPreview = $state<Profile | null>(null);

  async function parseUrl() {
    urlError = null;
    urlPreview = null;
    const s = urlInput.trim();
    if (!s) return;
    urlBusy = true;
    try {
      urlPreview = await api.parseAny(s);
    } catch (e) {
      urlError = String(e);
    } finally {
      urlBusy = false;
    }
  }

  async function saveUrl() {
    if (!urlPreview) return;
    urlBusy = true;
    try {
      await api.profilesAdd(urlPreview);
      await catalog.refresh();
      toast.show("Profile added");
      onSaved();
    } catch (e) {
      urlError = String(e);
    } finally {
      urlBusy = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      urlPreview ? saveUrl() : parseUrl();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="field">
  <label for="url">Profile URL</label>
  <textarea
    id="url"
    class="input"
    bind:value={urlInput}
    placeholder="vless://… or vmess://… or trojan://… or ss://…"
    spellcheck="false"
    autocomplete="off"
  ></textarea>
  <div class="proto-list">
    <span class="p">vless</span>
    <span class="p">vmess</span>
    <span class="p">trojan</span>
    <span class="p">shadowsocks</span>
    <span class="p">hysteria2</span>
    <span class="p">tuic</span>
  </div>
</div>

{#if urlError}
  <div class="err">{urlError}</div>
{/if}

{#if urlPreview}
  <div class="preview">
    <div class="p-row"><span class="k">name</span><span class="v">{urlPreview.name}</span></div>
    <div class="p-row"><span class="k">protocol</span><span class="v">{urlPreview.protocol}</span></div>
    <div class="p-row"><span class="k">address</span><span class="v">{urlPreview.address}:{urlPreview.port}</span></div>
    <div class="p-row"><span class="k">transport</span><span class="v">{urlPreview.transport} · {urlPreview.security}</span></div>
  </div>
{/if}

<div class="actions">
  <span class="hint">⌘/Ctrl + Enter to submit · Esc to close</span>
  {#if !urlPreview}
    <button class="btn-primary" onclick={parseUrl} disabled={urlBusy || !urlInput.trim()}>
      {urlBusy ? "Parsing…" : "Parse"}
    </button>
  {:else}
    <button class="btn-ghost" onclick={() => { urlPreview = null; }}>Edit</button>
    <button class="btn-primary" onclick={saveUrl} disabled={urlBusy}>
      {urlBusy ? "Saving…" : "Save profile"}
    </button>
  {/if}
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

  .proto-list { display: flex; gap: 4px; flex-wrap: wrap; margin-top: 4px }
  .proto-list .p {
    font-family: var(--sans); font-size: 12px; color: var(--text-3);
    padding: 2px 6px; background: var(--surface-2);
    border-radius: 4px; border: 1px solid var(--border);
  }

  .err {
    font-family: var(--sans); font-size: 12px; color: var(--bad);
    background: color-mix(in oklab, var(--bad) 10%, transparent);
    border: 1px solid color-mix(in oklab, var(--bad) 40%, transparent);
    border-radius: 6px; padding: 8px 10px;
    white-space: pre-wrap; word-break: break-word;
  }

  .preview {
    margin-top: 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 7px; padding: 10px 12px;
    display: flex; flex-direction: column; gap: 6px;
  }
  .p-row { display: flex; justify-content: space-between; gap: 10px; font-family: var(--sans); font-size: 12px }
  .p-row .k { color: var(--text-3); text-transform: uppercase; letter-spacing: .5px; font-size: 12px }
  .p-row .v { color: var(--text); text-align: right; overflow-wrap: anywhere }

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
  .btn-ghost {
    font-family: var(--sans); font-size: 13px; font-weight: 500;
    background: transparent; color: var(--text-2);
    border: 0; border-radius: 7px;
    padding: 8px 14px; cursor: pointer;
  }
  .btn-ghost:hover { color: var(--text); background: var(--surface-3) }
</style>
