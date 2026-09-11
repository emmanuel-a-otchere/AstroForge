// CR-06 P1 — AI Enhancement Studio frontend state.
//
// P1 only persists the data model + provenance fields. The
// substantive behavior (analysis, recommendations, enhancement
// operations, masks, stacks, previews) lands in P2–P6; this
// module exposes:
//
//   - the safety-classification string union for the UI
//     (mirrors the Rust `AiSafetyClassification` enum)
//   - a per-image-version store keyed by id, with the
//     `loadFor(imageVersionId)` method that the project
//     lifecycle calls on open
//   - a `reset()` method the lifecycle calls on close
//
// The store keeps results as `unknown[]` (the wire shape)
// until P2 defines the analysis profile contract and P3
// defines the recommendation payload contract. Consumers in
// those phases will narrow the types as the contracts land.

import { get, writable, type Writable } from "svelte/store";
import {
  aiMaskList,
  aiOperationListForStage,
  aiRecommendationListForVersion,
  analyzeImage,
  enhancementApplyOperation,
  enhancementOperationsList,
  enhancementPreviewListForOperation,
  enhancementStackApplyMutation,
  enhancementStackBranch,
  enhancementStackCreate,
  enhancementStackGet,
  enhancementStackListForSource,
  generateAiRecommendations,
  imageAnalysisLatest,
  imageRegionList,
  imageVersionListForProject,
  type AnalyzeImageRequest,
  type ApplyAiOperationRequest,
  type BranchEnhancementStackRequest,
  type CreateEnhancementStackRequest,
  type EnhancementStackJson,
  type GenerateRecommendationsRequest,
  type OperationRegistryEntryJson,
  type StackMutation as StackMutationJson,
} from "../lib/astroforge-api";

/**
 * CR-06 §16 — three-way safety classification. The string
 * union mirrors the Rust `AiSafetyClassification` enum
 * (snake_case) so the wire shape round-trips without
 * translation. P2+ will surface this in every AI operation
 * card and applied-version header per ADR-06.3.
 */
export type AiSafetyClassification =
  | "deterministic"
  | "perceptual"
  | "generative";

export const AiSafetyClassification: Readonly<Record<AiSafetyClassification, AiSafetyClassification>> = Object.freeze({
  deterministic: "deterministic",
  perceptual: "perceptual",
  generative: "generative",
});

/** UI-friendly label for a safety classification. */
export function safetyLabel(c: AiSafetyClassification): string {
  switch (c) {
    case "deterministic":
      return "Deterministic";
    case "perceptual":
      return "Perceptual";
    case "generative":
      return "Generative";
  }
}

/**
 * CR-06 §17 — operations in the Perceptual / Generative
 * classes must surface a disclosure banner so the user
 * understands the result may reconstruct detail that was
 * not in the source image.
 */
export function safetyRequiresDisclosure(c: AiSafetyClassification): boolean {
  return c === "perceptual" || c === "generative";
}

/// Per-image-version AI Enhancement Studio state. */
export interface AiEnhancementState {
  analysis: unknown | null;
  regions: unknown[];
  recommendations: unknown[];
  /// CR-06 P3 — the recommendation engine's structured
  /// report (sequenced recommendations + sequencing
  /// notes). Distinct from `recommendations`, which is the
  /// flattened list of persisted `AiRecommendation` rows
  /// the store reads back; `recommendationsReport`
  /// carries the engine's full output (including the
  /// §23 sequencing rationale) so the UI can render
  /// "Detail was moved after Denoise — amplifying noise
  /// before denoise would bake it in" alongside each row.
  recommendationsReport: unknown | null;
  masks: unknown[];
  stacks: unknown[];
  previews: unknown[];
  /// CR-06 P4 — the active enhancement stack record
  /// (the typed view: ordered operations + lineage).
  /// Distinct from `stacks` (the persisted rows), the
  /// active stack is what the Studio panel mutates via
  /// `applyStackMutation` and applies via `applyOperation`.
  activeStack: EnhancementStackJson | null;
  /// CR-06 P4 — the canonical operations registry. The
  /// Studio panel renders the operation picker from this
  /// list (eleven operations across cleanup / denoise /
  /// restoration / star / detail / upscale / inpaint /
  /// background categories).
  operationsRegistry: OperationRegistryEntryJson[];
  loading: boolean;
  error: string | null;
}

const initial: AiEnhancementState = {
  analysis: null,
  regions: [],
  recommendations: [],
  recommendationsReport: null,
  masks: [],
  stacks: [],
  previews: [],
  activeStack: null,
  operationsRegistry: [],
  loading: false,
  error: null,
};

/**
 * The active project's AI Enhancement Studio state. The
 * store is project-scoped — the lifecycle calls `loadFor`
 * on open and `reset` on close (mirrors the R4 contract).
 */
export const aiEnhancementStore: Writable<AiEnhancementState> =
  writable(initial);

/**
 * Load every AI Enhancement Studio list for an Image
 * Version. Failures are non-fatal: the store records the
 * error and consumers render the `error` state. Each
 * underlying IPC is wrapped in a `try/catch` so a failure
 * in one doesn't poison the rest.
 */
