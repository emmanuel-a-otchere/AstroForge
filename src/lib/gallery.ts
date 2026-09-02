/**
 * GalleryStore — local-only gallery state for "Your Work".
 *
 * Phase 4: backed by rusqlite via Tauri IPC (`gallery_list`, `gallery_upsert`,
 * `gallery_delete` commands in `src-tauri/src/main.rs`).
 *
 * The Tauri runtime exposes IPC via `window.__TAURI_INTERNALS__`. When the
 * app is running in a plain browser (Vite dev without the Tauri shell, or
 * unit-test harness), we fall back to an in-memory placeholder so the
 * UI still renders.
 *
 * Field naming: Rust serialises with snake_case (e.g. `integration_hours`),
 * the Svelte side uses camelCase for consistency with the rest of the
 * frontend (`integrationHours`). The mapping happens at the IPC boundary.
 */

import { writable, type Writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

export type GalleryStatus = "pending" | "processing" | "completed";

export interface GalleryItem {
  id: string;
  name: string;
  target: string;
  integrationHours: number;
  palette: string;
  status: GalleryStatus;
  updatedAt: string; // ISO
  thumbnail?: string;
}

interface RustGalleryItem {
  id: string;
  name: string;
  target: string;
  integration_hours: number;
  palette: string;
  status: GalleryStatus;
  updated_at: string;
}

interface RustGalleryItemUpdate {
  id?: string | null;
  name: string;
  target: string;
  integration_hours: number;
  palette: string;
  status: GalleryStatus;
}

function fromRust(item: RustGalleryItem): GalleryItem {
  return {
    id: item.id,
    name: item.name,
    target: item.target,
    integrationHours: item.integration_hours,
    palette: item.palette,
    status: item.status,
    updatedAt: item.updated_at,
  };
}

function toRustUpdate(update: {
  id?: string | null;
  name: string;
  target: string;
  integrationHours: number;
  palette: string;
  status: GalleryStatus;
}): RustGalleryItemUpdate {
  return {
    id: update.id ?? null,
    name: update.name,
    target: update.target,
    integration_hours: update.integrationHours,
    palette: update.palette,
    status: update.status,
  };
}

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// ─── Placeholder fallback (browser dev mode, unit tests) ────────────────

const PLACEHOLDER_ITEMS: GalleryItem[] = [
  {
    id: "placeholder-m42",
    name: "M42_L-PRO_BIN1",
    target: "M42",
    integrationHours: 7.36,
    palette: "LRGB",
    status: "completed",
    updatedAt: "2026-08-30T14:22:11.000Z",
  },
  {
    id: "placeholder-m31",
    name: "M31_SHO_V2_FINAL",
    target: "M31",
    integrationHours: 13.75,
    palette: "SHO",
    status: "completed",
    updatedAt: "2026-08-28T20:11:05.000Z",
  },
  {
    id: "placeholder-ngc7000",
    name: "NGC7000_NARROW",
    target: "NGC 7000",
    integrationHours: 4.2,
    palette: "Hα",
    status: "processing",
    updatedAt: "2026-09-01T09:14:33.000Z",
  },
  {
    id: "placeholder-ic1396",
    name: "IC1396_HA_OIII",
    target: "IC 1396",
    integrationHours: 8.5,
    palette: "HSO",
    status: "pending",
    updatedAt: "2026-09-01T22:48:09.000Z",
  },
  {
    id: "placeholder-horsehead",
    name: "Horsehead_NB",
    target: "IC 434",
    integrationHours: 5.23,
    palette: "Hα + OIII",
    status: "pending",
    updatedAt: "2026-09-02T08:33:51.000Z",
  },
];

/**
 * The single source of truth for the gallery on the Svelte side. All
 * mutations (load, upsert, delete) go through this store so any
 * subscribed component re-renders.
 */
export const galleryStore: Writable<GalleryItem[]> = writable<GalleryItem[]>([]);

// ─── Public API ─────────────────────────────────────────────────────────

/**
 * Sync accessor — for components that need immediate access on first
 * render (ModeC, ModeD). Returns the current store value via `get()`.
 */
import { get } from "svelte/store";
export function getGallery(): GalleryItem[] {
  return get(galleryStore);
}

/**
 * Async load from the local rusqlite store via Tauri IPC. Falls back to
 * placeholder data when running outside Tauri (browser dev / tests).
 */
export async function loadGallery(): Promise<GalleryItem[]> {
  if (!isTauri()) {
    galleryStore.set([...PLACEHOLDER_ITEMS]);
    return get(galleryStore);
  }
  try {
    const items = await invoke<RustGalleryItem[]>("gallery_list");
    const mapped = items.map(fromRust);
    galleryStore.set(mapped);
    return mapped;
  } catch (err) {
    console.error("loadGallery failed; falling back to placeholder", err);
    galleryStore.set([...PLACEHOLDER_ITEMS]);
    return get(galleryStore);
  }
}

/**
 * Add or update a gallery item. Returns the persisted item with
 * server-assigned id/timestamp.
 */
export async function upsertGalleryItem(
  update: Omit<GalleryItem, "updatedAt"> & { id?: string | null }
): Promise<GalleryItem> {
  if (!isTauri()) {
    const next: GalleryItem = {
      ...update,
      id: update.id ?? `placeholder-${Date.now()}`,
      updatedAt: new Date().toISOString(),
    } as GalleryItem;
    galleryStore.update((items) => {
      const idx = items.findIndex((i) => i.id === next.id);
      if (idx >= 0) {
        const copy = items.slice();
        copy[idx] = next;
        return copy;
      }
      return [next, ...items];
    });
    return next;
  }
  const result = await invoke<RustGalleryItem>("gallery_upsert", {
    update: toRustUpdate({
      id: update.id ?? null,
      name: update.name,
      target: update.target,
      integrationHours: update.integrationHours,
      palette: update.palette,
      status: update.status,
    }),
  });
  const item = fromRust(result);
  galleryStore.update((items) => {
    const idx = items.findIndex((i) => i.id === item.id);
    if (idx >= 0) {
      const copy = items.slice();
      copy[idx] = item;
      return copy;
    }
    return [item, ...items];
  });
  return item;
}

/** Delete a gallery item by id. No-op if missing. */
export async function deleteGalleryItem(id: string): Promise<void> {
  if (!isTauri()) {
    galleryStore.update((items) => items.filter((i) => i.id !== id));
    return;
  }
  await invoke("gallery_delete", { id });
  galleryStore.update((items) => items.filter((i) => i.id !== id));
}

// ─── Display helpers ────────────────────────────────────────────────────

export type GalleryTab = "recent" | "processing" | "completed";

export function filterByTab(items: GalleryItem[], tab: GalleryTab): GalleryItem[] {
  switch (tab) {
    case "recent":
      // Most recent 8 items, regardless of status.
      return items.slice(0, 8);
    case "processing":
      return items.filter((i) => i.status === "processing");
    case "completed":
      return items.filter((i) => i.status === "completed");
  }
}

export function statusBadgeClass(status: GalleryStatus): string {
  return `gallery-status gallery-status-${status}`;
}

export function statusBadgeLabel(status: GalleryStatus): string {
  return status.toUpperCase();
}