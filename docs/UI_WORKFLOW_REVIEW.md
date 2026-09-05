# UI Workflow Review — AstroForge Frontend

**Scope:** End-to-end user-facing flow across all 5 `AppStage` values (`landing` → `select-files` → `session-setup` → `review-frames` → `processing`) and the 4 `LayoutMode` shells (`A` Load / `B` Library / `C` Automagic Pro / `D` Refine).

**Date:** 2026-09-05
**Method:** Static read of `src/App.svelte`, `src/components/AppShell.svelte`, the 4 `Mode*.svelte` shells, `src/lib/layout-mode.ts`, `src/lib/pipeline-store.ts`, plus the 19 Svelte components.

---

## Executive Summary

The workflow is well-architected at the **state-machine level** — `layout-mode.ts` cleanly separates "what stage am I in" from "which shell am I rendering", and `pipeline-store.ts` provides a real history-aware commit/undo/redo model with receipts. The **load-stage flow** (`landing` → `select-files` → `session-setup` → `review-frames`) is a coherent wizard.

However, the **processing-stage flow** has significant gaps between the UI shell design and the actual pipeline work:
- **Mode C (Automagic Pro) and Mode D (Refine)** are mock-ups — `ModeC.svelte:25-43` hardcodes `tuningParams` and `pipelineStages`, `ModeD.svelte:8` "placeholder gradient for now; will become PreviewCanvas in Phase 6 alignment". They render via mode-switch override but are **never reached by the natural load-stage flow**.
- The **forge-mode toggle** (`App.svelte:354-362`) duplicates the role of the **mode switcher** in `AppShell.svelte:85-97` — two parallel UI controls for the same concept ("which refinement view am I in"), each with different mechanics.
- `initSession()` is called **once** at `review-frames` confirm (`App.svelte:189-192`) but **never re-synced** when the user returns to a previously-saved session from the gallery, so a stale `sessionStore` carries across sessions.
- **Back navigation drops history**: `backToLanding()` clears `selectedFiles / sessionData / classificationFrames / analysisResult` but does not call `initSession()` or clear `sessionStore`; the pipeline-graph state from the previous session survives.
- `WizardBottomSheet` is the only refinement surface that talks to `pipeline-store`; `ParameterSidebar` and `NodeSidebar` (forge mode) read from the store but receive their initial state from `App.svelte`'s `previewParams` prop, creating a **dual source of truth**.

The recommended sequencing: address gaps in the order they would be hit by a user, top of funnel down.

---

## 1. The Happy Path (what works)

### Stage → Shell derivation

`layout-mode.ts:62-74` cleanly maps stages to shells:
| Stage | Default shell |
|---|---|
| `landing` | B (Library) |
| `select-files` | A (Load) |
| `session-setup` | A (Load) |
| `review-frames` | A (Load) |
| `processing` | B (Library) |

Manual overrides (`manualOverride` store) take precedence and persist until cleared (`layout-mode.ts:76-79`). The override surface is appropriately scoped: only `landing` and `processing` expose C and D in the mode switcher (`layout-mode.ts:111-120`); focused load stages only allow falling back to B.

### Wizard load flow (A/B modes only)

`App.svelte:96-204` walks the user through:
1. **Landing** (`ModeB`): `landing-hero` with "Load Your Data" CTA (`App.svelte:301-313`) plus recent sessions list (`ManifestReview`).
2. **Select files** (`ModeA`): drag-drop or browse; only file-extension filter, no FITS-header validation (`App.svelte:109-125`).
3. **Session setup** (`ModeA` overlay + `InitialDialog`): target name, camera type, focal length, lights-only/dithering toggles, object type (`App.svelte:146-173`).
4. **Frame review** (`ModeA` overlay + `ClassificationDialog`): per-file frame type with reclassify, lights/darks/flats/bias counts displayed (`App.svelte:183-192`).
5. **Processing** (`ModeB`): `initSession(undefined, "automagic")` then `currentStep = "processing"` (`App.svelte:189-192`).

Each `ScreenCard` has a `footer` snippet with Cancel/Continue buttons — clean pattern.

### Pipeline state machine

