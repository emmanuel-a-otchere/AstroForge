// CR-10 P1.6.2.1 / Slice P1.6.2.1 tests:
// NodePalette pure logic. The component's render
// surface is exercised by manual UI validation;
// this file pins the data shape (12 -> 7 group
// mapping, search filtering, empty messages,
// entry count, root-class composition) so future
// refactors cannot silently change the palette
// contract.

import { describe, it, expect } from "vitest";
import {
  PALETTE_GROUP_ORDER,
  PALETTE_GROUP_DESCRIPTIONS,
  PALETTE_GROUP_EMPTY_MESSAGES,
  USER_FACING_STAGE_TO_GROUP,
  groupEntriesByPalette,
  countPaletteEntries,
  paletteRootClassName,
  type PaletteGroupName,
} from "../node-palette-logic";
import type { NodeCatalog, NodeCatalogEntry } from "../node-catalog";

function mkEntry(
  stage_type: string,
  label: string,
  description: string,
  ai_uses_ai: boolean = false,
): NodeCatalogEntry {
  return {
    stage_type,
    label,
    description,
    ai_uses_ai,
    ai_model_id: ai_uses_ai ? "test_model_v1" : null,
    destructive: false,
    produces_image_version: false,
    undo_supported: false,
    default_params: {},
    supported_models: ai_uses_ai ? ["test_model_v1"] : [],
  };
}

const SAMPLE_CATALOG: NodeCatalog = {
  version: 1,
  entries: [
    mkEntry("ingest", "Ingest & Analyse", "Load image files, detect camera type"),
    mkEntry("crop_rotate", "Framing / Crop / Rotate", "Interactive crop with live rotation"),
    mkEntry("background_extraction", "Background", "Remove sky gradients"),
    mkEntry("color_calibration", "Colour Calibration / Balance", "Bounded white-balance corrections"),
    mkEntry("color_wb", "White Balance", "Pull WB off background or G2V reference"),
    mkEntry("color_scnr", "SCNR", "Remove green cast from OSC narrowband"),
    mkEntry("stretch", "Stretch", "Linear -> display stretch"),
    mkEntry("sharpen_deconvolution", "Sharpen (Deconv)", "Deconvolve seeing-limited stars"),
    mkEntry("denoise", "Denoise", "Noise reduction", true),
    mkEntry("star_handling", "Star Handling", "Separate stars, edit layers", true),
    mkEntry("creative_polish", "Creative Polish", "Curves, colour transmutation", true),
    mkEntry("export", "Export", "Multi-format export"),
  ],
};

