// CR-10 P1.6.1.3 / Slice P1.6.1.3 tests:
// NodeCatalog TS bridge round-trip + IPC behaviour.
// These tests pin the contract between the Rust
// `read_node_catalog` IPC and the TS `loadNodeCatalog`
// helper. The Rust-side types are mirrored 1:1 in
// `src/lib/node-catalog.ts`; this file guards the
// bridge.

import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  loadNodeCatalog,
  getCachedNodeCatalog,
  clearNodeCatalogCache,
  type NodeCatalog,
  type NodeCatalogEntry,
} from "../node-catalog";

// Mock the @tauri-apps/api/core module so each
// test can drive `invoke` independently. We use
// vi.mock at module scope (hoisted) and override
// the mock implementation per test via
// vi.mocked(...).mockResolvedValueOnce(...).
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
const mockedInvoke = vi.mocked(invoke);

const SAMPLE_CATALOG: NodeCatalog = {
  version: 1,
  entries: [
    {
      stage_type: "ingest",
      label: "Ingest & Analyse",
      description: "Load image files, detect camera type, filter set, and basic statistics",
      ai_uses_ai: false,
      ai_model_id: null,
      destructive: false,
      produces_image_version: false,
      undo_supported: false,
      default_params: {},
      supported_models: [],
    },
    {
      stage_type: "denoise",
      label: "Denoise",
      description: "Noise reduction with preview-stable statistics",
      ai_uses_ai: true,
      ai_model_id: "astroforge_denoise_v1",
      destructive: true,
      produces_image_version: true,
      undo_supported: false,
      default_params: { strength: 0.5, method: "swinir" },
      supported_models: ["astroforge_denoise_v1"],
    },
    {
      stage_type: "star_handling",
      label: "Star Handling",
      description: "Separate stars, edit layers independently",
      ai_uses_ai: true,
      ai_model_id: "astroforge_detail_v1.2",
      destructive: false,
      produces_image_version: true,
      undo_supported: false,
      default_params: { separationMethod: "exact", replaceStrength: 1.0, colorBoost: 0 },
      supported_models: ["astroforge_detail_v1.2"],
    },
    {
      stage_type: "export",
      label: "Export",
      description: "Multi-format export: FITS master, TIFF, JPEG, starless, stars-only",
      ai_uses_ai: false,
      ai_model_id: null,
      destructive: false,
      produces_image_version: false,
      undo_supported: false,
      default_params: { format: "tiff16", includeStarless: false, includeStarsOnly: false },
      supported_models: [],
    },
  ],
};

