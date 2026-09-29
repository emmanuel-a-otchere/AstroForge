// CR-10 P1.6.2.2 / Slice P1.6.2.2:
// Node-graph mode store.
//
// Holds the per-session mode flag (Wizard vs Node)
// that decides whether ProcessWorkspace mounts the
// NodePalette on the left side. The toggle UI that
// writes to this store lands in P1.6.2.3; this slice
// only ships the store + the derived predicate so
// downstream components can react.
//
// Persistence: this slice intentionally does NOT
// persist the mode. P1.6.2.3 wires the localStorage
// round-trip. Keeping persistence out of this slice
// means P1.6.2.2 ships without a side-effect on
// session reload, which makes the slice safe to
// revert if the mode-toggle UX gets redesigned.

import { writable, derived, type Writable, type Readable } from "svelte/store";

/** Wizard mode = the existing recipe-editor UX. Node mode = palette + canvas. */
export type NodeGraphMode = "wizard" | "node";

/** Default mode when the slice is loaded fresh. Wizard first, per spec. */
export const DEFAULT_NODE_GRAPH_MODE: NodeGraphMode = "wizard";

/** Writable backing store. P1.6.2.3 writes to this from the toggle. */
export const nodeGraphMode: Writable<NodeGraphMode> = writable<NodeGraphMode>(
  DEFAULT_NODE_GRAPH_MODE,
);

/**
 * Set the mode. Exported so P1.6.2.3 can wire the
 * toggle + the localStorage round-trip without
 * touching the store internals.
 */
export function setNodeGraphMode(mode: NodeGraphMode): void {
  nodeGraphMode.set(mode);
}

/**
 * Convenience predicate: is the session currently
 * in Node mode? Use this in templates (`{#if $isNodeMode}`)
 * rather than `{#if $nodeGraphMode === "node"}` so the
 * intent is grep-able.
 */
export const isNodeMode: Readable<boolean> = derived(
  nodeGraphMode,
  ($mode) => $mode === "node",
);

/**
 * Convenience predicate: is the session currently
 * in Wizard mode? Symmetric counterpart of isNodeMode.
 */
export const isWizardMode: Readable<boolean> = derived(
  nodeGraphMode,
  ($mode) => $mode === "wizard",
);

/**
 * Reset to the default. Used by tests; future
 * "reset session" tooling can call this too.
 */
export function resetNodeGraphMode(): void {
  nodeGraphMode.set(DEFAULT_NODE_GRAPH_MODE);
}