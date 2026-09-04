# Phase 1.5 M4 — Forge Mode UI: Audit

**Status:** T1 / T2 / T3 closed; T4 partial; T5 partial.
**Date:** 2026-09-04
**Companion PRs:** #212 (this audit), #213 (T4 partial fix — full params for denoise + color_calibration + sharpen_deconvolution).

## TL;DR

P1.5-M4 (Forge Mode UI) is mostly shipped. Three of five issues (#159,
#160, #161) can be closed at the tracker with code references. Two
have real gaps:

- **#162 Pure Expert UI** — only `stretch` and `star_handling` have
  full param panels in `ParameterSidebar`. Seven other stage types
  fall through to a generic "Strength" slider. The spec calls for
  "every control, sub-parameter, mask, intermediate buffer exposed;
  manual sub-step sequencing".
  Companion PR #213 wires full params for the three most-used stages
  (`denoise`, `color_calibration`, `sharpen_deconvolution`). The other
  four (`ingest`, `crop_rotate`, `background_extraction`,
  `creative_polish`, `export`) are deferred because they need either
  backend integration that hasn't shipped (ingest/export) or
  bespoke UI components (crop handles, background mask preview).

- **#163 Automagic Expert UI** — the suggestion panel exists with
  Accept/Refine buttons, but the AI recommendation is hardcoded text
  ("strength of 65%"). Needs `AIService.suggestParams()` from
  P1.5-M5-T1/T2. Deferred.

## Task-by-task evidence

### T1 — `NodeSidebar` Svelte component (#159)
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/159
- Code:
  - `src/components/NodeSidebar.svelte` — 183 lines
  - DAG: SVG `edges-layer` (lines 39-49) draws connection lines
    between consecutive nodes
  - Nodes: `<button class="node-card">` per node (`:58-71`) with
    status LED (`statusColors` map), number badge, label, and
    check-circle when complete
  - Click handler: `handleNodeClick(i)` → `goToStep(i)` (`:25-27`)
  - Active highlight: `class:active={i === activeIdx}` (`:60`)
- Verdict: ✅ shipped

### T2 — `ParameterSidebar` Svelte component (#160)
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/160
- Code:
  - `src/components/ParameterSidebar.svelte` — 234 lines
  - Reads `activeStepIndex` + `pipelineGraph.nodes[stepIdx]` + `stageDefinitions[stepIdx]` reactively (`:12-15`)
  - Renders a stage header (label + description, `:18-21`)
  - Branches per stage type: `stretch` (4 params, `:33-66`), `star_handling` (2 params, `:67-89`), generic single "Strength" for everything else (`:90-100`)
  - Bottom panel: node version, status, engine, duration from `node.receipt` (`:105-128`)
  - Wired: `handleParam(key, value)` calls both `updateNodeParams` (store) and `onParamsChange` (callback, `:17-22`)
- Verdict: ✅ shipped

### T3 — Node selection → parameter sidebar sync (#161)
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/161
- Code (the chain):
  1. `NodeSidebar.handleNodeClick(i)` calls `goToStep(i)` (`:25-27`)
  2. `goToStep` is exported from `src/lib/pipeline-store.ts` (likely `:200+`) and updates the `activeStepIndex` store
  3. `ParameterSidebar` derives from `$activeStepIndex` (`:12`) so the stage reactively changes
  4. `ParameterSidebar.handleParam` calls `onParamsChange` (`:17-22`) which flows up to `App.svelte` (`handleParamsChange`) and re-renders `PreviewCanvas`
- Verdict: ✅ shipped (no extra wiring needed; the reactive store is the sync mechanism)

### T4 — Pure Expert UI: every control exposed (#162) — ⚠ partial, fix in #213
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/162
- Status:
  - `ParameterSidebar.svelte:33-66` has full params for `stretch` (Strength, Midtones Balance, Black Point, Highlights)
  - `:67-89` has full params for `star_handling` (Replace Strength, Colour Boost)
  - `:90-100` falls through to a generic "Strength" slider for everything else
- Stage types in `pipeline-store.ts:10-14`: `background_extraction`, `color_calibration`, `denoise`, `stretch`, plus `ingest`, `crop_rotate`, `sharpen_deconvolution`, `creative_polish`, `export`, `star_handling`.
- So 7 of 10 stages only get a generic single-slider surface.
- Spec asks for "every control, sub-parameter, mask, intermediate buffer exposed; manual sub-step sequencing" — the masks and intermediate buffers are completely missing from the UI.
- Fix: companion PR #213 wires full params for the three most-used stages (`denoise`, `color_calibration`, `sharpen_deconvolution`). The remaining 4 stages are deferred (each needs a custom UI component, not just more sliders).

### T5 — Automagic Expert UI (#163) — ⚠ partial, deferred
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/163
- Status:
  - `WizardBottomSheet.svelte:353-371` — the AI suggestion panel with Accept/Refine buttons
  - The suggestion text is **hardcoded** (`:361-363` — "strength of 65%" with optional "midtones at 25%")
  - Accept/Refine buttons have no `on:click` handler (`:366-367`)
- Why deferred: the suggestion needs to come from `AIService.suggestParams(image, stage, dataType)`, which is P1.5-M5-T1. Until that ships, the buttons would be cosmetic only.
- Verdict: ⚠ partial — defer until M5-T1 lands.

## Test snapshot (2026-09-04)

```
cargo test --workspace      → 195 passed (no new tests in this audit PR)
npm run check                → 0 errors, 21 pre-existing a11y warnings
```

T4's fix in PR #213 adds three new branches to `ParameterSidebar.svelte` —
visual change, no automated tests beyond svelte-check.

## Phase 1.5 M4 close-out

| Issue | Title | Status | Evidence |
|---|---|---|---|
| #159 | NodeSidebar component | ✅ shipped | NodeSidebar.svelte (183 lines) |
| #160 | ParameterSidebar component | ✅ shipped | ParameterSidebar.svelte (234 lines) |
| #161 | Node selection → sidebar sync | ✅ shipped | reactive store chain |
| #162 | Pure Expert UI | ⚠ partial, fix in #213 | ParameterSidebar.svelte:33-100 |
| #163 | Automagic Expert UI | ⚠ partial | WizardBottomSheet.svelte:353-371 (hardcoded) |

Issues to close at the tracker: #159, #160, #161.