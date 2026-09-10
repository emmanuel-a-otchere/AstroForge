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
  enhancementPreviewListForOperation,
  enhancementStackListForSource,
  imageAnalysisLatest,
  imageRegionList,
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

/** Per-image-version AI Enhancement Studio state. */
export interface AiEnhancementState {
  analysis: unknown | null;
  regions: unknown[];
  recommendations: unknown[];
  masks: unknown[];
  stacks: unknown[];
  previews: unknown[];
  loading: boolean;
  error: string | null;
}

const initial: AiEnhancementState = {
  analysis: null,
  regions: [],
  recommendations: [],
  masks: [],
  stacks: [],
  previews: [],
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
    masks: [],
    stacks: [],
    previews: [],
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

// Snapshot helper used by tests / consumers that need a
// synchronous read of the current state.
export function snapshotAiEnhancement(): AiEnhancementState {
  return get(aiEnhancementStore);
}
