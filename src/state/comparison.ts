// CR-07 B4 — comparison workspace state.
//
// Holds the project-scoped decision records (B3 `ImageDecision`
// state machine) and comparison sets (§16) for the active project,
// plus the load/reset lifecycle hooks that `project-lifecycle.ts`
// coordinates (R4 pattern).
//
// Decisions load lazily per version (`ensureDecision`) because the
// Compare workspace's version timeline derives from the durable
// event log, while `list_decisions_for_project` joins on the
// `image_versions` table — the two only agree once version rows
// are written to the project store. Lazy per-version loads keep
// the panel correct regardless of that migration state.

import { get, writable } from "svelte/store";
import {
  applyImageDecision,
  deleteComparisonSet,
  listComparisonSetsForProject,
  loadImageDecision,
  saveComparisonSet,
  type ComparisonSet,
  type ImageDecision,
  type ImageDecisionState,
} from "../lib/astroforge-api";

interface ComparisonState {
  /** The active project whose decisions/sets we're showing. */
  project_id: string | null;
  /** Decisions loaded so far, keyed by version_id. Versions with
   *  no decision row yet are absent — the panel renders the
   *  implicit `working` default. */
  decisions: Record<string, ImageDecision>;
  sets: ComparisonSet[];
  loading: boolean;
  error: string | null;
}

const initial: ComparisonState = {
  project_id: null,
  decisions: {},
  sets: [],
  loading: false,
  error: null,
};

const internal = writable<ComparisonState>(initial);

export const comparisonState = {
  subscribe: internal.subscribe,

  /** Load the project's comparison sets. Decisions populate
   *  lazily via `ensureDecision`. Non-fatal on error: the error
   *  lands in state and the UI surfaces it. */
  async load(projectId: string): Promise<void> {
    internal.update((s) => ({
      ...s,
      project_id: projectId,
      loading: true,
      error: null,
    }));
    try {
      const sets = await listComparisonSetsForProject(projectId);
      internal.update((s) => ({ ...s, sets, loading: false }));
    } catch (e) {
      internal.update((s) => ({
        ...s,
        loading: false,
        error: e instanceof Error ? e.message : String(e),
      }));
    }
  },

  /** Load the decision for one version if we haven't already.
   *  `null` (no row yet) is a valid, non-error outcome. */
  async ensureDecision(versionId: string): Promise<void> {
    if (versionId in get(internal).decisions) return;
    const decision = await loadImageDecision(versionId);
    if (decision) {
      internal.update((s) => ({
        ...s,
        decisions: { ...s.decisions, [versionId]: decision },
      }));
    }
  },

  /** Apply a state-machine transition. Throws on invalid
   *  transitions; the panel catches and renders the message. */
  async applyDecision(
    versionId: string,
    newState: ImageDecisionState,
    reason?: string,
  ): Promise<ImageDecision> {
    const decision = await applyImageDecision(versionId, newState, reason);
    internal.update((s) => ({
      ...s,
      decisions: { ...s.decisions, [versionId]: decision },
    }));
    return decision;
  },

  /** Save the current A/B pair as a named comparison set. The id
   *  is generated client-side because the B3 command contract
   *  takes the full struct; uniqueness comes from a UUID suffix. */
  async saveSet(
    projectId: string,
    name: string,
    versionIds: string[],
    slotLabels: string[],
  ): Promise<void> {
    const set: ComparisonSet = {
      id: `set-${crypto.randomUUID()}`,
      project_id: projectId,
      name,
      version_ids: versionIds,
      created_at: new Date().toISOString(),
      slot_labels: slotLabels,
    };
    await saveComparisonSet(set);
    internal.update((s) => ({ ...s, sets: [set, ...s.sets] }));
  },

  /** Delete a set. The underlying Image Versions are unaffected
   *  (ADR-07.4: comparison is non-destructive). */
  async removeSet(setId: string): Promise<void> {
    await deleteComparisonSet(setId);
    internal.update((s) => ({
      ...s,
      sets: s.sets.filter((set) => set.id !== setId),
    }));
  },

  reset(): void {
    internal.set(initial);
  },
};
