<script lang="ts">
  let { onClick }: { onClick: () => void } = $props();
</script>

<button
  type="button"
  class="card add"
  onclick={onClick}
  aria-label="Add profile"
>
  <div class="add-plus">+</div>
  <div class="add-label">Add profile</div>
  <div class="add-hint">Profile URL · Subscription · Manual</div>
</button>

<style>
  .card {
    position: relative;
    background: transparent;
    border: 1px solid var(--border);
    border-style: dashed;
    border-radius: var(--radius);
    padding: 14px;
    cursor: pointer;
    display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    gap: 10px;
    text-align: center;
    min-height: 168px;
    transition: all .15s cubic-bezier(.2,.7,.3,1);
    color: inherit;
    font: inherit;
  }
  .card:hover {
    background: var(--surface-2);
    border-color: var(--border-strong);
  }
  .add-plus {
    width: 40px; height: 40px; border-radius: 50%;
    border: 1.5px dashed var(--border-strong);
    display: grid; place-items: center;
    color: var(--text-2); font-size: 22px; font-weight: 300;
  }
  .card:hover .add-plus { color: var(--text); border-color: var(--text-2) }
  .add-label { font-size: 14px; font-weight: 500; color: var(--text-2) }
  .add-hint {
    font-family: var(--sans); font-size: 12px; color: var(--text-3);
    line-height: 1.4; max-width: 220px;
  }
</style>
