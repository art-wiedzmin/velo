// Barrel re-export. Individual stores live in $lib/stores/*.svelte.ts to keep
// import surfaces narrow; this file preserves the legacy `$lib/state.svelte`
// import path that every component uses.
export { core, CoreStore } from "./stores/core.svelte";
export { catalog, CatalogStore } from "./stores/catalog.svelte";
export { logs, LogsStore } from "./stores/logs.svelte";
export { stats, StatsStore } from "./stores/stats.svelte";
export { sysproxy, SysProxyStore } from "./stores/sysproxy.svelte";
export { routing, RoutingStore } from "./stores/routing.svelte";
export { toast, ToastStore } from "./stores/toast.svelte";
export { attachEventListeners } from "./bootstrap";
