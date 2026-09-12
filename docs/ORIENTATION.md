# AstroForge — Orientation

A short read for visitors landing in the repo for the first time. The full product specification lives at `docs/specs/`; the architectural Change Requests live in `docs/CR-*`. This document summarizes the architectural frame so a visitor can orient before going deeper.

## What AstroForge is

A cross-platform desktop application that turns raw smart-telescope captures into publication-ready deep-sky images. Built on Tauri (Rust + webview) with an interactive, AI-augmented processing pipeline. Designed to scale from one-click simplicity to expert-level control.

## The three foundation CRs

AstroForge's architecture is established by three CRs in sequence. Each one answers a distinct question.

### CR-01 — Product & UX Foundation

**Question:** *How should AstroForge work?*

Defines the user journey: Discover → Create → Import → Understand → Process → Review → Enhance → Compare → Export → Reuse. Establishes the principle that the image is the primary object; everything else exists to help the user understand, improve, compare, and export that image. Defines the three user modes (Auto / Guided / Expert) as progressive disclosure of the same engine.

Cross-reference: `docs/specs/` (the originating product specification).

### CR-02 — Project, Session, Artifact Architecture

**Question:** *What does AstroForge own and remember?*

AstroForge Project is the durable unit of work; a filesystem folder is merely an input source. CR-02 introduces the canonical hierarchy:

```
Project
├── Target
├── Session
│   └── Source Assets
├── Recipe
├── Pipeline Runs
│   └── Stage Runs
├── Artifacts (intermediate / preview / final)
├── Image Versions
├── AI Operations
└── Exports
```

This is implemented in `crates/astroforge-core/src/` across the `domain`, `domain_store`, `artifact_store` (in `artifact.rs`), `project`, and `pipeline_run` modules. SQLite stores the lineage; the filesystem stores the artifact bytes (content-addressed by SHA-256, atomic temp-then-rename writes). Every Project is self-contained on disk under its own directory layout (CR-02 §18).

Cross-reference: `docs/CR-02-PROJECT-SESSION-ARTIFACT-ARCHITECTURE.md` for the full specification; `docs/plans/2026-09-06-cr02-domain-model/PLAN.md` for the phased implementation history.

### CR-03 — Application Shell & Studio Workspace

**Question:** *How does the user interact with the persistent model?*

The application has two navigation contexts:

- **Application context:** Home, Projects, Recipes, AI Models, Settings, Help. The user operates on the library.
- **Project / Studio context:** Overview, Import, Process, Enhance, Compare, Export. The user operates inside one project.

The Studio uses a three-zone workspace (left context / center image / right controls) with the image canvas visually dominant (~85–90% of the workspace). Controls are contextual: stretch controls appear when stretching, denoise controls when denoising, etc. AI is embedded throughout the workflow as Observation → Recommendation → Confidence → Preview → User Decision, not isolated as a standalone mode.

Cross-reference: `docs/CR-03-APPLICATION-SHELL-STUDIO.md` for the full specification; `docs/plans/2026-09-06-cr03-application-shell/PLAN.md` for the phased implementation plan.

### CR-04 — Intelligent Import, Session Understanding & Target Detection

**Question:** *How does AstroForge understand what the user has given it?*

Import is not a file picker. When the user drops a folder of raw telescope captures into a project, AstroForge scans it, identifies every file, extracts whatever metadata is available (FITS headers, EXIF, image dimensions), classifies each frame as Light / Dark / Flat / Bias / Unknown / Unsupported / Invalid, detects the Bayer pattern with explicit confidence + evidence, groups files into coherent Sessions belonging to a Target, determines whether the dataset is deep-sky or planetary/lunar, identifies narrowband acquisition groups, and produces a recommendation: the appropriate processing recipe for this dataset.

Every inference carries **observation → evidence → confidence → decision**. Ambiguity triggers a user-confirmation prompt; AstroForge never silently routes an ambiguous dataset into an irreversible pipeline. Every user override becomes part of provenance.

The intelligence hierarchy is: deterministic metadata first, deterministic image statistics second, AI third, user authority fourth (per ADR-04.5).

Cross-reference: `docs/CR-04-INTELLIGENT-IMPORT.md` for the full specification; `docs/plans/2026-09-06-cr04-intelligent-import/PLAN.md` for the phased implementation plan (P0–P10).

### CR-05 — Intelligent Processing Workspace & Adaptive Pipeline Execution

**Question:** *How does AstroForge turn a Session Understanding into a real, observable processing experience?*

