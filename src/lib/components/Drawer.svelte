<script lang="ts">
  // Shared right-side drawer shell: scrim + slide-in panel + header + close.
  // Body content is supplied via the default snippet.

  import IconClose from "$lib/components/icons/IconClose.svelte";

  let {
    open,
    title,
    tag,
    onClose,
    children,
  }: {
    open: boolean;
    title: string;
    tag?: string;
    onClose: () => void;
    children: import("svelte").Snippet;
  } = $props();
</script>

<div
  class="dr-scrim"
  class:on={open}
  role="presentation"
  onclick={(e) => { if (e.target === e.currentTarget) onClose() }}
></div>
<div class="drawer" class:on={open} role="dialog" aria-label={title} aria-hidden={!open}>
  <header class="dr-head">
    <h3>{title}</h3>
    <div class="dr-actions">
      {#if tag}<span class="tag">{tag}</span>{/if}
      <button class="dr-close" aria-label="Close" onclick={onClose}>
        <IconClose />
      </button>
    </div>
  </header>
  <div class="dr-body">{@render children()}</div>
</div>

<style>
  .dr-scrim {
    position: fixed; inset: 0;
    background: rgba(0,0,0,.4);
    z-index: 90;
    animation: fade .15s ease-out;
    display: none;
  }
  .dr-scrim.on { display: block }
  .drawer {
    position: fixed; top: 0; right: 0; bottom: 0;
    width: 460px; max-width: 95vw;
    background: var(--surface);
    border-left: 1px solid var(--border);
    z-index: 95;
    display: flex; flex-direction: column;
    transform: translateX(100%);
    transition: transform .25s cubic-bezier(.2,.7,.3,1);
    box-shadow: -30px 0 60px -20px rgba(0,0,0,.5);
  }
  .drawer.on { transform: translateX(0) }
  .dr-head {
    display: flex; align-items: center; justify-content: space-between;
    padding: 16px 18px 12px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .dr-head h3 { margin: 0; font-size: 16px; font-weight: 600; letter-spacing: -.01em; line-height: 1 }
  .dr-actions { display: flex; align-items: center; gap: 10px }
  .tag {
    font-family: var(--sans); font-size: 12px; line-height: 1; color: var(--text-3);
    padding: 5px 9px; border: 1px solid var(--border); border-radius: 999px;
    display: inline-flex; align-items: center;
  }
  .dr-close {
    width: 28px; height: 28px; border-radius: 6px;
    border: 0; background: transparent; color: var(--text-3);
    cursor: pointer; display: grid; place-items: center;
  }
  .dr-close:hover { background: var(--surface-2); color: var(--text) }

  .dr-body { flex: 1; min-height: 0; overflow-y: auto; padding: 14px 18px }
</style>
