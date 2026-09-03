/**
 * Pipeline bridge — Svelte side of the Phase 9 vertical slice.
 *
 * Three layers:
 *   1. Folder picker (Tauri dialog plugin) — user picks a directory of
 *      FITS files. In browser dev, falls back to a manual prompt.
 *   2. ingest_scan_directory IPC — walks the directory and classifies
 *      each frame (Light / Dark / Flat / Bias) with header info.
 *   3. pipeline_run_session IPC — runs the MVP pipeline end-to-end
 *      against the directory's light frames.
 *
 * Returns DTOs (snake_case from Rust) with camelCase mapped fields
 * for the UI. Mirrors the gallery / session bridge conventions.
 */

import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

export type FrameType = "LIGHT" | "DARK" | "FLAT" | "BIAS";

export interface IngestFrame {
  path: string;
  frameType: FrameType;
  exptime: number | null;
  filter: string | null;
  width: number | null;
  height: number | null;
  binning: number | null;
  anomalies: string[];
}

interface RustIngestFrame {
  path: string;
  frame_type: string;
  exptime: number | null;
  filter: string | null;
  width: number | null;
  height: number | null;
  binning: number | null;
  anomalies: string[];
}

function fromRust(f: RustIngestFrame): IngestFrame {
  return {
    path: f.path,
    frameType: f.frame_type as FrameType,
    exptime: f.exptime,
    filter: f.filter,
    width: f.width,
    height: f.height,
    binning: f.binning,
    anomalies: f.anomalies ?? [],
  };
}

export interface PreviewImage {
  width: number;
  height: number;
  /** Raw RGBA bytes (length = width × height × 4). */
  rgba: number[];
}

export interface PipelineReport {
  sessionId: string;
  frameStats: {
    totalFrames: number;
    lights: number;
    darks: number;
    flats: number;
    biases: number;
    totalExposure: number;
  };
  rejectedFrames: unknown[];
  stageParameters: Array<{
    stageId: string;
    params: Record<string, string>;
  }>;
  exportPath: string | null;
}

export interface PipelineResult {
  success: boolean;
  report: PipelineReport;
  preview: PreviewImage | null;
  error: string | null;
}

interface RustPipelineResult {
  success: boolean;
  report: RustPipelineReport;
  preview: RustPreviewImage | null;
  error: string | null;
}

interface RustPipelineReport {
  session_id: string;
  frame_stats: RustFrameStats;
  rejected_frames: unknown[];
  stage_parameters: RustStageParams[];
  export_path: string | null;
}

interface RustPreviewImage {
  width: number;
  height: number;
  rgba: number[];
}

interface RustFrameStats {
  total_frames: number;
  lights: number;
  darks: number;
  flats: number;
  biases: number;
  total_exposure: number;
}

interface RustStageParams {
  stage_id: string;
  params: Record<string, string>;
}

function reportFromRust(r: RustPipelineReport): PipelineReport {
  return {
    sessionId: r.session_id,
    frameStats: {
      totalFrames: r.frame_stats.total_frames,
      lights: r.frame_stats.lights,
      darks: r.frame_stats.darks,
      flats: r.frame_stats.flats,
      biases: r.frame_stats.biases,
      totalExposure: r.frame_stats.total_exposure,
    },
    rejectedFrames: r.rejected_frames ?? [],
    stageParameters: (r.stage_parameters ?? []).map((s) => ({
      stageId: s.stage_id,
      params: s.params,
    })),
    exportPath: r.export_path,
  };
}

function previewFromRust(p: RustPreviewImage): PreviewImage {
  return {
    width: p.width,
    height: p.height,
    // Some Tauri serialisers wrap byte arrays in plain JS arrays;
    // ensure we always have a fresh array the WebGL renderer can consume.
    rgba: Array.from(p.rgba ?? []),
  };
}

function resultFromRust(r: RustPipelineResult): PipelineResult {
  return {
    success: r.success,
    report: reportFromRust(r.report),
    preview: r.preview ? previewFromRust(r.preview) : null,
    error: r.error,
  };
}

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// ─── Browser fallback ──────────────────────────────────────────────────
//
// In dev mode (Vite, no Tauri shell) the dialog plugin isn't available
// and the IPC commands don't exist. Surface a friendly error so the UI
// doesn't crash; tests can use the in-memory pipeline-store mocks.

