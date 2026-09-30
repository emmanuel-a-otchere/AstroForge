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

/**
 * localStorage key used by P1.6.2.3 to persist the
 * mode across reloads. The value stored is a JSON-
 * encoded `NodeGraphMode` (i.e. `"wizard"` or
 * `"node"` as a JSON string). Anything else at this
 * key (or the absence of the key) falls back to
 * `DEFAULT_NODE_GRAPH_MODE`.
 */
export const NODE_GRAPH_MODE_STORAGE_KEY = "astroforge.session.mode";

/**
 * Read the persisted mode from localStorage. SSR /
 * non-browser environments (no `window`) return the
 * default. Any parse error or unrecognized value
 * also returns the default.
 *
 * Exported so tests can drive the round-trip
 * directly with a mock storage object.
 */
export function readPersistedNodeGraphMode(
  storage: Pick<Storage, "getItem"> | null,
): NodeGraphMode {
  if (storage === null) return DEFAULT_NODE_GRAPH_MODE;
  try {
    const raw = storage.getItem(NODE_GRAPH_MODE_STORAGE_KEY);
    if (raw === null) return DEFAULT_NODE_GRAPH_MODE;
    const parsed = JSON.parse(raw) as unknown;
    if (parsed === "wizard" || parsed === "node") return parsed;
    return DEFAULT_NODE_GRAPH_MODE;
  } catch {
    return DEFAULT_NODE_GRAPH_MODE;
  }
}

/**
 * Persist the mode to localStorage. SSR / non-browser
 * environments (no `window`) are a no-op. Storage
 * write failures (quota, privacy mode) are swallowed
 * so a transient storage failure never breaks the
 * in-memory mode toggle.
 */
export function writePersistedNodeGraphMode(
  storage: Pick<Storage, "setItem"> | null,
  mode: NodeGraphMode,
): void {
  if (storage === null) return;
  try {
    storage.setItem(NODE_GRAPH_MODE_STORAGE_KEY, JSON.stringify(mode));
  } catch {
    // Intentional: storage failures are non-fatal.
    // The store remains the source of truth.
  }
}

/**
 * Hydrate the store from localStorage. Call once at
 * app boot (after the store is created) so the mode
 * the user picked last time survives a reload.
 *
 * Returns the hydrated mode so the caller can log /
 * display the source of the value.
 */
export function hydrateNodeGraphModeFromStorage(
  storage: Pick<Storage, "getItem"> | null,
): NodeGraphMode {
  const persisted = readPersistedNodeGraphMode(storage);
  nodeGraphMode.set(persisted);
  return persisted;
}

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