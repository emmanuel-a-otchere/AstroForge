/**
 * RecipeFeed — recipe gallery + search (P4-M2-T2 + T3).
 *
 * Spec reference: §11.3 "Sharing Mechanisms" — "In-app Recipe Gallery:
 * browsable, filterable by target/equipment/palette."
 *
 * Field naming: Rust serialises with snake_case (e.g. `target_type`),
 * the Svelte side uses camelCase (e.g. `targetType`). The mapping
 * happens at the IPC boundary here. This mirrors profile-store.ts.
 *
 * Browser fallback: when running in plain Vite (no Tauri shell),
 * `invoke()` throws. We fall back to a small in-memory fixture set
 * so the UI still renders during dev. The fixture mirrors the
 * Rust unit-test feed so the gallery shape is testable in isolation.
 */

import { writable, type Writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

export interface EquipmentHints {
  camera: string;
  filters: string[];
}

export interface SharedRecipeStage {
  stage: string;
  params: Record<string, unknown>;
}

export interface SharedIntegrity {
  perceptualModelsUsed: boolean;
}

export interface SharedRecipe {
  recipeVersion: string;
  appVersion: string;
  name: string;
  author: string;
  targetType: string;
  equipmentHints: EquipmentHints;
  pipeline: SharedRecipeStage[];
  modelVersions: Record<string, string>;
  integrity: SharedIntegrity;
}

export type FilterPalette =
  | "Ha"
  | "OIII"
  | "SII"
  | "SHO"
  | "HOO"
  | "LRGB"
  | "Broadband";

export type SortOrder = "Relevance" | "DateDesc" | "DateAsc" | "Popularity";

export interface FilterCriteria {
  query?: string;
  target?: string;
  equipment?: string;
  palette?: FilterPalette;
  sort?: SortOrder;
}

interface RustSharedRecipe {
  recipe_version: string;
  app_version: string;
  name: string;
  author: string;
  target_type: string;
  equipment_hints: { camera: string; filters: string[] };
  pipeline: { stage: string; params: Record<string, unknown> }[];
  model_versions: Record<string, string>;
  integrity: { perceptual_models_used: boolean };
}

function fromRust(r: RustSharedRecipe): SharedRecipe {
  return {
    recipeVersion: r.recipe_version,
    appVersion: r.app_version,
    name: r.name,
    author: r.author,
    targetType: r.target_type,
    equipmentHints: {
      camera: r.equipment_hints.camera,
      filters: r.equipment_hints.filters,
    },
    pipeline: r.pipeline,
    modelVersions: r.model_versions,
    integrity: { perceptualModelsUsed: r.integrity.perceptual_models_used },
  };
}

const FALLBACK: SharedRecipe[] = [
  {
    recipeVersion: "1.0",
    appVersion: "1.4.0",
    name: "HOO Narrowband - M42",
    author: "astro_emmanuel",
    targetType: "deep_sky",
    equipmentHints: { camera: "Seestar S50", filters: ["Ha", "OIII"] },
    pipeline: [{ stage: "calibration", params: {} }],
    modelVersions: { "swinir-denoise-astro": "1.0" },
    integrity: { perceptualModelsUsed: false },
  },
  {
    recipeVersion: "1.0",
    appVersion: "1.4.0",
    name: "SHO Hubble Palette - NGC7000",
    author: "cosmic_cam",
    targetType: "deep_sky",
    equipmentHints: { camera: "ASI2600MM Pro", filters: ["Ha", "OIII", "SII"] },
    pipeline: [{ stage: "calibration", params: {} }],
    modelVersions: {},
    integrity: { perceptualModelsUsed: false },
  },
  {
    recipeVersion: "1.0",
    appVersion: "1.4.0",
    name: "LRGB M31 Wide Field",
    author: "luminance_lab",
    targetType: "deep_sky",
    equipmentHints: { camera: "ASI2600MM Pro", filters: ["L", "R", "G", "B"] },
    pipeline: [{ stage: "calibration", params: {} }],
    modelVersions: {},
    integrity: { perceptualModelsUsed: false },
  },
  {
    recipeVersion: "1.0",
    appVersion: "1.4.0",
    name: "Lunar Crater Close-up",
    author: "selenophile",
    targetType: "planetary",
    equipmentHints: { camera: "ASI224MC", filters: [] },
    pipeline: [{ stage: "calibration", params: {} }],
    modelVersions: {},
    integrity: { perceptualModelsUsed: false },
  },
  {
    recipeVersion: "1.0",
    appVersion: "1.4.0",
    name: "Solar Disc H-alpha",
    author: "helio_hank",
    targetType: "solar",
    equipmentHints: { camera: "Lunt80", filters: ["Ha"] },
    pipeline: [{ stage: "calibration", params: {} }],
    modelVersions: {},
    integrity: { perceptualModelsUsed: false },
  },
  {
    recipeVersion: "1.0",
    appVersion: "1.4.0",
    name: "M42 Quick Edit",
    author: "astro_emmanuel",
    targetType: "deep_sky",
    equipmentHints: { camera: "Seestar S50", filters: [] },
    pipeline: [{ stage: "calibration", params: {} }],
    modelVersions: {},
    integrity: { perceptualModelsUsed: false },
  },
  {
    recipeVersion: "1.0",
    appVersion: "1.4.0",
    name: "M42 Broadband Stacking",
    author: "deep_sky_dave",
    targetType: "deep_sky",
    equipmentHints: { camera: "ASI533MC", filters: [] },
    pipeline: [{ stage: "calibration", params: {} }],
    modelVersions: {},
    integrity: { perceptualModelsUsed: false },
  },
];

function derivePalettes(filters: string[]): FilterPalette[] {
  const has = (s: string) => filters.some((f) => f.toLowerCase() === s.toLowerCase());
  const out: FilterPalette[] = [];
  if (has("Ha") && has("OIII") && has("SII")) out.push("SHO");
  if (has("Ha") && has("OIII") && !has("SII")) out.push("HOO");
  if (has("L") && has("R") && has("G") && has("B")) out.push("LRGB");
  if (has("Ha") && !out.some((p) => p === "HOO" || p === "SHO")) out.push("Ha");
  if (has("OIII") && !out.some((p) => p === "HOO" || p === "SHO")) out.push("OIII");
  if (has("SII") && !out.some((p) => p === "SHO")) out.push("SII");
  if (filters.length === 0) out.push("Broadband");
  return out;
}

function matchesQuery(recipe: SharedRecipe, q: string): number {
  const needle = q.toLowerCase();
  let score = 0;
  if (recipe.name.toLowerCase().includes(needle)) score += 1;
  if (recipe.author.toLowerCase().includes(needle)) score += 1;
  if (recipe.targetType.toLowerCase().includes(needle)) score += 1;
  return score;
}

export function searchLocal(
  recipes: SharedRecipe[],
  criteria: FilterCriteria,
): SharedRecipe[] {
  let hits: { score: number; recipe: SharedRecipe }[] = [];
  for (const recipe of recipes) {
    if (criteria.query) {
      const score = matchesQuery(recipe, criteria.query);
      if (score === 0) continue;
      hits.push({ score, recipe });
    } else {
      hits.push({ score: 0, recipe });
    }
  }
  hits = hits.filter(({ recipe }) => {
    if (criteria.target && recipe.targetType.toLowerCase() !== criteria.target.toLowerCase())
      return false;
    if (
      criteria.equipment &&
      recipe.equipmentHints.camera.toLowerCase() !== criteria.equipment.toLowerCase()
    )
      return false;
    if (criteria.palette) {
      const palettes = derivePalettes(recipe.equipmentHints.filters);
      if (!palettes.includes(criteria.palette)) return false;
    }
    return true;
  });
  const sort = criteria.sort ?? "Relevance";
  if (sort === "Relevance") {
    hits.sort((a, b) => b.score - a.score || b.recipe.recipeVersion.localeCompare(a.recipe.recipeVersion));
  } else if (sort === "DateDesc") {
    hits.sort((a, b) => b.recipe.recipeVersion.localeCompare(a.recipe.recipeVersion));
  } else if (sort === "DateAsc") {
    hits.sort((a, b) => a.recipe.recipeVersion.localeCompare(b.recipe.recipeVersion));
  } else if (sort === "Popularity") {
    hits.sort((a, b) => a.recipe.name.localeCompare(b.recipe.name));
  }
  return hits.map((h) => h.recipe);
}

export const galleryStore: Writable<SharedRecipe[]> = writable(FALLBACK);

export async function loadGallery(): Promise<SharedRecipe[]> {
  try {
    const raw = (await invoke("recipe_feed_list")) as RustSharedRecipe[];
    const mapped = raw.map(fromRust);
    galleryStore.set(mapped);
    return mapped;
  } catch {
    // No Tauri shell — use the fallback fixture.
    galleryStore.set(FALLBACK);
    return FALLBACK;
  }
}

export async function searchGallery(
  criteria: FilterCriteria,
): Promise<SharedRecipe[]> {
  const all = await loadGallery();
  return searchLocal(all, criteria);
}