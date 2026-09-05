import { writable, derived, get } from "svelte/store";

// ─── Types ──────────────────────────────────────────────────────────────────

export type ProcessingMode = "automagic" | "automagic_expert" | "pure_expert";

export type PipelineStageType =
  | "ingest"
  | "crop_rotate"
  | "background_extraction"
  | "color_calibration"
  | "color_wb"
  | "color_scnr"
  | "sharpen_deconvolution"
  | "denoise"
  | "stretch"
  | "star_handling"
  | "creative_polish"
  | "export";

export type NodeStatus = "pending" | "running" | "completed" | "failed" | "skipped" | "active";

export interface PipelineNode {
  id: string;
  type: PipelineStageType;
  label: string;
  params: Record<string, unknown>;
  status: NodeStatus;
  version: number;
  receipt?: StageReceipt;
}

export interface PipelineEdge {
  from: string;
  to: string;
}

export interface PipelineGraph {
  nodes: PipelineNode[];
  edges: PipelineEdge[];
}

export interface StageReceipt {
  stageId: string;
  timestamp: string;
  durationMs: number;
  parameters: Record<string, unknown>;
  warnings: string[];
  metrics: Record<string, number>;
  engine?: string;
  success: boolean;
}

export interface HistoryEntry {
  nodeId: string;
  version: number;
  params: Record<string, unknown>;
  status: NodeStatus;
  receipt?: StageReceipt;
  timestamp: string;
  action: "commit" | "undo" | "redo" | "mode_switch" | "reapply";
}

export interface ImageStats {
  mean: number;
  median: number;
  stdDev: number;
  min: number;
  max: number;
  bitDepth: number;
  channels: number;
  width: number;
  height: number;
  isLinear: boolean;
}

export interface DataTypeDeclaration {
  cameraType: "osc" | "dual_band" | "mono" | "smart_telescope" | "unknown";
  filterSet: string[];
  bitDepth: number;
  isLinear: boolean;
  deviceModel?: string;
}

export interface SessionState {
  sessionId: string;
  currentMode: ProcessingMode;
  activeStepIndex: number;
  pipelineGraph: PipelineGraph;
  history: HistoryEntry[];
  historyPointer: number;
  imageStats: ImageStats | null;
  dataType: DataTypeDeclaration | null;
  canUndo: boolean;
  canRedo: boolean;
  /// Session-level feature toggles (per D-6). Used to gate profile
  /// params that depend on runtime context rather than the profile itself.
  /// Example: `coreProtectMask: true` activates core-protection params
  /// in `sharpen_deconvolution.coreProtectRequired` stages.
  sessionFlags: Record<string, boolean>;
}

// ─── Stage Definitions ─────────────────────────────────────────────────────

export interface StageDefinition {
  type: PipelineStageType;
  label: string;
  description: string;
  defaultParams: Record<string, unknown>;
}

export const PIPELINE_STAGES: StageDefinition[] = [
  {
    type: "ingest",
    label: "Ingest & Analyse",
    description: "Load image files, detect camera type, filter set, and basic statistics",
    defaultParams: {},
  },
  {
    type: "crop_rotate",
    label: "Framing / Crop / Rotate",
    description: "Interactive crop with live rotation and aspect-ratio presets",
    defaultParams: { rotation: 0, aspectRatio: "free" },
  },
  {
    type: "background_extraction",
    label: "Gradient / Background Extraction",
    description: "Remove sky gradients while preserving nebulosity",
    defaultParams: { strength: 0.8, model: "polynomial" },
  },
  {
    type: "color_calibration",
    label: "Colour Calibration / Balance",
    description: "Bounded white-balance corrections for dual-band and mono data",
    defaultParams: { method: "auto", strength: 0.7 },
  },
  {
    type: "sharpen_deconvolution",
    label: "Sharpen / Deconvolution",
    description: "Richardson-Lucy or van Cittert sharpening with PSF from stars",
    defaultParams: { algorithm: "richardson_lucy", iterations: 10 },
  },
  {
    type: "denoise",
    label: "Denoise",
    description: "Noise reduction with preview-stable statistics",
    defaultParams: { strength: 0.5, method: "swinir" },
  },
  {
    type: "stretch",
    label: "Stretch",
    description: "Data-anchored Deep stretch with multi-preview grid",
    defaultParams: { blackPoint: 0, midtones: 0.25, highlights: 1, mode: "deep" },
  },
  {
    type: "star_handling",
    label: "Star Handling",
    description: "Separate stars, edit layers independently, exact or soft replace",
    defaultParams: { separationMethod: "exact", replaceStrength: 1.0, colorBoost: 0 },
  },
  {
    type: "creative_polish",
    label: "Creative / Final Polish",
    description: "Curves, colour transmutation spells, narrowband palette mixes",
    defaultParams: { saturation: 0, curves: [] },
  },
  {
    type: "export",
    label: "Export",
    description: "Multi-format export: FITS master, TIFF, JPEG, starless, stars-only",
    defaultParams: { format: "tiff16", includeStarless: false, includeStarsOnly: false },
  },
];

