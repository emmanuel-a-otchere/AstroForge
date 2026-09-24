/**
 * ProfileStore — telescope pipeline profile persistence.
 *
 * Phase 1.5 PR-A: backs the "telescope pipeline profile" feature
 * (docs/PROFILE_PIPELINES_PLAN.md). The Svelte side calls into this
 * module to list available profiles, retrieve a specific version, and
 * save new versions of a profile. Reads use Recipe::from_json_migrated
 * (via Rust) so older v1 payloads upgrade transparently.
 *
 * Field naming: Rust serialises with snake_case (e.g. `target_type`),
 * the Svelte side uses camelCase for consistency with the rest of the
 * frontend (e.g. `targetType`). The mapping happens at the IPC boundary
 * here.
 *
 * Browser fallback: when running in plain Vite (no Tauri shell),
 * `invoke()` throws. We fall back to an in-memory placeholder so the
 * UI still renders during dev. The placeholder carries the DwarfII v1
 * profile so the user can experiment with the data shape without
 * needing the full Tauri stack.
 */

import { writable, type Writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

export interface RecipeStage {
  stageId: string;
  enabled: boolean;
  params: Record<string, unknown>;
  // CR-08 §10.2 Guided tier: per-stage AI
  // Enhancement Level override. Optional so
  // legacy callers + browser-mode fixtures
  // that predate §10.2 still type-check.
  aiEnhancementOverride?:
    | "off"
    | "conservative"
    | "recommended"
    | "advanced"
    | null;
}

export interface IntegrityBadge {
  perceptualModelsUsed: boolean;
  deterministicModelsUsed: boolean;
  seedRecorded: boolean;
  models: ModelUsage[];
}

export interface ModelUsage {
  modelName: string;
  modelType: "deterministic" | "perceptual";
}

export interface Recipe {
  schemaVersion: string;
  name: string;
  description: string;
  targetType: string;
  stages: RecipeStage[];
  requiredModels: string[];
  integrity: IntegrityBadge;
  version: number;
  parentVersion: number | null;
  branch: string;
  createdAt: string;
  flags: string[];
  // CR-08 §15: Quality Profile axis. Defaults to
  // "natural" for legacy Recipes that did not
  // carry this field. The §15 Recipe Detail
  // surface reads this field to render the
  // "Style" meta-row. Variants mirror
  // `astroforge-api.ts::QUALITY_PROFILES` so the
  // TS contract stays in sync with the Rust enum.
  qualityProfile?:
    | "natural"
    | "detail"
    | "clean"
    | "publication";
  // CR-08 §10: Beginner-tier pick. Optional for
  // forward-compat with legacy Recipes; absent
  // means Recommended (the Rust serde default).
  aiEnhancementLevel?:
    | "off"
    | "conservative"
    | "recommended"
    | "advanced";
  // CR-08 §10.2 Guided tier: high-level
  // processing objectives the Recipe is designed
  // to honour.
  processingObjectives?: Array<
    | "preserve_star_colors"
    | "maximize_detail"
    | "maximize_smoothness"
    | "maximize_dynamic_range"
    | "maximize_reproducibility"
  >;
  // CR-08 §10.2 Guided tier: measurable quality
  // criteria the Recipe targets. Each field is
  // optional; absent fields don't constrain the
  // Recipe. Validated by §20 ranges.
  qualityTargets?: {
    target_snr_db?: number | null;
    target_sharpness?: number | null;
    target_background_smoothness?: number | null;
  };
  // CR-08 §10.2 Guided tier: optional stage IDs
  // the user elected to enable beyond the
  // §10.1 Beginner defaults.
  optionalOperations?: string[];
}

export interface RecipeSummary {
  id: number;
  profileId: string;
  schemaVersion: string;
  name: string;
  description: string;
  targetType: string;
  version: number;
  parentVersion: number | null;
  branch: string;
  createdAt: string;
  // CR-08 §15: 5-tab classification flags surfaced from
  // the Rust `RecipeSummary`. `isSystem` and `isArchived`
  // were already exposed via `mark_as_system` /
  // `archive_profile` IPCs (CR-08 §3.1, §22.4); the §15
  // slice folds them onto the head row so the UI can
  // classify without a second round-trip.
  isSystem: boolean;
  isArchived: boolean;
  isImported: boolean;
  lastUsedAt: string | null;
}

export interface RecipeVersion {
  version: number;
  parentVersion: number | null;
  branch: string;
  createdAt: string;
}

// ─── Rust-side snake_case shape (matches astroforge_core::recipe) ────────────

interface RustRecipe {
  schema_version: string;
  name: string;
  description: string;
  target_type: string;
  stages: RustRecipeStage[];
  required_models: string[];
  integrity: RustIntegrityBadge;
  version: number;
  parent_version: number | null;
  branch: string;
  created_at: string;
  flags: string[];
  // CR-08 §15: Quality Profile axis. Snake-case
  // mirror of the Rust enum so the IPC wire shape
  // matches the Rust struct 1:1. Variants mirror
  // `astroforge-api.ts::QUALITY_PROFILES`.
  quality_profile?:
    | "natural"
    | "detail"
    | "clean"
    | "publication";
  // CR-08 §10: Beginner-tier pick.
  ai_enhancement_level?:
    | "off"
    | "conservative"
    | "recommended"
    | "advanced";
  // CR-08 §10.2 Guided tier: high-level
  // processing objectives.
  processing_objectives?: Array<
    | "preserve_star_colors"
    | "maximize_detail"
    | "maximize_smoothness"
    | "maximize_dynamic_range"
    | "maximize_reproducibility"
  >;
  // CR-08 §10.2 Guided tier: measurable quality
  // criteria.
  quality_targets?: {
    target_snr_db?: number | null;
    target_sharpness?: number | null;
    target_background_smoothness?: number | null;
  };
  // CR-08 §10.2 Guided tier: optional stage IDs.
  optional_operations?: string[];
}

interface RustRecipeStage {
  stage_id: string;
  enabled: boolean;
  params: Record<string, unknown>;
  // CR-08 §10.2 Guided tier: per-stage AI
  // Enhancement Level override. Optional so
  // legacy callers + browser-mode fixtures
  // that predate §10.2 still type-check.
  ai_enhancement_override?:
    | "off"
    | "conservative"
    | "recommended"
    | "advanced"
    | null;
}

interface RustIntegrityBadge {
  perceptual_models_used: boolean;
  deterministic_models_used: boolean;
  seed_recorded: boolean;
  models: RustModelUsage[];
}

interface RustModelUsage {
  model_name: string;
  model_type: "deterministic" | "perceptual";
}

interface RustRecipeSummary {
  id: number;
  profile_id: string;
  schema_version: string;
  name: string;
  description: string;
  target_type: string;
  version: number;
  parent_version: number | null;
  branch: string;
  created_at: string;
  // CR-08 §15: Recipe Library 5-tab layout reads these
  // flags to classify each head row into System / My /
  // Project / Imported / Recently Used without an
  // extra round-trip.
  is_system: boolean;
  is_archived: boolean;
  is_imported: boolean;
  last_used_at: string | null;
}

interface RustRecipeVersion {
  version: number;
  parent_version: number | null;
  branch: string;
  created_at: string;
}

// ─── Mapping (Rust ↔ TS) ────────────────────────────────────────────────────

function fromRustRecipe(r: RustRecipe): Recipe {
  return {
    schemaVersion: r.schema_version,
    name: r.name,
    description: r.description,
    targetType: r.target_type,
    stages: r.stages.map((s) => ({
      stageId: s.stage_id,
      enabled: s.enabled,
      params: s.params,
    })),
    requiredModels: r.required_models,
    integrity: {
      perceptualModelsUsed: r.integrity.perceptual_models_used,
      deterministicModelsUsed: r.integrity.deterministic_models_used,
      seedRecorded: r.integrity.seed_recorded,
      models: r.integrity.models.map((m) => ({
        modelName: m.model_name,
        modelType: m.model_type,
      })),
    },
    version: r.version,
    parentVersion: r.parent_version,
    branch: r.branch,
    createdAt: r.created_at,
    flags: r.flags,
  };
}

function toRustRecipe(r: Recipe): RustRecipe {
  return {
    schema_version: r.schemaVersion,
    name: r.name,
    description: r.description,
    target_type: r.targetType,
    stages: r.stages.map((s) => ({
      stage_id: s.stageId,
      enabled: s.enabled,
      params: s.params,
      // CR-08 §10.2 Guided tier: per-stage AI
      // Enhancement Level override. The TS field
      // is optional so legacy Recipes (and the
      // existing Phase 1.5 fixtures) ship without
      // it; the Rust side defaults to `None`.
      ai_enhancement_override: s.aiEnhancementOverride ?? null,
    })),
    required_models: r.requiredModels,
    integrity: {
      perceptual_models_used: r.integrity.perceptualModelsUsed,
      deterministic_models_used: r.integrity.deterministicModelsUsed,
      seed_recorded: r.integrity.seedRecorded,
      models: r.integrity.models.map((m) => ({
        model_name: m.modelName,
        model_type: m.modelType,
      })),
    },
    version: r.version,
    parent_version: r.parentVersion,
    branch: r.branch,
    created_at: r.createdAt,
    flags: r.flags,
    // CR-08 §15 / §10 / §10.2: forward the new
    // schema-version 2 fields. All four are
    // optional on the Rust side (`#[serde(default)]`
    // or skip_serializing_if), so omitting a field
    // round-trips to the Rust default (Natural /
    // Recommended / empty vec / None).
    quality_profile: r.qualityProfile ?? "natural",
    ai_enhancement_level:
      r.aiEnhancementLevel ?? "recommended",
    processing_objectives: r.processingObjectives ?? [],
    quality_targets: r.qualityTargets
      ? {
          target_snr_db:
            r.qualityTargets.target_snr_db ?? null,
          target_sharpness:
            r.qualityTargets.target_sharpness ?? null,
          target_background_smoothness:
            r.qualityTargets.target_background_smoothness ??
            null,
        }
      : {
          target_snr_db: null,
          target_sharpness: null,
          target_background_smoothness: null,
        },
    optional_operations: r.optionalOperations ?? [],
  };
}

function fromRustSummary(s: RustRecipeSummary): RecipeSummary {
  return {
    id: s.id,
    profileId: s.profile_id,
    schemaVersion: s.schema_version,
    name: s.name,
    description: s.description,
    targetType: s.target_type,
    version: s.version,
    parentVersion: s.parent_version,
    branch: s.branch,
    createdAt: s.created_at,
    // CR-08 §15: pipe the 5-tab classification flags
    // through to the UI. Defaults match the Rust struct's
    // serde defaults (false / false / false / null) so
    // older payloads that predate §15 still parse.
    isSystem: s.is_system ?? false,
    isArchived: s.is_archived ?? false,
    isImported: s.is_imported ?? false,
    lastUsedAt: s.last_used_at ?? null,
  };
}

function fromRustVersion(v: RustRecipeVersion): RecipeVersion {
  return {
    version: v.version,
    parentVersion: v.parent_version,
    branch: v.branch,
    createdAt: v.created_at,
  };
}

// ─── Tauri availability check ────────────────────────────────────────────────

function isTauri(): boolean {
  return (
    typeof window !== "undefined" &&
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (window as any).__TAURI_INTERNALS__ !== undefined
  );
}

// ─── In-memory placeholder (browser dev mode) ───────────────────────────────

function dwarf2V1Placeholder(): Recipe {
  return {
    schemaVersion: "2.0",
    name: "DwarfII Smart Telescope \u00b7 OSC",
    description:
      "Core-preserved detail enhancement for DwarfII Smart Telescope OSC captures. (Browser-mode placeholder; full profile ships via Tauri.)",
    targetType: "smart_telescope_osc",
    stages: [
      { stageId: "ingest", enabled: true, params: { hotPixelSigma: 3.0 } },
      {
        stageId: "background_extraction",
        enabled: true,
        params: { polyOrder: 3, model: "polynomial" },
      },
      {
        stageId: "denoise",
        enabled: true,
        params: { method: "wavelet", layers: 3, strength: 0.25, edgeProtect: true },
      },
      {
        stageId: "color_wb",
        enabled: true,
        params: { wbReference: "G2V", bgNeutralRGB: [25, 25, 25] },
      },
      {
        stageId: "stretch",
        enabled: true,
        params: { blackPoint: 0.02, midtone: 0.4, highlights: 0.98 },
      },
      {
        stageId: "sharpen_deconvolution",
        enabled: true,
        params: {
          psfRadius: 2.0,
          iterations: 15,
          coreProtectRequired: true,
          coreProtectFalloff: 0.6,
        },
      },
      {
        stageId: "creative_polish",
        enabled: true,
        params: {
          claheRadius: 45,
          claheClip: 0.012,
          saturationBoost: 0.1,
          haOnly: true,
          upscaleTarget: [4096, 3072],
          resampleMethod: "lanczos",
          unsharpRadius: 1.2,
          unsharpAmount: 0.25,
        },
      },
      {
        stageId: "color_scnr",
        enabled: true,
        params: { strength: 0.6, targetChannel: "green" },
      },
    ],
    requiredModels: [],
    integrity: {
      perceptualModelsUsed: false,
      deterministicModelsUsed: false,
      seedRecorded: false,
      models: [],
    },
    version: 1,
    parentVersion: null,
    branch: "main",
    createdAt: "2026-09-05T00:00:00Z",
    flags: [],
  };
}

const PLACEHOLDER_SUMMARIES: RecipeSummary[] = [
  {
    id: 1,
    profileId: "prof_dwarfii-smart-telescope-osc_smart-telescope-osc",
    schemaVersion: "2.0",
    name: "DwarfII Smart Telescope \u00b7 OSC",
    description:
      "Core-preserved detail enhancement for DwarfII Smart Telescope OSC captures. (Browser-mode placeholder.)",
    targetType: "smart_telescope_osc",
    version: 1,
    parentVersion: null,
    branch: "main",
    createdAt: "2026-09-05T00:00:00Z",
    isSystem: false,
    isArchived: false,
    isImported: false,
    lastUsedAt: null,
  },
];

// ─── Store + actions ─────────────────────────────────────────────────────────

export const profileStore: Writable<RecipeSummary[]> = writable([]);

export async function loadProfiles(): Promise<RecipeSummary[]> {
  if (!isTauri()) {
    profileStore.set(PLACEHOLDER_SUMMARIES);
    return PLACEHOLDER_SUMMARIES;
  }
  const rust: RustRecipeSummary[] = await invoke("recipe_list");
  const summaries = rust.map(fromRustSummary);
  profileStore.set(summaries);
  return summaries;
}

export async function listProfileVersions(
  profileId: string,
): Promise<RecipeVersion[]> {
  if (!isTauri()) {
    return [{ version: 1, parentVersion: null, branch: "main", createdAt: "2026-09-05T00:00:00Z" }];
  }
  const rust: RustRecipeVersion[] = await invoke("recipe_list_versions", {
    profileId,
  });
  return rust.map(fromRustVersion);
}

export async function getProfile(
  profileId: string,
  version: number,
): Promise<Recipe> {
  if (!isTauri()) {
    if (version === 1) return dwarf2V1Placeholder();
    throw new Error(
      `Profile version not found in placeholder: profileId=${profileId}, version=${version}`,
    );
  }
  const rust: RustRecipe = await invoke("recipe_get", { profileId, version });
  return fromRustRecipe(rust);
}

export async function getProfileHead(profileId: string): Promise<Recipe> {
  if (!isTauri()) return dwarf2V1Placeholder();
  const rust: RustRecipe = await invoke("recipe_get_head", { profileId });
  return fromRustRecipe(rust);
}

export async function saveProfile(recipe: Recipe): Promise<RecipeSummary> {
  if (!isTauri()) {
    // In browser mode, "save" just bumps the in-memory placeholder so
    // the UI gets something to render. Real persistence lives in Tauri.
    const next: RecipeSummary = {
      ...PLACEHOLDER_SUMMARIES[0],
      version: recipe.version || PLACEHOLDER_SUMMARIES[0].version + 1,
      parentVersion: recipe.parentVersion,
      description: recipe.description,
    };
    PLACEHOLDER_SUMMARIES[0] = next;
    profileStore.set([...PLACEHOLDER_SUMMARIES]);
    return next;
  }
  const rustRecipe = toRustRecipe(recipe);
  const rustSummary: RustRecipeSummary = await invoke("recipe_save", {
    recipe: rustRecipe,
  });
  const summary = fromRustSummary(rustSummary);
  profileStore.set([summary, ...PLACEHOLDER_SUMMARIES]);
  return summary;
}

/**
 * CR-08 §22.1: duplicate an existing Recipe into a fresh
 * independent profile. Mirrors the Rust `recipe_duplicate` IPC:
 * reads the source Recipe, returns a RecipeSummary for the
 * new "(Copy)"-named profile. Caller refreshes profileStore
 * afterwards if they want the list to include the new entry.
 */
export async function duplicateProfile(
  profileId: string,
  version: number,
): Promise<RecipeSummary> {
  if (!isTauri()) {
    // In browser mode, synthesize a new summary off the
    // placeholder so the UI flow exercises end-to-end.
    const next: RecipeSummary = {
      ...PLACEHOLDER_SUMMARIES[0],
      profileId: `${profileId}-copy-${Date.now()}`,
      name: `${PLACEHOLDER_SUMMARIES[0].name} (Copy)`,
      version: 1,
      parentVersion: null,
    };
    PLACEHOLDER_SUMMARIES.push(next);
    profileStore.set([...PLACEHOLDER_SUMMARIES]);
    return next;
  }
  const rustSummary: RustRecipeSummary = await invoke("recipe_duplicate", {
    profileId,
    version,
  });
  const summary = fromRustSummary(rustSummary);
  // Optimistic list refresh: prepend the new profile.
  profileStore.update((list) => [summary, ...list]);
  return summary;
}

/**
 * CR-08 §19: export a Recipe (any profile; `version`
 * omitted exports the head) as its canonical JSON payload.
 * The file-save dialog + `.afrecipe` filename convention
 * are a UI follow-on; this returns the content.
 */
export async function exportProfile(
  profileId: string,
  version?: number,
): Promise<string> {
  if (!isTauri()) {
    const hit = PLACEHOLDER_SUMMARIES.find(
      (s) => s.profileId === profileId,
    );
    if (!hit) {
      throw new Error(`profile not found: ${profileId}`);
    }
    // Minimal placeholder payload mirroring the Rust shape.
    return JSON.stringify(
      {
        schema_version: "2.0",
        version: hit.version,
        parent_version: null,
        name: hit.name,
        target_type: hit.targetType,
        description: "",
        created_at: hit.createdAt,
        stages: [],
        required_models: [],
        integrity: {
          perceptual_models_used: false,
          deterministic_models_used: true,
          seed_recorded: false,
          models: [],
        },
        branch: "main",
        quality_profile: "natural",
        flags: [],
      },
      null,
      2,
    );
  }
  return (await invoke("recipe_export", {
    profileId,
    version: version ?? null,
  })) as string;
}

/**
 * CR-08 §19: import a Recipe from its canonical JSON
 * payload. The Rust handler re-lineages the recipe against
 * the local store: fresh names start at v1; matching
 * (name, target_type) profiles append as the next version
 * with `parent_version` pointing at the local head.
 */
export async function importProfile(
  json: string,
): Promise<RecipeSummary> {
  if (!isTauri()) {
    const parsed = JSON.parse(json) as {
      name?: string;
      target_type?: string;
      version?: number;
    };
    const name = (parsed.name ?? "").trim();
    const targetType = (parsed.target_type ?? "").trim();
    if (!name || !targetType) {
      throw new Error("imported recipe has an empty name or target_type");
    }
    const summary: RecipeSummary = {
      id: 0,
      profileId: profileIdFor(name, targetType),
      schemaVersion: "2.0",
      name,
      description: "",
      targetType,
      version: 1,
      parentVersion: null,
      branch: "main",
      createdAt: new Date().toISOString(),
      // §15: in browser-mode the import flow has no Rust
      // round-trip, so we set isImported optimistically;
      // the Tauri-mode round-trip below picks up the real
      // server-side flag via `fromRustSummary`.
      isSystem: false,
      isArchived: false,
      isImported: true,
      lastUsedAt: null,
    };
    PLACEHOLDER_SUMMARIES.unshift(summary);
    profileStore.set([...PLACEHOLDER_SUMMARIES]);
    return summary;
  }
  const rust = (await invoke("recipe_import", { json })) as RustRecipeSummary;
  const summary = fromRustSummary(rust);
  // Optimistic list refresh: if the profile already exists
  // in the cache, replace its entry with the new head;
  // otherwise prepend.
  profileStore.update((list) => {
    const idx = list.findIndex(
      (s) => s.profileId === summary.profileId,
    );
    if (idx >= 0) {
      const next = [...list];
      next[idx] = summary;
      return next;
    }
    return [summary, ...list];
  });
  return summary;
}

/**
 * CR-08 §3.1: mark a Recipe profile as a system Recipe.
 * Mirrors the Rust `recipe_mark_as_system` IPC: all
 * existing versions of the profile get `is_system = 1`
 * in the on-disk column, after which `recipe_save` and
 * `recipe_delete` refuse to mutate the profile. Returns
 * true when at least one row was updated, false when
 * the profile does not exist yet.
 */
export async function markProfileAsSystem(
  profileId: string,
): Promise<boolean> {
  if (!isTauri()) {
    // Browser-mode placeholder: mark the matching summary
    // in PLACEHOLDER_SUMMARIES.
    let touched = false;
    for (const s of PLACEHOLDER_SUMMARIES) {
      if (s.profileId === profileId) {
        // The browser-mode placeholder does not persist
        // is_system back to the Summary; the optimistic
        // local update is enough for end-to-end UI flow.
        touched = true;
      }
    }
    if (touched) profileStore.set([...PLACEHOLDER_SUMMARIES]);
    return touched;
  }
  const updated = (await invoke("recipe_mark_as_system", {
    profileId,
  })) as boolean;
  return updated;
}

/**
 * CR-08 §3.1: read whether any version of a profile is
 * currently marked as a system Recipe. The UI uses this
 * to render the "System" badge + gate destructive
 * actions in the RecipesScreen toolbar.
 */
export async function isSystemProfile(
  profileId: string,
): Promise<boolean> {
  if (!isTauri()) {
    // Browser-mode placeholder: none of the seeded
    // summaries are system Recipes.
    return false;
  }
  const value = (await invoke("recipe_is_system", {
    profileId,
  })) as boolean;
  return value;
}

/**
 * CR-08 §22.4: archive a Recipe profile. Archived
 * profiles remain on disk but are hidden from the
 * default `list()` results. Mirrors the Rust
 * `recipe_archive` IPC. Returns true when at least one
 * row was updated, false when the profile does not
 * exist yet.
 */
export async function archiveProfile(
  profileId: string,
): Promise<boolean> {
  if (!isTauri()) {
    // Browser-mode placeholder: none of the seeded
    // summaries are archived; the toggle is a no-op
    // until the IPC returns a real result.
    return false;
  }
  const updated = (await invoke("recipe_archive", {
    profileId,
  })) as boolean;
  return updated;
}

/**
 * CR-08 §22.4: unarchive a Recipe profile. Mirrors the
 * Rust `recipe_unarchive` IPC. Returns true when at
 * least one row was updated.
 */
export async function unarchiveProfile(
  profileId: string,
): Promise<boolean> {
  if (!isTauri()) {
    return false;
  }
  const updated = (await invoke("recipe_unarchive", {
    profileId,
  })) as boolean;
  return updated;
}

/**
 * CR-08 §22.4: read whether any version of a profile is
 * currently archived. Mirrors the Rust
 * `recipe_is_archived` IPC.
 */
export async function isArchivedProfile(
  profileId: string,
): Promise<boolean> {
  if (!isTauri()) {
    // Browser-mode placeholder: none of the seeded
    // summaries are archived.
    return false;
  }
  const value = (await invoke("recipe_is_archived", {
    profileId,
  })) as boolean;
  return value;
}

/**
 * CR-08 §22.2: delete a Recipe profile (every version).
 * Mirrors the Rust `recipe_delete` IPC: returns the number
 * of versions deleted. Idempotent: deleting a profile that
 * does not exist returns 0.
 */
export async function deleteProfile(profileId: string): Promise<number> {
  if (!isTauri()) {
    // In browser mode, remove the placeholder entries that
    // match so the UI flow exercises end-to-end.
    const before = PLACEHOLDER_SUMMARIES.length;
    for (let i = PLACEHOLDER_SUMMARIES.length - 1; i >= 0; i--) {
      if (PLACEHOLDER_SUMMARIES[i].profileId === profileId) {
        PLACEHOLDER_SUMMARIES.splice(i, 1);
      }
    }
    profileStore.set([...PLACEHOLDER_SUMMARIES]);
    return before - PLACEHOLDER_SUMMARIES.length;
  }
  const deleted = (await invoke("recipe_delete", { profileId })) as number;
  // Optimistic removal from the cached list.
  profileStore.update((list) =>
    list.filter((s) => s.profileId !== profileId),
  );
  return deleted;
}

/**
 * CR-08 §22.3: apply a Recipe and return the stage-id +
 * params pairs the caller should drive the next apply
 * round with. Mirrors the Rust `recipe_apply` IPC: loads
 * the Recipe via the store, performs the schema-version
 * guard + missing-models compatibility check, and
 * returns the enabled stages in order.
 *
 * `availableModels` is the caller's view of which models
 * the running app currently has registered. The
 * orchestrator's model list is the canonical source;
 * passing `undefined` skips the missing-models check
 * (the schema-version guard still runs).
 *
 * Per-stage parameters are returned in Recipe order.
 * This keeps the IPC a pure function over the Recipe
 * store; the chained apply is the UI's job.
 */
export interface RecipeApplyStage {
  stageId: string;
  params: Record<string, unknown>;
}
export interface RecipeApplyResult {
  profileId: string;
  version: number;
  branch: string;
  stages: RecipeApplyStage[];
}
export async function applyRecipe(
  profileId: string,
  version: number,
  availableModels?: string[],
): Promise<RecipeApplyResult> {
  if (!isTauri()) {
    // Browser-mode placeholder: read the first cached
    // summary, return an empty stages list so the UI
    // flow exercises end-to-end without IPC.
    const cached = PLACEHOLDER_SUMMARIES.find(
      (s) => s.profileId === profileId,
    );
    return {
      profileId,
      version: cached?.version ?? version,
      branch: cached?.branch ?? "main",
      stages: [],
    };
  }
  const result = (await invoke("recipe_apply", {
    profileId,
    version,
    availableModels: availableModels ?? null,
  })) as RecipeApplyResult;
  return result;
}

/**
 * CR-08 §14: "Save Pipeline as Recipe" UX. Loads a
 * PipelinePlan via the Rust `recipe_save_from_pipeline_plan`
 * IPC, builds a Recipe from the plan's stages, and
 * persists it via `RecipeStore::save`. Returns the
 * `RecipeSummary` of the saved v1 Recipe.
 *
 * @param planId  The `PipelinePlan.plan_id` to read.
 * @param name  The new Recipe's name (UI-side input).
 * @param targetType  Optional override; when omitted, the
 *   plan's `target_type` (serialized snake_case) is used.
 */
export async function savePipelinePlanAsRecipe(
  planId: string,
  name: string,
  targetType?: string,
): Promise<RecipeSummary> {
  if (!isTauri()) {
    // Browser-mode placeholder: build a synthetic summary
    // so the UI flow exercises end-to-end.
    const profileId = profileIdFor(name, targetType ?? "unknown");
    const summary: RecipeSummary = {
      id: Math.floor(Date.now()),
      profileId,
      schemaVersion: "2.0",
      name,
      description: `Saved from pipeline plan ${planId}`,
      targetType: targetType ?? "unknown",
      version: 1,
      parentVersion: null,
      branch: "main",
      createdAt: new Date().toISOString(),
      isSystem: false,
      isArchived: false,
      isImported: false,
      lastUsedAt: null,
    };
    return summary;
  }
  const rust = (await invoke("recipe_save_from_pipeline_plan", {
    planId,
    name,
    targetType: targetType ?? null,
  })) as RustRecipeSummary;
  return fromRustSummary(rust);
}

/**
 * CR-08 §22 round 2 / Slice B: "Save Processing as
 * Recipe" UX. Loads the `StageRunRecord`s for a
 * pipeline run via the Rust `recipe_save_from_stage_runs`
 * IPC, builds a Recipe from the terminal attempt per
 * stage, and persists it via `RecipeStore::save`.
 * Returns the `RecipeSummary` of the saved v1 Recipe.
 *
 * This is the execution-history sibling of
 * `savePipelinePlanAsRecipe` (§14): where §14 reads
 * from the planned `PipelinePlan`, Slice B reads from
 * the actual `StageRunRecord`s the engine produced
 * (so the saved Recipe captures what the engine really
 * did, including any per-stage overrides that diverged
 * from the plan).
 *
 * @param runId  The `PipelineRun.id` whose stage runs
 *   will be folded into the Recipe.
 * @param name  The new Recipe's name (UI-side input).
 * @param targetType  Optional override; when omitted,
 *   the Rust side falls back to `"unknown"` (the
 *   stage records don't carry a target_type; callers
 *   should pass one when they know it — typically the
 *   ImageVersion's target_type).
 */
export async function saveProcessingAsRecipe(
  runId: string,
  name: string,
  targetType?: string,
): Promise<RecipeSummary> {
  if (!isTauri()) {
    // Browser-mode placeholder: build a synthetic summary
    // so the UI flow exercises end-to-end.
    const profileId = profileIdFor(name, targetType ?? "unknown");
    const summary: RecipeSummary = {
      id: Math.floor(Date.now()),
      profileId,
      schemaVersion: "2.0",
      name,
      description: `Saved from processing run ${runId}`,
      targetType: targetType ?? "unknown",
      version: 1,
      parentVersion: null,
      branch: "main",
      createdAt: new Date().toISOString(),
      isSystem: false,
      isArchived: false,
      isImported: false,
      lastUsedAt: null,
    };
    return summary;
  }
  const rust = (await invoke("recipe_save_from_stage_runs", {
    runId,
    name,
    targetType: targetType ?? null,
  })) as RustRecipeSummary;
  return fromRustSummary(rust);
}

/**
 * Compute the deterministic profile_id for a (name, targetType) pair,
 * mirroring `RecipeStore::profile_id_for` in Rust. Useful for lookups
 * that don't have the summary in hand.
 */
export function profileIdFor(name: string, targetType: string): string {
  const sanitize = (s: string): string =>
    s
      .toLowerCase()
      .replace(/[^a-z0-9_-]+/g, "-")
      .replace(/^-+|-+$/g, "");
  return `prof_${sanitize(name)}_${sanitize(targetType)}`;
}
