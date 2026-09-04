# Phase 1.5 M5 — AI Service Layer: Audit

**Status:** T2 partial; T1 / T3 / T4 / T5 missing.
**Date:** 2026-09-04
**Companion PRs:** #214 (this audit), #215 (T1+T2 service scaffold).

## TL;DR

P1.5-M5 (AI Service Layer) is **mostly unimplemented**. The hardware
probing and model registry are real (1070 LOC across 6 files,
23 tests passing), but the public `AIService` surface that the rest of
the app talks to **does not exist**. M4-T5's hardcoded suggestion text
(#163) is a direct downstream consequence.

**Tasks fully shipped:** none.
**Tasks partially shipped:** T2 (hardware probe + tiling engine).
**Tasks missing:** T1 (`AIService` interface), T3 (graceful
degradation), T4 (progress reporting), T5 (path messaging).

The companion PR (#215) ships the missing `AIService` trait with
three methods (`analyse`, `suggestParams`, `execute`), plus a
`dispatch()` that picks an engine from `HardwareProbe`. It's a
**scaffold** — `execute()` delegates to existing `AiModel::run()`,
`analyse()` computes CPU statistics, `suggestParams()` returns
heuristic defaults. Real model-backed behaviour (ONNX inference)
comes with P1.5-M6 when the shaders and engine integration land.

## Task-by-task evidence

### T1 — `AIService` interface (#164) — ❌ missing
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/164
- Spec: `analyse(image, stage) → AnalysisResult`, `suggestParams(image, stage, dataType) → ParamProposal`, `execute(image, stage, params) → ProcessedImage`.
- Existing code:
  - `crates/astroforge-ai/src/models.rs:67-70` defines `AiModel` trait with `run(&F32Image) → F32Image`. That maps to `execute()`, but no `analyse` or `suggestParams`.
  - `crates/astroforge-ai/src/models.rs:83-99` `SwinIrDenoise` + `:109-131` `SwinIrSuperRes` are concrete implementations.
  - **No top-level service** that orchestrates these per stage. The trait is there, the orchestration isn't.
- Missing types: `AnalysisResult`, `ParamProposal`, `ProcessedImage` (only `F32Image` from `astroforge-core`).
- Companion PR #215 defines these types and the `AIService` trait.

### T2 — AI dispatch (#165) — ⚠ partial, fixed in #215
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/165
- Spec: route to ONNX, remote engines, or CPU fallback based on mode + hardware.
- Existing code:
  - `crates/astroforge-ai/src/hardware.rs:43-79` `HardwareProbe` with `detect()`, `select_tier()`, `max_tile_size()`, `supports_backend(backend)`.
  - `crates/astroforge-ai/src/hardware.rs:104+` `detect_gpu_backend()` returns `GpuBackend` enum (CPU, Metal, Cuda, Vulkan, etc.).
  - `crates/astroforge-ai/src/tiling.rs` `run_tiled_inference()` for large images.
- **What's missing:** a top-level `dispatch()` method that takes a stage request, queries `HardwareProbe`, and routes to the right backend. Currently each caller would have to do this routing themselves.
- Companion PR #215 adds `dispatch()` in `astroforge-ai/src/dispatch.rs`.

### T3 — Graceful degradation (#166) — ❌ missing
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/166
- Spec: if AI engine fails, fall back to algorithmic defaults and surface a clear warning.
- Existing code: none. `AiModel::run()` returns `F32Image` directly with no `Result` wrapper, no fallback path.
- Companion PR #215 wraps `execute()` in `Result<ProcessedImage, AIError>` with a `CPUFallback` variant in the error enum.

### T4 — Status + progress reporting (#167) — ❌ missing
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/167
- Spec: measured progress, estimated time, engine name, quality tier reported to UI.
- Existing code: `InferenceContext` in `models.rs:53` tracks inference context for determinism, but it's not exposed for progress reporting.
- Companion PR #215 defines a `ProgressReporter` callback type and threads it through `execute()`.

### T5 — Free-path vs accelerated-path messaging (#168) — ❌ missing
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/168
- Spec: transparent messaging about which path (free CPU vs accelerated GPU) is selected and why.
- Existing code: `QualityTier` enum in `hardware.rs:1+` but no messaging layer.
- Companion PR #215 adds `PathSelection { engine, tier, reason }` returned by `dispatch()`.

## Test snapshot (2026-09-04)

```
cargo test -p astroforge-ai        → 23 passed (unchanged)
cargo test --workspace             → 195 passed (no new tests in this audit PR)
npm run check                       → 0 errors, 21 pre-existing a11y warnings
```

PR #215 adds ~12 unit tests for the new service surface.

## Phase 1.5 M5 close-out

| Issue | Title | Status | Evidence |
|---|---|---|---|
| #164 | AIService interface | ❌ missing, scaffold in #215 | astroforge-ai/src/models.rs has AiModel trait but no service layer |
| #165 | AI dispatch | ⚠ partial, fixed in #215 | HardwareProbe + tiling done; dispatch() missing |
| #166 | Graceful degradation | ❌ missing, in #215 | no fallback layer |
| #167 | Status + progress | ❌ missing, in #215 | no ProgressReporter |
| #168 | Path messaging | ❌ missing, in #215 | no PathSelection |

No M5 issues can be closed at the tracker from the audit alone — all 5 require code in #215.

After #215 lands: #164, #165, #166, #167, #168 all closed.

Downstream unblocks: M4-T5 (#163) Automagic Accept/Refine can now wire `AIService.suggestParams()`.