export const STAGE_COUNT = PIPELINE_STAGES.length;

// ─── Helpers ────────────────────────────────────────────────────────────────

function createInitialGraph(): PipelineGraph {
  const nodes: PipelineNode[] = PIPELINE_STAGES.map((stage, i) => ({
    id: `node_${i + 1}`,
    type: stage.type,
    label: stage.label,
    params: { ...stage.defaultParams },
    status: i === 0 ? "active" : "pending",
    version: 0,
  }));

  const edges: PipelineEdge[] = [];
  for (let i = 0; i < nodes.length - 1; i++) {
    edges.push({ from: nodes[i].id, to: nodes[i + 1].id });
  }

  return { nodes, edges };
}

function createInitialSession(sessionId?: string): SessionState {
  return {
    sessionId: sessionId ?? `session_${Date.now()}`,
    currentMode: "automagic",
    activeStepIndex: 0,
    pipelineGraph: createInitialGraph(),
    history: [],
    historyPointer: -1,
    imageStats: null,
    dataType: null,
    canUndo: false,
    canRedo: false,
    sessionFlags: {},
  };
}

// ─── Store ──────────────────────────────────────────────────────────────────

export const sessionStore = writable<SessionState>(createInitialSession());

export const currentMode = derived(sessionStore, ($s) => $s.currentMode);
export const activeStepIndex = derived(sessionStore, ($s) => $s.activeStepIndex);
export const pipelineGraph = derived(sessionStore, ($s) => $s.pipelineGraph);
export const history = derived(sessionStore, ($s) => $s.history);
export const canUndo = derived(sessionStore, ($s) => $s.canUndo);
export const canRedo = derived(sessionStore, ($s) => $s.canRedo);
export const activeNode = derived(sessionStore, ($s) => {
  return $s.pipelineGraph.nodes[$s.activeStepIndex] ?? null;
});

export const stageDefinitions = PIPELINE_STAGES;

// ─── Actions ────────────────────────────────────────────────────────────────

export function initSession(sessionId?: string, mode?: ProcessingMode): void {
  const state = createInitialSession(sessionId);
  if (mode) state.currentMode = mode;
  sessionStore.set(state);
}

export function setMode(mode: ProcessingMode, keepPixelState: boolean = true): void {
  sessionStore.update((state) => {
    const entry: HistoryEntry = {
      nodeId: state.pipelineGraph.nodes[state.activeStepIndex]?.id ?? "",
      version: state.pipelineGraph.nodes[state.activeStepIndex]?.version ?? 0,
      params: state.pipelineGraph.nodes[state.activeStepIndex]?.params ?? {},
      status: state.pipelineGraph.nodes[state.activeStepIndex]?.status ?? "pending",
      timestamp: new Date().toISOString(),
      action: "mode_switch",
    };

    const newState: SessionState = {
      ...state,
      currentMode: mode,
      history: [...state.history.slice(0, state.historyPointer + 1), entry],
      historyPointer: state.historyPointer + 1,
    };

    if (!keepPixelState) {
      newState.pipelineGraph = createInitialGraph();
      newState.activeStepIndex = 0;
    }

    newState.canUndo = newState.historyPointer >= 0;
    newState.canRedo = false;
    return newState;
  });
}

