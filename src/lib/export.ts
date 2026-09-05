/**
 * Multi-format export bridge (M7 T3).
 *
 * Phase 1.5 PR-B3a: lets the UI export the current pipeline result to
 * one or more formats in a single call. The Rust side handles the
 * dispatch; we ship the pixel buffer + a list of `ExportFormat` enum
 * values and get back the list of absolute paths.
 *
 * Browser-mode fallback returns `[]` so the dev server (no Tauri) keeps
 * working — dev tooling can mock the export later if needed.
 */
import { invoke } from "@tauri-apps/api/core";

export type ExportFormat =
  | { format: "tiff16" }
  | { format: "png8" }
  | { format: "jpeg8"; quality: number }
  | { format: "xisf"; history_json: Record<string, unknown> }
  | { format: "fits32" }
  | { format: "sidecar_json"; recipe_json: Record<string, unknown> };

export interface MultiExportArgs {
  /** Flat float32 pixel buffer, channel-major (R, G, B, ...) then
   * row-major within each channel. */
  pixels: Float32Array | number[];
  width: number;
  height: number;
  channels: number;
  /** Base path WITHOUT extension; the Rust dispatcher appends
   * the extension per format. */
  base_path: string;
  /** Formats to emit. */
  formats: ExportFormat[];
  /** Sidecar context: required when SidecarJson is in formats.
   * Ignored otherwise. */
  report?: ProcessingReport;
}

export interface ProcessingReport {
  session_id: string;
  frame_stats: {
    total_frames: number;
    lights: number;
    darks: number;
    flats: number;
    biases: number;
    total_exposure: number;
  };
  rejected_frames: { path: string; reason: string }[];
  stage_parameters: { stage_id: string; params: Record<string, string> }[];
  export_path?: string | null;
}

/**
 * Dispatch a multi-format export. Returns the list of written file paths.
 */
export async function multiExport(args: MultiExportArgs): Promise<string[]> {
  if (!isTauri()) return [];
  // Float32Array is not JSON-serializable through the invoke bridge in
  // every Tauri version; convert to a plain array.
  const pixels = Array.from(args.pixels);
  return invoke<string[]>("export_multi_format", {
    args: {
      pixels,
      width: args.width,
      height: args.height,
      channels: args.channels,
      base_path: args.base_path,
      formats: args.formats,
      report: args.report,
    },
  });
}

function isTauri(): boolean {
  return typeof (globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ !== "undefined";
}