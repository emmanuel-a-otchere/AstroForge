/**
 * Preview store — holds the most recent PreviewImage so the preview
 * canvas (in ModeD) can pick it up after the MVP pipeline runs.
 *
 * Lives at module scope (Svelte 5 `writable`) so any component can
 * push and any other component can subscribe via `$preview`.
 */
import { writable } from "svelte/store";
import type { PreviewImage } from "./pipeline";

export interface PreviewRecord {
  /** Session ID this preview belongs to. */
  sessionId: string;
  /** Source folder the user picked. */
  folder: string;
  /** The 8-bit RGBA preview bitmap. */
  preview: PreviewImage;
  /** Wall-clock timestamp (ms since epoch) when the run finished. */
  finishedAt: number;
}

export const previewStore = writable<PreviewRecord | null>(null);

export function publishPreview(rec: PreviewRecord): void {
  previewStore.set(rec);
}

export function clearPreview(): void {
  previewStore.set(null);
}