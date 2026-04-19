<script lang="ts">
  // Shared centered modal shell: backdrop-filter scrim + panel + header + close.
  // Escape closes; Ctrl/Cmd+Enter and other keys remain the consumer's concern.

  import IconClose from "$lib/components/icons/IconClose.svelte";

  let {
    open,
    title,
    onClose,
    children,
    footer,
  }: {
    open: boolean;
    title: string;
    onClose: () => void;
    children: import("svelte").Snippet;
    footer?: import("svelte").Snippet;
  } = $props();

  function onKeydown(e: KeyboardEvent) {
    if (open && e.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div
    class="scrim"
    role="presentation"
    onclick={(e) => { if (e.target === e.currentTarget) onClose() }}
  >
    <div class="modal" role="dialog" aria-modal="true" aria-label={title}>
      <header class="m-head">
        <h3>{title}</h3>
        <button class="close" aria-label="Close" onclick={onClose}>
          <IconClose />
        </button>
      </header>
      <div class="m-body">{@render children()}</div>
      {#if footer}<footer class="m-foot">{@render footer()}</footer>{/if}
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed; inset: 0;
    background: rgba(0,0,0,.6);
    backdrop-filter: blur(4px);
    display: flex; align-items: center; justify-content: center;
    z-index: 100;
    animation: fade .15s ease-out;
  }
  .modal {
    width: 560px; max-width: calc(100vw - 40px);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 14px;
    overflow: hidden;
    box-shadow: 0 40px 80px -20px rgba(0,0,0,.6);
    animation: rise .2s cubic-bezier(.2,.7,.3,1);
  }
  .m-head {
    padding: 18px 20px 14px;
    border-bottom: 1px solid var(--border);
    display: flex; align-items: center; justify-content: space-between;
  }
  .m-head h3 { margin: 0; font-size: 16px; font-weight: 600; letter-spacing: -.01em }
  .close {
    width: 28px; height: 28px; border-radius: 6px;
    border: 0; background: transparent; color: var(--text-3);
    cursor: pointer; display: grid; place-items: center;
  }
  .close:hover { background: var(--surface-2); color: var(--text) }

  .m-body { padding: 20px }
  .m-foot {
    padding: 14px 20px; border-top: 1px solid var(--border);
    display: flex; justify-content: space-between; align-items: center;
    background: var(--surface-2);
  }
</style>
