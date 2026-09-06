# CR-04 Implementation Plan — Intelligent Import, Session Understanding & Target Detection

**Date:** 2026-09-06
**Source CR:** [CR-04 — Intelligent Import, Session Understanding & Target Detection](../../CR-04-INTELLIGENT-IMPORT.md) (Status: Proposed, Priority: Critical)
**Strategy:** Phase-decomposed into ~10 PRs (P0–P10), matching the CR-02 and CR-03 pattern. Each phase is one PR-sized tranche.

## Current state (as of 529c85d, CR-03 P6 merge)

What already exists in `crates/astroforge-core`:

- `ingest.rs` — partial classification + manifest builder (`FrameType`, `FrameInfo`, `SessionManifest`, `LightGroup`, `scan_directory`, `classify_frame`, `group_lights`, `detect_anomalies`, 10+ tests).
- `bayer_detection.rs` — `detect_bayer(image)`, `route_by_confidence()`, `match_camera_signature()` (camera DB).
- `calibration.rs` — master dark/flat/bias builders + apply_calibration.
- `domain.rs` + `domain_store.rs` — CR-02 substrate: Project / Target / Session / SourceAsset / Artifact / ImageVersion / PipelineRun / StageRunRecord / AiOperation / Export / ProjectEvent. `SourceAsset` already carries `bayer_pattern` and the immutable-content-hash identity.
- `db.rs` — separate legacy `SessionStore` schema (CR-02 lives alongside, doesn't replace).

What already exists in `src/components`:

- `ImportWorkspace.svelte` — placeholder drop-folder empty state with disabled "Select files…" button. Note in the component already references CR-04 as the scope that ships the intelligent ingest flow.

What does NOT yet exist:

- A canonical **Import Intelligence API**. The UI calls neither CR-02 IPC nor any scan orchestrator.
- A **target detection** module.
- A **session grouping** module (beyond the existing `group_lights()` in ingest.rs).
- A **capture-analysis** module (deep-sky vs planetary routing).
- A **narrowband detection** module.
- A **recommendation** engine that produces a `ImportUnderstanding` object.
- A multi-step **Import Review UI** (drop folder → scan → understand → confirm).

## Decisions

- **Decision D-CR04-1 (phase structure):** P0–P10 matching CR-02/03 conventions. Backend (P1–P7 + P10) and frontend (P8–P9) interleaved so each phase yields a smoke-testable surface.
- **Decision D-CR04-2 (asset strategy):** Wrap, do not replace. The new `import_scan.rs` lives alongside the existing `ingest.rs`. Both expose IPC commands; the UI picks. **Known drift risk:** keeping two code paths for the same problem invites divergence. Documented explicitly in the §CR-04-Drift-Risk section below; consolidation can land later in a single housekeeping PR.
- **Decision D-CR04-3 (AI boundary):** P10 ships a thin AI integration hook. When deterministic confidence is below a configurable floor AND the dataset images are small enough to fit in model context, dispatch to `astroforge-ai::service::classify_target` (stubbed to return the deterministic fallback for now). Frontend surfaces "AI confirming…" so the user sees the boundary working.
- **Decision D-CR04-4 (data model extension):** Extend `crates/astroforge-core/src/domain.rs` with new entities `ImportRun`, `ImportAsset`, `AssetObservation`, `ClassificationEvidence`, `SessionDetection`, `UserOverride`. New migration `v3_import_intelligence.sql` in `domain_store.rs`. Existing tables untouched.
- **Decision D-CR04-5 (IPC surface):** Seven new `#[tauri::command]`s in `src-tauri/src/commands_import.rs`: `start_import`, `get_import_progress`, `get_import_understanding`, `confirm_import`, `override_import_classification`, `create_session_from_import`, `get_pipeline_recommendation`. New typed wrapper in `src/lib/astroforge-api.ts`. Existing CR-02 commands untouched.
- **Decision D-CR04-6 (state machine):** Deterministic state machine per CR-04 §16. Each transition emits a `ProjectEvent` (existing CR-02 audit trail). Failure states are recoverable (retry / cancel / inspect).
- **Decision D-CR04-7 (UI structure):** The existing `ImportWorkspace.svelte` becomes the Import Review UI shell. New sub-components: `ImportDropZone.svelte`, `ImportScanProgress.svelte`, `ImportUnderstandingPanel.svelte`, `ImportRecommendationPanel.svelte`, `ImportAmbiguityDialog.svelte`, `ImportOverridePanel.svelte`. Each surface has its own loading/empty/error states.
- **Decision D-CR04-8 (provenance):** Every inference + every override is recorded as a `ProjectEvent` (existing CR-02 audit trail). `UserOverride` records previous_value + new_value + reason + timestamp. Never destructive.
- **Decision D-CR04-9 (test strategy):** Each backend module ships a fixture-corpus test (per CR-04 §23). Frontend ships a typed-stub test for the Import Review UI states. CI smoke jobs (`smoke (macos-latest / ubuntu-latest / windows-latest)`) gate each PR.
- **Decision D-CR04-10 (deterministic-first AI):** Per CR-04 §20 ADR-04.5. Deterministic heuristics (metadata, filename, directory, exposure, image stats) precede AI. AI only when deterministic confidence < floor. P10 stubs the AI dispatch path; the stub returns the deterministic fallback + a `was_ai_consulted: false` marker so the boundary is observable.

## §CR-04-Drift-Risk (explicit)

Per Decision D-CR04-2 (Option C in the user's choice), both `ingest.rs` and the new `import_scan.rs` will coexist. The two paths:

- `ingest.rs::scan_directory()` returns `Vec<PathBuf>` and is used by today's (legacy) flow.
- `import_scan.rs::start_import()` returns a full `ImportUnderstanding` and persists SourceAsset records via CR-02's `domain_store`.

Drift risk: any future change to one path risks breaking the other. The contract between them is "neither path is the canonical scan API; the canonical path is whichever the UI chose at runtime." This is documented in code comments and a follow-up housekeeping CR (post-CR-04) consolidates them by deleting the legacy ingest.rs IPC surface once the new path proves itself in production. **Action item:** record this as a follow-up ticket under `docs/HOUSEKEEPING.md` (new file in this PR).

## Phase structure

| Phase | Status | Reference |
|-------|--------|-----------|
| P0 | done | #249 |
| P1 | done | #250 |
| P2 | done | #251 |
| P3 | done | #252 |
| P4 | in_progress | this PR (`core::target_detection` — FITS OBJECT + filename/directory patterns + small embedded catalog of M/NGC/IC/Sh2 + 8 astronomical TargetKind values + confidence + evidence) |
| P5 | pending | `core::session_grouping` (target/date/instrument/filter/binning/exposure/directory relationships) |
| P6 | pending | `core::capture_analysis` (deep-sky vs planetary/lunar routing, confidence + ambiguity flag) |
| P7 | pending | `core::narrowband` (filter detection, channel grouping, HOO/SHO composition suggestion) |
| P8 | pending | IPC layer (7 commands + typed wrapper + commands_import.rs) + Import state machine |
| P9 | pending | Import Review UI shell (4-step wizard + drop zone + scan progress + understanding panel + recommendation + ambiguity dialog) |
| P10 | pending | AI stub hook (deterministic confidence floor + `astroforge-ai::service::classify_target` stub) + override panel + provenance + drift-risk housekeeping ticket |

## Phase ordering rationale

- P1 must come first (no UI can work without scan).
- P2–P7 are independent classifications; any can ship alone but the UI can only consume them together via P8.
- P8 is the IPC gateway; P9 cannot ship until P8 lands.
- P10 closes the AI boundary and adds the override UX; it depends on P2 (so we have classifications to override) and P9 (so the override panel has a UI home).

## Constraints (carry-over from CR-02/03)

- Workspace deps via `[workspace = true]`.
- Clippy strict: `cargo clippy --all-targets --no-deps -- -D warnings`.
- Format strict: `cargo fmt --all` before every commit.
- Frontend: `npm run check` 0 errors. Warnings tolerated at the established baseline.
- Build: `npm run build` clean.
- Each PR includes the README pointer block update and the orientation doc update if the phase adds visitor-facing knowledge.
- No credentials in commits.

## Risks

- **R-1:** Rust 1.81 toolchain in this session can't fully build `src-tauri` (needs edition2024 / Rust 1.85+). We've worked around this in CR-02/03 by trusting CI smoke jobs. CR-04 adds 9 new core modules + 7 new IPC commands; a clippy or test failure blind to the agent will block the PR. Mitigation: keep new modules conservative in their dependency surface (no edition2024 features).
- **R-2:** Image-content fingerprinting (CR-04 §18 duplicate detection) on large datasets can be slow if done naively. Mitigation: SHA-256 of file bytes (fast, already in CR-02's artifact store). Image-content fingerprinting (perceptual hash) deferred to a later CR.
- **R-3:** The CR-04 §23 fixture corpus (18 categories) cannot ship in this repo (binary assets too large). Mitigation: ship a small fixture corpus + a typed-stub harness that generates synthetic frames for unit tests.

## Definition of Done (per CR-04 §26)

A user launches AstroForge → creates/opens a project → drops a raw telescope folder → AstroForge scans it → identifies files → extracts metadata → classifies frames → detects Bayer characteristics → identifies target/session relationships → determines likely acquisition type → detects calibration/narrowband characteristics → explains its conclusions → recommends the appropriate processing workflow → user accepts or corrects the interpretation → AstroForge creates a persistent Session and processing recommendation → Project can be closed and reopened with the entire interpretation intact.

This DoD spans P0–P10. No phase is "done" until the end-to-end flow described above is reproducible.