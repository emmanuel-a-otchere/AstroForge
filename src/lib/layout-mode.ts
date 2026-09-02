/**
 * LayoutModeStore — context-driven shell layout selection.
 *
 * The App shell renders one of four modes depending on the user's current
 * workflow stage. The mode is derived from `currentStep` (App.svelte local
 * state) and any manual override the user picks via the mode switcher.
 *
 * Modes:
 *   A — Load: floating overlay cards on persistent canvas (file selection,
 *        session setup, frame review)
 *   B — Library: 3-column gallery | canvas | workflow (landing, processing
 *        default view)
 *   C — Automagic Pro: gallery strip + tabbed screen-cards (AI-assisted
 *        parameter tuning)
 *   D — Refinement: full-screen screen-card on ambient backdrop (image
 *        refinement — denoise, stretch, histogram, compose)
 *
 * Default mapping from currentStep:
 *   landing        -> B
 *   select-files   -> A
 *   session-setup  -> A
 *   review-frames  -> A
 *   processing     -> B  (manual override unlocks C and D)
 *
 * Manual overrides are sticky until the user explicitly clears them, then
 * the store reverts to deriving from currentStep.
 */
import { writable, derived, get } from "svelte/store";

export type LayoutMode = "a" | "b" | "c" | "d";
export type AppStage =
  | "landing"
  | "select-files"
  | "session-setup"
  | "review-frames"
  | "processing";

// Writable: the user's manual pin (null = follow context)
const manualOverride = writable<LayoutMode | null>(null);

// Writable: the current workflow stage (mirrors App.svelte's currentStep)
const currentStage = writable<AppStage>("landing");

export function setStage(stage: AppStage): void {
  currentStage.set(stage);
}

export function setModeOverride(mode: LayoutMode | null): void {
  manualOverride.set(mode);
}

export function clearModeOverride(): void {
  manualOverride.set(null);
}

/**
 * Default mapping: which mode is contextually right for each stage.
 * C and D are never auto-derived — they're manual-only. That matches the
 * user's intent: A and B are the natural mode for each stage; C and D
 * are explicit "I'm in this kind of work" signals.
 */
export function defaultModeForStage(stage: AppStage): LayoutMode {
  switch (stage) {
    case "landing":
    case "processing":
      return "b";
    case "select-files":
    case "session-setup":
    case "review-frames":
      return "a";
    default:
      return "b";
  }
}

export const currentLayoutMode = derived(
  [manualOverride, currentStage],
  ([$override, $stage]) => $override ?? defaultModeForStage($stage)
);

/**
 * Returns true if the user is currently free to pick C or D. We only
 * expose mode-switch controls in stages where they make sense — landing
 * and processing. The other stages are too focused for C/D to be useful.
 */
export const modeSwitchAvailable = derived(currentStage, ($stage) => {
  return $stage === "processing" || $stage === "landing";
});

/**
 * Human-readable label for the mode switcher. Used in the header.
 */
export function labelForMode(mode: LayoutMode): string {
  switch (mode) {
    case "a":
      return "Load";
    case "b":
      return "Library";
    case "c":
      return "Automagic Pro";
    case "d":
      return "Refine";
  }
}

/**
 * Available override modes for the mode-switch control. We always include
 * A and B (contextual defaults), and C and D if the current stage
 * supports them.
 */
export function availableOverrideModes(
  stage: AppStage
): readonly LayoutMode[] {
  if (stage === "processing" || stage === "landing") {
    return ["a", "b", "c", "d"] as const;
  }
  // In focused Load stages, the only sensible override is to drop back
  // to B (Library) if the user wants to abandon load and view the gallery.
  return ["b"] as const;
}

/** Test/dev helper: get the current mode imperatively. */
export function getCurrentMode(): LayoutMode {
  return get(currentLayoutMode);
}