CR-05 is the processing core. It takes the Session Understanding + recipe recommendation produced by CR-04 and turns it into the user-facing processing workspace: a human-readable pipeline (Calibrate → Stack → Color → Stretch → Enhance), three processing modes (Auto / Guided / Expert) with progressive disclosure of the same engine, a three-zone studio (Versions / Image Canvas / Intelligence & Controls), per-stage recommendations with explicit evidence and confidence, previews before expensive commits, full start / pause / resume / cancel / retry / skip / re-run execution semantics, checkpoints with visible recovery, a quality feedback loop that adapts subsequent recommendations, pipeline branching for AI experimentation, recipe-versus-pipeline separation, resource-aware execution that respects 4–8 GB RAM targets, and explainable AI boundaries (deterministic versus perceptual operations are explicitly labelled).

CR-05 extends CR-02 with six new persistent entities (`pipeline_plan`, `pipeline_stage`, `stage_execution`, `quality_metric`, `processing_decision`, `preview_run`) and introduces seven ADRs (ADR-05.1 through ADR-05.7). The most important architectural rule is explicit in the spec: the workspace must never become a visual mirror of the internal DAG; the user cares about their image being aligned, stacked, cleaned and improved, not about which stage numbers ran.

Cross-reference: `docs/CR-05-INTELLIGENT-PROCESSING.md` for the full specification; `docs/plans/2026-09-07-cr05-intelligent-processing/PLAN.md` for the phased implementation plan (P0–P7).

## The architectural direction (CR-01 + CR-02 + CR-03 + CR-04 + CR-05 together)

```
SQLite (durable state)
       │
       ▼
Rust domain types (Project, Session, PipelineRun, Artifact, ImageVersion)
       │
       ▼
Application services (ProjectManager, pipeline persistence driver)
       │
       ▼
Tauri commands (commands_project.rs + the existing recipe/session/gallery commands)
       │
       ▼
Frontend stores (Svelte)
       │
       ▼
UI (Svelte components: ApplicationShell, StudioShell, ImportTab, ProcessTab, ...)
```

The UI is a *projection* of persistent state, not the canonical source. This prevents the common desktop-app failure mode where the UI says one thing while the processing engine has another state. Every durable change in the UI flows through a Tauri command into an application service that updates the domain store.

## How the data flows

1. User creates a Project (Application → Tauri `project_create` → `ProjectManager::create_project` → SQLite + filesystem layout).
2. User imports a Session (CR-04 owns the intelligent ingest: drop-folder scan → metadata extraction → frame classification → target/session grouping → deep-sky/planetary routing → narrowband detection → recommendation → user confirmation → persistent Session). The Import workspace shell ships in CR-03; the intelligence lives in CR-04.
3. User starts a pipeline (Tauri `pipeline_run_list` etc.; the persistent DAG driver from CR-02.5 records every stage).
4. User reviews image versions; selects one; exports (the existing `export_multi_format` IPC handles the export).
5. User closes AstroForge. State is durable; on next launch, the user returns to the same project with the same context.

## Where to go next

- For the product vision: `README.md`
- For the full product specification: `docs/specs/`
- For the durable data model: `docs/CR-02-PROJECT-SESSION-ARTIFACT-ARCHITECTURE.md`
- For the application shell + studio: `docs/CR-03-APPLICATION-SHELL-STUDIO.md`
- For intelligent import + session understanding: `docs/CR-04-INTELLIGENT-IMPORT.md`
- For intelligent processing & adaptive pipeline execution: `docs/CR-05-INTELLIGENT-PROCESSING.md`
- For implementation history and decisions: `docs/plans/` (CR-02, CR-03, CR-04, and CR-05 plans live here)
- For audit history: `docs/M*_AUDIT.md` (the milestone audits that established each tranche)
- For closed phase work: `docs/PHASE_8_CLOSE.md`, `docs/PHASE_9_CLOSE.md`
- For housekeeping follow-ups: `docs/HOUSEKEEPING.md` (wizard dead-code cleanup, ingest/import-scan consolidation; created in CR-04 P0)

## Roadmap note

CR-02 → CR-07 all shipped (2026-09-12): CR-02 domain architecture, CR-03
application shell, CR-04 intelligent import (P0–P10), CR-05 intelligent
processing (P0–P7), CR-06 AI enhancement studio (P0–P7 + P5.1 real ONNX
inference), CR-07 Zone B canvas + compare surfaces (PR #308 + #309). The
M9 audit is closed at 39/39 §35 criteria shipped.

Active slice (2026-09-12): CR-07 follow-on 2 — ImageCanvas WebGL back-end
(self-contained GPU shader path for > 4 MP renders, with Canvas 2D
auto-fallback). Forward-look: spec bump 1.3.0 → 1.4.0 paperwork carrier,
DP#4 license verification (#135), GPU providers for `ort` (P5.2),
plate-solve dependency decision (#73), smart-telescope SDK decision (#133),
recipe system (Phase 3 M2), packaging (Phase 3 M3), plugin architecture
(Phase 4).