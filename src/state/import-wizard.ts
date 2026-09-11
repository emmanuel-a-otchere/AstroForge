// CR-04 P9 — Import Wizard state.
//
// The §12 wizard is a 4-step state machine:
//   step-1: Add Data (drop zone + scan)
//   step-2: Scanning (per-file progress)
//   step-3: Understanding (target + capture + narrowband + ambiguity dialog)
//   step-4: Confirm (override + finalise)
//
// The store owns:
//   - the current wizard step
//   - the scanned frames (from `ingest_scan_directory`)
//   - the resulting `SessionAnalysis` (from `import_analyse_session`)
//   - the user's override choices
//   - the final materialised session_id (from `import_set_materialised`)
//
// The store is intentionally small — each component owns its
// own visual state. Cross-step state lives here.

import { get, writable, type Writable } from "svelte/store";
import type {
  AssetMetadataInputJson,
  CaptureKind,
  NarrowbandComposition,
  SessionAnalysisJson,
  SessionClassificationJson,
} from "../lib/astroforge-api";
import {
  importAnalyseSession,
  importConfirm,
  importGetUnderstanding,
  importOverrideClassification,
  importSetMaterialised,
} from "../lib/astroforge-api";

export type {
  AssetMetadataInputJson,
  CaptureKind,
  NarrowbandComposition,
  SessionAnalysisJson,
  SessionClassificationJson,
};

// ─── Wizard step ──────────────────────────────────────────────

export type WizardStep = "add-data" | "scanning" | "understanding" | "confirm";

const VALID_TRANSITIONS: Record<WizardStep, WizardStep[]> = {
  "add-data": ["scanning"],
  scanning: ["understanding"],
  understanding: ["confirm", "add-data"],
  confirm: [],
};

export function canTransition(from: WizardStep, to: WizardStep): boolean {
  return VALID_TRANSITIONS[from].includes(to);
}

// ─── Scanned frame ─────────────────────────────────────────────

export interface ScannedFrame {
  path: string;
  frameType: string;
  exptime: number | null;
  filter: string | null;
  width: number | null;
  height: number | null;
  binning: number | null;
  anomalies: string[];
}

// ─── State ─────────────────────────────────────────────────────

export interface ImportWizardState {
  step: WizardStep;
  scanDir: string;
  frames: ScannedFrame[];
  scanning: boolean;
  scanError: string | null;
  sessionId: string | null;
  projectId: string | null;
  analysis: SessionAnalysisJson | null;
  classification: SessionClassificationJson | null;
  // User overrides captured in the Confirm step.
  overrideCaptureKind: CaptureKind | null;
  overrideNarrowbandComposition: NarrowbandComposition | null;
}

const initialState: ImportWizardState = {
  step: "add-data",
  scanDir: "",
  frames: [],
  scanning: false,
  scanError: null,
  sessionId: null,
  projectId: null,
  analysis: null,
  classification: null,
  overrideCaptureKind: null,
  overrideNarrowbandComposition: null,
};

export const importWizard: Writable<ImportWizardState> =
  writable(initialState);

// ─── Helpers ───────────────────────────────────────────────────

export function resetImportWizard(): void {
  importWizard.set({ ...initialState });
}

export function setScanDir(dir: string): void {
  importWizard.update((s) => ({ ...s, scanDir: dir }));
}

export function setScanning(scanning: boolean): void {
  importWizard.update((s) => ({ ...s, scanning }));
}

export function setScanError(err: string | null): void {
  importWizard.update((s) => ({ ...s, scanError: err }));
}

export function setFrames(frames: ScannedFrame[]): void {
  importWizard.update((s) => ({ ...s, frames }));
}

export function goToStep(next: WizardStep): void {
  importWizard.update((s) => {
    if (!canTransition(s.step, next)) {
      // No-op: silently reject invalid transitions.
      // Components are responsible for guarding before
      // calling `goToStep`.
      return s;
    }
    return { ...s, step: next };
  });
}

// ─── Map IngestFrameDto → AssetMetadataInput ────────────────────

export function framesToAssetMetadataInput(
  frames: ScannedFrame[],
): AssetMetadataInputJson[] {
  return frames.map((f) => ({
    path: f.path,
    // The orchestrator only reads a small subset of fields:
    // filter, exptime (as exptime), naxis1/naxis2 (as
    // width/height). Everything else is best-effort null
    // — the analysis runs on what it has.
    metadata: {
      object: null,
      exptime: f.exptime,
      filter: f.filter,
      xbinning: f.binning,
      ybinning: f.binning,
      ccd_temp: null,
      naxis1: f.width,
      naxis2: f.height,
      bitpix: null,
      bayerpat: null,
      telescop: null,
      instrume: null,
      focallen: null,
      gain: null,
      offset: null,
      date_obs: null,
      width: f.width,
      height: f.height,
      bit_depth: null,
      camera: null,
    },
  }));
}

