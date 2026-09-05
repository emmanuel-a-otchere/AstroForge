# M7 Audit — Non-destructive Editing & History

## Scope

P1.5-M7 is the "non-destructive editing & history" milestone, covering the user-facing promise that **no operation in AstroForge is destructive** — every change is reversible, re-runnable, and exportable without ending the session.

**5 tasks (issues #179-#183):**

| # | Task | Issue |
|---|---|---|
| T1 | Versioned artefact store: each stage commit stores params + mask separately from pixel data | [#179](https://github.com/emmanuel-a-otchere/AstroForge/issues/179) |
| T2 | "Re-apply from here": re-running an earlier stage re-executes it and all downstream stages with current params | [#180](https://github.com/emmanuel-a-otchere/AstroForge/issues/180) |
| T3 | Multi-save: export multiple formats/versions without terminating session | [#181](https://github.com/emmanuel-a-otchere/AstroForge/issues/181) |
| T4 | Explicit warning when re-running an already-applied AI or irreversible-looking step | [#182](https://github.com/emmanuel-a-otchere/AstroForge/issues/182) |
| T5 | Exact reversibility for crop, stretch, and star-replace (restore exact pre-operation state) | [#183](https://github.com/emmanuel-a-otchere/AstroForge/issues/183) |

## Findings

| # | Task | Status | Evidence | Severity |
|---|---|---|---|---|
| T1 | Versioned artefact store | **partial** | `pipeline-store.ts:21-29` `PipelineNode` carries `version`, `params`, `receipt` per node. SessionStore persists to rusqlite via `crates/astroforge-core/src/session.rs` + IPC `session_record_stage`. **But**: pixel data is not stored separately — the version bump tracks metadata only, not the actual F32Image at each step. Re-running a stage cannot restore the previous pixel buffer; it can only re-execute and produce a new one. | MEDIUM |
| T2 | Re-apply from here | **missing** | `HistoryEntry.action` enum at `pipeline-store.ts:61` lists `"reapply"` as a typed action, but no `reapply()` function exists. `goToStep()` at line 316 lets the user navigate to an earlier step but does not re-execute. `commitStage()` always advances forward. | HIGH |
| T3 | Multi-save | **missing** | `crates/astroforge-core/src/export.rs` implements `export_tiff_16bit()` only (single format). `PipelineConfig` carries `export_path: Option<PathBuf>` (single path). `PipelineResult.exported_to: Option<PathBuf>` (single result). No batch export, no FITS/JPEG/PNG variants, no starless-only or stars-only export. | HIGH |
| T4 | Irreversible-step warning | **missing** | No warning anywhere in the codebase for re-running an already-applied AI step. `goToStep()` lets the user click any node and re-commit without confirmation. The ReceiptsPanel shows past runs but does not gate future ones. | MEDIUM |
| T5 | Exact reversibility | **partial** | `undo()` at `pipeline-store.ts:354-383` walks `historyPointer` and restores `params + status + receipt` from `HistoryEntry`. **But** at line 361: `params: prevVersion <= 0 ? PIPELINE_STAGES.find(...)?.defaultParams ?? {} : n.params` — **when undoing back to version 0, the node's params are replaced with defaults**, losing any custom params the user set on the very first version. Also: undo doesn't restore pixel data — only metadata. So "undo" is metadata-only and destructive on first-version rollback. | MEDIUM |

### Summary

- **2 of 5 tasks missing entirely** (T2 re-apply, T3 multi-save, T4 warning)
- **2 of 5 partial** (T1 missing pixel-data persistence, T5 destructive on first-version undo)
- **1 of 5 shipped** (none fully clean — even the partial ones have gaps)

The strongest piece is the **history + undo/redo + receipt persistence chain** that landed in M1 (#137-#144): `commitStage()` advances `historyPointer`, `undo()` restores, receipts persist to `stage_runs` via the `session_record_stage` IPC. The weakest piece is the **pixel-data side**: AstroForge currently has no per-stage pixel artefact cache, so any "re-run" or "restore previous state" is metadata-only.

## Cross-cutting observations

### Pixel data vs metadata split

`PipelineNode` carries `params + receipt` (metadata) but no reference to a stored pixel buffer. The Rust `mvp_pipeline::run_pipeline` runs the entire pipeline end-to-end and returns one `PipelineResult` — there's no "stage N's output image" snapshot.

For M7 to land cleanly, we need either:
- **Option A**: Snapshot the F32Image after each stage to `session_stage_artifacts` table (parallel to `stage_runs`). Heavy storage (TIFF/FITS per stage per session).
- **Option B**: Re-run from the chosen stage on demand (re-execute downstream stages). Cheaper storage, slower UX.
- **Option C**: Hybrid — keep original (pre-pipeline) frame set always; re-run from any stage by feeding the original through stages N..end with current params.

**Recommendation: B for T2 (re-apply), A for T5 (exact reversibility)**. B is sufficient because re-apply is a user-initiated, deliberate action; A is needed for T5 because users may want to undo without paying the re-execution cost.

### Receipt system already robust

`StageReceipt { stageId, timestamp, durationMs, parameters, warnings, metrics, engine?, success }` — exact shape the audit needs. Persisted via `session_record_stage` IPC, surfaced by `ReceiptsPanel.svelte`. T1 partially delivered via this chain; missing piece is just the pixel-data half.

### Export scaffolding present, single-format only

`PipelineConfig.export_path` + `PipelineResult.exported_to` + `crates/astroforge-core/src/export.rs::export_tiff_16bit` give us a working single-format export. T3 needs:
- FITS 32-bit export (for archival/reprocessing)
- JPEG / PNG (for sharing)
- Starless / stars-only / both variants
- Multiple outputs per session (write to a directory, not a single file)

### Warnings are surfaced, not gated

`StageReceipt.warnings: string[]` carries warning strings, and `ReceiptsPanel` renders them. But there's no **proactive gate** for destructive operations — the user is expected to notice warnings in the past receipts. T4 wants a confirmation modal/dialog when re-running an AI or irreversible step.

## Recommended PR scope for M7 implementation

Given the size (5 tasks, 2 missing entirely), I propose two PRs:

### PR-A — Audit only (this PR)
- This document
- PROJECT_PLAN.md updates: T1→partial, T2→pending, T3→pending, T4→pending, T5→partial
- No code changes

### PR-B — Implementation
Three sub-deliverables, bundled or split:

1. **Re-apply from here (T2)** — `reapply(stageId)` action in `pipeline-store.ts` that re-executes the chosen stage and all downstream stages with current params. Triggers `commitStage` with action `"reapply"` (already in the type union). Hooks into WizardBottomSheet's "Re-run from here" button + NodeSidebar's context menu.
2. **Pixel artefact snapshot (T1 complete)** — new `stage_artifacts` table in `db.rs` schema; `PipelineNode` gains `artifactPath: string | null`; `commitStage()` writes a PNG snapshot (cheap) on success; `undo()` to a previous version restores the artifact path.
3. **Destructive-step warning (T4)** — gate `commitStage()` for stages marked `requiresConfirmation: true` (crop, sharpen_deconvolution, denoise — anything with side effects beyond param mutation); prompt user with "Re-run will discard current params. Continue?"
4. **Multi-save (T3)** — extend `export.rs` with `export_fits_32bit`, `export_jpeg`, `export_png`; add `MultiExportConfig` carrying an array of `(format, suffix)` tuples; emit all formats to a directory.
5. **Exact reversibility fix (T5)** — remove the `prevVersion <= 0 ? defaultParams : n.params` fallback in `undo()`; instead, look up the prior `HistoryEntry` and restore its params verbatim.

**Estimated effort**: 4–6 days. Most of the shape (T1 partial, T5 partial, T2 type stub) is already in place; the work is mostly the pixel snapshot table + the multi-format export.

## Open questions (resolved 2026-09-05)

- **Q-1**: Per-stage pixel snapshot — **PNG only** (cheap, displayable thumbnail; FITS deferred to v2)
- **Q-2**: Re-apply trigger — **explicit 'Re-run from here' button** (no auto-on-commit)
- **Q-3**: Which stages are "irreversible-looking"? — **gate `crop_rotate`, `sharpen_deconvolution`, `denoise`, `background_extraction`, `color_calibration`**. Leave `stretch`, `star_handling`, `creative_polish`, `ingest`, `crop_rotate` is gated but not strictly destructive — included to surface the intent before flipping pixels.

## Files reviewed

- `src/lib/pipeline-store.ts` (456 lines — store, history, undo/redo, receipt shape)
- `src/lib/session.ts` (253 lines — receipt persistence IPC)
- `src/components/ReceiptsPanel.svelte` (288 lines — read-only receipt log)
- `src/components/WizardBottomSheet.svelte` (868 lines — undo/redo buttons, Process button)
- `crates/astroforge-core/src/session.rs` (SessionStore, stage_runs schema, IPC commands)
- `crates/astroforge-core/src/db.rs` (SESSION_SCHEMA_SQL with stage_runs + checkpoints tables)
- `crates/astroforge-core/src/export.rs` (export_tiff_16bit only)
- `crates/astroforge-core/src/mvp_pipeline.rs` (PipelineConfig.export_path, PipelineResult.exported_to)
- `crates/astroforge-core/src/pipeline.rs` (Stage, StageType, StageDefinition)

## Out of scope

- Tauri IPC bridge changes for new commands (rolled into PR-B)
- Pipeline runner changes for re-apply (rolled into PR-B)
- Per-stage FITS output format (Q-1 → PNG only for v1)
