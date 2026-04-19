<script lang="ts">
  let { size = 14, stroke = "currentColor" }: { size?: number; stroke?: string } = $props();
</script>
<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={stroke} stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline>
</svg>
