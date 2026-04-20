// Typed wrappers around every Tauri `invoke` call. Kept in one file so the
// IPC surface is easy to audit — if a backend command signature drifts from
// the frontend, both ends show up side by side here.

import { invoke } from "@tauri-apps/api/core";
import type {
  Profile,
  ProcessInfo,
  RoutingRule,
  StoredProfile,
  StoredSubscription,
  SubscriptionResult,
  SyncApplied,
} from "./types";

// --- Parsing & subscription fetch ---------------------------------------

export const parseAny = (url: string): Promise<Profile> =>
  invoke("parse_any", { url });

export const fetchSubscription = (url: string): Promise<SubscriptionResult> =>
  invoke("fetch_subscription", { url });

// --- Core (runner) -------------------------------------------------------

export const coreStart = (profile: Profile, profileId?: number): Promise<void> =>
  invoke("core_start", { profile, profileId: profileId ?? null });

export const coreStop = (): Promise<void> => invoke("core_stop");

export const coreStatus = (): Promise<boolean> => invoke("core_status");

// --- System proxy --------------------------------------------------------
// Enable/disable is owned server-side by `core_start`/`core_stop`, gated
// by the active mode. Frontend only reads status to render the indicator.

export const sysproxyStatus = (): Promise<boolean> => invoke("sysproxy_status");

// --- Profiles ------------------------------------------------------------

export const profilesList = (): Promise<StoredProfile[]> =>
  invoke("profiles_list");

export const profilesAdd = (profile: Profile): Promise<StoredProfile> =>
  invoke("profiles_add", { profile });

export const profilesUpdate = (
  id: number,
  profile: Profile,
): Promise<void> => invoke("profiles_update", { id, profile });

export const profilesDelete = (id: number): Promise<void> =>
  invoke("profiles_delete", { id });

// --- Subscriptions -------------------------------------------------------

export const subscriptionsList = (): Promise<StoredSubscription[]> =>
  invoke("subscriptions_list");

export const subscriptionsAdd = (
  name: string,
  url: string,
): Promise<StoredSubscription> =>
  invoke("subscriptions_add", { name, url });

export const subscriptionsRename = (
  id: number,
  name: string,
): Promise<void> => invoke("subscriptions_rename", { id, name });

export const subscriptionsDelete = (id: number): Promise<void> =>
  invoke("subscriptions_delete", { id });

export const subscriptionsSync = (id: number): Promise<SyncApplied> =>
  invoke("subscriptions_sync", { id });


export const profilesSetFavorite = (id: number, favorite: boolean): Promise<void> =>
  invoke("profiles_set_favorite", { id, favorite });

export const profilesDuplicate = (id: number): Promise<StoredProfile> =>
  invoke("profiles_duplicate", { id });

// --- Settings -----------------------------------------------------------

export const settingsGet = (key: string): Promise<string | null> =>
  invoke("settings_get", { key });

export const settingsSet = (key: string, value: string): Promise<void> =>
  invoke("settings_set", { key, value });

// --- Routing ------------------------------------------------------------

export const routingList = (): Promise<RoutingRule[]> =>
  invoke("routing_list");

export const routingAdd = (
  appPath: string,
  appName: string | null,
): Promise<RoutingRule> =>
  invoke("routing_add", { appPath, appName });

export const routingDelete = (id: number): Promise<void> =>
  invoke("routing_delete", { id });

export const routingSetEnabled = (id: number, enabled: boolean): Promise<void> =>
  invoke("routing_set_enabled", { id, enabled });

export const routingProcessesSnapshot = (): Promise<ProcessInfo[]> =>
  invoke("routing_processes_snapshot");

export const isElevated = (): Promise<boolean> => invoke("is_elevated");

export const relaunchAsAdmin = (): Promise<void> => invoke("relaunch_as_admin");

// --- Launch environment -------------------------------------------------

export const isPortable = (): Promise<boolean> => invoke("is_portable");

export const isAutostartLaunch = (): Promise<boolean> =>
	invoke("is_autostart_launch");

// --- Autostart (elevated, via Task Scheduler) ----------------------------

export const autostartElevatedStatus = (): Promise<boolean> =>
	invoke("autostart_elevated_status");

export const autostartElevatedEnable = (): Promise<void> =>
	invoke("autostart_elevated_enable");

export const autostartElevatedDisable = (): Promise<void> =>
	invoke("autostart_elevated_disable");