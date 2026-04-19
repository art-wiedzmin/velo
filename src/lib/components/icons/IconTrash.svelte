<script lang="ts">
  // Two trash variants exist in callers:
  //   short  — polyline + lid path (RoutingDrawer app-del)
  //   full   — adds the handle stroke (ContextMenu Delete)
  // Default to `full` so the ContextMenu use is the natural one.
  let {
    size = 14,
    stroke = "currentColor",
    variant = "full",
  }: { size?: number; stroke?: string; variant?: "full" | "short" } = $props();
</script>
<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={stroke} stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <polyline points="3 6 5 6 21 6"></polyline>
  {#if variant === "full"}
    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
  {:else}
    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"></path>
  {/if}
</svg>
