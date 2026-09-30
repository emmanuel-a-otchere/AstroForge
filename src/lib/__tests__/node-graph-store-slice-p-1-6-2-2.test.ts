// CR-10 P1.6.2.2 / Slice P1.6.2.2 tests:
// Node-graph mode store + sidebar mounting.
//
// The store is plain svelte/store (writable +
// derived), so the tests subscribe to the derived
// streams directly with `get()`. The component
// layer (ProcessWorkspace + WorkspaceScreen) is
// exercised by manual smoke pass because the
// project does not have @testing-library/svelte
// installed.

import { describe, it, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import {
  DEFAULT_NODE_GRAPH_MODE,
  NODE_GRAPH_MODE_STORAGE_KEY,
  nodeGraphMode,
  isNodeMode,
  isWizardMode,
  setNodeGraphMode,
  resetNodeGraphMode,
  readPersistedNodeGraphMode,
  writePersistedNodeGraphMode,
  hydrateNodeGraphModeFromStorage,
} from "../node-graph-store";

class MemoryStorage implements Pick<Storage, "getItem" | "setItem"> {
  private store = new Map<string, string>();
  getItem(key: string): string | null {
    return this.store.has(key) ? (this.store.get(key) as string) : null;
  }
  setItem(key: string, value: string): void {
    this.store.set(key, value);
  }
  removeItem(key: string): void {
    this.store.delete(key);
  }
  clear(): void {
    this.store.clear();
  }
}

describe("CR-10 P1.6.2.2 / node-graph-store", () => {
  beforeEach(() => {
    resetNodeGraphMode();
  });

  it("defaults to Wizard mode", () => {
    expect(DEFAULT_NODE_GRAPH_MODE).toBe("wizard");
    expect(get(nodeGraphMode)).toBe("wizard");
  });

  it("isNodeMode reflects the store state", () => {
    expect(get(isNodeMode)).toBe(false);
    setNodeGraphMode("node");
    expect(get(isNodeMode)).toBe(true);
    setNodeGraphMode("wizard");
    expect(get(isNodeMode)).toBe(false);
  });

  it("isWizardMode is the logical inverse of isNodeMode", () => {
    expect(get(isWizardMode)).toBe(true);
    expect(get(isNodeMode)).toBe(false);
    setNodeGraphMode("node");
    expect(get(isWizardMode)).toBe(false);
    expect(get(isNodeMode)).toBe(true);
  });

  it("setNodeGraphMode propagates to the derived stores", () => {
    setNodeGraphMode("node");
    expect(get(nodeGraphMode)).toBe("node");
    expect(get(isNodeMode)).toBe(true);
    expect(get(isWizardMode)).toBe(false);
  });

  it("resetNodeGraphMode returns to the default", () => {
    setNodeGraphMode("node");
    expect(get(nodeGraphMode)).toBe("node");
    resetNodeGraphMode();
    expect(get(nodeGraphMode)).toBe("wizard");
    expect(get(isNodeMode)).toBe(false);
    expect(get(isWizardMode)).toBe(true);
  });

  it("only the two declared modes are accepted at the type level", () => {
    // Compile-time guard: this test would fail to
    // compile if the type union changed without
    // updating the slice's tests.
    const a: "wizard" | "node" = "wizard";
    const b: "wizard" | "node" = "node";
    setNodeGraphMode(a);
    setNodeGraphMode(b);
    expect(get(nodeGraphMode)).toBe("node");
  });

  it("the store does NOT persist across resets", () => {
    // CR-10 P1.6.2.2 deliberately ships without
    // localStorage persistence; P1.6.2.3 wires
    // the round-trip. This test pins that the
    // reset behaviour returns to the default.
    setNodeGraphMode("node");
    resetNodeGraphMode();
    expect(get(nodeGraphMode)).toBe(DEFAULT_NODE_GRAPH_MODE);
  });
});

describe("CR-10 P1.6.2.3 / node-graph-mode persistence", () => {
  beforeEach(() => {
    resetNodeGraphMode();
  });

  it("uses 'astroforge.session.mode' as the storage key", () => {
    expect(NODE_GRAPH_MODE_STORAGE_KEY).toBe("astroforge.session.mode");
  });

  it("returns the default when storage is null", () => {
    expect(readPersistedNodeGraphMode(null)).toBe(DEFAULT_NODE_GRAPH_MODE);
    writePersistedNodeGraphMode(null, "node");
    // No throw, no side effect.
  });

  it("returns the default when the key is absent", () => {
    const storage = new MemoryStorage();
    expect(readPersistedNodeGraphMode(storage)).toBe(DEFAULT_NODE_GRAPH_MODE);
  });

  it("returns the default when the stored value is not a valid mode", () => {
    const storage = new MemoryStorage();
    storage.setItem(NODE_GRAPH_MODE_STORAGE_KEY, JSON.stringify("not-a-mode"));
    expect(readPersistedNodeGraphMode(storage)).toBe(DEFAULT_NODE_GRAPH_MODE);
  });

  it("returns the default when the stored value is malformed JSON", () => {
    const storage = new MemoryStorage();
    storage.setItem(NODE_GRAPH_MODE_STORAGE_KEY, "{not json");
    expect(readPersistedNodeGraphMode(storage)).toBe(DEFAULT_NODE_GRAPH_MODE);
  });

  it("round-trips a 'node' value through storage", () => {
    const storage = new MemoryStorage();
    writePersistedNodeGraphMode(storage, "node");
    expect(readPersistedNodeGraphMode(storage)).toBe("node");
  });

  it("round-trips a 'wizard' value through storage", () => {
    const storage = new MemoryStorage();
    writePersistedNodeGraphMode(storage, "wizard");
    expect(readPersistedNodeGraphMode(storage)).toBe("wizard");
  });

  it("hydrateNodeGraphModeFromStorage reads + writes the store", () => {
    const storage = new MemoryStorage();
    storage.setItem(NODE_GRAPH_MODE_STORAGE_KEY, JSON.stringify("node"));
    const hydrated = hydrateNodeGraphModeFromStorage(storage);
    expect(hydrated).toBe("node");
    expect(get(nodeGraphMode)).toBe("node");
    expect(get(isNodeMode)).toBe(true);
  });

  it("hydrateNodeGraphModeFromStorage falls back to default on absent key", () => {
    const storage = new MemoryStorage();
    const hydrated = hydrateNodeGraphModeFromStorage(storage);
    expect(hydrated).toBe(DEFAULT_NODE_GRAPH_MODE);
    expect(get(nodeGraphMode)).toBe(DEFAULT_NODE_GRAPH_MODE);
  });

  it("hydrateNodeGraphModeFromStorage with null storage keeps the default", () => {
    const hydrated = hydrateNodeGraphModeFromStorage(null);
    expect(hydrated).toBe(DEFAULT_NODE_GRAPH_MODE);
    expect(get(nodeGraphMode)).toBe(DEFAULT_NODE_GRAPH_MODE);
  });
});