export function commitStage(
  params: Record<string, unknown>,
  receipt?: Omit<StageReceipt, "timestamp">
): void {
  sessionStore.update((state) => {
    const node = state.pipelineGraph.nodes[state.activeStepIndex];
    if (!node) return state;

    const newVersion = node.version + 1;
    const fullReceipt: StageReceipt | undefined = receipt
      ? { ...receipt, timestamp: new Date().toISOString() }
      : undefined;

    const updatedNodes = state.pipelineGraph.nodes.map((n, i) => {
      if (i === state.activeStepIndex) {
        return {
          ...n,
          params: { ...params },
          status: "completed" as NodeStatus,
          version: newVersion,
          receipt: fullReceipt,
        };
      }
      return n;
    });

    const nextIndex = Math.min(state.activeStepIndex + 1, updatedNodes.length - 1);
    if (nextIndex !== state.activeStepIndex) {
      updatedNodes[nextIndex] = {
        ...updatedNodes[nextIndex],
        status: "active",
      };
    }

    const entry: HistoryEntry = {
      nodeId: node.id,
      version: newVersion,
      params: { ...params },
      status: "completed",
      receipt: fullReceipt,
      timestamp: new Date().toISOString(),
      action: "commit",
    };

    const newHistory = [...state.history.slice(0, state.historyPointer + 1), entry];

    return {
      ...state,
      pipelineGraph: { ...state.pipelineGraph, nodes: updatedNodes },
      activeStepIndex: nextIndex,
      history: newHistory,
      historyPointer: newHistory.length - 1,
      canUndo: true,
      canRedo: false,
    };
  });
}

export function goToStep(index: number): void {
  sessionStore.update((state) => {
    if (index < 0 || index >= state.pipelineGraph.nodes.length) return state;

    const updatedNodes = state.pipelineGraph.nodes.map((n, i) => {
      if (i === index) return { ...n, status: "active" as NodeStatus };
      if (n.status === "active") return { ...n, status: "completed" as NodeStatus };
      return n;
    });

    return {
      ...state,
      activeStepIndex: index,
      pipelineGraph: { ...state.pipelineGraph, nodes: updatedNodes },
    };
  });
}

export function nextStep(): void {
  const state = get(sessionStore);
  const next = Math.min(state.activeStepIndex + 1, state.pipelineGraph.nodes.length - 1);
  goToStep(next);
}

export function prevStep(): void {
  const state = get(sessionStore);
  const prev = Math.max(state.activeStepIndex - 1, 0);
  goToStep(prev);
}

export function undo(): void {
  sessionStore.update((state) => {
    if (state.historyPointer < 0) return state;

    const entry = state.history[state.historyPointer];
    const newPointer = state.historyPointer - 1;

    const updatedNodes = state.pipelineGraph.nodes.map((n) => {
      if (n.id === entry.nodeId) {
        if (entry.action === "commit") {
          // M7 T5 fix: restore the prior history entry's params instead
          // of resetting to defaults. Falls back to defaults only when
          // there is no prior entry (i.e. we're undoing the very first
          // commit and there's nothing to restore from).
          const prior = newPointer >= 0 ? state.history[newPointer] : null;
          const restoredParams =
            prior?.params ?? PIPELINE_STAGES.find((s) => s.type === n.type)?.defaultParams ?? {};
          return {
            ...n,
            version: Math.max(entry.version - 1, 0),
            status: "active" as NodeStatus,
            params: restoredParams,
            receipt: undefined,
          };
        }
      }
      return n;
    });

    const stepIndex = updatedNodes.findIndex((n) => n.id === entry.nodeId);

    return {
      ...state,
      pipelineGraph: { ...state.pipelineGraph, nodes: updatedNodes },
      activeStepIndex: stepIndex >= 0 ? stepIndex : state.activeStepIndex,
      historyPointer: newPointer,
      canUndo: newPointer >= 0,
      canRedo: true,
    };
  });
}

export function redo(): void {
  sessionStore.update((state) => {
    if (state.historyPointer >= state.history.length - 1) return state;

    const newPointer = state.historyPointer + 1;
    const entry = state.history[newPointer];

    const updatedNodes = state.pipelineGraph.nodes.map((n) => {
      if (n.id === entry.nodeId) {
        return {
          ...n,
          version: entry.version,
          params: { ...entry.params },
          status: entry.status,
          receipt: entry.receipt,
        };
      }
      return n;
    });

    const stepIndex = updatedNodes.findIndex((n) => n.id === entry.nodeId);
    const nextIndex = stepIndex >= 0 && entry.action === "commit"
      ? Math.min(stepIndex + 1, updatedNodes.length - 1)
      : state.activeStepIndex;

    return {
      ...state,
      pipelineGraph: { ...state.pipelineGraph, nodes: updatedNodes },
      activeStepIndex: nextIndex,
      historyPointer: newPointer,
      canUndo: true,
      canRedo: newPointer < state.history.length - 1,
    };
  });
}