// ─── High-level workflow ────────────────────────────────────────

/**
 * Run the §12 P8 orchestrator against the wizard's
 * current frames + the supplied session + project ids.
 * Updates the wizard state with the analysis result.
 */
export async function runAnalysis(
  projectId: string,
  sessionId: string,
  targetName: string | null,
): Promise<void> {
  const s = get(importWizard);
  const result = await importAnalyseSession(
    projectId,
    sessionId,
    framesToAssetMetadataInput(s.frames),
    targetName,
  );
  importWizard.update((cur) => ({
    ...cur,
    sessionId,
    projectId,
    analysis: result.analysis,
    classification: result.classification,
  }));
}

/**
 * Fetch the persisted understanding for a session. Used
 * by the §13 ambiguity dialog to refresh after an
 * override without re-running the full pipeline.
 */
export async function refreshUnderstanding(
  sessionId: string,
): Promise<void> {
  const classification = await importGetUnderstanding(sessionId);
  importWizard.update((cur) => ({
    ...cur,
    sessionId,
    classification,
  }));
}

/**
 * Confirm the current classification. Transitions the
 * session to `Confirmed`.
 */
export async function confirmUnderstanding(): Promise<void> {
  const s = get(importWizard);
  if (!s.sessionId) {
    throw new Error("no session id; call runAnalysis first");
  }
  const classification = await importConfirm(s.sessionId);
  importWizard.update((cur) => ({ ...cur, classification }));
}

/**
 * Apply user overrides + transition to `Confirmed`.
 * Either or both override fields may be null — null
 * means "keep the orchestrator's suggestion".
 */
export async function applyOverrides(): Promise<void> {
  const s = get(importWizard);
  if (!s.sessionId) {
    throw new Error("no session id; call runAnalysis first");
  }
  const classification = await importOverrideClassification(
    s.sessionId,
    s.overrideCaptureKind,
    s.overrideNarrowbandComposition,
  );
  importWizard.update((cur) => ({ ...cur, classification }));
}

/**
 * Mark the session as materialised (the wizard is
 * complete). The caller (P9 shell) is responsible for
 * routing the user to the imported session.
 */
export async function materialiseSession(): Promise<string | null> {
  const s = get(importWizard);
  if (!s.sessionId) {
    throw new Error("no session id; call runAnalysis first");
  }
  const classification = await importSetMaterialised(s.sessionId);
  importWizard.update((cur) => ({ ...cur, classification }));
  return s.sessionId;
}

export function setOverrideCaptureKind(kind: CaptureKind | null): void {
  importWizard.update((s) => ({ ...s, overrideCaptureKind: kind }));
}

export function setOverrideNarrowbandComposition(
  composition: NarrowbandComposition | null,
): void {
  importWizard.update((s) => ({
    ...s,
    overrideNarrowbandComposition: composition,
  }));
}

// ─── Tests ─────────────────────────────────────────────────────

/**
 * Pure helper tests, used by the Svelte components to
 * derive display strings.
 */
export function describeStep(step: WizardStep): string {
  switch (step) {
    case "add-data":
      return "Step 1 of 4 — Add Data";
    case "scanning":
      return "Step 2 of 4 — Scanning";
    case "understanding":
      return "Step 3 of 4 — Understanding";
    case "confirm":
      return "Step 4 of 4 — Confirm";
  }
}

export function captureKindLabel(kind: CaptureKind | null): string {
  switch (kind) {
    case "deep_sky":
      return "Deep-Sky";
    case "planetary_lunar":
      return "Planetary / Lunar";
    case "ambiguous":
      return "Ambiguous";
    case null:
      return "Unknown";
  }
}

export function narrowbandCompositionLabel(
  composition: NarrowbandComposition | null,
): string {
  switch (composition) {
    case "hoo":
      return "HOO (Hubble bi-colour)";
    case "sho":
      return "SHO (Hubble tri-colour)";
    case "hoo_or_sho":
      return "HOO or SHO — both possible";
    case "lrgb":
      return "LRGB";
    case "mono":
      return "Mono";
    case "none":
    case null:
      return "No narrowband structure";
  }
}

export function confidenceLabel(confidence: number | null): string {
  if (confidence === null) return "n/a";
  const pct = (confidence * 100).toFixed(0);
  if (confidence >= 0.8) return `High (${pct}%)`;
  if (confidence >= 0.6) return `Medium (${pct}%)`;
  return `Low (${pct}%)`;
}