describe("CR-10 P1.6.1.3 / NodeCatalog TS bridge", () => {
  beforeEach(() => {
    clearNodeCatalogCache();
    mockedInvoke.mockReset();
  });

  it("loads the catalog via the read_node_catalog IPC", async () => {
    mockedInvoke.mockResolvedValueOnce(SAMPLE_CATALOG);
    const cat = await loadNodeCatalog();
    expect(cat.version).toBe(1);
    expect(cat.entries.length).toBe(4);
    expect(mockedInvoke).toHaveBeenCalledWith("read_node_catalog");
  });

  it("caches the catalog in module scope", async () => {
    mockedInvoke.mockResolvedValueOnce(SAMPLE_CATALOG);
    const cat1 = await loadNodeCatalog();
    const cat2 = await loadNodeCatalog();
    // Same reference: cache hit.
    expect(cat1).toBe(cat2);
    // IPC was called exactly once.
    expect(mockedInvoke).toHaveBeenCalledTimes(1);
  });

  it("getCachedNodeCatalog returns null until first load", () => {
    expect(getCachedNodeCatalog()).toBeNull();
  });

  it("clearNodeCatalogCache resets the cache", async () => {
    mockedInvoke.mockResolvedValueOnce(SAMPLE_CATALOG);
    await loadNodeCatalog();
    expect(getCachedNodeCatalog()).not.toBeNull();
    clearNodeCatalogCache();
    expect(getCachedNodeCatalog()).toBeNull();
    // A second clear is safe.
    clearNodeCatalogCache();
    expect(getCachedNodeCatalog()).toBeNull();
    expect(mockedInvoke).toHaveBeenCalledTimes(1);
  });

  it("shares inflight Promise across concurrent callers (no thundering herd)", async () => {
    let resolveInvoke: () => void = () => {};
    const invokePromise = new Promise<NodeCatalog>((resolve) => {
      resolveInvoke = () => resolve(SAMPLE_CATALOG);
    });
    mockedInvoke.mockReturnValueOnce(invokePromise as unknown as Promise<NodeCatalog>);
    // Start the concurrent calls; do not await
    // yet, then resolve, then await.
    const inFlight = Promise.all([
      loadNodeCatalog(),
      loadNodeCatalog(),
      loadNodeCatalog(),
    ]);
    // Yield so all three callers reach the
    // `inflight !== null` branch and return the
    // shared Promise before we resolve.
    await new Promise((r) => setTimeout(r, 0));
    resolveInvoke();
    const [a, b, c] = await inFlight;
    expect(a).toBe(b);
    expect(b).toBe(c);
    // Still one IPC under concurrent callers.
    expect(mockedInvoke).toHaveBeenCalledTimes(1);
  });

  it("falls back to an empty catalog when the IPC is not registered (browser-mode)", async () => {
    mockedInvoke.mockRejectedValueOnce(
      new Error("read_node_catalog is not available in this build"),
    );
    const cat = await loadNodeCatalog();
    expect(cat.version).toBe(0);
    expect(cat.entries).toEqual([]);
  });

  it("rethrows unexpected IPC errors", async () => {
    mockedInvoke.mockRejectedValueOnce(new Error("internal Tauri bridge failure"));
    await expect(loadNodeCatalog()).rejects.toThrow("internal Tauri bridge failure");
  });
});

describe("CR-10 P1.6.1.3 / NodeCatalog TS interface shape", () => {
  it("every required field is present on a typical entry", () => {
    const e: NodeCatalogEntry = SAMPLE_CATALOG.entries[1];
    expect(e).toHaveProperty("stage_type");
    expect(e).toHaveProperty("label");
    expect(e).toHaveProperty("description");
    expect(e).toHaveProperty("ai_uses_ai");
    expect(e).toHaveProperty("ai_model_id");
    expect(e).toHaveProperty("destructive");
    expect(e).toHaveProperty("produces_image_version");
    expect(e).toHaveProperty("undo_supported");
    expect(e).toHaveProperty("default_params");
    expect(e).toHaveProperty("supported_models");
  });

  it("default_params is always a JSON object (never null)", () => {
    for (const e of SAMPLE_CATALOG.entries) {
      expect(typeof e.default_params).toBe("object");
      expect(e.default_params).not.toBeNull();
    }
  });

  it("supported_models is populated iff ai_uses_ai is true", () => {
    for (const e of SAMPLE_CATALOG.entries) {
      if (e.ai_uses_ai) {
        expect(e.supported_models.length).toBeGreaterThan(0);
      } else {
        expect(e.supported_models).toEqual([]);
      }
    }
  });

  it("ai_model_id is set iff ai_uses_ai is true", () => {
    for (const e of SAMPLE_CATALOG.entries) {
      if (e.ai_uses_ai) {
        expect(typeof e.ai_model_id).toBe("string");
        expect(e.ai_model_id).not.toBeNull();
      } else {
        expect(e.ai_model_id).toBeNull();
      }
    }
  });

  it("catalog version is a non-negative integer", () => {
    expect(Number.isInteger(SAMPLE_CATALOG.version)).toBe(true);
    expect(SAMPLE_CATALOG.version).toBeGreaterThanOrEqual(0);
  });
});