# CR-06 P5.1 Implementation Plan — Real ONNX Inference, Tile Execution, Mask-Aware Apply

**Date:** 2026-09-12
**Source CR:** [CR-06 — AI Enhancement Studio & Intelligent Image Revamp](../../CR-06-AI-ENHANCEMENT-STUDIO.md) (Status: P1–P7 + P5.1 landed; spec bump 1.2.0 → 1.3.0)
**Strategy:** Single PR replacing the P4 passthrough dispatcher body with a real ONNX Runtime 1.28 path, threading the §37 segmentation-leakage gate to compare real mask deltas, and persisting the produced pixels as TIFF artifacts + quality reports.

## Why this slice now

The M9 audit's [§ Cross-cutting observations](../../M9_AUDIT.md) flagged five partial criteria in CR-06, three of which (E4 tiled inference, U2 preview rendering, RM3/4 enforcement at apply time) close the moment the apply round produces a real result image distinct from the source. P5.1 is that swap.

The remaining two (U3 before/after, U4 split comparison) are CR-07 Zone B canvas surfaces that depend on real pixels. P5.1 unblocks CR-07 because the canvas has something to render.

## What landed

### New modules

- `crates/astroforge-ai/src/inference.rs` (~700 lines) — `ort` 2.0.0-rc.13 wrapper. `BuiltinModel` registry with five classical-kernel ONNX graphs (`builtin-blur-blend`, `builtin-sharpen-blend`, `builtin-upscale-2x`, `builtin-hotpixel`, `builtin-masked-fill`) embedded via `include_bytes!` and pinned by real SHA-256 digests verified at session-build time. `OnnxEngine::open_builtin` / `open_catalog` / `kind` / `run` form the runtime surface. Multichannel images run per-channel; the masked-fill graph takes a second tensor input.

- `scripts/generate_builtin_models.py` — Python generator using `onnx` 1.22.0. Produces the five `.onnx` fixtures with pinned opset 17 / IR 9. Re-runnable; the embedded `BUILTIN_MODELS` table's digests are emitted by the script and must be updated in the same commit when a graph changes.

### Dispatcher swap (`crates/astroforge-ai/src/operations/mod.rs`)

- New `OperationModelBinding` table wires every registered operation onto a builtin graph (and an optional catalog model that supersedes it when `<models_dir>/<name>.onnx` exists).
- `dispatch_operation` now takes `(source, mask, probe, models_dir)` and returns a `DispatchResult` (outcome + pixels + model identity + tile config + duration). The `OperationOutcome` metadata contract is unchanged.
- Tile inference via the existing `tiling::run_tiled_inference` (probe-clamped tile size, cosine-blended overlap). The tiler closure is now `Fn(&F32Image, &Tile) -> F32Image` so per-tile mask subregions can be extracted.
- The 2x super-resolution graph runs untiled (the tiler assumes input-sized outputs).
- Mask-aware composite: `mask * inferred + (1 - mask) * source` (skipped for the masked-fill graph, which already honours the mask internally).

### §37 segmentation-leakage gate (`crates/astroforge-core/src/quality_gates/`)

- `segmentation_leakage(source, result, mask, thresholds)` — was a no-op in P6. Now compares inside / outside mean absolute pixel deltas with a configurable ratio + absolute floor. New thresholds `leak_abs_floor`, `leak_warn_ratio`, `leak_fail_ratio` (sensible defaults; serde-roundtrip stable).
- `report::run` signature gains an `Option<&Mask>` parameter; all callers (including the new apply round and the standalone `run_ai_quality_report`) thread the mask through.

### Persistence