export function updateNodeParams(nodeId: string, params: Record<string, unknown>): void {
  sessionStore.update((state) => {
    const updatedNodes = state.pipelineGraph.nodes.map((n) => {
      if (n.id === nodeId) {
        return { ...n, params: { ...n.params, ...params } };
      }
      return n;
    });
    return {
      ...state,
      pipelineGraph: { ...state.pipelineGraph, nodes: updatedNodes },
    };
  });
}

export function setImageStats(stats: ImageStats): void {
  sessionStore.update((state) => ({ ...state, imageStats: stats }));
}

export function setDataType(dataType: DataTypeDeclaration): void {
  sessionStore.update((state) => ({ ...state, dataType: dataType }));
}

export function getSessionState(): SessionState {
  return get(sessionStore);
}

export function serializeSession(): string {
  return JSON.stringify(get(sessionStore), null, 2);
}

export function deserializeSession(json: string): void {
  try {
    const state = JSON.parse(json) as SessionState;
    sessionStore.set(state);
  } catch {
    // invalid JSON, keep current state
  }
}

export function setSessionFlag(key: string, value: boolean): void {
  sessionStore.update((state) => ({
    ...state,
    sessionFlags: { ...state.sessionFlags, [key]: value },
  }));
}

// ─── Profile application (Phase 1.5 PR-B) ────────────────────────────────────
//
// `applyProfileToPipeline` is a pure function: it takes a Recipe and a
// PipelineGraph, and returns a new PipelineGraph with each node's
// `params` overwritten by the recipe's stage params (when present) and
// `status` set to `active` or `skipped` based on `RecipeStage.enabled`.
//
// Session-flag gating (D-6): if a stage has a `coreProtectRequired: true`
// param, that param is only applied when `sessionFlags.coreProtectMask`
// is true. Without the flag, the param is omitted and a warning is
// returned via the second tuple element.
//
// Unmatched stages (in graph but not in profile) keep their defaults.

import type { Recipe } from "./profile-store";

export interface ProfileApplyWarning {
  /// Stage id where the warning applies.
  stageId: string;
  /// Param key that was gated off (e.g. "coreProtectRequired").
  paramKey: string;
  /// Human-readable explanation.
  message: string;
}

export interface ProfileApplyResult {
  graph: PipelineGraph;
  warnings: ProfileApplyWarning[];
}

export function applyProfileToPipeline(
  profile: Recipe,
  graph: PipelineGraph,
  sessionFlags: Record<string, boolean> = {},
): ProfileApplyResult {
  const warnings: ProfileApplyWarning[] = [];
  const coreProtectOn = sessionFlags["coreProtectMask"] === true;

  // Build a lookup from stage_id → RecipeStage for O(1) application.
  const byStageId = new Map<string, (typeof profile.stages)[number]>();
  for (const stage of profile.stages) {
    byStageId.set(stage.stageId, stage);
  }

  const nodes = graph.nodes.map((node) => {
    const recipeStage = byStageId.get(node.type);
    if (!recipeStage) {
      // No mapping for this node — keep defaults, but mark skipped if
      // the recipe explicitly excluded every stage of this type. (We
      // can't know that here; skip only when profile.enabled = false.)
      return node;
    }
    if (!recipeStage.enabled) {
      return { ...node, status: "skipped" as NodeStatus, version: 0, receipt: undefined };
    }

    // Apply params with session-flag gating.
    const filtered: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(recipeStage.params)) {
      if (k === "coreProtectRequired" && !coreProtectOn) {
        warnings.push({
          stageId: node.type,
          paramKey: k,
          message:
            "coreProtectRequired param omitted — enable sessionFlags.coreProtectMask to activate core-protection params.",
        });
        continue;
      }
      filtered[k] = v;
    }

    return {
      ...node,
      params: { ...node.params, ...filtered },
      status: "active" as NodeStatus,
      version: 0,
      receipt: undefined,
    };
  });

  return { graph: { ...graph, nodes }, warnings };
}
