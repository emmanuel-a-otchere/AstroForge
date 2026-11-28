// CR-10 P1.6.2.1 / Slice P1.6.2.1:
// NodePalette pure logic. Extracted from
// `src/components/NodePalette.svelte` so the
// rendering + interaction logic can be tested
// without booting Svelte in jsdom.
//
// The palette groups the 12 user-facing stage types
// into the seven palette groups defined in
// AstroForge_Spec_v1.4.0.md §7.7.3 (Node Palette).

import type { NodeCatalog, NodeCatalogEntry } from "./node-catalog";

/** Palette group names, in display order. */
export const PALETTE_GROUP_ORDER = [
  "Input",
  "Calibration",
  "Calibration-Free",
  "Stacking",
  "Stretch",
  "Refinement",
  "Output",
] as const;

export type PaletteGroupName = (typeof PALETTE_GROUP_ORDER)[number];

/**
 * 12 user-facing stages -> 7 palette groups.
 * Stages not in this map are dropped from the palette
 * (e.g. canonical-only stages like `stack` / `register` /
 * `debayer` / `background` / `color` / `detail` are
 * exposed through the user-facing aliases below).
 */
export const USER_FACING_STAGE_TO_GROUP: Record<string, PaletteGroupName> = {
  ingest: "Input",
  crop_rotate: "Calibration",
  background_extraction: "Calibration",
  color_calibration: "Calibration-Free",
  color_wb: "Calibration-Free",
  color_scnr: "Calibration-Free",
  stretch: "Stretch",
  sharpen_deconvolution: "Refinement",
  denoise: "Refinement",
  star_handling: "Refinement",
  creative_polish: "Refinement",
  export: "Output",
};

/**
 * Display metadata per group: label (same as name here,
 * but kept as a separate map so future slices can
 * localize without changing component code) +
 * description.
 */
export const PALETTE_GROUP_DESCRIPTIONS: Record<PaletteGroupName, string> = {
  Input: "Load frames + read metadata",
  Calibration: "Crop, background, plus dark/flat/bias compensation",
  "Calibration-Free": "Color corrections for OSC + mono data without flats",
  Stacking: "Align + combine frames (handled automatically upstream)",
  Stretch: "Linear -> display stretch",
  Refinement: "Sharpen, denoise, separate stars, creative polish",
  Output: "Export the final image set",
};

/**
 * Empty-group message per group. Most groups use a
 * generic "no matching entries" placeholder; some
 * groups (e.g. Stacking) deserve a custom message
 * because they are intentionally empty at the
 * user-facing layer.
 */
export const PALETTE_GROUP_EMPTY_MESSAGES: Partial<
  Record<PaletteGroupName, string>
> = {
  Stacking: "Stacking happens automatically upstream",
};

/**
 * Filter + group a NodeCatalog into the seven palette
 * groups. Entries whose `stage_type` is not in
 * `USER_FACING_STAGE_TO_GROUP` are dropped (they belong
 * to the canonical layer only).
 *
 * Search is a case-insensitive substring match against
 * label, description, and stage_type. Empty search
 * returns all entries.
 */
export function groupEntriesByPalette(
  catalog: NodeCatalog,
  search: string,
): Map<PaletteGroupName, NodeCatalogEntry[]> {
  const map = new Map<PaletteGroupName, NodeCatalogEntry[]>();
  for (const g of PALETTE_GROUP_ORDER) map.set(g, []);
  const needle = search.trim().toLowerCase();
  for (const entry of catalog.entries) {
    const group = USER_FACING_STAGE_TO_GROUP[entry.stage_type];
    if (group === undefined) continue;
    if (needle.length > 0) {
      const hay =
        entry.label.toLowerCase() +
        "\n" +
        entry.description.toLowerCase() +
        "\n" +
        entry.stage_type.toLowerCase();
      if (!hay.includes(needle)) continue;
    }
    map.get(group)!.push(entry);
  }
  return map;
}

/**
 * Return the total number of palette entries across
 * all groups (entries that have a group mapping).
 */
export function countPaletteEntries(catalog: NodeCatalog): number {
  let n = 0;
  for (const entry of catalog.entries) {
    if (USER_FACING_STAGE_TO_GROUP[entry.stage_type] !== undefined) {
      n += 1;
    }
  }
  return n;
}

/**
 * Build the full HTML/CSS class name list for a palette
 * root element. Extracted for testability so we can
 * verify the component's root element gets the right
 * class string without rendering the component.
 */
export function paletteRootClassName(
  userClass: string,
  disabled: boolean,
): string {
  const parts = ["node-palette"];
  if (userClass.length > 0) parts.push(userClass);
  if (disabled) parts.push("disabled");
  return parts.join(" ");
}