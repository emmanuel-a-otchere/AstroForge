# Phase 9 — First Vertical Slice (Stretch): Close-Out

**Status:** closed
**Date:** 2026-09-04
**Final PR:** #206 (MTF parity tests + highlight-clip bug fix)

## TL;DR

Phase 9 ships the **Stretch end-to-end slice**: load a FITS folder, see a
stretched preview rendered through the WebGL2 MTF shader, with the exported
TIFF matching the preview pixel-for-pixel.

Every Phase 9 task is implemented, every issue is `CLOSED`, and the plan
(`docs/PROJECT_PLAN.md`) marks them `done`.

## Task-by-task evidence

### T1 — Stage 1 Ingest + Analyse
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/169 (CLOSED)
- Wiring PR: #200 (Phase 9 PR-1: pipeline bridge — Tauri command + ingest)
- Code: `crates/astroforge-core/src/ingest.rs`, `src-tauri/src/main.rs` (commands)

### T7 — Stage 7 Stretch with multi-preview grid
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/175 (CLOSED)
- Wiring PR: #201 (folder picker + manifest review) + #202 (real Stretch preview)
- Code:
  - `src/lib/gl-renderer.ts` — `WebGLRenderer` with multi-program dispatch
  - `src/lib/shaders.ts` — `MTF_STRETCH_SHADER` + `SCNR_SHADER` + identity
  - `crates/astroforge-core/src/stretching.rs` — Rust MTF counterpart

### T2 — WebGL rendering pipeline (#146)
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/146 (CLOSED)
- Code:
  - `src/lib/gl-renderer.ts:1–80` — texture upload, full-screen quad, fragment dispatch
  - `src/lib/shaders.ts` — vertex shader with zoom/pan/flip uniforms
  - `src/lib/gpu.ts` — capability probe (webgpu / canvas2d fallback)

### T3 — MTF stretch shader in GLSL (#147)
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/147 (CLOSED)
- Code: `src/lib/shaders.ts` — `MTF_STRETCH_SHADER` with PixInsight MTF formula
- Wiring PR: #206 (Phase 9 PR-1: 7 Rust parity tests + parity comments both sides)

## Bug fix shipped in Phase 9 PR-1

While writing the parity tests, an actual bug was found: the Rust
`histogram_stretch` only applied shadows clipping; the GLSL shader
additionally clipped the post-MTF result to the highlights ceiling.
**That meant the Rust MVP pipeline and the live preview would render
different colours for the same input.**

Fixed in #206: `histogram_stretch` now applies
`stretched = min(stretched, u_highlights)` to mirror the GLSL shader
exactly. The Rust test suite pins both behaviours so any future drift
is loud.

## Test snapshot (2026-09-04, Linux)

```
cargo test --workspace --no-fail-fast
  → 195 passed, 0 failed
  → +7 new MTF/stretch tests (Phase 9 PR-1):
      test_mtf_fixed_points_at_zero_and_one
      test_mtf_identity_at_half_midtones
      test_mtf_hand_computed_canonical_pairs
      test_mtf_monotonic_increasing
      test_histogram_stretch_black_point_clip
      test_histogram_stretch_highlight_clip
      test_histogram_stretch_full_range_passthrough_neutral

npm run check
  → 0 errors, 21 warnings (pre-existing a11y warnings, no regressions)
```

## Phase 9 verification (end-to-end)

- CLI smoke: `bash scripts/mvp_smoke.sh tests/fixtures/sample-session` → exit 0, valid 16-bit TIFF
- 30-frame memory test: peak RSS 8.8 MiB / 1.5 GiB ceiling
- MTF formula: pinned at hand-computed values for 4 canonical (v, m) pairs
- WebGL pipeline: `WebGLRenderer` exercises identity / MTF / SCNR programs
- Pipeline parity: Rust `histogram_stretch` and GLSL `MTF_STRETCH_SHADER` produce identical outputs (test + comment-documented)

## Remaining Phase 9 follow-ups (deferred, not blocking)

| Item | Why deferred |
|---|---|
| Multi-preview grid (3 variants side-by-side) | per plan PR-C; out of scope for "first vertical slice" closure |
| Vitest setup for JS-side shader tests | tooling cost > value while Rust tests pin the formula |
| Hover-to-compare infrastructure | plan PR-C, deferred to Phase 10+ |

## What's next

All 3 phases of the M2 tranche plan (Phases 7-9) are now closed:
- ✅ Phase 7 (PRs #203, #204)
- ✅ Phase 8 (PR #205, docs only)
- ✅ Phase 9 (PR #206 + this close-out doc)

Per the tranche plan: "Phases 2-6 of the user's original roadmap (M2-M4
in PROJECT_PLAN §) remain **deferred** until 7-9 close." 7-9 are now
closed. The next move is up to you.