export async function loadAiEnhancementFor(
  imageVersionId: string,
): Promise<void> {
  aiEnhancementStore.update((s) => ({ ...s, loading: true, error: null }));
  const update: Partial<AiEnhancementState> = {
    analysis: null,
    regions: [],
    recommendations: [],
    recommendationsReport: null,
    masks: [],
    stacks: [],
    previews: [],
    activeStack: null,
    operationsRegistry: [],
    error: null,
  };
  let firstError: string | null = null;
  const safeCall = async (
    label: string,
    fn: () => Promise<unknown>,
  ): Promise<unknown | null> => {
    try {
      return await fn();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (firstError === null) firstError = `${label}: ${msg}`;
      return null;
    }
  };
  const analysis = await safeCall("image_analysis_latest", () =>
    imageAnalysisLatest(imageVersionId),
  );
  if (analysis) update.analysis = analysis;
  const regions = await safeCall("image_region_list", () =>
    imageRegionList(imageVersionId),
  );
  if (Array.isArray(regions)) update.regions = regions;
  const recs = await safeCall("ai_recommendation_list_for_version", () =>
    aiRecommendationListForVersion(imageVersionId),
  );
  if (Array.isArray(recs)) update.recommendations = recs;
  const masks = await safeCall("ai_mask_list", () => aiMaskList(imageVersionId));
  if (Array.isArray(masks)) update.masks = masks;
  // CR-06 P4 — fetch the canonical operations registry
  // alongside the per-version lists. The Studio panel
  // renders the operation picker from this list, so
  // loading it eagerly (per project, not per image
  // version) keeps the picker interactive.
  const ops = await safeCall("enhancement_operations_list", () =>
    enhancementOperationsList(),
  );
  if (Array.isArray(ops)) update.operationsRegistry = ops;
  const stacks = await safeCall("enhancement_stack_list_for_source", () =>
    enhancementStackListForSource(imageVersionId),
  );
  if (Array.isArray(stacks)) update.stacks = stacks;
  // Previews are loaded on demand per operation (P4 will
  // call this from the operation card), so P1 leaves the
  // list empty at load time.
  void enhancementPreviewListForOperation;
  aiEnhancementStore.update((s) => ({
    ...s,
    ...update,
    loading: false,
    error: firstError,
  }));
}

/** Reset the store back to its initial empty state. */
export function resetAiEnhancement(): void {
  aiEnhancementStore.set(initial);
}

/**
 * CR-06 P2 — run the analyzer over a pixel buffer and
 * refresh the stored report. The pixel buffer is
 * expected in `[0, 1]` row-major order. The function
 * returns the persisted analysis id; the report JSON is
 * also written to the store's `analysis` field.
 */
export async function analyzeImageVersion(
  request: AnalyzeImageRequest,
): Promise<string> {
  aiEnhancementStore.update((s) => ({ ...s, loading: true, error: null }));
  try {
    const response = (await analyzeImage(request)) as {
      analysis_id: string;
      profile_json: string;
    };
    aiEnhancementStore.update((s) => ({
      ...s,
      analysis: JSON.parse(response.profile_json),
      loading: false,
      error: null,
    }));
    return response.analysis_id;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    aiEnhancementStore.update((s) => ({
      ...s,
      loading: false,
      error: msg,
    }));
    throw e;
  }
}

/**
 * The previews helper is wrapped for callers in P4; it
 * returns the current state's `previews` array. P1 only
 * preserves the binding so the lifecycle compiles cleanly.
 */
export function previewListFor(operationId: string): Promise<unknown[]> {
  return enhancementPreviewListForOperation(operationId).then((r) =>
    Array.isArray(r?.items) ? r.items : [],
  );
}

/**
 * CR-06 P1 also includes a stub for stage-scoped AI
 * operation listings. P4 surfaces the operation history of
 * a single stage execution; P1 just re-exports the API
 * wrapper so consumers can `import` it from one place.
 */
export function listAiOperationsForStage(stageRunId: string): Promise<unknown[]> {
  return aiOperationListForStage(stageRunId).then((r) =>
    Array.isArray(r?.items) ? r.items : [],
  );
}

/**
 * CR-06 P3 — run the recommendation engine over the
 * latest analysis for an Image Version. The command
 * persists the resulting `AiRecommendation` rows and
 * returns the full engine report (sequenced
 * recommendations + sequencing notes). The store keeps
 * both the report and the persisted row list so the UI
 * can render either shape.
 *
 * On failure (no analysis exists, or the IPC errors
 * out), the function records the error on the store
 * and re-throws so the caller can show a banner.
 */
export async function generateRecommendationsFor(
  request: GenerateRecommendationsRequest,
): Promise<void> {
  aiEnhancementStore.update((s) => ({ ...s, loading: true, error: null }));
  try {
    const response = await generateAiRecommendations(request);
    let report: unknown | null = null;
    if (typeof response.report_json === "string" && response.report_json.length > 0) {
      try {
        report = JSON.parse(response.report_json);
      } catch {
        report = null;
      }
    }
    const items = Array.isArray(response.recommendations?.items)
      ? response.recommendations.items
      : [];
    aiEnhancementStore.update((s) => ({
      ...s,
      recommendations: items,
      recommendationsReport: report,
      loading: false,
      error: null,
    }));
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    aiEnhancementStore.update((s) => ({
      ...s,
      loading: false,
      error: msg,
    }));
    throw e;
  }
}

