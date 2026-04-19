<script lang="ts">
  let { size = 14, stroke = "currentColor" }: { size?: number; stroke?: string } = $props();
</script>
<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={stroke} stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <polygon points="5 3 19 12 5 21 5 3"></polygon>
</svg>
