<script lang="ts">
  import { toast } from "$lib/state.svelte";
</script>

<div class="toast" class:on={toast.visible} role="status" aria-live="polite">
  {toast.message ?? ""}
</div>

<style>
  .toast {
    position: fixed;
    bottom: 24px; left: 50%;
    transform: translateX(-50%) translateY(8px);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 14px;
    font-size: 13px; color: var(--text);
    box-shadow: 0 16px 32px -10px rgba(0,0,0,.5);
    opacity: 0; pointer-events: none;
    transition: all .18s ease;
    font-family: var(--sans);
    z-index: 70;
  }
  .toast.on { opacity: 1; transform: translateX(-50%) translateY(0) }
</style>
