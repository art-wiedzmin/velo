<script lang="ts">
  // Add-profile modal — shell + tab selector.
  // Per-tab state and submit logic live in AddFromUrlTab / AddFromSubTab.

  import Modal from "$lib/components/Modal.svelte";
  import AddFromUrlTab from "$lib/components/AddFromUrlTab.svelte";
  import AddFromSubTab from "$lib/components/AddFromSubTab.svelte";

  type Tab = "url" | "sub";
  let {
    onClose,
    initialTab = "url",
  }: { onClose: () => void; initialTab?: Tab } = $props();

  // svelte-ignore state_referenced_locally
  let tab = $state<Tab>(initialTab);
</script>

<Modal open={true} title="Add profile" {onClose}>
  <div class="m-tabs">
    <button class="m-tab" class:on={tab === "url"} onclick={() => (tab = "url")}>Profile URL</button>
    <button class="m-tab" class:on={tab === "sub"} onclick={() => (tab = "sub")}>Subscription</button>
  </div>

  {#if tab === "url"}
    <AddFromUrlTab onSaved={onClose} />
  {:else}
    <AddFromSubTab onSaved={onClose} />
  {/if}
</Modal>

<style>
  /* Tabs sit flush with modal edges: counter the shell's m-body padding. */
  .m-tabs {
    display: flex; gap: 4px;
    padding: 0 20px;
    margin: -20px -20px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .m-tab {
    font-family: var(--sans); font-size: 13px; font-weight: 500;
    color: var(--text-2);
    border: 0; background: transparent;
    padding: 10px 4px; margin-right: 16px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    position: relative; top: 1px;
  }
  .m-tab:hover { color: var(--text) }
  .m-tab.on { color: var(--text); border-bottom-color: var(--accent) }
</style>