- Migration v9: `ai_quality_reports` table keyed by `result_image_version_id`. `record_ai_quality_report` + `latest_ai_quality_report_for_version`. The apply round writes the report with the same `verdict` / `report_json` it surfaces in the response.
- The apply round now persists the produced pixels as a 16-bit TIFF artifact under `~/.astroforge/applied/<project>/<result_version_id>.tif` (best-effort — the dispatcher's in-memory pixels are authoritative when the filesystem is read-only).

### Apply round (`src-tauri/src/commands_ai_enhancement.rs`)

- `ApplyAiOperationRequest` gains `mask_id: Option<String>`; `ApplyAiOperationResponse` gains `quality_verdict: String` + `quality_report: serde_json::Value`.
- `enhancement_apply_operation` reads the source pixels (on-disk TIFF when one exists, synthetic gradient stub otherwise so headless / test paths still exercise the dispatcher end-to-end), runs real inference, composites with the mask, persists the artifact + provenance + quality report.
- `AiOperation` row gains real `model_id` / `model_version` / `model_hash` / `runtime` / `backend` / `tile_configuration` / `resource_metrics` provenance.

## Out of scope (P5.2 / CR-07 follow-ups)

- **GPU execution providers** — `ort` exposes CUDA / DirectML / Metal via compile-time features; P5.1 ships CPU only. P5.2 enables per-platform providers on per-platform CI runners.
- **Catalog-model digest pinning** — builtins are pinned by real SHA-256 today. Catalog artifacts (`swinir-denoise-astro`, `swinir-sr-astro-2x`, etc.) ship with placeholder hashes pending DP#4 license verification. Until then, the dispatcher skips digest checks for catalog artifacts; the actual hash is recorded in the provenance row.
- **OnnxEngine session cache** — every apply round builds one session. The build is cheap but a key-by-model-id cache would yield 5–10x speed-up. Measure first; ship only if the profile shows the cost matters.
- **CR-07 Zone B canvas + before/after + split comparison** — depends on P5.1's real pixels.

## Verification

- `cargo test -p astroforge-ai` — 92 tests passing (engine + dispatcher); real ONNX fixtures verified end-to-end through the runtime.
- `cargo test -p astroforge-core` — 642 tests passing (gate additions, schema migration, persistence round-trip).
- `cargo build --bin astroforge` — clean.
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session` — clean (the apply round is not on the smoke script's path today; verified separately via the dispatcher tests + a manual round-trip through the artifact code path).
- `cargo fmt --all -- --check` — pending final pass.

## Forward-look references

- `docs/M9_AUDIT.md` § Cross-cutting observations (the M9 forward-look this slice closes).
- `docs/PROJECT_PLAN.md` — refreshed; P5.1 noted as the active slice.
- `CHANGELOG.md` `[Unreleased]` — full P5.1 entry.

## File map (new + edited)

New:

- `crates/astroforge-ai/src/inference.rs`
- `crates/astroforge-ai/models/builtin/builtin-{blur-blend,sharpen-blend,upscale-2x,hotpixel,masked-fill}.onnx`
- `scripts/generate_builtin_models.py`
- `docs/plans/2026-09-12-cr06-p5-1-real-onnx/PLAN.md` (this file)

Edited:

- `crates/astroforge-ai/Cargo.toml` (ort + tls-rustls)
- `crates/astroforge-ai/src/lib.rs` (re-export OnnxEngine, BUILTIN_MODELS, DispatchResult, OperationModelBinding)
- `crates/astroforge-ai/src/operations/mod.rs` (model_binding table, real dispatch path, new error variants)
- `crates/astroforge-ai/src/tiling.rs` (closure signature gains `&Tile`)
- `crates/astroforge-core/src/quality_gates/mod.rs` (leak thresholds + defaults)
- `crates/astroforge-core/src/quality_gates/gates.rs` (segmentation_leakage with mask)
- `crates/astroforge-core/src/quality_gates/report.rs` (mask parameter on run)
- `crates/astroforge-core/src/domain.rs` (AiQualityReport struct)
- `crates/astroforge-core/src/domain_store.rs` (migration v9 + DAO methods)
- `crates/astroforge-core/src/project.rs` (schema_version assertion bump)
- `crates/astroforge-core/tests/ai_enhancement.rs` (signature migration)
- `crates/astroforge-core/tests/dod_enhancement.rs` (signature migration)
- `src-tauri/src/commands_ai_enhancement.rs` (real apply round)
- `docs/CR-06-AI-ENHANCEMENT-STUDIO.md` (status header: P5.1 landed; spec bump target 1.3.0)
- `docs/M9_AUDIT.md` (P5.1 forward-look → closed)
- `docs/PROJECT_PLAN.md` (P5.1 noted as active slice)
- `docs/specs/SPEC_INDEX.md` (P5.1 in-flight block; bump target 1.3.0)
- `CHANGELOG.md` ([Unreleased] entry)
