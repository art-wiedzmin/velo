<script lang="ts">
  // Top status bar: state light, headline + meta, profile counts,
  // Sysproxy/Tunnel segmented, Routing icon, settings, fixed-width Connect.
  //
  // The Connect button is explicitly 140x36 — state-driven label changes
  // (Connect / Cancel / Disconnect / Retry) don't reflow neighbors.
  //
  // Mode switch (Sysproxy/Tunnel) writes to settings and takes effect on the
  // next connect — we don't hot-reload sing-box mid-session.

  import { catalog, core, routing } from "$lib/state.svelte";
  import { dispatchConnectAction } from "$lib/actions/connect";
  import IconRouting from "$lib/components/icons/IconRouting.svelte";
  import IconSettings from "$lib/components/icons/IconSettings.svelte";

  let {
    onOpenSettings,
    onOpenRouting,
  }: { onOpenSettings: () => void; onOpenRouting: () => void } = $props();

  type StateClass = "connected" | "connecting" | "error" | "disconnecting" | "switching" | "";

  let stateClass = $derived<StateClass>(
    core.state.kind === "disconnected" ? "" : core.state.kind,
  );

  let headline = $derived.by(() => {
    switch (core.state.kind) {
      case "disconnected": return "Disconnected";
      case "connecting": return "Connecting…";
      case "connected": return `Connected to ${core.state.active.profile.name}`;
      case "switching": return "Switching server…";
      case "disconnecting": return "Disconnecting…";
      case "error": return "Connection failed";
    }
  });

  let meta = $derived.by(() => {
    switch (core.state.kind) {
      case "disconnected": return "Select a profile and press Connect";
      case "connecting":
        return `${core.state.target.profile.protocol} · ${core.state.target.profile.address}`;
      case "connected": {
        const p = core.state.active.profile;
        return `${p.protocol} · ${p.address}:${p.port}`;
      }
      case "switching":
        return `${core.state.from.profile.name} → ${core.state.to.profile.name}`;
      case "disconnecting":
        return core.state.wasActive.profile.name;
      case "error":
        return core.state.reason;
    }
  });

  let btnLabel = $derived.by(() => {
    switch (core.state.kind) {
      case "disconnected": return "Connect";
      case "connecting": return "Cancel";
      case "connected": return "Disconnect";
      case "switching": return "Cancel";
      case "disconnecting": return "…";
      case "error": return "Retry";
    }
  });

  let btnClass = $derived(
    core.state.kind === "connected" || core.state.kind === "connecting" || core.state.kind === "switching"
      ? "sb-connect disconnect"
      : "sb-connect",
  );

  let btnDisabled = $derived(
    core.state.kind === "disconnecting" ||
    (core.state.kind === "disconnected" && catalog.selectedId == null),
  );

  let favCount = $derived(catalog.favorites.size);
  let profCount = $derived(catalog.profiles.length);

  const onConnect = () => void dispatchConnectAction();
</script>

