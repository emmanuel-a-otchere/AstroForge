// CR-07 B13c: provenance store.
//
// Holds the live `Recipe` (or honest "not recorded" state) for
// the currently selected Image Version. The ProvenancePanel
// (B14) subscribes to this store and renders the data.
//
// Three load outcomes:
//   1. recipe != null -> the version was produced by a named
//      Recipe profile; the panel renders Recipe name,
//      IntegrityBadge, ModelUsage list.
//   2. recipe == null + status == 'loaded' -> the version
//      exists but has no recorded recipe (legacy /
//      AI-applied); the panel renders "Profile not recorded".
//   3. status == 'error' -> the IPC call failed; the panel
//      renders the error string.
//
// The store does NOT cache across versionId changes: every
// `load(versionId)` resets to a fresh fetch. The Compare
// workspace calls `load(versionA)` + `load(versionB)` when
// the user picks two versions; B14 may add a derived store
// that keeps both loaded side by side.

import { writable, type Writable } from "svelte/store";
import { recipeGetForImageVersion } from "../lib/astroforge-api";

// CR-07 B13c: local Svelte-side Recipe shape, in camelCase
// to match the rest of the frontend. The IPC returns
// `RecipeFromRust` (snake_case) and we translate on load.
// The shape mirrors the Rust `astroforge_core::recipe::Recipe`
// struct; if that grows fields, this grows in lockstep.
export interface ProvenanceRecipe {
  schemaVersion: string;
  name: string;
  description: string;
  targetType: string;
  stages: Array<{
    stageId: string;
    enabled: boolean;
    params: Record<string, unknown>;
  }>;
  requiredModels: string[];
  integrity: {
    perceptualModelsUsed: boolean;
    deterministicModelsUsed: boolean;
    seedRecorded: boolean;
    models: Array<{
      modelName: string;
      modelType: "deterministic" | "perceptual";
    }>;
  };
  version: number;
  parentVersion: number | null;
  branch: string;
  createdAt: string;
  flags: string[];
}

export type ProvenanceStatus = "idle" | "loading" | "loaded" | "error";

export interface ProvenanceState {
  versionId: string | null;
  recipe: ProvenanceRecipe | null;
  status: ProvenanceStatus;
  error: string | null;
}

const initial: ProvenanceState = {
  versionId: null,
  recipe: null,
  status: "idle",
  error: null,
};

const internal: Writable<ProvenanceState> = writable(initial);

function fromRust(rust: import("../lib/astroforge-api").RecipeFromRust): ProvenanceRecipe {
  return {
    schemaVersion: rust.schema_version,
    name: rust.name,
    description: rust.description,
    targetType: rust.target_type,
    stages: rust.stages.map((s) => ({
      stageId: s.stage_id,
      enabled: s.enabled,
      params: s.params,
    })),
    requiredModels: rust.required_models,
    integrity: {
      perceptualModelsUsed: rust.integrity.perceptual_models_used,
      deterministicModelsUsed: rust.integrity.deterministic_models_used,
      seedRecorded: rust.integrity.seed_recorded,
      models: rust.integrity.models.map((m) => ({
        modelName: m.model_name,
        modelType: m.model_type,
      })),
    },
    version: rust.version,
    parentVersion: rust.parent_version,
    branch: rust.branch,
    createdAt: rust.created_at,
    flags: rust.flags,
  };
}

export const provenanceStore = {
  subscribe: internal.subscribe,

  /** Load the provenance Recipe for the given Image Version.
   *  Resets the store to `loading`, fetches
   *  `recipeGetForImageVersion`, and sets either the live
   *  Recipe or the honest "not recorded" state. The store
   *  is also reset via `reset()` when the Compare workspace
   *  unmounts (handled by the component, not here). */
  async load(versionId: string): Promise<void> {
    internal.update((s) => ({
      ...s,
      versionId,
      status: "loading",
      error: null,
    }));
    try {
      const rust = await recipeGetForImageVersion(versionId);
      internal.update((s) => ({
        ...s,
        // Note: the versionId may have changed while the
        // IPC was in flight (the user clicked another
        // version). When that happens, ignore the stale
        // result: the newer load() will overwrite.
        ...(s.versionId === versionId
          ? {
              recipe: rust ? fromRust(rust) : null,
              status: "loaded" as const,
              error: null,
            }
          : {}),
      }));
    } catch (e) {
      internal.update((s) => ({
        ...s,
        ...(s.versionId === versionId
          ? {
              recipe: null,
              status: "error" as const,
              error: e instanceof Error ? e.message : String(e),
            }
          : {}),
      }));
    }
  },

  reset(): void {
    internal.set(initial);
  },
};
