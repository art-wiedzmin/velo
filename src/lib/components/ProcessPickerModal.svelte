<script lang="ts">
  // Modal for picking a running process. Loads the snapshot when opened.

  import * as api from "$lib/api";
  import { toast } from "$lib/state.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import type { ProcessInfo } from "$lib/types";

  let { open, onClose, onPick }: {
    open: boolean;
    onClose: () => void;
    onPick: (path: string, name: string | null) => void;
  } = $props();

  let processes = $state<ProcessInfo[]>([]);
  let pickerBusy = $state(false);
  let pickerSearch = $state("");

  async function pickFromRunning() {
    pickerBusy = true;
    try {
      processes = await api.routingProcessesSnapshot();
    } catch (e) {
      toast.show(String(e));
      onClose();
    } finally {
      pickerBusy = false;
    }
  }

  $effect(() => {
    if (open) void pickFromRunning();
  });

  function addFromPicker(p: ProcessInfo) {
    onPick(p.path, p.name);
    onClose();
  }

  const pickerFiltered = $derived(
    pickerSearch.trim() === ""
      ? processes
      : processes.filter((p) => {
          const q = pickerSearch.toLowerCase();
          return p.name.toLowerCase().includes(q) || p.path.toLowerCase().includes(q);
        }),
  );
</script>

<Modal {open} title="Add from running processes" {onClose}>
  <input class="input" placeholder="Filter…" bind:value={pickerSearch} />
  {#if pickerBusy}
    <div class="loading">Enumerating processes…</div>
  {:else}
    <div class="pick-list">
      {#each pickerFiltered as p (p.path)}
        <button class="pick-row" onclick={() => addFromPicker(p)}>
          <div class="pick-name">{p.name}</div>
          <div class="pick-path">{p.path}</div>
        </button>
      {:else}
        <div class="empty-hint">No processes match.</div>
      {/each}
    </div>
  {/if}
</Modal>

<style>
  .input {
    font-family: var(--sans); font-size: 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 7px;
    color: var(--text);
    padding: 9px 11px;
    outline: 0;
    transition: border-color .12s;
    margin-bottom: 10px;
    width: 100%;
    box-sizing: border-box;
  }
  .input:focus { border-color: var(--accent) }
  .loading { color: var(--text-3); font-family: var(--sans); font-size: 12px; padding: 16px }
  .pick-list { max-height: 50vh; overflow-y: auto; display: flex; flex-direction: column; gap: 2px }
  .pick-row {
    display: flex; flex-direction: column; align-items: flex-start;
    gap: 2px; width: 100%;
    font: inherit; text-align: left;
    background: transparent; border: 0;
    padding: 8px 10px; border-radius: 6px; cursor: pointer;
  }
  .pick-row:hover { background: var(--surface-2) }
  .pick-name { font-size: 13px; color: var(--text); font-weight: 500 }
  .pick-path { font-family: var(--sans); font-size: 12px; color: var(--text-3); overflow-wrap: anywhere }
  .empty-hint {
    text-align: center; color: var(--text-3);
    font-family: var(--sans); font-size: 12px;
    padding: 24px 12px;
  }
</style>
