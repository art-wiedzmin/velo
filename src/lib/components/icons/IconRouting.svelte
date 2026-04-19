<script lang="ts">
  let { size = 16, stroke = "currentColor" }: { size?: number; stroke?: string } = $props();
</script>
<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={stroke} stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <circle cx="6" cy="19" r="3"></circle>
  <circle cx="18" cy="5" r="3"></circle>
  <path d="M6 16V8a4 4 0 0 1 4-4h4"></path>
  <path d="M18 8v8a4 4 0 0 1-4 4h-4"></path>
</svg>
