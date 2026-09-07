# CR-05 Implementation Plan — Intelligent Processing Workspace & Adaptive Pipeline Execution

**Date:** 2026-09-07
**Source CR:** [CR-05 — Intelligent Processing Workspace & Adaptive Pipeline Execution](../../CR-05-INTELLIGENT-PROCESSING.md) (Status: Proposed, Priority: Critical)
**Strategy:** Phase-decomposed into 8 PRs (P0–P7), following the CR-02 / CR-03 / CR-04 conventions. Each phase is one PR-sized tranche; each tranche is independently reviewable and CI-green before the next starts. P0 is documentation-only (this PR).

## Why a phased rollout

CR-05 is the largest CR to date in the AstroForge programme: 35 sections, 7 ADRs, 29 acceptance criteria, 6 new persistent entities, a new workspace UI, and the orchestration of processing capability that already exists across `astroforge-core`, `astroforge-ai`, and the Svelte frontend. A big-bang landing would be unreviewable and would block CR-06 (AI Enhancement Studio), CR-07 onward for the duration. The phased structure keeps each PR small enough to reason about end-to-end and keeps the catalogue of in-flight PRs honest.

## Current state (as of c86c07b, CR-04 P4 target detection)

What already exists in `crates/astroforge-core`:

- `domain.rs` — Project / Target / Session / SourceAsset / Artifact / ImageVersion / PipelineRun / StageRunRecord / AiOperation / Export / ProjectEvent (CR-02 substrate).
- `domain_store.rs` — SQLite persistence with migrations.
- `artifact_store` / `artifact.rs` — content-addressed filesystem storage (SHA-256, atomic temp-then-rename).
- `pipeline.rs` (89 lines) — minimal `Stage`, `StageType`, `StageDefinition`. No plan / execution model yet.
- `pipeline_run.rs` — `PipelineRun` driver that records `StageRunRecord` rows.
- `session.rs` — `SessionStore`, `stage_runs` schema, IPC commands.
- `db.rs` — `SESSION_SCHEMA_SQL` with `stage_runs` and `checkpoints` tables.
- `export.rs` — `export_multi_format`, `export_fits_32bit`, `export_jpeg`, `export_png` (per M7_AUDIT).
- `mvp_pipeline.rs` — `PipelineConfig`, `PipelineResult.exported_to` (legacy wizard pipeline).
- `calibration.rs`, `bayer_detection.rs`, `ingest.rs`, and the existing processing modules (stretch, color calibration, background extraction, denoise, etc.).

What already exists in `src/` (Svelte):

- `pipeline.ts` (288 lines) — typed pipeline model on the frontend.
- `pipeline-store.ts` (710 lines) — store, history, undo / redo, receipt shape, `commitStage()`, `undo()`, `stage_artifacts`.
- `WizardBottomSheet.svelte` (868 lines) — undo / redo + Process button (legacy wizard pattern; CR-03 marked it for deprecation in P3 but it still owns the run-pipeline UX).
- `ReceiptsPanel.svelte` (288 lines) — read-only receipt log.
- `StudioShell.svelte` (CR-03) — three-zone workspace scaffold (left context / center image / right controls).
- `ImportTab.svelte` / `ProcessTab.svelte` (CR-03) — tab structure, not yet wired to CR-05 entities.

What does NOT yet exist:

