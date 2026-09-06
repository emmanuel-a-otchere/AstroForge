// CR-03 P5 — image version + AI recommendation data model.
//
// Typed shape for image versions, AI recommendations, and the
// version timeline. The CR-02.6 IPC scaffold ships projects +
// pipeline runs; image versions + AI operations land in a future
// Rust migration. P5 ships the frontend model + a typed
// placeholder data layer so the workspace UIs (Overview timeline,
// Enhance recommendations, Compare picker, Export picker) can be
// reviewed end-to-end before the backend lands.

import { writable, derived } from "svelte/store";

// ─── Image Version ──────────────────────────────────────────────────────

/** Status of a single image version. */
export type VersionStatus =
  | "draft"
  | "in_review"
  | "final"
  | "exported";

/** A single image version of the project's working image. Versions
 *  form a lineage; each version has zero or one parent. */
export interface ImageVersion {
  id: string;
  project_id: string;
  /** 1-indexed human-readable label, e.g. "v1", "v2", "v3". */
  label: string;
  /** Optional human-friendly title from AI or user. */
  title: string;
  /** What produced this version: a pipeline run, a manual edit,
   *  or an accepted AI recommendation. */
  source:
    | { kind: "pipeline_run"; run_id: string }
    | { kind: "manual" }
    | { kind: "ai_recommendation"; recommendation_id: string };
  /** Parent version id when this version supersedes another. */
  parent_version_id: string | null;
  status: VersionStatus;
  created_at: string; // ISO-8601
  notes: string;
}

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

  /** Load (placeholder) versions + recommendations for a project.
   *  P5 ships deterministic placeholders; P5a replaces with real
   *  IPC when the Rust migration lands. */
  load(project_id: string): void {
    internal.update((s) => ({ ...s, project_id, loading: true, error: null }));
    try {
      const versions = placeholderVersions(project_id);
      const recommendations = placeholderRecommendations(project_id, versions);
      internal.set({
        project_id,
        versions,
        recommendations,
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

// ─── Placeholder data (P5 only; replaced in P5a) ────────────────────────

function placeholderVersions(project_id: string): ImageVersion[] {
  const base = "2026-09-06T12:00:00Z";
  return [
    {
      id: `${project_id}-v1`,
      project_id,
      label: "v1",
      title: "Stacked integration",
      source: { kind: "pipeline_run", run_id: "run-001" },
      parent_version_id: null,
      status: "in_review",
      created_at: base,
      notes: "First integration of 60 light frames at 120s exposure.",
    },
    {
      id: `${project_id}-v2`,
      project_id,
      label: "v2",
      title: "Stretched + star reduction",
      source: { kind: "ai_recommendation", recommendation_id: "rec-001" },
      parent_version_id: `${project_id}-v1`,
      status: "in_review",
      created_at: "2026-09-06T13:30:00Z",
      notes: "Asinh stretch with star-reduction applied per AI recommendation.",
    },
    {
      id: `${project_id}-v3`,
      project_id,
      label: "v3",
      title: "Color balanced",
      source: { kind: "ai_recommendation", recommendation_id: "rec-002" },
      parent_version_id: `${project_id}-v2`,
      status: "final",
      created_at: "2026-09-06T14:15:00Z",
      notes: "SCNR green + per-channel background neutralization.",
    },
  ];
}

function placeholderRecommendations(
  project_id: string,
  versions: readonly ImageVersion[],
): AiRecommendation[] {
  if (versions.length === 0) return [];
  const latest = versions[versions.length - 1];
  return [
    {
      id: "rec-001",
      project_id,
      version_id: latest.id,
      kind: "star_reduction",
      title: "Reduce star bloat in highlights",
      description:
        "Compress stars tighter in the highlights to recover nebula detail behind bright field stars.",
      rationale:
        "Detected 12 stars with peak luma above the 95th-percentile threshold; the surrounding nebulosity is being washed out.",
      parameter_patch: {
        star_reduction_amount: 0.35,
        luma_threshold: 0.85,
      },
      status: "accepted",
      created_at: "2026-09-06T13:00:00Z",
    },
    {
      id: "rec-002",
      project_id,
      version_id: latest.id,
      kind: "color_balance",
      title: "Neutralize green cast in background",
      description:
        "Pull the green channel down to match red/blue, restoring a neutral sky background.",
      rationale:
        "Background pixels show a +5% green offset typical of an uncalibrated OSC sensor; SCNR green will correct without affecting emission-line regions.",
      parameter_patch: {
        scnr_amount: 0.5,
        scnr_target: "green",
      },
      status: "pending",
      created_at: "2026-09-06T13:45:00Z",
    },
    {
      id: "rec-003",
      project_id,
      version_id: latest.id,
      kind: "tone_adjustment",
      title: "Shrink highlights on the trapezium core",
      description:
        "Use a luminance mask to recover detail in the brightest 0.5% of pixels without affecting midtones.",
      rationale:
        "Central trapezium region is clipping in R/G/B; a targeted HDR compression restores the four-star separation.",
      parameter_patch: {
        highlights_recovery: 0.6,
        mask_radius: 12,
      },
      status: "pending",
      created_at: "2026-09-06T14:00:00Z",
    },
  ];
}