describe("CR-10 P1.6.2.1 / NodePalette logic", () => {
  describe("PALETTE_GROUP_ORDER", () => {
    it("has exactly 7 groups", () => {
      expect(PALETTE_GROUP_ORDER.length).toBe(7);
    });

    it("lists groups in the spec §7.7.3 order", () => {
      expect(Array.from(PALETTE_GROUP_ORDER)).toEqual([
        "Input",
        "Calibration",
        "Calibration-Free",
        "Stacking",
        "Stretch",
        "Refinement",
        "Output",
      ]);
    });
  });

  describe("USER_FACING_STAGE_TO_GROUP", () => {
    it("maps exactly the 12 user-facing stages", () => {
      expect(Object.keys(USER_FACING_STAGE_TO_GROUP).length).toBe(12);
      for (const stage of [
        "ingest",
        "crop_rotate",
        "background_extraction",
        "color_calibration",
        "color_wb",
        "color_scnr",
        "stretch",
        "sharpen_deconvolution",
        "denoise",
        "star_handling",
        "creative_polish",
        "export",
      ]) {
        expect(USER_FACING_STAGE_TO_GROUP[stage]).toBeDefined();
      }
    });

    it("places ingest in Input", () => {
      expect(USER_FACING_STAGE_TO_GROUP["ingest"]).toBe("Input");
    });

    it("places crop_rotate + background_extraction in Calibration", () => {
      expect(USER_FACING_STAGE_TO_GROUP["crop_rotate"]).toBe("Calibration");
      expect(USER_FACING_STAGE_TO_GROUP["background_extraction"]).toBe("Calibration");
    });

    it("places color_calibration + color_wb + color_scnr in Calibration-Free", () => {
      expect(USER_FACING_STAGE_TO_GROUP["color_calibration"]).toBe(
        "Calibration-Free",
      );
      expect(USER_FACING_STAGE_TO_GROUP["color_wb"]).toBe("Calibration-Free");
      expect(USER_FACING_STAGE_TO_GROUP["color_scnr"]).toBe("Calibration-Free");
    });

    it("places stretch in Stretch", () => {
      expect(USER_FACING_STAGE_TO_GROUP["stretch"]).toBe("Stretch");
    });

    it("places sharpen_deconvolution + denoise + star_handling + creative_polish in Refinement", () => {
      expect(USER_FACING_STAGE_TO_GROUP["sharpen_deconvolution"]).toBe("Refinement");
      expect(USER_FACING_STAGE_TO_GROUP["denoise"]).toBe("Refinement");
      expect(USER_FACING_STAGE_TO_GROUP["star_handling"]).toBe("Refinement");
      expect(USER_FACING_STAGE_TO_GROUP["creative_polish"]).toBe("Refinement");
    });

    it("places export in Output", () => {
      expect(USER_FACING_STAGE_TO_GROUP["export"]).toBe("Output");
    });
  });

  describe("PALETTE_GROUP_DESCRIPTIONS", () => {
    it("has a description for every group", () => {
      for (const g of PALETTE_GROUP_ORDER) {
        expect(typeof PALETTE_GROUP_DESCRIPTIONS[g]).toBe("string");
        expect(PALETTE_GROUP_DESCRIPTIONS[g].length).toBeGreaterThan(0);
      }
    });
  });

  describe("PALETTE_GROUP_EMPTY_MESSAGES", () => {
    it("has a custom message for Stacking", () => {
      expect(PALETTE_GROUP_EMPTY_MESSAGES["Stacking"]).toBeDefined();
      expect(PALETTE_GROUP_EMPTY_MESSAGES["Stacking"]).toContain(
        "automatically",
      );
    });
  });

  describe("groupEntriesByPalette", () => {
    it("returns a map with all 7 groups", () => {
      const result = groupEntriesByPalette(SAMPLE_CATALOG, "");
      expect(result.size).toBe(7);
      for (const g of PALETTE_GROUP_ORDER) {
        expect(result.has(g)).toBe(true);
      }
    });

    it("places each of the 12 entries in the right group (empty search)", () => {
      const result = groupEntriesByPalette(SAMPLE_CATALOG, "");
      const total = Array.from(result.values()).reduce(
        (acc, list) => acc + list.length,
        0,
      );
      expect(total).toBe(12);
      expect(result.get("Input")!.length).toBe(1);
      expect(result.get("Calibration")!.length).toBe(2);
      expect(result.get("Calibration-Free")!.length).toBe(3);
      expect(result.get("Stacking")!.length).toBe(0);
      expect(result.get("Stretch")!.length).toBe(1);
      expect(result.get("Refinement")!.length).toBe(4);
      expect(result.get("Output")!.length).toBe(1);
    });

    it("filters by label substring (case-insensitive)", () => {
      const result = groupEntriesByPalette(SAMPLE_CATALOG, "color");
      const input = result.get("Calibration-Free")!;
      expect(input.length).toBe(3);
      expect(input.every((e) => /color/i.test(e.label + e.description + e.stage_type))).toBe(
        true,
      );
      // Calibration group should be empty after a color-only search.
      expect(result.get("Calibration")!.length).toBe(0);
    });

    it("filters by description substring", () => {
      const result = groupEntriesByPalette(SAMPLE_CATALOG, "gradient");
      expect(result.get("Calibration")!.length).toBe(1);
      expect(result.get("Calibration")![0].stage_type).toBe(
        "background_extraction",
      );
    });

    it("filters by stage_type substring", () => {
      const result = groupEntriesByPalette(SAMPLE_CATALOG, "color_wb");
      expect(result.get("Calibration-Free")!.length).toBe(1);
      expect(result.get("Calibration-Free")![0].stage_type).toBe("color_wb");
    });

    it("drops entries that are not in the user-facing stage map", () => {
      const cat: NodeCatalog = {
        version: 1,
        entries: [
          ...SAMPLE_CATALOG.entries,
          // Canonical-only stage: should be dropped from the palette.
          mkEntry("stack", "Stack", "Align + integrate"),
          mkEntry("register", "Register", "Plate-solve + align"),
        ],
      };
      const result = groupEntriesByPalette(cat, "");
      const total = Array.from(result.values()).reduce(
        (acc, list) => acc + list.length,
        0,
      );
      // Still 12; canonical stages are filtered out.
      expect(total).toBe(12);
    });

    it("returns empty arrays for every group when search matches nothing", () => {
      const result = groupEntriesByPalette(SAMPLE_CATALOG, "no_such_stage");
      for (const g of PALETTE_GROUP_ORDER) {
        expect(result.get(g)!.length).toBe(0);
      }
    });

    it("treats whitespace-only search as empty", () => {
      const result = groupEntriesByPalette(SAMPLE_CATALOG, "   ");
      const total = Array.from(result.values()).reduce(
        (acc, list) => acc + list.length,
        0,
      );
      expect(total).toBe(12);
    });
  });

  describe("countPaletteEntries", () => {
    it("returns 12 for the sample catalog", () => {
      expect(countPaletteEntries(SAMPLE_CATALOG)).toBe(12);
    });

    it("excludes canonical-only stages", () => {
      const cat: NodeCatalog = {
        version: 1,
        entries: [
          ...SAMPLE_CATALOG.entries,
          mkEntry("stack", "Stack", ""),
        ],
      };
      expect(countPaletteEntries(cat)).toBe(12);
    });

    it("returns 0 for an empty catalog", () => {
      expect(countPaletteEntries({ version: 1, entries: [] })).toBe(0);
    });
  });

  describe("paletteRootClassName", () => {
    it("starts with the base class", () => {
      expect(paletteRootClassName("", false)).toBe("node-palette");
    });

    it("appends the user class when provided", () => {
      expect(paletteRootClassName("my-palette", false)).toBe(
        "node-palette my-palette",
      );
    });

    it("appends 'disabled' when disabled=true", () => {
      expect(paletteRootClassName("", true)).toBe("node-palette disabled");
    });

    it("appends both user class + disabled when both apply", () => {
      expect(paletteRootClassName("my-palette", true)).toBe(
        "node-palette my-palette disabled",
      );
    });
  });

  describe("type guards", () => {
    it("PaletteGroupName matches PALETTE_GROUP_ORDER entries", () => {
      const sample: PaletteGroupName = "Refinement";
      expect(PALETTE_GROUP_ORDER).toContain(sample);
    });
  });
});