# UI Workflow Audit — Frontend Integrity Tranche

**Date:** 2026-09-05
**Source:** `docs/UI_WORKFLOW_REVIEW.md` (PR #218, 2026-09-04) — 8 gaps (G-1..G-8) and 9 recommendations (R-1..R-9).
**Scope:** Bridge the review into a sequenced implementation tranche, mirroring the M7 audit pattern. Closes the user-facing dead-ends and correctness bugs identified in the review.

## Status snapshot (after M7 close-out + profile tranche)

| # | Recommendation | Status | Evidence |
|---|---|---|---|
| **R-1** | Hide Mode C/D from mode-switcher until real | **CLOSED (PR #219)** | `src/lib/layout-mode.ts:118-127` `availableOverrideModes()` returns `["a","b"]` for `processing` and `landing`; Mode C/D are no longer reachable via the UI. Defensive path sufficient for v1 — full Mode D `PreviewCanvas` wiring deferred to a future milestone. |
| **R-2** | `sessionStore` as single source of truth for params; remove `previewParams` mirror | **pending (HIGH)** | `src/App.svelte:85-92` declares `previewParams` as local `$state`. `handleParamsChange` at line 251 only mutates the local mirror. `ParameterSidebar` mutates `blackPoint/midtones/highlights` but never calls `updateNodeParams()`; pipeline-store params stay stale until `commitStage()`. |
| **R-3** | `viewPreview(sessionId)` rehydrates `sessionStore` | **pending (MEDIUM)** | `src/App.svelte:108-115` accepts `_sessionId` but ignores it (`_` prefix signals unused). Only `setModeOverride("d")` fires. Mode D renders `$galleryStore[0]`, not the clicked session. |
| **R-4** | `backToLanding()` resets pipeline state | **pending (MEDIUM)** | `src/App.svelte:237-245` clears wizard local state but never calls `initSession()`. `sessionStore.pipelineGraph` from the previous session survives until the user re-confirms a fresh classification. |
| R-5 | Forge toggle in `AppShell` | pending | Out of scope for this tranche (orthogonal UI work). |
| R-6 | Per-file errors in file list | pending (LOW) | Out of scope. |
| R-7 | `ModeA` overlay responsive width | pending (LOW) | Out of scope. |
| R-8 | GPU badge gating | pending (LOW) | Out of scope. |
| R-9 | Vitest setup | pending (MEDIUM) | Out of scope — larger infrastructure change. |

## Tranche scope (this audit, 3 implementation PRs + 1 doc PR)

This audit closes **R-2, R-3, R-4** — the user-facing correctness bugs from the review. R-5 through R-9 are deferred to a future tranche.

| # | Task | Closes | PR split |
|---|---|---|---|
| **T1** | `previewParams` consolidation: replace local `$state` mirror with derivation from `pipeline-store`; route slider writes through `updateNodeParams()` | R-2 | PR-1 |
| **T2** | Gallery session rehydration: `viewPreview(sessionId)` calls `deserializeSession()` from persisted store, then `setModeOverride("d")` | R-3 | PR-2 |
| **T3** | `backToLanding()` state reset: add `resetSession()` action to `pipeline-store`; call from `backToLanding()` | R-4 | PR-3 |

**Estimated effort:** 2-3 days. Each PR is surgical (1-3 files). All four PRs land within M8 / 1.5P scope.

## Findings (deep dive)

### T1 — `previewParams` dual source of truth (HIGH)

**Where:**
- `src/App.svelte:85-92` declares `previewParams` as local `$state` with default stretch params.
- `src/App.svelte:251-253` `handleParamsChange(params)` only mutates local: `previewParams = { ...previewParams, ...params }`.
- `<PreviewCanvas>`, `<ParameterSidebar>`, `<WizardBottomSheet>` all receive `previewParams` as prop (lines 388, 396, 417, 424).

**Impact:**
- Stretch slider in `ParameterSidebar` mutates `blackPoint/midtones/highlights` locally — live preview changes but `pipeline-store` `nodes[stretch].params` is **stale** until `commitStage()` runs.
- If the user adjusts the slider then undoes a later commit, the slider snaps back to the persisted params (not the unsaved local edits), losing the user's current preview state.
- Two sources of truth + no write-back = undo/redo behaves incorrectly.

**Fix (Option B from review):**
- Remove the local `$state` `previewParams` in `App.svelte`.
- Replace with a derived value from `pipeline-store`:
  ```ts
  let previewParams = $derived(derivePreviewParams($pipelineGraph, $activeStepIndex));
  ```
- `handleParamsChange(params)` routes through `updateNodeParams(activeNodeId, params)`.
- `updateNodeParams` exists at `src/lib/pipeline-store.ts:515` — primitive is ready.
- Slider drag → debounced writes → commit-on-blur for stage persist (mirrors WizardBottomSheet's slider pattern).

**Edge case:** When `pipelineGraph.nodes` is empty (no session), `previewParams` should fall back to safe defaults. Use the existing `PIPELINE_STAGES` defaultParams as the fallback.

### T2 — Gallery session rehydration (MEDIUM)

**Where:**
- `src/App.svelte:108-115` `viewPreview(_sessionId)` ignores the param (note the `_` prefix) and only calls `setModeOverride("d")`.
- `src/lib/gallery.ts` likely has `loadSession(sessionId)` or `getSession(sessionId)` already (M1 work). Confirm during PR.

**Impact:**
- Clicking a session card in `<ManifestReview>` silently lands on a different session's image (or no image if `galleryStore` is empty).
- The most-recent `sessionStore` state from `initSession()` leaks across gallery clicks.

**Fix:**
- Look up the session metadata from the gallery store (likely `getSessionMetadata(sessionId)`).
- Call `deserializeSession(json)` from `pipeline-store:546` — already implemented (M1-T4 work).
- Then `setModeOverride("d")` to land on Mode D.
- Add a brief loading state ("Loading session…") while deserialization completes.
- If the session has no persisted JSON (e.g. ad-hoc viewer session), fall back to current behavior with a warning chip.

**Edge case:** `sessionStore.sessionId` is the only stable identifier. After deserialization, the store's sessionId should match the gallery's sessionId; if not, surface a warning.

### T3 — `backToLanding()` state reset (MEDIUM)

**Where:**
- `src/App.svelte:237-245` `backToLanding()` clears local wizard state but never touches `sessionStore`.
- `initSession(undefined, mode)` at line 190 in `handleClassificationConfirm` reinitializes the pipeline graph, but only when the user reaches the confirm step. Between `backToLanding()` and re-confirm, the previous session's `pipelineGraph` is still in the store.

**Impact:**
- If the user navigates from processing → back → starts a new session but doesn't reach confirm, `NodeSidebar` / `ParameterSidebar` / `WizardBottomSheet` (if reachable via dev tools or accidental rendering) would render against the previous session's params.
- `activeStepIndex` from the previous session also persists.

**Fix:**
- Expose a `resetSession()` action in `pipeline-store` that calls `createInitialSession()` with no session id (equivalent to `initSession(undefined)` without a mode arg).
- Wire `backToLanding()` to call `resetSession()`.
- Confirm via unit-level test that `pipelineGraph.nodes.length === 10` after reset.

**Edge case:** If a save-on-debounce is in flight when `backToLanding` fires, the save should be cancelled. Most of the persistence is fire-and-forget IPC, so the worst case is a stale row in `stage_runs`; acceptable.

## Open questions

- **Q-1**: For T1, should slider writes be **debounced** (live preview, commit on rest) or **commit-on-blur** (commit when slider releases)? The current `WizardBottomSheet` pattern is commit-on-blur (next-step click). Recommend matching that for consistency.

- **Q-2**: For T2, if the persisted session JSON is missing or corrupt, should we (a) silently fall back to current behavior, (b) show a "session not loadable" error toast, or (c) open a new empty session? Recommend (b).

- **Q-3**: For T3, should `resetSession()` also clear `sessionFlags`, `pendingProfileId`, etc.? Recommend yes — `createInitialSession()` already produces a clean state.

## Recommended PR sequencing

| Order | PR | Risk | Files | Tests |
|---|---|---|---|---|
| 1 | **Doc PR** — this audit + update `UI_WORKFLOW_REVIEW.md` R-1 status | LOW | 2 | 0 |
| 2 | **PR-1** — `previewParams` consolidation (T1) | HIGH | 3-4 (App.svelte, ParameterSidebar, WizardBottomSheet, pipeline-store if needed) | 1-2 (pipeline-store derive fn unit test) |
| 3 | **PR-2** — Gallery rehydration (T2) | MEDIUM | 2-3 (App.svelte, gallery.ts, maybe pipeline-store) | 1 (deserializeSession round-trip) |
| 4 | **PR-3** — `backToLanding()` reset (T3) | LOW | 2 (App.svelte, pipeline-store) | 1 (`resetSession()` unit test) |

## Out of scope (deferred)

- R-5 (forge toggle into `AppShell`)
- R-6 (per-file errors)
- R-7 (`ModeA` overlay responsive width)
- R-8 (GPU badge gating)
- R-9 (Vitest setup)

These remain in `docs/UI_WORKFLOW_REVIEW.md` as future-work items.

## Files to be modified

- `docs/UI_WORKFLOW_AUDIT.md` (NEW) — this document
- `docs/UI_WORKFLOW_REVIEW.md` — update R-1 row to reflect PR #219 closure
- `src/App.svelte` — T1, T2, T3 changes
- `src/components/ParameterSidebar.svelte` — T1 (route writes through `updateNodeParams`)
- `src/components/WizardBottomSheet.svelte` — T1 (verify existing commit path doesn't conflict with new model)
- `src/lib/pipeline-store.ts` — T1 (new `derivePreviewParams`), T3 (new `resetSession`)
- `src/lib/gallery.ts` — T2 (verify `getSession` API matches what `viewPreview` needs)