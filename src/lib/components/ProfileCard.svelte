<script lang="ts">
  // Server card. Renders both regular and subscription variants — the
  // subscription-specific bar + expiry line live under a conditional so
  // the DOM stays compact for non-sub rows.
  //
  // State-driven classes (active / connecting / switching-in / switching-out /
  // error) follow CoreStore:
  //   - active:         state.activeProfile.id === this.id && connected
  //   - connecting:     state=connecting && target.id === this.id
  //   - switching-in:   state=switching  && to.id === this.id
  //   - switching-out:  state=switching  && from.id === this.id
  //   - error:          state=error      && lastTarget?.id === this.id
  //
  // Clicking the card: if disconnected -> connect; if connected to another
  // card -> switchTo (triggers animated seamless switch); if connected to
  // this card -> no-op (user must use the Connect button to disconnect).

  import { catalog, core, toast } from "$lib/state.svelte";
  import type { StoredProfile, StoredSubscription } from "$lib/types";
  import SubscriptionQuota from "./SubscriptionQuota.svelte";

  let {
    profile,
    subscription = null,
    onContextMenu,
  }: {
    profile: StoredProfile;
    subscription?: StoredSubscription | null;
    onContextMenu?: (e: MouseEvent, p: StoredProfile) => void;
  } = $props();

  let isFav = $derived(catalog.isFavorite(profile.id));

  let active = $derived(
    core.state.kind === "connected" && core.state.active.id === profile.id,
  );
  let connecting = $derived(
    core.state.kind === "connecting" && core.state.target.id === profile.id,
  );
  let switchingIn = $derived(
    core.state.kind === "switching" && core.state.to.id === profile.id,
  );
  let switchingOut = $derived(
    core.state.kind === "switching" && core.state.from.id === profile.id,
  );
  let errored = $derived(
    core.state.kind === "error" && core.state.lastTarget?.id === profile.id,
  );
  let selected = $derived(catalog.selectedId === profile.id);

  async function onClick() {
    // Click semantics:
    //   - Already running: hot-swap to this card (was the only working path
    //     before; user explicitly keeps this).
    //   - Otherwise: just mark as selected — starting the runner is a
    //     separate, explicit action via the Connect button.
    if (active) return;
    if (core.state.kind === "connected" || core.state.kind === "switching") {
      try {
        await core.switchTo(profile);
        catalog.selectedId = profile.id;
      } catch (e) {
        toast.show(String(e));
      }
      return;
    }
    catalog.selectedId = profile.id;
  }

  function onStar(e: MouseEvent) {
    e.stopPropagation();
    catalog.toggleFavorite(profile.id);
    toast.show(isFav ? "Removed from favorites" : "Added to favorites");
  }

  function onCtxMenu(e: MouseEvent) {
    if (onContextMenu) {
      e.preventDefault();
      onContextMenu(e, profile);
    }
  }

  async function commitRename(value: string) {
    try {
      await catalog.commitRename(profile.id, value);
    } catch (e) {
      toast.show(String(e));
    }
  }
  function onRenameKey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      void commitRename((e.currentTarget as HTMLInputElement).value);
    } else if (e.key === "Escape") {
      e.preventDefault();
      catalog.cancelRename();
    }
    e.stopPropagation();
  }
  function onRenameBlur(e: FocusEvent) {
    void commitRename((e.currentTarget as HTMLInputElement).value);
  }
</script>

<div
  class="card"
  class:active
  class:selected
  class:connecting
  class:switching-in={switchingIn}
  class:switching-out={switchingOut}
  class:error={errored}
  class:sub={!!subscription}
  role="button"
  tabindex="0"
  onclick={onClick}
  onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onClick(); } }}
  oncontextmenu={onCtxMenu}
