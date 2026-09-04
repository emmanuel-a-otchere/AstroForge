# Phase 1.5 M2 — PreviewCanvas & Live Preview: Audit

**Status:** T1 / T4 / T6 / T7 closed; T5 / T8 documented as partial.
**Date:** 2026-09-04
**Companion PRs:** #208 (this audit), #209 (T8 reduced-res implementation)

## TL;DR

P1.5-M2 (PreviewCanvas & Live Preview) is mostly shipped. Four of six
issues (#145, #148, #150, #151) can be closed at the tracker with code
references. Two (#149 preview stats stability, #152 debounced reduced-res)
are partial — the architecture is in place but the specific spec behaviour
is not yet implemented.

This document is the audit trail. The fix for T8 ships in the
companion PR; T5 is deferred because the underlying denoise shader is
itself deferred to a later phase.

## Task-by-task evidence

### T1 — `PreviewCanvas` Svelte component (#145)
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/145
- Code:
  - `src/components/PreviewCanvas.svelte` — full implementation, 110+ lines
  - Mounted persistently in `src/App.svelte` (does not unmount across wizard ↔ forge transitions)
  - Public API: `params`, `renderMode`, `compareMode`, `imageData`, `floatData`, `sessionId`
- Verdict: ✅ shipped

### T4 — SCNR Green-be-Gone shader in GLSL (#148)
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/148
- Code:
  - `src/lib/shaders.ts` — `SCNR_SHADER` (full implementation, two methods: min(R,B) and average(R,B))
  - `src/lib/gl-renderer.ts:214` — wires `u_method` from `params.scnrMethod`
  - `src/lib/gl-renderer.ts:42` — default `scnrMethod = 0` (min)
- Verdict: ✅ shipped (no Rust parity test yet — the SCNR formula is GLSL-only; not part of the Rust MVP pipeline, so the Phase 9 MTF parity pattern doesn't apply)

### T6 — Real-pixel zoom / pan / refit on PreviewCanvas (#150)
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/150
- Code:
  - `src/components/PreviewCanvas.svelte:47-58` — `handleMouseDown/Move/Up` (panning)
  - `src/components/PreviewCanvas.svelte:60-69` — `handleWheel` (zoom, clamped to `[0.05, 50]`)
  - `src/components/PreviewCanvas.svelte:71-77` — `handleDoubleClick` (refit)
  - `src/lib/gl-renderer.ts:167-175` — `refit()` calculates fit zoom from canvas / image aspect ratio
  - `src/lib/gl-renderer.ts:158-165` — `resize()` keeps canvas in sync with container
- Verdict: ✅ shipped

### T7 — Hold to Compare and side-by-side original vs current (#151)
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/151
- Code:
  - `src/components/PreviewCanvas.svelte:79-83` — `handleCompareDown/Up` swaps the active program to `identity` while the user holds
  - `src/components/PreviewCanvas.svelte:112-113` — reactive toggle: `renderer.render(showOriginal ? "identity" : renderMode)`
  - `src/components/PreviewCanvas.svelte:15` — `compareMode` prop exposed
  - `src/components/BeforeAfterSlider.svelte` — side-by-side component also present
- Verdict: ✅ shipped

### T5 — Preview statistics stability (#149) — ⚠ partial
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/149
- Status:
  - `src/lib/pipeline-store.ts:139` describes the noise-reduction stage as
    "preview-stable" in its description field
  - **No enforcement**: the spec calls for "denoise/sharpen shaders must not alter display stretch statistics" — that means the preview's display stretch operates on the *original* linear statistics, not the post-denoise ones. This invariant is documented as a stage property but not enforced in code.
- Why deferred: the denoise / sharpen shaders themselves are deferred (P1.5-M6-T5, T6 — both `pending`). There is no shader to constrain. Enforcing the invariant requires the shaders to exist first.
- Verdict: ⚠ partial — defer until M6-T5/T6 land.

### T8 — Debounced full-resolution render (#152) — ⚠ partial, fix in PR #209
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/152
- Status:
  - `src/lib/gl-renderer.ts:237-243` — `requestDebouncedRender(mode, delayMs = 150)` exists
  - `src/components/PreviewCanvas.svelte:62-65` (wheel) and `:48-50` (pan) call `renderer.render()` **immediately**, not through the debounced path
  - The current debouncer is only used for parameter changes, not for pan/zoom gestures
- Spec gap: the spec asks for "preview renders at reduced res during slider drag, full res on rest" — there is no reduced-resolution path during drag at all.
- Fix: companion PR #209 lowers the GL viewport to 50% during pan/zoom gestures and restores full res 150 ms after the gesture stops.

## Test snapshot (2026-09-04)

```
cargo test --workspace      → 195 passed (no new tests in this audit PR)
npm run check                → 0 errors, 21 pre-existing a11y warnings
```

T8's fix in PR #209 will add a Vitest smoke test for the debouncer
timing logic if a JS test runner is available by then; otherwise the
behaviour is verified manually.

## Phase 1.5 M2 close-out

| Issue | Title | Status | Evidence |
|---|---|---|---|
| #145 | PreviewCanvas component | ✅ shipped | PreviewCanvas.svelte, App.svelte |
| #148 | SCNR shader | ✅ shipped | shaders.ts SCNR_SHADER |
| #149 | Preview stats stability | ⚠ partial | deferred to M6-T5/T6 |
| #150 | Zoom / pan / refit | ✅ shipped | PreviewCanvas.svelte (47-77), gl-renderer.ts (167-175) |
| #151 | Hold to Compare | ✅ shipped | PreviewCanvas.svelte (79-83, 112-113), BeforeAfterSlider.svelte |
| #152 | Debounced reduced-res | ⚠ fix in #209 | gl-renderer.ts debounce + PR #209 reduced-res path |

Issues to close at the tracker: #145, #148, #150, #151.
