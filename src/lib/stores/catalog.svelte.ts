import * as api from "$lib/api";
import type { FilterKey, StoredProfile, StoredSubscription } from "$lib/types";
import { core } from "./core.svelte";

export class CatalogStore {
  profiles = $state<StoredProfile[]>([]);
  subscriptions = $state<StoredSubscription[]>([]);
  loading = $state(false);
  lastError = $state<string | null>(null);
  /** Current filter chip. Single-select across scope and region. */
  filter = $state<FilterKey>("all");
  /** User's current selection — used to enable the Connect button when
   * nothing is running yet. `null` means "no profile picked". Clicking a
   * card updates this; the Connect button reads it. Active profile is a
   * separate concept tracked by CoreStore. */
  selectedId = $state<number | null>(null);
  /** Inline rename target. Set by ContextMenu, consumed by ProfileCard, which
   * renders an input and commits via `commitRename`. */
  renamingId = $state<number | null>(null);

  async refresh(): Promise<void> {
    this.loading = true;
    this.lastError = null;
    try {
      const [profiles, subscriptions] = await Promise.all([
        api.profilesList(),
        api.subscriptionsList(),
      ]);
      this.profiles = profiles;
      this.subscriptions = subscriptions;
      // If nothing is selected yet and we just loaded exactly one profile,
      // pre-select it — a single-profile install shouldn't make the user
      // click the card before Connect works.
      if (this.selectedId == null && profiles.length === 1) {
        this.selectedId = profiles[0].id;
      }
      // Drop a stale selection pointing at a deleted row.
      if (this.selectedId != null && !profiles.some((p) => p.id === this.selectedId)) {
        this.selectedId = null;
      }
    } catch (e) {
      this.lastError = String(e);
    } finally {
      this.loading = false;
    }
  }

  /** Live-updating set view of favorite ids, derived from the server state. */
  get favorites(): Set<number> {
    return new Set(this.profiles.filter((p) => p.favorite).map((p) => p.id));
  }

  async toggleFavorite(id: number): Promise<void> {
    const p = this.profiles.find((x) => x.id === id);
    if (!p) return;
    const next = !p.favorite;
    // Optimistic update so the star flips immediately; refresh syncs the
    // canonical state from SQLite.
    this.profiles = this.profiles.map((x) => (x.id === id ? { ...x, favorite: next } : x));
    try {
      await api.profilesSetFavorite(id, next);
    } catch (e) {
      // Roll back on failure.
      this.profiles = this.profiles.map((x) => (x.id === id ? { ...x, favorite: !next } : x));
      throw e;
    }
  }

  startRename(id: number): void {
    this.renamingId = id;
  }
  cancelRename(): void {
    this.renamingId = null;
  }
  async commitRename(id: number, rawName: string): Promise<void> {
    this.renamingId = null;
    const name = rawName.trim();
    const sp = this.profiles.find((x) => x.id === id);
    if (!sp || !name || sp.profile.name === name) return;
    // Optimistic: reflect the new name immediately; refresh reconciles on
    // success, rolls back on throw.
    const prev = sp.profile;
    this.profiles = this.profiles.map((x) =>
      x.id === id ? { ...x, profile: { ...x.profile, name } } : x,
    );
    core.patchProfile(id, { name });
    try {
      await api.profilesUpdate(id, { ...prev, name });
    } catch (e) {
      this.profiles = this.profiles.map((x) =>
        x.id === id ? { ...x, profile: prev } : x,
      );
      core.patchProfile(id, { name: prev.name });
      throw e;
    }
  }
  isFavorite(id: number): boolean {
    const p = this.profiles.find((x) => x.id === id);
    return !!p?.favorite;
  }

  /** The full profile list sorted with favorites first, then natural order. */
  get sorted(): StoredProfile[] {
    return [...this.profiles].sort((a, b) => {
      const fa = a.favorite ? 0 : 1;
      const fb = b.favorite ? 0 : 1;
      return fa - fb;
    });
  }

  /** Ordered list of recently-connected profile ids. Derived from
   * `last_connected_at` on the server — nulls are excluded, so a fresh
   * install has an empty Recent chip. Capped at 6 to match the design. */
  get recent(): number[] {
    return this.profiles
      .filter((p) => p.last_connected_at != null)
      .sort((a, b) => (b.last_connected_at ?? 0) - (a.last_connected_at ?? 0))
      .slice(0, 6)
      .map((p) => p.id);
  }

  /** Profiles filtered by the current chip. */
  get filtered(): StoredProfile[] {
    const src = this.sorted;
    switch (this.filter) {
      case "all":
        return src;
      case "favorites":
        return src.filter((p) => p.favorite);
      case "recent": {
        const rank = new Map(this.recent.map((id, i) => [id, i]));
        return src
          .filter((p) => rank.has(p.id))
          .sort((a, b) => (rank.get(a.id) ?? 0) - (rank.get(b.id) ?? 0));
      }
    }
  }

  /** Live counts for filter chips. */
  get counts(): Record<FilterKey, number> {
    return {
      all: this.profiles.length,
      favorites: this.profiles.filter((p) => p.favorite).length,
      recent: this.recent.length,
    };
  }
}

export const catalog = new CatalogStore();