>
  <div class="c-head">
    <span class="proto">{profile.profile.protocol}</span>
    <div class="spacer"></div>
    <button
      class="star"
      class:on={isFav}
      aria-label={isFav ? "Remove from favorites" : "Add to favorites"}
      aria-pressed={isFav}
      onclick={onStar}
    >
      <svg width="13" height="13" viewBox="0 0 24 24" fill={isFav ? "currentColor" : "none"} stroke="currentColor" stroke-width="2" stroke-linejoin="round">
        <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"></polygon>
      </svg>
    </button>
  </div>

  <div>
    {#if catalog.renamingId === profile.id}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="c-name-input"
        type="text"
        value={profile.profile.name}
        autofocus
        onkeydown={onRenameKey}
        onblur={onRenameBlur}
        onclick={(e) => e.stopPropagation()}
      />
    {:else}
      <div class="c-name" title={profile.profile.name}>{profile.profile.name}</div>
    {/if}
    <div class="c-host">{profile.profile.address}:{profile.profile.port}</div>
  </div>

  {#if subscription}<SubscriptionQuota sub={subscription} />{/if}

  <div class="c-foot">
    {#if subscription}<span class="badge">sub</span>{/if}
    {#if active}<span class="badge" style="margin-left:auto">active</span>{/if}
  </div>
</div>

<style>
  .card {
    position: relative;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px;
    cursor: pointer;
    display: flex; flex-direction: column; gap: 10px;
    min-height: 168px;
    transition: all .15s cubic-bezier(.2,.7,.3,1);
    overflow: hidden;
  }
  .card::after {
    content: ""; position: absolute; inset: 0; border-radius: inherit; pointer-events: none;
    background: linear-gradient(135deg, rgba(255,255,255,.03), transparent 40%);
  }
  .card:hover { border-color: var(--border-strong); transform: translateY(-1px) }
  .card:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px }
  .card.selected:not(.active):not(.connecting):not(.switching-in):not(.error) {
    border-color: color-mix(in oklab, var(--accent) 45%, var(--border));
    box-shadow: 0 0 0 1px color-mix(in oklab, var(--accent) 22%, transparent) inset;
  }
  .card.active {
    background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 8%, var(--surface-2)), var(--surface-2));
    border-color: color-mix(in oklab, var(--accent) 60%, var(--border));
    box-shadow: 0 0 0 1px color-mix(in oklab, var(--accent) 30%, transparent) inset;
  }
  .card.connecting {
    border-color: color-mix(in oklab, var(--warn) 60%, var(--border));
  }
  .card.connecting::before {
    content: ""; position: absolute; inset: 0; border-radius: inherit; pointer-events: none;
    border: 1px solid color-mix(in oklab, var(--warn) 60%, transparent);
    animation: breathe 1.4s ease-in-out infinite;
  }
  .card.switching-in {
    border-color: color-mix(in oklab, var(--accent) 70%, var(--border));
    animation: switchIn .5s cubic-bezier(.2,.7,.3,1);
  }
  .card.switching-out {
    opacity: .55;
    transition: opacity .3s;
  }
  .card.error {
    border-color: color-mix(in oklab, var(--bad) 55%, var(--border));
    background: linear-gradient(180deg, color-mix(in oklab, var(--bad) 10%, var(--surface-2)), var(--surface-2));
  }

  .c-head { display: flex; align-items: center; gap: 8px }
  .proto {
    font-family: var(--sans); font-size: 12px; font-weight: 600;
    text-transform: uppercase; letter-spacing: .5px;
    padding: 3px 7px; border-radius: 4px;
    background: var(--surface-3); color: var(--text-2);
    border: 1px solid var(--border);
  }
  .c-head .spacer { flex: 1 }
  .star {
    width: 22px; height: 22px; border-radius: 5px;
    display: grid; place-items: center;
    color: var(--text-3); cursor: pointer;
    background: transparent; border: 0;
  }
  .star:hover { color: var(--text-2); background: var(--surface-3) }
  .star.on { color: var(--accent) }

  .c-name-input {
    font: inherit;
    font-size: 17px; font-weight: 600; letter-spacing: -.02em; line-height: 1.1;
    color: var(--text); background: var(--surface-3);
    border: 1px solid color-mix(in oklab, var(--accent) 50%, var(--border));
    border-radius: 6px; padding: 2px 6px;
    width: 100%; box-sizing: border-box;
    outline: none;
  }
  .c-name {
    font-size: 17px; font-weight: 600; letter-spacing: -.02em; line-height: 1.1;
    /* Long provider names (often UUID-spliced) must clip cleanly inside
       the 240–260px card column. The hover/aria-label can carry the full
       string if needed later. */
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .c-host {
    font-family: var(--sans); font-size: 12px; color: var(--text-3);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    margin-top: 4px;
  }

  .c-foot { display: flex; align-items: center; gap: 10px; margin-top: auto }
  .badge {
    font-family: var(--sans); font-size: 12px; font-weight: 600;
    padding: 3px 8px; border-radius: 999px;
    background: var(--accent-soft); color: var(--accent);
    border: 1px solid color-mix(in oklab, var(--accent) 40%, var(--border));
  }

  /* subscription layout order within the card flex column */
  .card.sub .c-foot { order: 99 }
  .card.sub :global(.sub-meta) { order: 50 }
</style>
