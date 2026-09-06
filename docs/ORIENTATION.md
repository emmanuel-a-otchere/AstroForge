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

## The architectural direction (CR-01 + CR-02 + CR-03 + CR-04 together)

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
- For implementation history and decisions: `docs/plans/` (CR-02, CR-03, and CR-04 plans live here)
- For audit history: `docs/M*_AUDIT.md` (the milestone audits that established each tranche)
- For closed phase work: `docs/PHASE_8_CLOSE.md`, `docs/PHASE_9_CLOSE.md`
- For housekeeping follow-ups: `docs/HOUSEKEEPING.md` (wizard dead-code cleanup, ingest/import-scan consolidation; created in CR-04 P0)

## Roadmap note

CR-04 — Intelligent Import, Session Understanding & Target Detection — implementation is in flight (P0–P10 phase structure; see `docs/plans/2026-09-06-cr04-intelligent-import/PLAN.md`). It takes the existing ingest, Bayer detection, frame classification, session grouping, and deep-sky/planetary routing logic and turns them into the first genuinely intelligent user experience: Drop Folder → Detect → Classify → Explain → Confirm → Build Session → Recommend Pipeline. That is the point where AstroForge begins to demonstrate its intelligence before a single processing stage is executed.

After CR-04 ships, the next CR is CR-05 — Intelligent Processing Workspace & Adaptive Pipeline Execution — which takes the Session Understanding + Recommendation produced by CR-04 and turns it into the actual image-processing experience (human-readable pipeline, live previews, execution controls, checkpoints, adaptive parameter recommendations, progress, recovery, and creation of Image Versions).