// Snapshot helper used by tests / consumers that need a
// synchronous read of the current state.
export function snapshotAiEnhancement(): AiEnhancementState {
  return get(aiEnhancementStore);
}

/**
 * CR-06 P4 — create an enhancement stack from an initial
 * operation list. The Studio panel calls this once per
 * image version (after the user accepts the engine's
 * recommendation list). The new stack is stored as the
 * `activeStack` so subsequent `applyStackMutation` /
 * `applyOperation` calls operate on it.
 */
export async function createEnhancementStack(
  request: CreateEnhancementStackRequest,
): Promise<EnhancementStackJson> {
  aiEnhancementStore.update((s) => ({ ...s, loading: true, error: null }));
  try {
    const response = await enhancementStackCreate(request);
    aiEnhancementStore.update((s) => ({
      ...s,
      activeStack: response.stack,
      loading: false,
      error: null,
    }));
    return response.stack;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    aiEnhancementStore.update((s) => ({
      ...s,
      loading: false,
      error: msg,
    }));
    throw e;
  }
}

/**
 * CR-06 P4 — load a stack by id (the user clicks a
 * previous branch in the Studio panel). The fetched
 * record becomes the `activeStack`.
 */
export async function loadEnhancementStack(
  stackId: string,
): Promise<EnhancementStackJson | null> {
  aiEnhancementStore.update((s) => ({ ...s, loading: true, error: null }));
  try {
    const stack = await enhancementStackGet(stackId);
    aiEnhancementStore.update((s) => ({
      ...s,
      activeStack: stack,
      loading: false,
      error: null,
    }));
    return stack;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    aiEnhancementStore.update((s) => ({
      ...s,
      loading: false,
      error: msg,
    }));
    throw e;
  }
}

/**
 * CR-06 P4 — apply a `StackMutation` (reorder, set
 * enabled, remove, mark needs preview, append) to the
 * active stack. The Tauri command persists the updated
 * stack; the helper refreshes the local `activeStack` so
 * the UI re-renders with the new state.
 */
export async function applyStackMutation(
  stackId: string,
  mutation: StackMutationJson,
): Promise<EnhancementStackJson | null> {
  aiEnhancementStore.update((s) => ({ ...s, loading: true, error: null }));
  try {
    const stack = await enhancementStackApplyMutation(stackId, mutation);
    aiEnhancementStore.update((s) => ({
      ...s,
      activeStack: stack,
      loading: false,
      error: null,
    }));
    return stack;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    aiEnhancementStore.update((s) => ({
      ...s,
      loading: false,
      error: msg,
    }));
    throw e;
  }
}

/**
 * CR-06 P4 — branch the active stack at the given
 * cutoff. Returns the new stack record (with a fresh
 * `source_image_version_id` the caller supplies — the
 * branch round typically creates the new Image Version
 * via `applyOperation` first, then branches the stack).
 */
export async function branchEnhancementStack(
  request: BranchEnhancementStackRequest,
): Promise<EnhancementStackJson> {
  aiEnhancementStore.update((s) => ({ ...s, loading: true, error: null }));
  try {
    const stack = await enhancementStackBranch(request);
    aiEnhancementStore.update((s) => ({
      ...s,
      activeStack: stack,
      loading: false,
      error: null,
    }));
    return stack;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    aiEnhancementStore.update((s) => ({
      ...s,
      loading: false,
      error: msg,
    }));
    throw e;
  }
}

/**
 * CR-06 P4 — apply a single AI operation to the source
 * Image Version. The Tauri command runs the dispatcher
 * (P4 passthrough; P5 will swap in real ONNX
 * inference), persists a fresh Image Version row + an
 * `AiOperation` provenance row, and returns the new
 * version id.
 */
export async function applyOperation(
  request: ApplyAiOperationRequest,
): Promise<{ versionId: string }> {
  aiEnhancementStore.update((s) => ({ ...s, loading: true, error: null }));
  try {
    const response = await enhancementApplyOperation(request);
    aiEnhancementStore.update((s) => ({
      ...s,
      loading: false,
      error: null,
    }));
    return { versionId: response.image_version.version_id };
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    aiEnhancementStore.update((s) => ({
      ...s,
      loading: false,
      error: msg,
    }));
    throw e;
  }
}

/**
 * CR-06 P4 — list the Image Versions for a project,
 * sequence-asc. The Studio panel uses this to render
 * the version timeline (parent of Zone A in §13).
 */
export async function listImageVersions(
  projectId: string,
): Promise<unknown[]> {
  try {
    const response = await imageVersionListForProject(projectId);
    return Array.isArray(response?.items) ? response.items : [];
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    aiEnhancementStore.update((s) => ({
      ...s,
      error: msg,
    }));
    return [];
  }
}
