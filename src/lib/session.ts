/**
 * Session autosave — local-only persistence for the processing train.
 *
 * Phase 8 (M2 tranche): mirrors the GalleryStore pattern. The Rust
 * `SessionStore` lives in `crates/astroforge-core/src/session.rs`; this
 * module exposes a thin Tauri-IPC bridge that falls back to an in-memory
 * placeholder when running in the browser (Vite dev / tests).
 *
 * Wiring model: the WizardBottomSheet calls `recordStage` on every
 * `commitStage` so the session survives crashes (see issue #144 + spec §5 NFR).
 *
 * Field naming: Rust serialises with snake_case (e.g. `source_dir`,
 * `params_json`); the Svelte side uses camelCase. Mapping happens at the
 * IPC boundary.
 */

import { invoke } from "@tauri-apps/api/core";

export type SessionStatus = "created" | "running" | "completed" | "failed" | "interrupted";

export interface SessionRecord {
  id: string;
  projectId: string;
  sourceDir?: string | null;
  verbosity: string;
  status: SessionStatus;
  createdAt: string;
  updatedAt: string;
}

export interface StageRunRecord {
  id: number;
  sessionId: string;
  stageId: string;
  status: string;
  paramsJson?: string | null;
  metricsJson?: string | null;
  error?: string | null;
  startedAt?: string | null;
  completedAt?: string | null;
}

// Shape as it comes back from serde over the IPC bridge.
interface RustSession {
  id: string;
  project_id: string;
  source_dir: string | null;
  verbosity: string;
  status: string;
  created_at: string;
  updated_at: string;
}

interface RustStageRun {
  id: number;
  session_id: string;
  stage_id: string;
  status: string;
  params_json: string | null;
  metrics_json: string | null;
  error: string | null;
  started_at: string | null;
  completed_at: string | null;
}

function fromRust(s: RustSession): SessionRecord {
  return {
    id: s.id,
    projectId: s.project_id,
    sourceDir: s.source_dir,
    verbosity: s.verbosity,
    status: s.status as SessionStatus,
    createdAt: s.created_at,
    updatedAt: s.updated_at,
  };
}

function fromRustRun(r: RustStageRun): StageRunRecord {
  return {
    id: r.id,
    sessionId: r.session_id,
    stageId: r.stage_id,
    status: r.status,
    paramsJson: r.params_json,
    metricsJson: r.metrics_json,
    error: r.error,
    startedAt: r.started_at,
    completedAt: r.completed_at,
  };
}

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// ─── Browser fallback ───────────────────────────────────────────────────
//
// When running outside Tauri (Vite dev / tests), we keep a private
// in-memory map so the UI still functions. Writes succeed silently;
// reads return only what was written in this session.

interface InMemorySession extends RustSession {}

const inMemoryProjects = new Map<string, { name: string; targetType: string | null }>();
const inMemorySessions = new Map<string, InMemorySession>();
const inMemoryRuns: RustStageRun[] = [];

function mint(prefix: string): string {
  return `${prefix}_${Date.now()}_${Math.floor(Math.random() * 1e6)}`;
}

// ─── Public API ─────────────────────────────────────────────────────────

/**
 * Create a project. Returns the assigned project id.
 */
export async function createProject(
  name: string,
  targetType?: string | null
): Promise<string> {
  if (!isTauri()) {
    const id = mint("proj");
    inMemoryProjects.set(id, { name, targetType: targetType ?? null });
    return id;
  }
  return invoke<string>("session_create_project", {
    name,
    targetType: targetType ?? null,
  });
}

/**
 * Create a session under a project. Returns the session id.
 */
export async function createSession(
  projectId: string,
  sourceDir?: string | null,
  verbosity: string = "beginner"
): Promise<string> {
  if (!isTauri()) {
    const id = mint("sess");
    const now = new Date().toISOString();
    inMemorySessions.set(id, {
      id,
      project_id: projectId,
      source_dir: sourceDir ?? null,
      verbosity,
      status: "created",
      created_at: now,
      updated_at: now,
    });
    return id;
  }
  return invoke<string>("session_create", {
    projectId,
    sourceDir: sourceDir ?? null,
    verbosity,
  });
}

/**
 * Record a stage run. Auto-completes if `status` is `completed` / `failed` /
 * `skipped`, matching the Rust command's behaviour.
 *
 * Returns the assigned run id (useful for downstream UI that wants to show
 * "stage #123 committed").
 */