`pipeline-store.ts:172-203` builds a 10-node DAG from `PIPELINE_STAGES` (ingest → crop_rotate → background_extraction → color_calibration → sharpen_deconvolution → denoise → stretch → star_handling → creative_polish → export). `commitStage()` advances the active step, writes a `HistoryEntry`, and bumps `version` (`pipeline-store.ts:258-313`). `undo/redo` walks `historyPointer` (`pipeline-store.ts:346-416`). Receipts carry `timestamp, durationMs, parameters, warnings, metrics, engine?, success` — exactly the right shape for reproducibility.

### Transitions

`App.svelte:335-382` uses Svelte 5 `fly` transitions from `svelte/transition` with `quintOut` easing: forge-layout slides in from left (`x:-240`), `ParameterSidebar` from right (`x:240`), `WizardBottomSheet` from below (`y:200`), bare `PreviewCanvas` from above (`y:40`). 350 ms in / 280 ms out. Implemented in M3-T4 (PR #211).

---

## 2. Gaps and Risks (ordered by user impact)

### G-1 — Mode C and Mode D are mock shells (HIGH)

**Where:** `ModeC.svelte:25-43` (hardcoded `tuningParams`, `pipelineStages`, `outputPreview`), `ModeD.svelte:8` (placeholder gradient; comment "will become PreviewCanvas in Phase 6 alignment").

**Impact:**
- The mode switcher in `AppShell.svelte:85-97` exposes "Automagic Pro" and "Refine" as user-selectable modes. Clicking them lands the user on a screen with fabricated data.
- Worse: the visible content does **not** reflect `sessionStore` — if the user clicks Mode C mid-session, the tuning sliders show static demo values that do not match the actual pipeline graph.
- These modes also have **no nav buttons** — once the user is in C or D, the natural back/next affordances from the wizard are absent (no Continue/Cancel footer).

**Why it exists:** Phase 5 untracked `docs/ui-schematics/options/` contains the design mock-ups; Mode C and Mode D were scaffolded to host them. Real implementation was deferred.

**Fix order:**
- Either (a) wire Mode D to `PreviewCanvas` + `ParameterSidebar` and remove the placeholder (smallest realistic scope: 1–2 days), or (b) remove C and D from `availableOverrideModes` until they're real (5 minutes, prevents user dead-ends).

### G-2 — Two parallel "view switchers" (MEDIUM)

**Where:**
- **Forge toggle** (`App.svelte:354-362`): button in `processing-canvas` swaps between `<NodeSidebar>` + `<PreviewCanvas>` ("Pipeline" / "Forge") and bare `<PreviewCanvas>` ("Guided" / "Wizard").
- **Mode switcher** (`AppShell.svelte:85-97`): header pills that swap between `ModeA / ModeB / ModeC / ModeD` shells.

**Conflict:** The forge toggle swaps **only within Mode B's processing step** (`App.svelte:331-383`). The mode switcher can take the user out of processing entirely (to A, C, D). The two systems are conceptually orthogonal (forge-toggle = "show sidebar refinement", mode-switch = "which shell"), but a user hitting "Refine" in the header while in forge mode ends up in Mode D's mock with no clear way back — the forge toggle is invisible there because it's hard-coded inside the Mode B processing branch.

**Fix:** Promote the forge toggle's concept ("Guided / Pipeline") into `AppShell` so it persists across mode overrides. Add a "refinement sidebar" toggle that is visible in both Mode B and Mode D processing contexts.

### G-3 — Gallery → existing-session navigation drops history (MEDIUM)

**Where:** `App.svelte:325-328` (`<ManifestReview onViewPreview={viewPreview}>`). `viewPreview()` at `App.svelte:100-107` calls `setModeOverride("d")` but **never rehydrates `sessionStore`** with the clicked session's data. Mode D then renders `$galleryStore[0]` (`ModeD.svelte:17-19`), not the session the user just clicked.

**Impact:** Clicking a session in the gallery silently lands on a different session's image (or no image if `galleryStore` is empty in Tauri dev). The `sessionStore` from the most-recent `initSession()` call leaks across gallery clicks.

**Fix:** `viewPreview(sessionId)` must (a) call `initSession(sessionId)` or `deserializeSession(...)` to rehydrate the pipeline graph, and (b) leave `setModeOverride("d")` only as a UI shell hint. Until then, hide the click affordance or surface a clear "Loading session..." state.

### G-4 — `previewParams` dual source of truth (MEDIUM)

**Where:**
- `App.svelte:77-84` owns `previewParams` as local `$state`.
- `App.svelte:206-208` `handleParamsChange` writes back to that local state.
- `WizardBottomSheet`, `ParameterSidebar`, and `PreviewCanvas` all receive `previewParams` as a prop.
- Meanwhile, `pipeline-store` holds `params` per `PipelineNode` (`pipeline-store.ts:21-29`), updated via `commitStage()` / `updateNodeParams()`.

**Conflict:** The stretch slider in `ParameterSidebar` mutates `previewParams.blackPoint / midtones / highlights`, but **does not call `commitStage()`** — so the pipeline graph's `nodes[N].params` for stretch is stale. A user adjusting the slider sees the live preview change, but the persisted session does not record the adjustment until something else triggers `commitStage`. Undo/redo will then revert to a state the user never saw.

**Fix:** Either (a) every slider change in `ParameterSidebar` / `WizardBottomSheet` calls `updateNodeParams(activeNodeId, ...)` and is committed-on-blur, or (b) make `previewParams` a derived view of `pipeline-store` (no local mirror). Option (b) is cleaner.

### G-5 — `backToLanding()` leaves pipeline state dangling (LOW)

**Where:** `App.svelte:194-200` clears the wizard's local state but never calls `initSession()` to reset `sessionStore`. The next time the user starts a fresh load, `initSession()` at line 190 *does* reset, but until they hit "Confirm" on the classification dialog the UI shows the **previous session's pipeline graph** (e.g. `activeNode` derived store at `pipeline-store.ts:215-217` still references the old `sessionStore`).

**Impact:** Between "back to landing" and "Confirm" on a fresh session, `NodeSidebar` / `ParameterSidebar` / `WizardBottomSheet` (if the user manages to reach them) would render against the previous session's params.

**Fix:** In `backToLanding()`, also call `initSession(undefined)` to reset the pipeline graph (or expose a `resetSession()` action that does this). The current `initSession()` overwrites via `createInitialSession()` which is exactly what we want.

### G-6 — File-extension filter is the only validation (LOW)

**Where:** `App.svelte:113` and `App.svelte:122` filter on file extension only. FITS/TIFF/DNG files with malformed headers will pass `analyzeFiles()` and only fail at `classificationFrames` mapping.

**Fix:** Surface `analyzeFiles()` failures (currently caught and stored as `analysisResult = null` at `App.svelte:138-141`) with a per-file error row in the file list. Right now a single bad FITS file results in a silent fall-through to `selectedFiles.map(...)` with the `guessFrameType()` heuristic.

### G-7 — `ModeA` overlay panel is fixed-width 480px regardless of viewport (LOW)

**Where:** `ModeA.svelte:48-54` sets `width: 480px` on the overlay. On a 1366×768 display with `ModeB` rail (320px) and workflow (360px), the overlay covers the canvas entirely. The `@media (max-width: 1024px)` breakpoint repositions to bottom, but 1025–1365px is the worst case.

**Fix:** Constrain overlay width as `min(480px, calc(100vw - 320px - 360px - 64px))` so it shrinks as the parent canvas shrinks.

### G-8 — GPU badge is decorative (LOW)

**Where:** `AppShell.svelte:99-105` shows "GPU: WebGPU / WebGL / Detecting…". It does not gate any feature (PreviewCanvas uses `gl-renderer.ts` regardless). Misleading.

**Fix:** Either remove the badge or wire it to `gpu.ts:probeGpu()` so it actually gates a path (e.g. "GPU mode disabled — fallback to CPU render"). The `gpu.ts` file at 15 lines is a candidate for either expansion or removal.

---

## 3. Cross-cutting observations

### State management

- `sessionStore` (Svelte writable) and local `$state` in `App.svelte` are **not unified**. `App.svelte` owns `selectedFiles / sessionData / classificationFrames / analysisResult` locally; `sessionStore` owns the pipeline graph. The boundary is implicit and fragile (G-5).
- `previewStore.ts` (29 lines) exists but is barely used. Mode D references `$galleryStore` instead of `previewStore` for "what to render".

### Component layering

```
AppShell (header + footer + canvas)
└── Mode{A,B,C,D} (3-col grid / overlay / tabs)
    └── ScreenCard (kicker + title + content + footer snippet)
        └── Domain components (ParameterSidebar, NodeSidebar, etc.)
```

This is a clean hierarchy. The `ScreenCard` `footer` snippet pattern (`App.svelte:284-295`) is well-used.

### Test coverage

- **Rust:** 213 tests passing (most recent: M6 close-out).
- **Frontend:** No unit tests. `npm run check` (svelte-check) only. No Playwright / Vitest setup. This is a known gap (Phase 9 audit noted it); M2/M3/M4 audits did not address it because the scope was milestone-specific.

### Styling

- Design tokens are tokenized (`var(--cobalt-accent)`, `var(--sp-md)`, etc.) — Phase 5 token sweep (PR #194) landed cleanly. No hard-coded color hexes found in the reviewed files.
- Mode C/D `pipelineStages` mock data shows non-tokenized status strings ("done", "active", "queued") — would need to map to the `NodeStatus` enum if real.

---

## 4. Recommendations (sequenced)

| # | Recommendation | Effort | Impact | Blocks |
|---|---|---|---|---|
| R-1 | Hide Mode C and D from mode-switcher until they ship real `PreviewCanvas` wiring (or wire Mode D to `PreviewCanvas` + `ParameterSidebar` first) | 1 day | HIGH | User dead-ends | **CLOSED via PR #219** (`layout-mode.ts` defensive path returns `["a","b"]`). Full Mode D wiring deferred. |
| R-2 | Make `sessionStore` the single source of truth for params; remove `previewParams` mirror in `App.svelte` | 1 day | HIGH | Undo/redo correctness |
| R-3 | `viewPreview(sessionId)` rehydrates `sessionStore` before `setModeOverride("d")` | 0.5 day | MEDIUM | Gallery navigation |
| R-4 | `backToLanding()` calls `initSession(undefined)` to reset graph | 0.25 day | MEDIUM | Fresh-session race |
| R-5 | Promote forge-toggle into `AppShell` (visible across Mode B and D) | 1 day | MEDIUM | UI consistency |
| R-6 | Surface per-file errors from `analyzeFiles()` in the file list | 0.5 day | LOW | Validation UX |
| R-7 | Constrain `ModeA` overlay width responsively | 0.25 day | LOW | Mid-viewport layout |
| R-8 | Either remove GPU badge or gate a path on it | 0.25 day | LOW | UI honesty |
| R-9 | Add frontend smoke test (Vitest + Svelte Testing Library) | 1–2 days | MEDIUM | Regression safety |

**Recommended next milestone scope:** R-2 + R-3 + R-4 as a single "UI workflow integrity" tranche (~2–3 days). R-1 closed via PR #219 (defensive path). See `docs/UI_WORKFLOW_AUDIT.md` (2026-09-05) for the implementation audit.

---

## 5. Files reviewed

- `src/App.svelte` (665 lines)
- `src/components/AppShell.svelte` (248 lines)
- `src/components/ModeA.svelte` (67 lines)
- `src/components/ModeB.svelte` (92 lines)
- `src/components/ModeC.svelte` (506 lines, mock content)
- `src/components/ModeD.svelte` (499 lines, mock content)
- `src/lib/layout-mode.ts` (124 lines)
- `src/lib/pipeline-store.ts` (456 lines)
- `src/lib/preview-store.ts` (29 lines)
- `src/lib/gpu.ts` (15 lines)
- `src/components/NodeSidebar.svelte`, `ParameterSidebar.svelte`, `WizardBottomSheet.svelte`, `PreviewCanvas.svelte`, `Gallery.svelte`, `ManifestReview.svelte` (referenced, not re-read this pass)

## 6. Not in scope

- Tauri IPC bridge wiring (`src/lib/ipc.ts`) — covered by M5 audit.
- Rust pipeline work — covered by M6 audit.
- Real model integration (RL deconv, SwinIR) — covered by M6-T5/T6 deferred items.