- A canonical `pipeline_plan` / `pipeline_stage` / `stage_execution` schema (CR-05 §26).
- A `recommendation` engine that produces per-stage recommendations with `observation / evidence / confidence` (CR-05 §7, §12, §13, §20).
- A preview-before-commit flow with `preview_run` persistence (CR-05 §11).
- A checkpoint UX with visible recovery (CR-05 §10).
- Branching for pipeline experiments (CR-05 §17).
- A `quality_metric` framework (CR-05 §14).
- A `processing_decision` audit trail (CR-05 §20).
- Auto / Guided / Expert mode semantics in the UI (CR-05 §4).
- Resource-aware execution + memory budgeting (CR-05 §21, §22).
- Three-zone Process workspace with stage states and the stage model (CR-05 §5–§8).
- The semantic UI-to-engine API of §27 (today's commands are mixed semantic + implementation-level).
- The 4-question error-recovery UX (CR-05 §28).

## Constraints

- **No big-bang.** Eight PRs (P0–P7). Each green before the next starts.
- **Orchestration, not reimplementation.** Per CR-05 §31, CR-05 orchestrates existing capabilities rather than reimplementing them. Every existing module in `astroforge-core` and `astroforge-ai` is read-only except for explicit extension points.
- **UI is a projection.** The UI must remain a projection of persistent state, not the canonical source. Every durable change flows through a Tauri command into an application service that updates the domain store (per ORIENTATION §"The architectural direction").
- **Auto / Guided / Expert are progressive disclosure.** The same engine; one mode unlocks more controls. No fork in the code path.
- **CR-04 boundary.** CR-05 consumes the Session Understanding + recipe recommendation produced by CR-04; CR-05 does not duplicate session-intelligence logic. P2 depends on CR-04 P5/P6 landing first.
- **CR-02 lineage preserved.** Every `ImageVersion` produced by a CR-05 stage run carries full provenance (CR-02 §11 + CR-04 provenance rules). Branching (CR-05 §17) is implemented as additional `PipelineRun` rows with a shared `parent_run_id`.
- **AI boundary explicit.** Deterministic stages never silently invoke AI. Per ADR-05.7, every AI call records `model_id`, `deterministic`, `seed`, and the user decision (per CR-04 §"Intelligence contract").
- **ADR-05.6 first-class.** Memory budgeting is a product concern; the workspace surfaces "this will be tiled" rather than crashing.
- **Visitor-facing docs.** Each phase PR adds a short "what this phase means for a visitor" note to `docs/ORIENTATION.md` when behaviour changes.

## Decisions

- **Decision D-CR05-1 (phase structure):** P0 doc + plan + orientation update; P1 schema + plan generator + Deep-Sky OSC recipe (vertical slice for Auto mode); P2 stage runner + checkpoint persistence + start / pause / resume / cancel; P3 quality metrics + recommendation engine + Guided-mode recommendations; P4 preview-before-commit + Branching; P5 Expert-mode DAG view + resource-aware execution; P6 processing timeline + error-recovery UX; P7 visual regression + cross-platform resource tests + Definition-of-Done scenario verification.
- **Decision D-CR05-2 (schema migration):** Extend `crates/astroforge-core/src/domain.rs` with six new entities (`PipelinePlan`, `PipelineStage`, `StageExecution`, `QualityMetric`, `ProcessingDecision`, `PreviewRun`). New migration `v4_pipeline_plans.sql` in `domain_store.rs`. Existing tables untouched. Branching is `PipelineRun.parent_run_id` (foreign key to another `PipelineRun.id`); no separate `branch` entity.
- **Decision D-CR05-3 (pipeline runner):** New `crates/astroforge-core/src/pipeline/` module with `plan.rs`, `stage.rs`, `execution.rs`, `preview.rs`, `checkpoint.rs`, `recovery.rs`, `adaptive.rs` (per CR-05 §31). The runner is the canonical orchestrator; the existing `mvp_pipeline.rs` wizard path is deprecated after P2 (legacy IPC commands emit a deprecation warning and forward to the new runner).
- **Decision D-CR05-4 (recommendation engine):** New `crates/astroforge-core/src/recommendation/` module: deterministic-first (per CR-04 ADR-04.5), observation-evidence-confidence, AI hook stub (similar to CR-04 D-CR04-3). Hook delegates to `astroforge-ai::service::recommend_parameters` (stub returns deterministic fallback + `was_ai_consulted: false` so the boundary is observable).
- **Decision D-CR05-5 (preview path):** `PreviewRun` is a real persisted entity (not ephemeral). Preview artifacts are written under `<artifact_root>/previews/<stage_execution_id>/` and pinned to the `StageExecution` they preview. Full-resolution re-runs always re-validate against the same parameters that produced the preview (parameter hash on `StageExecution`).
- **Decision D-CR05-6 (checkpoint strategy):** Checkpoint after every completed `StageExecution`. Recovery state machine: `Idle → Running → Paused → Resumed → Completed | Cancelled | Failed`. On app launch, `find_resumable_run()` (Tauri command) returns the most recent interrupted run with a valid checkpoint. UX per CR-05 §10.
- **Decision D-CR05-7 (modes):** `mode` is a property of `PipelinePlan`, not the UI. UI renders more controls as mode escalates; the engine does not change behaviour. This keeps the engine invariant under mode changes (no silent bugs from "Auto did something different than Expert").
- **Decision D-CR05-8 (resource budget):** New `crates/astroforge-core/src/execution/resource.rs` owns the budget; backends advertise capability at startup (`enumerate_backends()`). Tile size and concurrency derived from budget + dataset size; never hard-coded.
- **Decision D-CR05-9 (branching):** A branch is a `PipelineRun` whose `parent_run_id` points at the originating run's checkpoint. The originating run is read-only once branched. The user can compare any two runs that share a common ancestor through the existing Compare surface (CR-03 §Compare).
- **Decision D-CR05-10 (error recovery UX):** `StageExecution.error` is structured (`{ what_happened, what_was_preserved, suggested_actions[] }`). The UI renders the 4-question template from §28 by mapping structured fields to the canonical wording.
- **Decision D-CR05-11 (test strategy):** Pipeline-generation tests (CR-05 §30) live as table-driven unit tests in `crates/astroforge-core/src/pipeline/plan.rs` (one row per dataset fixture × recipe). Execution tests use the existing `tests/pipeline_e2e.rs` shell with new cases. Visual regression: a small set of fixed reference datasets committed under `crates/astroforge-core/tests/fixtures/`; histograms + FITS stats compared with tolerance.
- **Decision D-CR05-12 (reversibility):** CR-05 §16 / §17 supersede the M7_AUDIT T1/T2/T3/T4/T5 reversibility work. The new `PipelineStage.undo_supported: bool` flag is the authoritative signal; the wizard's ad-hoc `undo()` becomes a wrapper over the new runner.

## Phase structure

| Phase | Focus | Reference | CR-05 sections |
|-------|-------|-----------|----------------|
| P0 | CR-05 doc + plan + orientation + README pointer | this PR | §33, §35 |
| P1 | Schema (`pipeline_plan`, `pipeline_stage`, `stage_execution`) + recipe → plan generator + Deep-Sky OSC Balanced recipe | tbd | §2, §18, §19, §26 |
| P2 | Stage runner + checkpoint persistence + start / pause / resume / cancel + `stage_execution` records | tbd | §9, §10, §16, §26, §27 |
| P3 | `quality_metric` + `processing_decision` + recommendation engine (deterministic-first) + Guided-mode recommendations | tbd | §7, §12, §13, §14, §15, §20, §26 |
| P4 | `preview_run` + Preview-before-commit UX + Branching | tbd | §11, §17, §26, §28 |
| P5 | Expert-mode DAG view + `enumerate_backends()` + resource-aware execution + memory budget | tbd | §4, §21, §22, §24, §27 |
| P6 | Processing timeline UI + 4-question error-recovery UX + AI-boundary labelling (deterministic vs perceptual) | tbd | §5, §6, §23, §25, §28 |
| P7 | Visual regression corpus + cross-platform resource tests + Definition-of-Done scenario + ADR-05 audit + CHANGELOG | tbd | §29, §30, §33, §34 |

## Phase details

### P0 — CR-05 doc + plan + orientation + README pointer (this PR)

- Save CR-05 to `docs/CR-05-INTELLIGENT-PROCESSING.md`.
- Add `docs/plans/2026-09-07-cr05-intelligent-processing/PLAN.md` (this file).
- Update `docs/ORIENTATION.md` with CR-05 entry, cross-reference, and roadmap note.
- Add CR-05 pointer block to `README.md`.
- No code changes.

### P1 — Schema + plan generator + Deep-Sky OSC Balanced recipe (vertical slice: Auto mode)

- Extend `crates/astroforge-core/src/domain.rs` with `PipelinePlan`, `PipelineStage`, `StageExecution`. New migration `v4_pipeline_plans.sql`.
- New `crates/astroforge-core/src/pipeline/plan.rs`: `generate_plan(session_understanding: &SessionUnderstanding, recipe: &Recipe, mode: Mode) -> Result<PipelinePlan, PlanError>`.
- New `crates/astroforge-core/src/recipe/builtin/deep_sky_osc_balanced.rs`: 10 stages (Calibrate, Debayer, QualityFilter, Register, Stack, Background, Color, Stretch, Denoise, DetailEnhancement) with defaults. `required` vs `optional` per stage.
- One `#[tauri::command]`: `create_pipeline_plan`.
- Frontend: `src/lib/pipeline-plan-store.ts` + a smoke-testable "Auto mode" panel inside `ProcessTab.svelte` that calls `create_pipeline_plan` and renders the human-readable pipeline (per CR-05 §2.1).
- Tests: table-driven plan-generation tests for `(deep_sky_osc_balanced × calibration_present)`, `(deep_sky_osc_balanced × no_calibration)`, `(planetary × high_frame_count → planetary branch)` (per CR-05 §30).
- Acceptance: the existing `Process` button on `WizardBottomSheet.svelte` is **not** wired yet (still runs the wizard path); the new Auto panel runs in parallel and can produce a plan but not execute it.

### P2 — Stage runner + checkpoint persistence + start / pause / resume / cancel

- New `crates/astroforge-core/src/pipeline/execution.rs`: `run_stage(plan, stage_id) -> Result<StageExecution, StageError>`. Invokes the existing processing modules by `StageType`.
- New `crates/astroforge-core/src/pipeline/checkpoint.rs`: checkpoint write per completed stage; `find_resumable_run(project_id) -> Option<PipelineRun>`.
- Extend `crates/astroforge-core/src/session.rs` (or new `pipeline/runner.rs`) to drive a plan from `start` through `complete | cancel | fail`. Persist `StageExecution` rows.
- Recovery state machine: `Idle → Running → Paused → Resumed → Completed | Cancelled | Failed` (per Decision D-CR05-6).
- Four new `#[tauri::command]`s: `start_pipeline_run`, `pause_pipeline_run`, `resume_pipeline_run`, `cancel_pipeline_run`.
- Frontend: `src/components/ProcessingControls.svelte` with Start / Pause / Resume / Cancel buttons; status indicator on each stage (per CR-05 §8). Pipeline state read from the runner, not the wizard.
- Recovery UX: `src/components/RecoveryBanner.svelte` shown on Project open if `find_resumable_run` returns one (per CR-05 §10).
- Tests: execution tests for start / pause / resume / cancel / partial failure (per CR-05 §30).
- Acceptance: end-to-end "deep-sky OSC balanced plan runs to completion with checkpoints" on the existing smoke fixture.

### P3 — Quality metrics + recommendation engine + Guided-mode recommendations

- New `crates/astroforge-core/src/quality/{metrics.rs, frame_quality.rs, image_quality.rs}` (per CR-05 §14, §31).
- New `crates/astroforge-core/src/recommendation/` module (per Decision D-CR05-4): deterministic-first (metadata + image stats + session understanding), AI hook stub.
- Extend `domain.rs` with `QualityMetric` + `ProcessingDecision`. Migration `v5_quality_and_decisions.sql`.
- Extend `StageExecution` to record metric snapshots at completion.
- New `#[tauri::command]`: `get_recommendation(stage_execution_id) -> Recommendation`.
- Frontend: `src/components/IntelligencePanel.svelte` shows the recommendation panel pattern from CR-05 §6 Zone C. The "Frame Quality" copy from §15.
- Tests: metric determinism (same image → same metric); recommendation determinism for the test corpus.
- Acceptance: a stage that completes records its metrics; the next stage's panel can show a recommendation derived from those metrics.

### P4 — Preview-before-commit + Branching

- New `crates/astroforge-core/src/pipeline/preview.rs`: `preview_stage(plan, stage_id, params, source_version) -> Result<PreviewRun, PreviewError>`. Reduced-resolution by default; full-resolution preview opt-in.
- Extend `domain.rs` with `PreviewRun`. Migration `v6_previews.sql`.
- Pin `parameters_hash` on `StageExecution` so a full-resolution run re-validates against the same parameters that produced the preview (per Decision D-CR05-5).
- Branching: extend `PipelineRun` with `parent_run_id` + `branch_point_stage_id` (Decision D-CR05-9). UI: `src/components/BranchExplorer.svelte` with the Stacked → (Natural | Artistic) → Denoise → Compare pattern from CR-05 §17.
- New `#[tauri::command]`s: `preview_stage`, `apply_stage`, `create_branch`.
- Tests: preview determinism, preview → full-resolution parameter parity, branch isolation.
- Acceptance: every expensive stage can be previewed before commit; a user can branch the pipeline at any stage and compare two outcomes.

### P5 — Expert-mode DAG view + Resource-aware execution + Memory budget

- New `crates/astroforge-core/src/execution/resource.rs`: `enumerate_backends()`, `derive_budget(dataset_size, available_ram)`, `tile_size_for(stage, budget)` (Decision D-CR05-8).
- `pipeline/execution.rs` now selects backend + tile size from `resource::derive_budget`. Tiled execution path for any stage that exceeds budget (per CR-05 §22).
- New `pipeline/adaptive.rs`: parameter adaptation from metrics (CR-05 §12).
- Expert-mode DAG view: `src/components/ExpertDagView.svelte` renders the underlying DAG (CR-05 §24 Expert). Auto / Guided / Expert are mode escalations of the same engine (Decision D-CR05-7).
- New `#[tauri::command]`: `get_processing_metrics(stage_execution_id) -> Vec<QualityMetric>`.
- Memory-budget UX: pre-flight check before expensive stages; the "This operation requires more memory…" copy from CR-05 §22.
- Tests: `enumerate_backends()` determinism, tile-size derivation table tests, low-memory fixture scenario.
- Acceptance: a 4 GB RAM target can process a 200-frame 16-bit OSC dataset without exceeding the budget; Expert mode exposes the DAG without changing engine behaviour.

### P6 — Processing timeline + Error-recovery UX + AI-boundary labelling

- `src/components/ProcessingTimeline.svelte`: the 10:31 / 10:33 / ... timeline from CR-05 §25; click event → reveal Image Version.
- Structured error rendering: extend `StageExecution.error` (Decision D-CR05-10) with `{ what_happened, what_was_preserved, suggested_actions[] }`. `src/components/ErrorRecoveryPanel.svelte` maps structured fields to the 4-question wording (CR-05 §28).
- AI-boundary labelling: every `StageExecution` records `uses_ai: bool`, `model_id?: String`, `deterministic: bool`, `seed?: u64`. `src/components/StageCard.svelte` shows the "AI Enhancement" badge from CR-05 §23 when applicable.
- New `#[tauri::command]`s: `retry_stage`, `skip_stage` (per CR-05 §9).
- Tests: error-recovery mapping table; AI-boundary labelling unit tests.
- Acceptance: every error presents the 4-question template; AI stages carry their provenance badge.

### P7 — Visual regression + Resource tests + DoD scenario + ADRs audit + CHANGELOG

- Commit a small visual-regression corpus under `crates/astroforge-core/tests/fixtures/` (per CR-05 §30): 3 deep-sky + 2 planetary + 1 narrowband.
- Cross-platform resource tests: `crates/astroforge-core/tests/resource.rs` exercising low-memory / normal desktop / GPU-enabled / CPU-only (per CR-05 §30).
- DoD scenario (CR-05 §34) end-to-end test: drop folder → understand → generate plan → preview meaningful operations → process → checkpoints → Image Versions → adapt → pause / resume / retry / branch → final version with provenance. Asserts every step.
- ADR audit: confirm ADR-05.1 through ADR-05.7 are implemented; produce `docs/M8_AUDIT.md`.
- `CHANGELOG.md` entry; CR-05 status flips to **Implemented** in `SPEC_INDEX.md` (or its successor).
- Acceptance: the DoD scenario passes in CI on at least one platform; ADRs have a verified implementation; CHANGELOG published.

## Drift risks (explicit)

- **P1 vs wizard pipeline.** The legacy `mvp_pipeline.rs` path coexists with the new runner. Until P2 lands the deprecation wrapper, two paths can produce different receipts for the same operation. Action: track in `docs/HOUSEKEEPING.md`; consolidate after P7.
- **P3 AI boundary.** Deterministic-first + AI hook stub mirrors the CR-04 D-CR04-3 pattern. Drift between the two AI boundaries (CR-04 target detection vs CR-05 recommendations) must be documented in a single place — add `crates/astroforge-ai/src/BOUNDARY.md` if not already present.
- **P5 backend enumeration.** `enumerate_backends()` is platform-specific (CUDA / DirectML / CoreML / OpenVINO / CPU). Drift across CI runners is expected; resource tests must run on at least macOS + Linux + Windows to be meaningful.
- **P6 vs M7_AUDIT reversibility.** M7_AUDIT T1–T5 work partially overlaps with CR-05 §16 / §17 / P2 reversibility + P4 branching. After P2 lands, the wizard `undo()` becomes a wrapper. Track the migration in M7_AUDIT as "superseded by CR-05 P2".

## Acceptance across all phases

The CR-05 §29 acceptance checklist maps to phases as follows:

| §29 group | Satisfied by |
|-----------|--------------|
| Pipeline (6 items) | P1 + P5 |
| Execution (7 items) | P2 + P6 |
| Image Versions (5 items) | P2 + P4 |
| Intelligence (5 items) | P3 |
| Preview (3 items) | P4 |
| Performance (4 items) | P5 |
| Standalone (5 items) | P2 + P5 + P6 |

P7 verifies the DoD scenario and audits ADR-05.1 through ADR-05.7 against the implementation.

## Files introduced across CR-05 (anticipated)

```
crates/astroforge-core/src/
  domain.rs                                  (extended)
  domain_store.rs                            (extended; v4/v5/v6 migrations)
  pipeline/
    mod.rs
    plan.rs
    stage.rs
    execution.rs
    preview.rs
    checkpoint.rs
    recovery.rs
    adaptive.rs
  quality/
    mod.rs
    metrics.rs
    frame_quality.rs
    image_quality.rs
  recommendation/
    mod.rs
    engine.rs
    evidence.rs
  execution/
    mod.rs
    resource.rs
    backend.rs
  recipe/
    builtin/
      deep_sky_osc_balanced.rs
      deep_sky_osc_high_quality.rs
      planetary.rs
  tests/
    pipeline_e2e.rs                          (extended)
    resource.rs
    fixtures/                                (visual-regression corpus)

crates/astroforge-ai/src/
  BOUNDARY.md                                (if not present)
  service/
    recommend_parameters.rs                  (stub + hook)

src-tauri/src/
  commands_pipeline.rs                       (new)
  commands_processing.rs                     (new)

src/lib/
  pipeline-plan-store.ts                     (new)
  astroforge-api.ts                          (extended; new commands typed)

src/components/
  ProcessingControls.svelte                  (new)
  RecoveryBanner.svelte                      (new)
  IntelligencePanel.svelte                   (new)
  BranchExplorer.svelte                      (new)
  ExpertDagView.svelte                       (new)
  ProcessingTimeline.svelte                  (new)
  ErrorRecoveryPanel.svelte                  (new)
  StageCard.svelte                           (new)
  ProcessTab.svelte                          (extended)

docs/
  CR-05-INTELLIGENT-PROCESSING.md
  plans/2026-09-07-cr05-intelligent-processing/PLAN.md
  M8_AUDIT.md
  HOUSEKEEPING.md                            (extended)
```

## Out of scope (per CR-05 §31 and §23)

- Reimplementing existing processing modules (stretch, calibration, color, etc.).
- New AI models (deferred to CR-06).
- Headless / CLI invocation of the runner (deferred; the Tauri commands are the canonical surface for v1.0).
- Per-stage FITS output format (per M7_AUDIT Q-1; deferred to v2).
- Cross-project recipe sharing (deferred).