<section class="sb {stateClass}">
  <div class="sb-state">
    <div class="sb-light" aria-hidden="true"></div>
    <div class="sb-text">
      <div class="sb-head" title={headline}>{headline}</div>
      <div class="sb-meta" title={meta}>{meta}</div>
    </div>
  </div>

  <div class="sb-sep"></div>

  <div class="sb-stat">
    <div class="sb-stat-v">{profCount}</div>
    <div class="sb-stat-k">profiles</div>
  </div>

  {#if favCount > 0}
    <div class="sb-stat">
      <div class="sb-stat-v accent">{favCount}</div>
      <div class="sb-stat-k">favorites</div>
    </div>
  {/if}

  <div class="mode-seg" role="tablist" aria-label="traffic mode">
    <button
      class:on={routing.coreMode === "sysproxy"}
      aria-pressed={routing.coreMode === "sysproxy"}
      onclick={() => { void routing.setCoreMode("sysproxy") }}
    >Sysproxy</button>
    <button
      class:on={routing.coreMode === "tun"}
      aria-pressed={routing.coreMode === "tun"}
      title="Tunnel mode — takes effect on next connect (requires admin)"
      onclick={() => { void routing.setCoreMode("tun") }}
    >Tunnel</button>
  </div>

  <button class="sb-ghost" title="Routing" aria-label="Routing" onclick={onOpenRouting}>
    <IconRouting />
  </button>

  <button class="sb-ghost" title="Settings" aria-label="Settings" onclick={onOpenSettings}>
    <IconSettings />
  </button>

  <button class={btnClass} onclick={onConnect} disabled={btnDisabled}>
    {btnLabel}
  </button>
</section>

<style>
  .sb {
    display: flex;
    align-items: center;
    gap: 18px;
    flex-wrap: nowrap;
    padding: 14px 18px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .sb-state {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    flex: 1 1 auto;
  }
  /* Only the text wrapper grows. Previously `.sb-state > div` also caught
     `.sb-light` (it's a div too) — that made the indicator dot stretch to
     fill the row on narrow widths. */
  .sb-state > .sb-text { min-width: 0; flex: 1 }
  .sb-light {
    width: 10px; height: 10px; border-radius: 50%;
    /* Disconnected default uses text-2, not text-3, so the dot reads as a
       muted-but-present indicator against the surface-2 status bar. */
    background: var(--text-2);
    box-shadow:
      0 0 0 1px color-mix(in oklab, var(--text-2) 40%, transparent),
      0 0 0 4px color-mix(in oklab, var(--text-2) 15%, transparent);
    transition: all .2s;
    flex-shrink: 0;
  }
  .sb.connected .sb-light {
    background: var(--good);
    box-shadow: 0 0 0 4px color-mix(in oklab, var(--good) 25%, transparent);
  }
  .sb.connecting .sb-light {
    background: var(--warn);
    box-shadow: 0 0 0 4px color-mix(in oklab, var(--warn) 25%, transparent);
    animation: pulse 1.2s ease-in-out infinite;
  }
  .sb.error .sb-light {
    background: var(--bad);
    box-shadow: 0 0 0 4px color-mix(in oklab, var(--bad) 25%, transparent);
  }
  .sb.disconnecting .sb-light {
    background: var(--text-3);
    animation: pulse 1.2s ease-in-out infinite;
  }
  .sb.switching .sb-light {
    background: var(--accent);
    box-shadow: 0 0 0 4px color-mix(in oklab, var(--accent) 25%, transparent);
    animation: pulse 1.2s ease-in-out infinite;
  }
  .sb.error .sb-head { color: var(--bad) }

  .sb-head {
    font-size: 18px; font-weight: 600; letter-spacing: -.02em; line-height: 1.1;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 100%;
  }
  .sb-meta {
    font-family: var(--sans); font-size: 13px; color: var(--text-2);
    margin-top: 3px;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 100%;
  }

  .sb-sep { width: 1px; align-self: stretch; background: var(--border); flex-shrink: 0 }

  .sb-stat { text-align: left; white-space: nowrap; flex-shrink: 0 }
  .sb-stat-v { font-size: 18px; font-weight: 600; letter-spacing: -.02em; line-height: 1 }
  .sb-stat-v.accent { color: var(--accent) }
  .sb-stat-k {
    font-family: var(--sans); font-size: 12px; color: var(--text-3);
    margin-top: 3px;
  }

  .mode-seg {
    display: inline-flex;
    background: var(--surface-3);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 2px;
    flex-shrink: 0;
    height: 36px;
  }
  .mode-seg button {
    font-family: var(--sans); font-size: 13px; font-weight: 500;
    border: 0; background: transparent; color: var(--text-2);
    padding: 0 14px; cursor: pointer; border-radius: 6px;
    transition: all .12s;
  }
  .mode-seg button:hover:not(:disabled) { color: var(--text) }
  .mode-seg button.on {
    background: var(--surface); color: var(--text);
    box-shadow: 0 1px 0 rgba(255,255,255,.04), 0 1px 2px rgba(0,0,0,.3);
  }
  .mode-seg button:disabled {
    opacity: .45;
    cursor: not-allowed;
  }

  .sb-ghost {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-2);
    width: 36px; height: 36px; border-radius: 8px;
    display: grid; place-items: center;
    cursor: pointer; flex-shrink: 0;
  }
  .sb-ghost:hover { color: var(--text); border-color: var(--border-strong) }

  .sb-connect {
    font-family: var(--sans); font-size: 14px; font-weight: 600;
    background: var(--accent); color: var(--accent-ink);
    border: 0; border-radius: 8px; cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
    gap: 7px; flex-shrink: 0; white-space: nowrap;
    width: 140px; height: 36px; padding: 0;
    transition: background .12s, color .12s, box-shadow .12s;
    box-shadow:
      0 0 0 1px color-mix(in oklab, var(--accent) 40%, transparent),
      0 8px 16px -6px color-mix(in oklab, var(--accent) 60%, transparent);
  }
  .sb-connect:hover:not(:disabled) { filter: brightness(1.08) }
  .sb-connect:disabled { opacity: .6; cursor: wait }
  .sb-connect.disconnect {
    background: var(--surface-3);
    color: var(--text);
    box-shadow: 0 0 0 1px var(--border-strong);
  }
  .sb-connect.disconnect:hover:not(:disabled) {
    background: var(--bad); color: #fff;
    box-shadow: 0 0 0 1px var(--bad);
  }
</style>
