/**
 * GalleryStore — local-only gallery state for "Your Work".
 *
 * Phase 4 will back this with rusqlite via Tauri IPC. For now we ship
 * placeholder data so the visual surface renders correctly without
 * touching the existing Supabase autosave flow.
 *
 * The interface is intentionally async-ready: load() returns a Promise so
 * the call sites don't change when the backing store moves to SQLite.
 */

export type GalleryStatus = "pending" | "processing" | "completed";

export interface GalleryItem {
  id: string;
  name: string;
  target: string; // e.g. "M42", "NGC 7000"
  integrationHours: number;
  palette: string; // e.g. "LRGB", "SHO", "Hα"
  status: GalleryStatus;
  updatedAt: string; // ISO
  /** Optional thumbnail data URL — currently a radial gradient placeholder. */
  thumbnail?: string;
}

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

let cachedItems: GalleryItem[] | null = null;

/**
 * Async to match the eventual rusqlite-backed API. Returns the gallery
 * items in display order (most-recent first).
 */
/**
 * Sync accessor — useful for components that need immediate access
 * (ModeC, ModeD). Same cached list as loadGallery(); the async version
 * is preferred for first-load since it lets us await the rusqlite path
 * in Phase 4.
 */
export function getGallery(): GalleryItem[] {
  if (cachedItems === null) {
    cachedItems = [...PLACEHOLDER_ITEMS];
  }
  return cachedItems;
}

export async function loadGallery(): Promise<GalleryItem[]> {
  return getGallery();
}

/**
 * Filter helper used by the gallery tabs (Recent / Processing / Completed).
 * "Recent" is the most-recent slice; "Processing" / "Completed" filter by
 * status.
 */
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

/** Status badge label and CSS modifier — kept here so the gallery stays
 * the single source of truth for status semantics. */
export function statusBadgeClass(status: GalleryStatus): string {
  return `gallery-status gallery-status-${status}`;
}

export function statusBadgeLabel(status: GalleryStatus): string {
  return status.toUpperCase();
}