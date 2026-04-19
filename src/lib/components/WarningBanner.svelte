<script lang="ts">
  // Inline warning banner: icon + title/message + optional action button.
  // Icon is a snippet so callers can pass any glyph (warning, lock, etc.).

  import IconWarning from "$lib/components/icons/IconWarning.svelte";

  let {
    title,
    message,
    action,
    onAction,
    icon,
  }: {
    title: string;
    message: string;
    action?: string;
    onAction?: () => void;
    icon?: import("svelte").Snippet;
  } = $props();
</script>

<div class="warn">
  {#if icon}{@render icon()}{:else}<IconWarning />{/if}
  <div class="warn-text">
    <div class="warn-title">{title}</div>
    <div class="warn-msg">{message}</div>
  </div>
  {#if action && onAction}
    <button class="warn-act" onclick={onAction}>{action}</button>
  {/if}
</div>

<style>
  .warn {
    display: flex; align-items: flex-start; gap: 10px;
    padding: 10px 12px; border-radius: 8px;
    background: color-mix(in oklab, var(--warn) 12%, var(--surface-2));
    border: 1px solid color-mix(in oklab, var(--warn) 35%, var(--border));
    margin-bottom: 14px;
  }
  .warn :global(svg) { color: var(--warn); flex-shrink: 0; margin-top: 1px }
  .warn-text { flex: 1; min-width: 0 }
  .warn-title { font-size: 12px; font-weight: 600; color: var(--text) }
  .warn-msg { font-family: var(--sans); font-size: 12px; color: var(--text-2); margin-top: 2px; line-height: 1.5 }
  .warn-act {
    font-family: var(--sans); font-size: 12px; font-weight: 600;
    background: var(--warn); color: #1a1200;
    border: 0; border-radius: 6px; padding: 6px 10px; cursor: pointer;
    flex-shrink: 0; align-self: center;
  }
  .warn-act:hover { filter: brightness(1.1) }
</style>