async function pickFolderBrowser(): Promise<string | null> {
  // Native browsers can't return a folder path through window.prompt,
  // so we degrade to "type the path" — useful for local development
  // when the user wants to bypass the dialog.
  if (typeof window === "undefined") return null;
  // eslint-disable-next-line no-alert
  const entered = window.prompt(
    "Pick a FITS folder (Tauri dialog unavailable in browser). Enter absolute path:",
  );
  return entered && entered.length > 0 ? entered : null;
}

/**
 * Open the system folder picker. Returns the selected absolute path,
 * or null if the user cancelled.
 *
 * Browser fallback: prompt() that returns the entered path.
 */
export async function pickFitsFolder(): Promise<string | null> {
  if (!isTauri()) {
    return pickFolderBrowser();
  }
  const result = await openDialog({
    directory: true,
    multiple: false,
    title: "Pick a folder of FITS frames",
  });
  if (typeof result === "string") return result;
  return null;
}

/**
 * Walk a directory and classify every FITS file. Returns one DTO per
 * frame with header info the manifest UI can render.
 */
export async function scanDirectory(dirPath: string): Promise<IngestFrame[]> {
  if (!isTauri()) {
    throw new Error("scanDirectory is only available inside the Tauri runtime");
  }
  const rows = await invoke<RustIngestFrame[]>("ingest_scan_directory", {
    dirPath,
  });
  return (rows ?? []).map(fromRust);
}

/**
 * Run the MVP pipeline end-to-end against a directory of light frames.
 * Returns the full PipelineResult including the ProcessingReport.
 */
export async function runPipeline(
  sessionId: string,
  dirPath: string,
  verbosity: "beginner" | "intermediate" | "expert" = "beginner",
): Promise<PipelineResult> {
  if (!isTauri()) {
    throw new Error("runPipeline is only available inside the Tauri runtime");
  }
  const result = await invoke<RustPipelineResult>("pipeline_run_session", {
    sessionId,
    dirPath,
    verbosity,
  });
  return resultFromRust(result);
}

// ─── Helpers for the manifest UI ──────────────────────────────────────

export function countByType(frames: IngestFrame[]): Record<FrameType, number> {
  const counts: Record<FrameType, number> = {
    LIGHT: 0,
    DARK: 0,
    FLAT: 0,
    BIAS: 0,
  };
  for (const f of frames) {
    counts[f.frameType] = (counts[f.frameType] ?? 0) + 1;
  }
  return counts;
}

export function totalExposure(frames: IngestFrame[]): number {
  return frames
    .filter((f) => f.frameType === "LIGHT")
    .reduce((sum, f) => sum + (f.exptime ?? 0), 0);
}

export function formatExposure(seconds: number): string {
  if (seconds <= 0) return "—";
  if (seconds < 60) return `${seconds.toFixed(0)} s`;
  const m = Math.floor(seconds / 60);
  const s = Math.round(seconds % 60);
  return `${m}m ${s}s`;
}

/**
 * Convert a PreviewImage's raw RGBA bytes into an ImageData that the
 * WebGL renderer / a 2D canvas can consume directly. The returned
 * ImageData owns its `data` buffer (Uint8ClampedArray view of a copy).
 */
export function previewToImageData(p: PreviewImage): ImageData {
  const clamped = Uint8ClampedArray.from(p.rgba);
  return new ImageData(clamped, p.width, p.height);
}

/**
 * Render a PreviewImage to a temporary 2D canvas and return the canvas.
 * Useful for `WebGLRenderer.setImageFromCanvas`, which sidesteps the
 * `texImage2D` Float/texture-format compatibility surface.
 */
export function previewToCanvas(p: PreviewImage): HTMLCanvasElement {
  const canvas = document.createElement("canvas");
  canvas.width = p.width;
  canvas.height = p.height;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("2D canvas unavailable");
  ctx.putImageData(previewToImageData(p), 0, 0);
  return canvas;
}