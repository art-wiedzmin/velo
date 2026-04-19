<script lang="ts">
  let {
    onAdd,
    onImport,
  }: { onAdd: () => void; onImport: () => void } = $props();
</script>

<div class="empty">
  <div class="empty-ico" aria-hidden="true">
    <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <rect x="3" y="3" width="7" height="7"></rect>
      <rect x="14" y="3" width="7" height="7"></rect>
      <rect x="3" y="14" width="7" height="7"></rect>
      <rect x="14" y="14" width="7" height="7"></rect>
    </svg>
  </div>
  <div class="empty-title">No profiles yet</div>
  <div class="empty-sub">
    Paste a vless://, vmess://, trojan:// or ss:// link, or import a subscription
    URL to populate the grid.
  </div>
  <div class="empty-actions">
    <button class="empty-btn" onclick={onAdd}>+ Add profile</button>
    <button class="empty-btn ghost" onclick={onImport}>Import subscription</button>
  </div>
</div>

<style>
  .empty {
    grid-column: 1 / -1;
    align-self: stretch;
    display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    gap: 14px; padding: 56px 24px; text-align: center;
    border: 1px dashed var(--border);
    border-radius: var(--radius);
    background: color-mix(in oklab, var(--surface-2) 40%, transparent);
    min-height: 260px;
  }
  .empty-ico {
    width: 48px; height: 48px; border-radius: 12px;
    background: color-mix(in oklab, var(--accent) 12%, var(--surface-2));
    border: 1px solid color-mix(in oklab, var(--accent) 30%, var(--border));
    display: flex; align-items: center; justify-content: center;
    color: var(--accent);
  }
  .empty-title { font-size: 15px; font-weight: 600; letter-spacing: -.01em }
  .empty-sub {
    font-family: var(--sans); font-size: 12px; color: var(--text-3);
    max-width: 360px; line-height: 1.6;
  }
  .empty-actions { display: flex; gap: 8px; margin-top: 4px }
  .empty-btn {
    font-family: var(--sans); font-size: 12px; font-weight: 500;
    background: var(--accent); color: var(--accent-ink);
    border: 0; border-radius: 7px;
    padding: 0 14px; height: 32px; cursor: pointer;
    display: inline-flex; align-items: center; gap: 6px;
  }
  .empty-btn:hover { filter: brightness(1.08) }
  .empty-btn.ghost {
    background: transparent; color: var(--text-2);
    border: 1px solid var(--border);
  }
  .empty-btn.ghost:hover { background: var(--surface-2); color: var(--text) }
</style>
