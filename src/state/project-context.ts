// CR-03 P3 — active project context.
//
// Holds the project currently being edited in the Studio. P3
// introduces this so the StudioHeader (§4) can show project name +
// save state, the §10 trust panel can identify its target, and
// §11 the recovery banner can address the user. The context survives
// across navigation within the project (Overview → Process →
// Enhance ...) but is cleared when the user closes the project.

import { writable, derived } from "svelte/store";
import type { ProjectSummary } from "../lib/astroforge-api";

interface ProjectContextState {
  project: ProjectSummary;
  /** True when the in-memory state has unsaved differences from the
   *  durable store. P3 initializes false; P6 wires the actual
   *  save-state tracking. */
  dirty: boolean;
}

const internal = writable<ProjectContextState | null>(null);

export const projectContext = {
  subscribe: internal.subscribe,
  /** Open a project in the Studio. Clears any previous context. */
  open(project: ProjectSummary) {
    internal.set({ project, dirty: false });
  },
  /** Close the active project. Returns to application context. */
  close() {
    internal.set(null);
  },
  /** Mark the context as dirty / clean. P6 wires the actual
   *  change-tracking pipeline. */
  markDirty(dirty: boolean) {
    internal.update((state) => (state ? { ...state, dirty } : state));
  },
  /** Replace the active project (e.g. after rename). */
  replace(project: ProjectSummary) {
    internal.update((state) =>
      state ? { project, dirty: state.dirty } : null,
    );
  },
};

/** True when a project is currently open in the Studio. */
export const hasOpenProject = derived(internal, ($s) => $s !== null);

/** The active project, or null when no project is open. */
export const activeProject = derived(internal, ($s) => $s?.project ?? null);

/** True when the open project has unsaved differences. P3 defaults
 *  to false; P6 wires the tracking pipeline. */
export const isProjectDirty = derived(internal, ($s) => $s?.dirty ?? false);