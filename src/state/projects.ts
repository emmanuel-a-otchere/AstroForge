// CR-03 P2 — Projects store.
//
// Bridges the CR-02.6 IPC client (src/lib/astroforge-api.ts) into a
// reactive Svelte store. P2 populates the Projects grid + powers the
// create/open dialogs. P3 will read the active project from this store
// when Studio entry lands.

import { writable } from "svelte/store";
import type {
  ProjectStatus,
  ProjectSummary,
} from "../lib/astroforge-api";
import * as api from "../lib/astroforge-api";

export type ProjectAction =
  | "create"
  | "open"
  | "rename"
  | "archive"
  | "delete"
  | "recover";

export const PROJECT_ACTIONS: readonly ProjectAction[] = [
  "create",
  "open",
  "rename",
  "archive",
  "delete",
  "recover",
] as const;

interface ProjectsState {
  items: ProjectSummary[];
  loading: boolean;
  error: string | null;
}

const initial: ProjectsState = { items: [], loading: false, error: null };

const internal = writable<ProjectsState>(initial);

export const projectsStore = {
  subscribe: internal.subscribe,
  /** Refresh the list from the durable store. */
  async refresh(): Promise<void> {
    internal.update((s) => ({ ...s, loading: true, error: null }));
    try {
      const items = await api.projectList();
      internal.set({ items, loading: false, error: null });
    } catch (e) {
      internal.set({
        items: [],
        loading: false,
        error: e instanceof Error ? e.message : String(e),
      });
    }
  },
  reset() {
    internal.set(initial);
  },
};

/** True when the store currently has no projects and isn't loading. */
export function isProjectsEmpty(state: ProjectsState): boolean {
  return !state.loading && state.items.length === 0 && state.error === null;
}

/** Active project count by status. Used by §7 §26 empty-state messaging. */
export function countByStatus(
  items: readonly ProjectSummary[],
): Record<ProjectStatus, number> {
  const counts: Record<ProjectStatus, number> = {
    Active: 0,
    Archived: 0,
    Exported: 0,
  };
  for (const p of items) counts[p.status] += 1;
  return counts;
}