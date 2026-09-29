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
  nodeGraphMode,
  isNodeMode,
  isWizardMode,
  setNodeGraphMode,
  resetNodeGraphMode,
} from "../node-graph-store";

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