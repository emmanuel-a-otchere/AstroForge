// CR-03 P5 — image version + AI recommendation data model.
//
// Typed shape for image versions, AI recommendations, and the
// version timeline. P5 shipped deterministic placeholders; R3
// replaces the placeholder timeline with a real Tauri-backed
// load (`imageVersionList`) so the Compare workspace renders
// durable state instead of fake data.
//
// The Recommendation shape stays placeholder for now; the AI
// ops Rust migration is on a separate tranche and not R3 scope.

import { writable, derived } from "svelte/store";
import { imageVersionList, type ImageVersion } from "../lib/astroforge-api";

// ─── Image Version ──────────────────────────────────────────────────────

/** Status of a single image version. R3 note: status is
 *  intentionally separate from the durable `ImageVersion`
 *  shape returned by the IPC — the IPC owns metadata, the
 *  Svelte store derives a status from `sequence` (any version
 *  is "in_review" until the user promotes it; the promote
 *  action is a future tranche). */
export type VersionStatus =
  | "draft"
  | "in_review"
  | "final"
  | "exported";

/** Re-export the IPC's `ImageVersion` as the canonical Svelte
 *  side. The two shapes are intentionally aligned — the
 *  previous interface (id, label, source, parent_version_id,
 *  status, notes) is replaced by the durable schema. */
export type { ImageVersion };

/** The version timeline is sorted by created_at ascending. */
export type VersionTimeline = readonly ImageVersion[];

// ─── AI Recommendation ──────────────────────────────────────────────────

export type RecommendationKind =
  | "tone_adjustment"
  | "star_reduction"
  | "color_balance"
  | "noise_reduction"
  | "stretch_recommendation";

export type RecommendationStatus =
  | "pending"
  | "accepted"
  | "rejected"
  | "superseded";

/** A single AI-generated enhancement proposal attached to an image
 *  version. Real data lands with the AI ops Rust migration. */
export interface AiRecommendation {
  id: string;
  project_id: string;
  version_id: string;
  kind: RecommendationKind;
  title: string;
  /** What this recommendation would change. */
  description: string;
  /** Why the AI is suggesting it (input data + reasoning). */
  rationale: string;
  /** Specific parameter values to apply on accept. */
  parameter_patch: Record<string, number | string | boolean>;
  status: RecommendationStatus;
  created_at: string;
}

// ─── Store ──────────────────────────────────────────────────────────────

interface VersionState {
  /** The active project whose timeline we're showing. */
  project_id: string | null;
  versions: VersionTimeline;
  recommendations: readonly AiRecommendation[];
  loading: boolean;
  error: string | null;
}

const initial: VersionState = {
  project_id: null,
  versions: [],
  recommendations: [],
  loading: false,
  error: null,
};

const internal = writable<VersionState>(initial);

export const versionStore = {
  subscribe: internal.subscribe,

  /** Load the real version timeline + recommendations for a
   *  project. R3: the version list comes from the durable
   *  event log via `image_version_list`. Recommendations
   *  remain an empty list for now (the AI ops Rust migration
   *  is on a separate tranche). On any IPC error the timeline
   *  is empty and the error is recorded so the Compare
   *  workspace can render a clear notice. */
  async load(project_id: string): Promise<void> {
    internal.update((s) => ({ ...s, project_id, loading: true, error: null }));
    try {
      const response = await imageVersionList(project_id);
      internal.set({
        project_id,
        versions: response.versions,
        recommendations: [],
        loading: false,
        error: null,
      });
    } catch (e) {
      internal.set({
        project_id,
        versions: [],
        recommendations: [],
        loading: false,
        error: e instanceof Error ? e.message : String(e),
      });
    }
  },

  reset(): void {
    internal.set(initial);
  },

  /** Apply an accept/reject action locally. P5a wires the real IPC;
   *  the optimistic-update pattern stays the same. */
  applyRecommendationStatus(id: string, status: RecommendationStatus): void {
    internal.update((s) => ({
      ...s,
      recommendations: s.recommendations.map((r) =>
        r.id === id ? { ...r, status } : r,
      ),
    }));
  },
};

/** Latest version per project — the canonical "current image" the
 *  Export workspace uses by default. */
export const latestVersion = derived(
  internal,
  ($s): ImageVersion | null => {
    if ($s.versions.length === 0) return null;
    return $s.versions[$s.versions.length - 1];
  },
);

/** Pending recommendations for the active project. */
export const pendingRecommendations = derived(
  internal,
  ($s) => $s.recommendations.filter((r) => r.status === "pending"),
);