export async function recordStage(input: {
  sessionId: string;
  stageId: string;
  status: "running" | "completed" | "failed" | "skipped";
  params?: Record<string, unknown> | null;
  metrics?: Record<string, unknown> | null;
  warnings?: string[] | null;
  error?: string | null;
}): Promise<number> {
  // Receipt shape that survives the IPC boundary unchanged: the Rust
  // side stores metrics/warnings/error as separate columns, but we
  // also fold them into a single `metrics_json` blob so a future
  // receipt-query command can reconstruct the full StageReceipt.
  const paramsJson = input.params ? JSON.stringify(input.params) : null;
  const metricsBlob: Record<string, unknown> = {
    ...(input.metrics ?? {}),
  };
  if (input.warnings && input.warnings.length > 0) {
    metricsBlob.warnings = input.warnings;
  }
  const metricsJson = Object.keys(metricsBlob).length > 0 ? JSON.stringify(metricsBlob) : null;

  if (!isTauri()) {
    const id = inMemoryRuns.length + 1;
    const now = new Date().toISOString();
    const completedAt =
      input.status === "running" ? null : now;
    inMemoryRuns.push({
      id,
      session_id: input.sessionId,
      stage_id: input.stageId,
      status: input.status,
      params_json: paramsJson,
      metrics_json: metricsJson,
      error: input.error ?? null,
      started_at: now,
      completed_at: completedAt,
    });
    return id;
  }

  return invoke<number>("session_record_stage", {
    sessionId: input.sessionId,
    stageId: input.stageId,
    status: input.status,
    paramsJson,
    metricsJson,
    error: input.error ?? null,
  });
}

/**
 * Find sessions that were left in the `running` state — i.e. the app
 * crashed or was force-killed before they could be marked `completed`.
 * Returns the session ids; caller decides whether to resume or discard.
 */
export async function findInterruptedSessions(): Promise<string[]> {
  if (!isTauri()) {
    return [];
  }
  return invoke<string[]>("session_find_interrupted");
}

// ─── Display helpers (re-export the Rust types for convenience) ─────────

export { fromRustRun as stageRunFromRust };
export { fromRust as sessionFromRust };

// ─── Receipt history ────────────────────────────────────────────────────

/**
 * Fetch the persisted receipt log for a session (one row per stage
 * run, oldest first). Each entry carries params_json / metrics_json /
 * error so the UI can rebuild a StageReceipt shape for display.
 *
 * Browser fallback returns [] — in dev mode there's no persistent
 * store, so the in-memory pipeline-store history is the only source.
 */
export async function fetchReceipts(sessionId: string): Promise<StageRunRecord[]> {
  if (!isTauri()) {
    return [];
  }
  const rows = await invoke<RustStageRun[]>("session_get_receipts", {
    sessionId,
  });
  return rows.map(fromRustRun);
}

// ─── Stage checkpoints (M7 T1) ─────────────────────────────────────────────
//
// Snapshot of per-stage pixel data. The Tauri side persists the path
// string; the actual PNG bytes live on disk under <app_data_dir>/artifacts/<sessionId>/.
// Frontend just tracks which stage ran at which path and on which date.

export interface CheckpointRecord {
  id: number;
  sessionId: string;
  stageId: string;
  artifactPath: string;
  createdAt: string;
}

/**
 * Snapshot the current pixel output of a stage. Returns the checkpoint
 * row id. Browser fallback returns 0 — no-op in dev mode.
 */
export async function saveCheckpoint(
  sessionId: string,
  stageId: string,
  artifactPath: string,
): Promise<number> {
  if (!isTauri()) return 0;
  return invoke<number>("session_save_checkpoint", {
    sessionId,
    stageId,
    artifactPath,
  });
}

/**
 * Latest checkpoint for a session (one row, or null). Browser fallback
 * returns null.
 */
export async function getLatestCheckpoint(
  sessionId: string,
): Promise<CheckpointRecord | null> {
  if (!isTauri()) return null;
  const info = await invoke<RustCheckpointInfo | null>(
    "session_get_latest_checkpoint",
    { sessionId },
  );
  return info ? fromRustCheckpoint(info) : null;
}

/**
 * All checkpoints for a session, oldest first. Used by the version
 * timeline in the checkpoint panel.
 */
export async function listCheckpoints(
  sessionId: string,
): Promise<CheckpointRecord[]> {
  if (!isTauri()) return [];
  const rows = await invoke<RustCheckpointInfo[]>("session_get_checkpoints", {
    sessionId,
  });
  return rows.map(fromRustCheckpoint);
}

interface RustCheckpointInfo {
  id: number;
  session_id: string;
  stage_id: string;
  artifact_path: string;
  created_at: string;
}

function fromRustCheckpoint(r: RustCheckpointInfo): CheckpointRecord {
  return {
    id: r.id,
    sessionId: r.session_id,
    stageId: r.stage_id,
    artifactPath: r.artifact_path,
    createdAt: r.created_at,
  };
}