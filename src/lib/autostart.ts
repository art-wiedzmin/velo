// Autostart preferences. Velo's exe manifest forces every launch elevated,
// so a plain HKCU\...\Run entry would UAC-prompt at every logon. The only
// silent path is a Windows Task Scheduler entry registered with /RL HIGHEST
// — `commands::autostart::*` shells out to `schtasks` for that.
//
// We persist the auxiliary preferences (minimized bit, auto-connect policy)
// through the existing settings table so Rust can read them at launch time.

import * as api from "$lib/api";
import { catalog, core, toast } from "$lib/state.svelte";
import type { StoredProfile } from "$lib/types";

const MINIMIZED_KEY = "autostart.minimized";
const AUTOCONNECT_MODE_KEY = "autostart.autoconnect.mode";
const AUTOCONNECT_PROFILE_KEY = "autostart.autoconnect.profile_id";

/** Off: never auto-connect. Last: most recent `last_connected_at` wins.
 *  Pinned: use `autoconnectProfileId` if it still exists in the catalog. */
export type AutoconnectMode = "off" | "last" | "pinned";

export type AutostartState = {
	enabled: boolean;
	minimized: boolean;
	autoconnect: AutoconnectMode;
	autoconnectProfileId: number | null;
};

function parseMode(raw: string | null): AutoconnectMode {
	return raw === "last" || raw === "pinned" ? raw : "off";
}

export async function loadAutostart(): Promise<AutostartState> {
	const [enabled, minRaw, modeRaw, pinnedRaw] = await Promise.all([
		api.autostartElevatedStatus(),
		api.settingsGet(MINIMIZED_KEY),
		api.settingsGet(AUTOCONNECT_MODE_KEY),
		api.settingsGet(AUTOCONNECT_PROFILE_KEY),
	]);
	const pinned = pinnedRaw != null ? Number(pinnedRaw) : NaN;
	return {
		enabled,
		// Default `true`: enabling autostart implies a quiet launch.
		minimized: minRaw !== "false",
		autoconnect: parseMode(modeRaw),
		autoconnectProfileId: Number.isFinite(pinned) ? pinned : null,
	};
}

export async function setEnabled(on: boolean): Promise<void> {
	if (on) await api.autostartElevatedEnable();
	else await api.autostartElevatedDisable();
}

export async function setMinimized(on: boolean): Promise<void> {
	await api.settingsSet(MINIMIZED_KEY, on ? "true" : "false");
}

export async function setAutoconnectMode(mode: AutoconnectMode): Promise<void> {
	await api.settingsSet(AUTOCONNECT_MODE_KEY, mode);
}

export async function setAutoconnectProfileId(id: number): Promise<void> {
	await api.settingsSet(AUTOCONNECT_PROFILE_KEY, String(id));
}

function resolveTarget(
	mode: AutoconnectMode,
	pinnedId: number | null,
	profiles: readonly StoredProfile[],
): StoredProfile | null {
	if (mode === "last") {
		const withTs = profiles.filter((p) => p.last_connected_at != null);
		if (withTs.length === 0) return null;
		return withTs.reduce((a, b) =>
			(a.last_connected_at ?? 0) >= (b.last_connected_at ?? 0) ? a : b,
		);
	}
	if (mode === "pinned" && pinnedId != null) {
		return profiles.find((p) => p.id === pinnedId) ?? null;
	}
	return null;
}

/** Called once on app boot. No-op unless Rust says this was an autostart
 *  launch AND a resolvable target exists. Assumes catalog is already
 *  populated — caller is responsible for awaiting `catalog.refresh()`. */
export async function runAutostartConnect(): Promise<void> {
	let isAutostart: boolean;
	try {
		isAutostart = await api.isAutostartLaunch();
	} catch {
		return;
	}
	if (!isAutostart) return;

	const [modeRaw, pinnedRaw] = await Promise.all([
		api.settingsGet(AUTOCONNECT_MODE_KEY),
		api.settingsGet(AUTOCONNECT_PROFILE_KEY),
	]);
	const mode = parseMode(modeRaw);
	if (mode === "off") return;
	const pinnedId = pinnedRaw != null ? Number(pinnedRaw) : null;
	const target = resolveTarget(mode, pinnedId, catalog.profiles);
	if (!target) return;
	try {
		await core.connect(target);
	} catch (e) {
		toast.show(String(e));
	}
}
