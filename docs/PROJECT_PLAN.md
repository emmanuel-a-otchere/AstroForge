# AstroForge — Living Project Plan

**Last updated:** 2026-09-03
**Current phase:** Phase 0 + Phase 1 (UI scaffolding complete; 6 Phase 1 MVP tasks pending);
Phase 1.5 (UI chrome complete; **50 of 50 processing-pipeline issues OPEN**);
Phase 2+ deferred.
**Spec version:** 1.1.0
**Active CR:** AF-CR-2026-09-01-IMG-PIPELINE
**Active plan:** [`docs/plans/2026-09-03-m2-tranche/`](plans/2026-09-03-m2-tranche/PLAN.md)
**Re-plan rationale:** Phases 1-6 of the Coder session shipped UI chrome, design tokens,
local GalleryStore, and Supabase removal. The M1.5 processing pipeline (50 issues) and
MVP smoke tests (4 issues) are still pending. M2 = finish what was promised in M1.5,
not start new work. See `plans/2026-09-03-m2-tranche/PLAN.md` for the tranche plan.

> This is a **living document**. It is rebased frequently against actual work
> progress. When a task is completed, its status is updated here and the plan is
> re-prioritized. When scope changes, the spec is updated first (per the
> spec-driven governance policy), then this plan is adjusted to match.
>
> **Spec-driven rule:** No task in this plan exists without a corresponding spec
> section. If a task doesn't trace to the spec, either the spec is updated first
> or the task is removed.

---

## How This Plan Works

### Phases
The project is divided into **6 phases**, each with a clear exit criterion.
Phases map to the spec's build roadmap (§16) but break it into actionable
milestones with concrete deliverables.

| Phase | Spec mapping | Exit criterion |
|---|---|---|
| **Phase 0** — Foundation & Scaffolding | §4 Architecture, §2 Constraints | Tauri shell builds and runs on Windows + macOS; project skeleton committed |
| **Phase 1** — MVP Core Pipeline | §16 MVP | End-to-end FITS → TIFF on 4 GB machine; beginner dialog smoke test passes |
| **Phase 1.5** — Guided Processing Train | AF-CR-2026-09-01 §4 | Interactive non-destructive processing train with 3 operating modes, live preview, and backend AI support |
| **Phase 2** — Full Deep-Sky Pipeline | §16 v1 (deep-sky subset) | All deep-sky stages functional; narrowband composition works; AI models integrated |
| **Phase 3** — Planetary, Recipes & Polish | §16 v1 (remaining) + §11 | Planetary pipeline functional; recipe export/import works; all dialog modes |
| **Phase 4** — Ecosystem & Research | §16 v2 | Plugin API; recipe gallery; platform optimizations; experimental models |

### Milestones
Each phase contains **milestones** — checkpoint deliverables that gate progress.
A milestone is only "done" when all its tasks are complete and its acceptance
criteria are met.

### Tasks
Tasks are the atomic unit of work. Each task has:
- **ID:** `P<phase>-M<milestone>-T<task>` (e.g., `P0-M1-T3`)
- **Spec ref:** the spec section or CR section it implements
- **Status:** `pending` · `in_progress` · `done` · `blocked` · `deferred`
- **Depends on:** other task IDs that must complete first

### Rebase cadence
- After every completed milestone, the plan is rebased: completed tasks are
  marked done, remaining estimates are adjusted, and blocked items are
  re-evaluated.
- If the spec changes (new version), this plan is reviewed against the diff and
  updated within the same PR.

---

## Phase 0 — Foundation & Scaffolding

**Goal:** A building-ready project skeleton. Tauri app launches, dev tooling is
configured, CI passes, and the architecture is in place for pipeline work.

**Exit criterion:** `astroforge` launches as a desktop window on Windows and
macOS with a placeholder UI, the Rust core crate compiles, and CI runs
formatting + tests on every push.

**Status:** Scaffolding complete. Tauri shell, Svelte frontend, Rust workspace,
CI, FITS I/O, and core architecture all implemented.

### Milestone 0.1 — Project Skeleton & Tooling

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P0-M1-T1 | [#1](https://github.com/emmanuel-a-otchere/AstroForge/issues/1) | Initialize Tauri 2.x project (Rust + Svelte/SolidJS frontend) | §4 | done | — |
| P0-M1-T2 | [#2](https://github.com/emmanuel-a-otchere/AstroForge/issues/2) | Configure Vite + Svelte/SolidJS with WebGPU probe and Canvas2D fallback | §4 | done | T1 |
| P0-M1-T3 | [#3](https://github.com/emmanuel-a-otchere/AstroForge/issues/3) | Set up Rust workspace: `astroforge-core` (engine), `astroforge-ai` (ONNX), `astroforge-app` (Tauri) | §4 | done | T1 |
| P0-M1-T4 | [#4](https://github.com/emmanuel-a-otchere/AstroForge/issues/4) | Configure CI (GitHub Actions): `cargo fmt`, `cargo clippy`, `cargo test`, `npm run build` on push | §4 | done | T3 |
| P0-M1-T5 | [#5](https://github.com/emmanuel-a-otchere/AstroForge/issues/5) | Add `.gitignore`, `.editorconfig`, `rust-toolchain.toml`, `prettier` config | — | done | T1 |
| P0-M1-T6 | [#6](https://github.com/emmanuel-a-otchere/AstroForge/issues/6) | Create placeholder UI: app window with "AstroForge" branding, empty workspace | §4 | done | T2 |

### Milestone 0.2 — Core Architecture Scaffolding

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P0-M2-T1 | [#7](https://github.com/emmanuel-a-otchere/AstroForge/issues/7) | Define `Stage` trait and `PipelineDag` structure in `astroforge-core` | §4, §7 | done | M1-T3 |
| P0-M2-T2 | [#8](https://github.com/emmanuel-a-otchere/AstroForge/issues/8) | Implement `ArtifactStore` (filesystem + metadata) with FITS/TIFF write stubs | §4, §17 | done | M1-T3 |
| P0-M2-T3 | [#9](https://github.com/emmanuel-a-otchere/AstroForge/issues/9) | Set up SQLite schema for project/session state (projects, sessions, stages, checkpoints) | §14.5 | done | M1-T3 |
| P0-M2-T4 | [#10](https://github.com/emmanuel-a-otchere/AstroForge/issues/10) | Implement `Orchestrator` skeleton: DAG runner with pause/resume/checkpoint stubs | §4 | done | T1, T3 |
| P0-M2-T5 | [#11](https://github.com/emmanuel-a-otchere/AstroForge/issues/11) | Define IPC contract between frontend and Rust backend (Tauri commands/events) | §4 | done | M1-T1 |
| P0-M2-T6 | [#12](https://github.com/emmanuel-a-otchere/AstroForge/issues/12) | Implement WebGPU capability probe with Canvas2D fallback selection | §4 | done | M1-T2 |

### Milestone 0.3 — FITS I/O Foundation

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P0-M3-T1 | [#13](https://github.com/emmanuel-a-otchere/AstroForge/issues/13) | Integrate `fitsrs` / `cfitsio` bindings for FITS read/write | §4, §5 | done | M2-T2 |
| P0-M3-T2 | [#14](https://github.com/emmanuel-a-otchere/AstroForge/issues/14) | Implement FITS header parser: extract `IMAGETYP`, `EXPTIME`, `FILTER`, `DATE-OBS`, `CCD-TEMP`, `BAYERPAT`, `XBAYROFF`, `YBAYROFF` | §5.1, §5.2 | done | T1 |
| P0-M3-T3 | [#15](https://github.com/emmanuel-a-otchere/AstroForge/issues/15) | Implement 32-bit float image buffer type (`F32Image`) with ndarray backing | §4 | done | M1-T3 |
| P0-M3-T4 | [#16](https://github.com/emmanuel-a-otchere/AstroForge/issues/16) | Write unit tests for FITS read/write round-trip with sample files | §5 | done | T1, T3 |

---

## Phase 1 — MVP Core Pipeline

**Goal:** End-to-end deep-sky processing from FITS light+dark+flat folder to
exported 16-bit TIFF, runnable on a 4 GB machine, with beginner dialog mode.

**Exit criterion (Definition of Done — from spec §16):**
1. Drop a FITS light+dark+flat folder → exported 16-bit TIFF.
2. Kappa-sigma stack of ≥30 frames on a 4 GB machine without OOM.
3. Beginner dialog mode passes a scripted smoke test on Windows + macOS.

**Status:** Core algorithms implemented in Rust (calibration, registration,
stacking, stretching, narrowband, planetary). Frontend has file intake wizard
with auto-detection of focal length and object type. Pipeline not yet wired
end-to-end through the UI.

### Milestone 1.1 — Ingest & Classification

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P1-M1-T1 | [#17](https://github.com/emmanuel-a-otchere/AstroForge/issues/17) | Implement folder scan: recursive directory walk, file classification via FITS headers | §5.1, §5.3 | done | P0-M3-T2 |
| P1-M1-T2 | [#18](https://github.com/emmanuel-a-otchere/AstroForge/issues/18) | Implement auto-classification fallback (exposure-based: Bias/Dark/Flat/Light) | §5.3 | done | T1 |
| P1-M1-T3 | [#19](https://github.com/emmanuel-a-otchere/AstroForge/issues/19) | Group lights by filter and binning | §5.3 | done | T1 |
| P1-M1-T4 | [#20](https://github.com/emmanuel-a-otchere/AstroForge/issues/20) | Build session manifest data structure (SQLite-backed) | §5.1, §14.5 | done | P0-M2-T3 |
| P1-M1-T5 | [#21](https://github.com/emmanuel-a-otchere/AstroForge/issues/21) | Implement "What did you shoot?" initial dialog (target name, camera type, focal length, lights-only toggle) | §5.4 | done | P0-M2-T5 |
| P1-M1-T6 | [#22](https://github.com/emmanuel-a-otchere/AstroForge/issues/22) | Implement classification confirmation dialog with sortable override | §5.3 | done | T2, T5 |
| P1-M1-T7 | — | Implement auto-detection of focal length and object type from FITS/EXIF headers | §5.4, §6 | done | T5 |

### Milestone 1.2 — Calibration

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P1-M2-T1 | [#23](https://github.com/emmanuel-a-otchere/AstroForge/issues/23) | Implement master dark builder (sigma-clipped median, exposure & temp scaling) | §7 Stage 4 | done | P1-M1-T1 |
| P1-M2-T2 | [#24](https://github.com/emmanuel-a-otchere/AstroForge/issues/24) | Implement master flat builder (normalized, sigma-clipped) | §7 Stage 4 | done | P1-M1-T1 |
| P1-M2-T3 | [#25](https://github.com/emmanuel-a-otchere/AstroForge/issues/25) | Implement master bias builder | §7 Stage 4 | done | P1-M1-T1 |
| P1-M2-T4 | [#26](https://github.com/emmanuel-a-otchere/AstroForge/issues/26) | Implement calibration application: `(Light − MasterDark) / MasterFlat` | §7 Stage 4 | done | T1, T2, T3 |
| P1-M2-T5 | [#27](https://github.com/emmanuel-a-otchere/AstroForge/issues/27) | Handle "lights only" path (skip dark, apply flat if present) | §7 Stage 4 | done | T4 |
| P1-M2-T6 | [#28](https://github.com/emmanuel-a-otchere/AstroForge/issues/28) | Streaming calibration: process one frame at a time, no full-session RAM hold | §12 | done | T4 |

### Milestone 1.3 — Registration & Stacking

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P1-M3-T1 | [#29](https://github.com/emmanuel-a-otchere/AstroForge/issues/29) | Implement star extraction (multiscale Laplacian + centroiding) | §7 Stage 6 | done | P1-M2-T6 |
| P1-M3-T2 | [#30](https://github.com/emmanuel-a-otchere/AstroForge/issues/30) | Implement auto-reference frame selection (best FWHM + central target) | §7 Stage 6 | done | T1 |
| P1-M3-T3 | [#31](https://github.com/emmanuel-a-otchere/AstroForge/issues/31) | Implement affine/similarity transform computation per frame | §7 Stage 6 | done | T1, T2 |
| P1-M3-T4 | [#32](https://github.com/emmanuel-a-otchere/AstroForge/issues/32) | Implement sub-pixel cross-correlation on star cutouts | §7 Stage 6 | done | T1 |
| P1-M3-T5 | [#33](https://github.com/emmanuel-a-otchere/AstroForge/issues/33) | Implement Kappa-Sigma clip stacking algorithm | §7 Stage 7 | done | T3 |
| P1-M3-T6 | [#34](https://github.com/emmanuel-a-otchere/AstroForge/issues/34) | Implement stacking accumulator (streaming, bounded memory) | §7 Stage 7, §12 | done | T5 |
| P1-M3-T7 | [#35](https://github.com/emmanuel-a-otchere/AstroForge/issues/35) | Output 32-bit float stack + weight map | §7 Stage 7 | done | T6 |

### Milestone 1.4 — Stretching & Export

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P1-M4-T1 | [#36](https://github.com/emmanuel-a-otchere/AstroForge/issues/36) | Implement basic non-linear stretch (histogram transfer / arcsinh) | §7 Stage 11 | done | P1-M3-T7 |
| P1-M4-T2 | [#37](https://github.com/emmanuel-a-otchere/AstroForge/issues/37) | Implement interactive histogram dialog | §7 Stage 11, §9 | done | T1, P0-M2-T5 |
| P1-M4-T3 | [#38](https://github.com/emmanuel-a-otchere/AstroForge/issues/38) | Implement 16-bit TIFF export | §7 Stage 17 | done | P1-M3-T7 |
| P1-M4-T4 | [#39](https://github.com/emmanuel-a-otchere/AstroForge/issues/39) | Implement processing report generation (frame stats, rejections, parameters) | §14 | done | P1-M1-T4 |

### Milestone 1.5 — Beginner Dialog Mode & Smoke Test

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P1-M5-T1 | [#40](https://github.com/emmanuel-a-otchere/AstroForge/issues/40) | Implement Auto mode (defaults, no prompts) for all MVP stages | §9 | done | M4-T3 |
| P1-M5-T2 | [#41](https://github.com/emmanuel-a-otchere/AstroForge/issues/41) | Implement beginner verbosity level (mostly Auto) | §9 | done | T1 |
| P1-M5-T3 | [#42](https://github.com/emmanuel-a-otchere/AstroForge/issues/42) | Wire end-to-end pipeline: ingest → calibrate → register → stack → stretch → export | §7 | done | M4-T3 |
| P1-M5-T4 | [#43](https://github.com/emmanuel-a-otchere/AstroForge/issues/43) | Write scripted smoke test: FITS folder → TIFF on Windows | §16 DoD | done | T3 |
| P1-M5-T5 | [#44](https://github.com/emmanuel-a-otchere/AstroForge/issues/44) | Write scripted smoke test: FITS folder → TIFF on macOS | §16 DoD | done | T3 |
| P1-M5-T6 | [#45](https://github.com/emmanuel-a-otchere/AstroForge/issues/45) | Memory test: 30-frame stack on 4 GB configuration without OOM | §16 DoD, §2 | done | T3 |

---

## Phase 1.5 — Guided Processing Train (CR: AF-CR-2026-09-01-IMG-PIPELINE)

**Goal:** Transform the MVP pipeline into an interactive, non-destructive image
processing train with three operating modes (Automagic, Automagic Expert, Pure
Expert), live GPU-accelerated preview, and backend AI support for every stage.

**Exit criterion:** A user can load a stacked image, process it through all 10
canonical stages in any of the three modes, undo/redo any stage including after
export, and see a stable live preview with real-time shader effects. Star
separation and replace is mathematically exact (verifiable by difference maps).

**CR reference:** [AF-CR-2026-09-01-IMG-PIPELINE](./CR_AF-CR-2026-09-01-IMG-PIPELINE.md)

### Milestone 1.5.1 — Processing Train State Machine & Data Model

> The central "brain": a versioned state machine that keeps the UI wizard steps
> synced with the underlying DAG. Every stage emits a versioned artefact, a
> receipt/log entry, and updated image statistics.

| ID | Task | CR ref | Status | Depends on |
|---|---|---|---|---|
| P1.5-M1-T1 | [#137](https://github.com/emmanuel-a-otchere/AstroForge/issues/137) | Define `ProcessingMode` type (Automagic / Automagic Expert / Pure Expert) and session-level mode state | §4.3 | done | P1-M5-T3 |
| P1.5-M1-T2 | [#138](https://github.com/emmanuel-a-otchere/AstroForge/issues/138) | Define `PipelineNode` and `PipelineGraph` TypeScript types matching CR JSON model (nodes with id, type, params, status; edges with from/to) | §4.1, §C.1 | done | T1 |
| P1.5-M1-T3 | [#139](https://github.com/emmanuel-a-otchere/AstroForge/issues/139) | Implement session state store (Svelte writable store) holding: session_id, current_mode, active_step_index, pipeline_graph, history_stack | §C.1 | done | T2 |
| P1.5-M1-T4 | [#140](https://github.com/emmanuel-a-otchere/AstroForge/issues/140) | Implement "Next Button" action logic: commit params to current node → append next node → wire edge → advance step index | §C.2 | done | T3 |
| P1.5-M1-T5 | [#141](https://github.com/emmanuel-a-otchere/AstroForge/issues/141) | Implement undo/redo history stack: every stage commit pushes a versioned snapshot (params + pixel ref); undo restores exact prior state | §4.2 | done | T3 |
| P1.5-M1-T6 | [#142](https://github.com/emmanuel-a-otchere/AstroForge/issues/142) | Implement mode-switch logic with confirmation: keep current pixel state OR re-process from chosen stage under new mode | §4.3 | done | T1, T5 |
| P1.5-M1-T7 | [#143](https://github.com/emmanuel-a-otchere/AstroForge/issues/143) | Implement stage receipt/log system: each stage emits human-readable entry with parameters, timing, warnings | §4.1 | done | T3 |
| P1.5-M1-T8 | [#144](https://github.com/emmanuel-a-otchere/AstroForge/issues/144) | Implement crash-safe autosave: persist session state (mode, history, intermediate refs) to local rusqlite (Tauri-side) on every stage commit | §5 NFR | done | T5 |

### Milestone 1.5.2 — PreviewCanvas & Live Preview System

> The central image canvas that never unmounts during UI transitions.
> Hardware-accelerated via WebGL/WebGPU shaders for zero-latency slider feedback.

| ID | Task | CR ref | Status | Depends on |
|---|---|---|---|---|
| P1.5-M2-T1 | [#145](https://github.com/emmanuel-a-otchere/AstroForge/issues/145) | Implement `PreviewCanvas` Svelte component: persistent DOM element that survives wizard/forge mode transitions, renders to WebGL context | §A.1 | pending | P1.5-M1-T3 |
| P1.5-M2-T2 | [#146](https://github.com/emmanuel-a-otchere/AstroForge/issues/146) | Implement WebGL rendering pipeline: texture upload from F32Image, full-screen quad, fragment shader output | §B | done | T1 |
| P1.5-M2-T3 | [#147](https://github.com/emmanuel-a-otchere/AstroForge/issues/147) | Implement MTF (Midtones Transfer Function) stretch shader in GLSL — black point clipping + midtone transfer per channel | §B.2 | done | T2 |
| P1.5-M2-T4 | [#148](https://github.com/emmanuel-a-otchere/AstroForge/issues/148) | Implement SCNR "Green-be-Gone" shader in GLSL — reduce green channel to min(R,B) with strength slider blend | §B.1 | pending | T2 |
| P1.5-M2-T5 | [#149](https://github.com/emmanuel-a-otchere/AstroForge/issues/149) | Implement preview statistics stability: denoise/sharpen shaders must not alter display stretch statistics (separate display stretch from data) | §4.4 | pending | T3, T4 |
| P1.5-M2-T6 | [#150](https://github.com/emmanuel-a-otchere/AstroForge/issues/150) | Implement real-pixel zoom/pan/refit on PreviewCanvas with synced multi-preview grid support | §4.4, §4.1 stage 7 | pending | T2 |
| P1.5-M2-T7 | [#151](https://github.com/emmanuel-a-otchere/AstroForge/issues/151) | Implement "Hold to Compare" and side-by-side original vs current view at any stage | §4.2 | pending | T2 |
| P1.5-M2-T8 | [#152](https://github.com/emmanuel-a-otchere/AstroForge/issues/152) | Implement debounced full-resolution render: preview renders at reduced res during slider drag, full res on rest | §4.4 | pending | T2 |

### Milestone 1.5.3 — Wizard Mode UI (Bottom Sheet)

> The beginner-friendly guided stepper. Maps to the portrait mockup bottom sheet.

| ID | Task | CR ref | Status | Depends on |
|---|---|---|---|---|
| P1.5-M3-T1 | [#153](https://github.com/emmanuel-a-otchere/AstroForge/issues/153) | Implement `WizardBottomSheet` component: stepper (step N of 10), large strength slider, Next/Back buttons | §A.1 | pending | P1.5-M1-T4, P1.5-M2-T1 |
| P1.5-M3-T2 | [#154](https://github.com/emmanuel-a-otchere/AstroForge/issues/154) | Implement stage-specific parameter panels that appear inside the bottom sheet per active step | §4.1 | pending | T1 |
| P1.5-M3-T3 | [#155](https://github.com/emmanuel-a-otchere/AstroForge/issues/155) | Implement "Reveal Pipeline / Expert Mode" toggle in top nav bar | §A.2 | pending | T1 |
| P1.5-M3-T4 | [#156](https://github.com/emmanuel-a-otchere/AstroForge/issues/156) | Implement wizard-to-forge transition animation: bottom sheet slides down + fades out, canvas shrinks, sidebars slide in, active step morphs into selected node | §A.2 | pending | T1, P1.5-M4-T1 |
| P1.5-M3-T5 | [#157](https://github.com/emmanuel-a-otchere/AstroForge/issues/157) | Implement Automagic mode UI: single "Process" button, per-stage "Auto" buttons, progress + final result only, hidden granularity | §4.3 | pending | T1, T2 |
| P1.5-M3-T6 | [#158](https://github.com/emmanuel-a-otchere/AstroForge/issues/158) | Implement mode indicator badge (persistent, always visible, colour-coded per mode) | §4.3, §7 | pending | T1 |

### Milestone 1.5.4 — Forge Mode UI (Node Graph + Sidebars)

> The advanced node-based compositor view. Maps to the landscape mockup sidebars.

| ID | Task | CR ref | Status | Depends on |
|---|---|---|---|---|
| P1.5-M4-T1 | [#159](https://github.com/emmanuel-a-otchere/AstroForge/issues/159) | Implement `NodeSidebar` component: visual DAG with nodes (stages), edges (connections), status colours, active node highlight in accent colour | §A.1, §C.1 | pending | P1.5-M1-T2 |
| P1.5-M4-T2 | [#160](https://github.com/emmanuel-a-otchere/AstroForge/issues/160) | Implement `ParameterSidebar` component: full parameter panel for selected node, all controls exposed | §A.1 | pending | T1 |
| P1.5-M4-T3 | [#161](https://github.com/emmanuel-a-otchere/AstroForge/issues/161) | Implement node selection → parameter sidebar sync: clicking a node in the graph loads its params in the sidebar and updates the preview canvas | §C | pending | T1, T2, P1.5-M2-T1 |
| P1.5-M4-T4 | [#162](https://github.com/emmanuel-a-otchere/AstroForge/issues/162) | Implement Pure Expert mode UI: every control, sub-parameter, mask, and intermediate buffer exposed; manual sub-step sequencing | §4.3 | pending | T2 |
| P1.5-M4-T5 | [#163](https://github.com/emmanuel-a-otchere/AstroForge/issues/163) | Implement Automagic Expert mode UI: AI proposals in dialogs with live preview, accept/reject/refine, "Apply equally to selected" batch control | §4.3 | pending | T2, P1.5-M5-T1 |

### Milestone 1.5.5 — Backend AI Service Layer

> Unified interface so any pipeline stage can request analysis, parameter
> suggestion, or full execution from the AI backend.

| ID | Task | CR ref | Status | Depends on |
|---|---|---|---|---|
| P1.5-M5-T1 | [#164](https://github.com/emmanuel-a-otchere/AstroForge/issues/164) | Define `AIService` interface: `analyse(image, stage) → AnalysisResult`, `suggestParams(image, stage, dataType) → ParamProposal`, `execute(image, stage, params) → ProcessedImage` | §4.3, §8 | pending | P1.5-M1-T3 |
| P1.5-M5-T2 | [#165](https://github.com/emmanuel-a-otchere/AstroForge/issues/165) | Implement AI service dispatch: route requests to local ONNX models, remote engines, or CPU fallback based on mode + hardware | §4.3 | pending | T1 |
| P1.5-M5-T3 | [#166](https://github.com/emmanuel-a-otchere/AstroForge/issues/166) | Implement graceful degradation: if AI engine fails, fall back to algorithmic defaults and surface a clear warning | §4.3, §4.4 | pending | T2 |
| P1.5-M5-T4 | [#167](https://github.com/emmanuel-a-otchere/AstroForge/issues/167) | Implement AI status + progress reporting to UI: measured progress, estimated time, engine name, quality tier | §5 NFR | pending | T2 |
| P1.5-M5-T5 | [#168](https://github.com/emmanuel-a-otchere/AstroForge/issues/168) | Implement free-path vs accelerated-path selection with transparent messaging | §4.3, §5 NFR | pending | T2 |

### Milestone 1.5.6 — Canonical Pipeline Stages (10-Stage Train)

> The strictly ordered processing train. Each stage produces a versioned,
> reversible result. Stages 7 (Stretch) and 8 (Star Handling) are prioritised
> per CR §8 implementation notes.

| ID | Task | CR ref | Status | Depends on |
|---|---|---|---|---|
| P1.5-M6-T1 | [#169](https://github.com/emmanuel-a-otchere/AstroForge/issues/169) | Stage 1 — Ingest & Analyse: load FITS/TIFF/XISF, auto-detect camera type, filter set, bit depth, linear vs stretched, basic stats; produce data-type declaration | §4.1 stage 1 | pending | P1.5-M1-T7 |
| P1.5-M6-T2 | [#170](https://github.com/emmanuel-a-otchere/AstroForge/issues/170) | Stage 2 — Framing / Crop / Rotate: interactive free-select crop, live rotation, aspect-ratio presets, meridian-flip awareness; explicit (never silent auto-crop) | §4.1 stage 2 | pending | T1, P1.5-M2-T6 |
| P1.5-M6-T3 | [#171](https://github.com/emmanuel-a-otchere/AstroForge/issues/171) | Stage 3 — Gradient / Background Extraction: 2D polynomial/spline model, nebulosity mask, live preview | §4.1 stage 3 | pending | T1 |
| P1.5-M6-T4 | [#172](https://github.com/emmanuel-a-otchere/AstroForge/issues/172) | Stage 4 — Colour Calibration / Balance: bounded corrections, dual-band and mono-aware, clear labelling | §4.1 stage 4 | pending | T1 |
| P1.5-M6-T5 | [#173](https://github.com/emmanuel-a-otchere/AstroForge/issues/173) | Stage 5 — Sharpen / Deconvolution: Richardson-Lucy or van Cittert with PSF from stars, live preview | §4.1 stage 5 | pending | T1 |
| P1.5-M6-T6 | [#174](https://github.com/emmanuel-a-otchere/AstroForge/issues/174) | Stage 6 — Denoise: SwinIR or wavelet fallback, preview-stable (no stat shift), live preview | §4.1 stage 6 | pending | T1, P1.5-M2-T5 |
| P1.5-M6-T7 | [#175](https://github.com/emmanuel-a-otchere/AstroForge/issues/175) | Stage 7 — Stretch: data-anchored "Deep" engine, multi-preview grid (Soft/Normal/Aggressive/Deep/Deep-keep-colours/Custom), "Keep this look" commit | §4.1 stage 7, §4.4 | pending | P1.5-M2-T3, P1.5-M2-T6 |
| P1.5-M6-T8 | [#176](https://github.com/emmanuel-a-otchere/AstroForge/issues/176) | Stage 8 — Star Handling: separation → independent starless/stars layers → exact or soft replace with strength + colour-boost; mathematically exact (verifiable by difference maps) | §4.1 stage 8, §4.4, §6 AC | pending | T7, P1.5-M5-T1 |
| P1.5-M6-T9 | [#177](https://github.com/emmanuel-a-otchere/AstroForge/issues/177) | Stage 9 — Creative / Final Polish: curves (saturation channel, colour-family targeting), colour-transmutation spells with editable recipes, narrowband palette mixes, tone + detail | §4.1 stage 9 | pending | T7 |
| P1.5-M6-T10 | [#178](https://github.com/emmanuel-a-otchere/AstroForge/issues/178) | Stage 10 — Export: multi-format (FITS master, TIFF, JPEG, starless, stars-only), non-destructive (session continues), success/failure messaging | §4.1 stage 10 | pending | T9 |

### Milestone 1.5.7 — Non-Destructive Editing & History

> Full history stack, parameter/mask separation, multi-save, and re-apply logic.

| ID | Task | CR ref | Status | Depends on |
|---|---|---|---|---|
| P1.5-M7-T1 | [#179](https://github.com/emmanuel-a-otchere/AstroForge/issues/179) | Implement versioned artefact store: each stage commit stores params + mask separately from pixel data | §4.2 | pending | P1.5-M1-T5 |
| P1.5-M7-T2 | [#180](https://github.com/emmanuel-a-otchere/AstroForge/issues/180) | Implement "re-apply from here": re-running an earlier stage re-executes it and all downstream stages with current params | §4.2 | pending | T1 |
| P1.5-M7-T3 | [#181](https://github.com/emmanuel-a-otchere/AstroForge/issues/181) | Implement multi-save: export multiple formats/versions without terminating session | §4.2 | pending | P1.5-M6-T10 |
| P1.5-M7-T4 | [#182](https://github.com/emmanuel-a-otchere/AstroForge/issues/182) | Implement explicit warning when re-running an already-applied AI or irreversible-looking step | §4.2 | pending | T1 |
| P1.5-M7-T5 | [#183](https://github.com/emmanuel-a-otchere/AstroForge/issues/183) | Implement exact reversibility for crop, stretch, and star-replace (restore exact pre-operation state) | §4.2, §6 AC3 | pending | T1, P1.5-M6-T7, P1.5-M6-T8 |

### Milestone 1.5.8 — Smart-Telescope & Data-Type Awareness

> Header/filename dialect recognition for common devices; data-type-driven guidance.

| ID | Task | CR ref | Status | Depends on |
|---|---|---|---|---|
| P1.5-M8-T1 | [#184](https://github.com/emmanuel-a-otchere/AstroForge/issues/184) | Implement smart-telescope device detection from FITS headers and filenames (Seestar, Dwarf family, etc.) | §4.4 | pending | P1.5-M6-T1 |
| P1.5-M8-T2 | [#185](https://github.com/emmanuel-a-otchere/AstroForge/issues/185) | Implement data-type declaration: OSC / dual-band / mono Ha/OIII/SII/LRGB, bit depth, linear vs stretched | §4.1 stage 1 | pending | T1 |
| P1.5-M8-T3 | [#186](https://github.com/emmanuel-a-otchere/AstroForge/issues/186) | Implement data-type-aware guidance: mode-specific tooltips, calibration decisions, and filter naming conventions | §4.4 | pending | T1, T2 |
| P1.5-M8-T4 | [#187](https://github.com/emmanuel-a-otchere/AstroForge/issues/187) | Implement honest feedback system: surface all warnings (hot pixels, blank frames, already-applied steps, imperfect alignment, linear data) as actionable messages | §4.4 | pending | T1 |

---

## Phase 2 — Full Deep-Sky Pipeline

**Goal:** All deep-sky pipeline stages from the spec are functional, including
AI model integration, narrowband composition, and plate solving.

**Exit criterion:** A user can process a multi-filter narrowband FITS session
through the full pipeline (stages 0.5–17) with AI denoising and
super-resolution, and export a finished image with a shareable recipe.

### Milestone 2.1 — AI Model Hub Integration

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P2-M1-T1 | [#46](https://github.com/emmanuel-a-otchere/AstroForge/issues/46) | Integrate ONNX Runtime (`ort` crate) with backend auto-selection (CPU, CUDA, DirectML, CoreML) | §10 | pending | Phase 1.5 |
| P2-M1-T2 | [#47](https://github.com/emmanuel-a-otchere/AstroForge/issues/47) | Implement model registry: download, SHA-256 verify, signed manifest check | §10.8 | pending | T1 |
| P2-M1-T3 | [#48](https://github.com/emmanuel-a-otchere/AstroForge/issues/48) | Implement tiling inference engine (512px tiles, 64px overlap, cosine blend) | §10.4 | pending | T1 |
| P2-M1-T4 | [#49](https://github.com/emmanuel-a-otchere/AstroForge/issues/49) | Implement hardware probe and quality tier selection (Fast/Balanced/Research/Perceptual) | §10.5 | pending | T1 |
| P2-M1-T5 | [#50](https://github.com/emmanuel-a-otchere/AstroForge/issues/50) | Integrate `swinir-denoise-astro` model (Stage 13) | §10.3, §7 Stage 13 | pending | T3 |
| P2-M1-T6 | [#51](https://github.com/emmanuel-a-otchere/AstroForge/issues/51) | Integrate `swinir-sr-astro-2x` model (Stage 15) | §10.3, §7 Stage 15 | pending | T3 |
| P2-M1-T7 | [#52](https://github.com/emmanuel-a-otchere/AstroForge/issues/52) | Integrate `swin2sr-dejpeg` model (Stage 0.5) | §10.3, §7 Stage 0.5 | pending | T3 |
| P2-M1-T8 | [#53](https://github.com/emmanuel-a-otchere/AstroForge/issues/53) | Integrate `star-seg-v1` model (Stage 14) | §10.3, §7 Stage 14 | pending | T3 |
| P2-M1-T9 | [#54](https://github.com/emmanuel-a-otchere/AstroForge/issues/54) | Integrate `cloud-score-v1` model (Stage 2) | §10.3, §7 Stage 2 | pending | T3 |
| P2-M1-T10 | [#55](https://github.com/emmanuel-a-otchere/AstroForge/issues/55) | Integrate `color-cal-net` model (Stage 9) | §10.3, §7 Stage 9 | pending | T3 |
| P2-M1-T11 | [#56](https://github.com/emmanuel-a-otchere/AstroForge/issues/56) | Integrate `trail-lama-tiny` model (Stage 14) | §10.3, §7 Stage 14 | pending | T3 |
| P2-M1-T12 | [#57](https://github.com/emmanuel-a-otchere/AstroForge/issues/57) | Implement determinism recording (model hash, backend, tile size, seed) | §10.6 | pending | T5 |

### Milestone 2.2 — Remaining Deep-Sky Stages

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P2-M2-T1 | [#58](https://github.com/emmanuel-a-otchere/AstroForge/issues/58) | Implement quality filter stage (FWHM, eccentricity, star count, SNR, background, cloud score; auto-reject worst 15%) | §7 Stage 2 | pending | P2-M1-T9 |
| P2-M2-T2 | [#59](https://github.com/emmanuel-a-otchere/AstroForge/issues/59) | Implement debayer stage (VNG + bilinear, camera white balance from metadata) | §7 Stage 3 | pending | Phase 1 |
| P2-M2-T3 | [#60](https://github.com/emmanuel-a-otchere/AstroForge/issues/60) | Implement cosmetic correction (hot/cold pixel detection, sigma-clip, interpolation) | §7 Stage 5 | pending | P1-M2-T6 |
| P2-M2-T4 | [#61](https://github.com/emmanuel-a-otchere/AstroForge/issues/61) | Implement background extraction (2D polynomial/spline model, nebulosity mask) | §7 Stage 8 | pending | P1-M3-T7 |
| P2-M2-T5 | [#62](https://github.com/emmanuel-a-otchere/AstroForge/issues/62) | Implement color calibration (white-balance via reference stars or user-picked neutral region) | §7 Stage 9 | pending | P2-M1-T10 |
| P2-M2-T6 | [#63](https://github.com/emmanuel-a-otchere/AstroForge/issues/63) | Implement crop and rotate (manual crop, rotate to cardinal, edge removal) | §7 Stage 10 | pending | P1-M4-T1 |
| P2-M2-T7 | [#64](https://github.com/emmanuel-a-otchere/AstroForge/issues/64) | Implement star segmentation and enhancement (star/background layers, color boost, bloat reduction) | §7 Stage 14 | pending | P2-M1-T8 |
| P2-M2-T8 | [#65](https://github.com/emmanuel-a-otchere/AstroForge/issues/65) | Implement final detail enhancement (multi-scale unsharp mask, local contrast) | §7 Stage 16 | pending | P1-M4-T1 |
| P2-M2-T9 | [#66](https://github.com/emmanuel-a-otchere/AstroForge/issues/66) | Implement full export (TIFF 16/32-bit, PNG, JPEG, XISF with history, sidecar JSON) | §7 Stage 17 | pending | P1-M4-T3 |

### Milestone 2.3 — Narrowband Composition

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P2-M3-T1 | [#67](https://github.com/emmanuel-a-otchere/AstroForge/issues/67) | Implement narrowband detection (≥2 light groups with Ha/OIII/SII filter names) | §7.6 | pending | P1-M1-T3 |
| P2-M3-T2 | [#68](https://github.com/emmanuel-a-otchere/AstroForge/issues/68) | Implement channel extraction from OSC (Ha→Red, SII→Red, OIII→Blue+Green) | §7.6 | pending | P2-M2-T2 |
| P2-M3-T3 | [#69](https://github.com/emmanuel-a-otchere/AstroForge/issues/69) | Implement inter-filter group registration | §7.6 | pending | P1-M3-T3 |
| P2-M3-T4 | [#70](https://github.com/emmanuel-a-otchere/AstroForge/issues/70) | Implement composition palettes (HOO, SHO, HSO, Custom) | §7.6 | pending | T3 |
| P2-M3-T5 | [#71](https://github.com/emmanuel-a-otchere/AstroForge/issues/71) | Implement SCNR (Subtractive Color Noise Reduction) per palette | §7 Stage 9.5 | pending | T4 |
| P2-M3-T6 | [#72](https://github.com/emmanuel-a-otchere/AstroForge/issues/72) | Implement channel ratio normalization (Ha:OIII balance) | §7 Stage 9.5 | pending | T4 |

### Milestone 2.4 — Plate Solving

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P2-M4-T1 | [#73](https://github.com/emmanuel-a-otchere/AstroForge/issues/73) | Evaluate plate-solve dependency: bundle ASTAP vs. online astrometry.net vs. defer | §17 item 8 | pending | — |
| P2-M4-T2 | [#74](https://github.com/emmanuel-a-otchere/AstroForge/issues/74) | Integrate ASTAP binary (bundled, offline star catalogs) | §7 Stage 6.5 | pending | T1 |
| P2-M4-T3 | [#75](https://github.com/emmanuel-a-otchere/AstroForge/issues/75) | Implement WCS output to FITS header | §7 Stage 6.5 | pending | T2 |
| P2-M4-T4 | [#76](https://github.com/emmanuel-a-otchere/AstroForge/issues/76) | Implement auto-crop to subject using WCS | §7 Stage 10 | pending | T3, P2-M2-T6 |
| P2-M4-T5 | [#77](https://github.com/emmanuel-a-otchere/AstroForge/issues/77) | Implement annotated star map overlay | §7 Stage 6.5 | pending | T3 |
| P2-M4-T6 | [#78](https://github.com/emmanuel-a-otchere/AstroForge/issues/78) | Implement photometric calibration anchoring (APASS/Gaia) | §7 Stage 9 | pending | T3, P2-M2-T5 |
| P2-M4-T7 | [#79](https://github.com/emmanuel-a-otchere/AstroForge/issues/79) | Implement graceful failure (skip if solve fails, registration still works) | §7 Stage 6.5 | pending | T2 |

### Milestone 2.5 — DIP Optional Stages

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P2-M5-T1 | [#80](https://github.com/emmanuel-a-otchere/AstroForge/issues/80) | Implement DIP deconvolution (zero-shot, PSF as forward operator) | §7 Stage 12, §10.7 | pending | P2-M1-T1 |
| P2-M5-T2 | [#81](https://github.com/emmanuel-a-otchere/AstroForge/issues/81) | Implement DIP denoise (linear data, blended in linear space) | §7 Stage 13, §10.7 | pending | P2-M1-T1 |
| P2-M5-T3 | [#82](https://github.com/emmanuel-a-otchere/AstroForge/issues/82) | Implement DIP inpaint (high-quality trail removal) | §7 Stage 14, §10.7 | pending | P2-M1-T1 |
| P2-M5-T4 | [#83](https://github.com/emmanuel-a-otchere/AstroForge/issues/83) | Implement DIP defaults (iteration count, early-stopping heuristic) | §17 item 6 | pending | T1 |

### Milestone 2.6 — Dialog Modes & Session Management

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P2-M6-T1 | [#84](https://github.com/emmanuel-a-otchere/AstroForge/issues/84) | Implement Confirm mode (preview + metrics, OK/Adjust/Skip) | §9 | pending | Phase 1.5 |
| P2-M6-T2 | [#85](https://github.com/emmanuel-a-otchere/AstroForge/issues/85) | Implement Manual mode (full parameter panel) | §9 | pending | T1 |
| P2-M6-T3 | [#86](https://github.com/emmanuel-a-otchere/AstroForge/issues/86) | Implement Intermediate and Expert verbosity levels | §9 | pending | T1, T2 |
| P2-M6-T4 | [#87](https://github.com/emmanuel-a-otchere/AstroForge/issues/87) | Implement before/after slider, histogram overlay, "revert to auto," "save as preset" | §9 | pending | T1 |
| P2-M6-T5 | [#88](https://github.com/emmanuel-a-otchere/AstroForge/issues/88) | Implement project/session persistence (SQLite: DAG state, checkpoint refs) | §14.5 | pending | P0-M2-T3 |
| P2-M6-T6 | [#89](https://github.com/emmanuel-a-otchere/AstroForge/issues/89) | Implement crash recovery (detect interrupted project on launch, offer resume) | §14.5 | pending | T5 |
| P2-M6-T7 | [#90](https://github.com/emmanuel-a-otchere/AstroForge/issues/90) | Implement checkpointing (intermediate FITS per stage, crash-safe) | §12 | pending | T5 |

---

## Phase 3 — Planetary, Recipes & Polish

**Goal:** Planetary/lunar pipeline functional, recipe sharing works, all dialog
modes complete, cross-platform packaging ready.

**Exit criterion:** A user can process a planetary lucky-imaging session, export
a recipe, import it on another machine, and reproduce the result. App is
packaged for Windows (.msix) and macOS (.dmg).

### Milestone 3.1 — Planetary / Lunar Pipeline

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P3-M1-T1 | [#91](https://github.com/emmanuel-a-otchere/AstroForge/issues/91) | Implement planetary routing (exposure < 2s + frame_count > 500) | §6.2 | pending | Phase 1.5 |
| P3-M1-T2 | [#92](https://github.com/emmanuel-a-otchere/AstroForge/issues/92) | Implement feature tracking / limb detection for registration | §8 | pending | P1-M3-T1 |
| P3-M1-T3 | [#93](https://github.com/emmanuel-a-otchere/AstroForge/issues/93) | Implement lucky imaging: rank by sharpness, stack best 10–30% | §8 | pending | T2 |
| P3-M1-T4 | [#94](https://github.com/emmanuel-a-otchere/AstroForge/issues/94) | Implement streaming two-pass rank/select for 50,000-frame ingest | §8, §17 item 10 | pending | T3 |
| P3-M1-T5 | [#95](https://github.com/emmanuel-a-otchere/AstroForge/issues/95) | Implement planetary drizzle | §8 | pending | T3 |
| P3-M1-T6 | [#96](https://github.com/emmanuel-a-otchere/AstroForge/issues/96) | Implement planetary stretching (aggressive contrast for surface detail) | §8 | pending | P1-M4-T1 |
| P3-M1-T7 | [#97](https://github.com/emmanuel-a-otchere/AstroForge/issues/97) | Implement planetary sharpening (aggressive unsharp / wavelet) | §8 | pending | T6 |
| P3-M1-T8 | [#98](https://github.com/emmanuel-a-otchere/AstroForge/issues/98) | Implement lunar HDR merge (exposure sets) | §8 | pending | T6 |
| P3-M1-T9 | [#99](https://github.com/emmanuel-a-otchere/AstroForge/issues/99) | Implement DIP-coadd for multi-frame lucky-imaging restoration | §8, §16 v2 | pending | P2-M5-T1 |

### Milestone 3.2 — Recipe System

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P3-M2-T1 | [#100](https://github.com/emmanuel-a-otchere/AstroForge/issues/100) | Define and implement recipe JSON schema (v1.0) | §11.1 | pending | Phase 2 |
| P3-M2-T2 | [#101](https://github.com/emmanuel-a-otchere/AstroForge/issues/101) | Implement recipe export (sanitized: strip paths, GPS, machine info) | §11.2 | pending | T1 |
| P3-M2-T3 | [#102](https://github.com/emmanuel-a-otchere/AstroForge/issues/102) | Implement recipe import (validate compatibility, check required models) | §11.3 | pending | T1 |
| P3-M2-T4 | [#103](https://github.com/emmanuel-a-otchere/AstroForge/issues/103) | Implement recipe application (set parameters, prompt model download) | §11.3, §11.4 | pending | T3 |
| P3-M2-T5 | [#104](https://github.com/emmanuel-a-otchere/AstroForge/issues/104) | Implement integrity badge (tag exports/recipes with perceptual model usage) | §10.6 | pending | T1 |

### Milestone 3.3 — Cross-Platform Packaging

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P3-M3-T1 | [#105](https://github.com/emmanuel-a-otchere/AstroForge/issues/105) | Configure Windows packaging (.msix) with optional CUDA installer | §13 | pending | Phase 2 |
| P3-M3-T2 | [#106](https://github.com/emmanuel-a-otchere/AstroForge/issues/106) | Configure macOS packaging (.dmg, Apple Silicon native, notarization) | §13 | pending | Phase 2 |
| P3-M3-T3 | [#107](https://github.com/emmanuel-a-otchere/AstroForge/issues/107) | Configure Linux packaging (.AppImage + .deb + .rpm + Flatpak) | §13 | pending | Phase 2 |
| P3-M3-T4 | [#108](https://github.com/emmanuel-a-otchere/AstroForge/issues/108) | Implement Tauri updater with signed payloads | §13 | pending | T1, T2 |
| P3-M3-T5 | [#109](https://github.com/emmanuel-a-otchere/AstroForge/issues/109) | Implement opt-in crash-reporting and telemetry (configured on first launch) | §13 | pending | T1 |

### Milestone 3.4 — PNG/JPG/DNG Bayer Detection

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P3-M4-T1 | [#110](https://github.com/emmanuel-a-otchere/AstroForge/issues/110) | Implement statistical Bayer detection (autocorrelation, green variance, camera signature DB) | §5.2 | pending | Phase 1 |
| P3-M4-T2 | [#111](https://github.com/emmanuel-a-otchere/AstroForge/issues/111) | Implement DNG parser (TIFF tags: CFAPattern, CFARepeatPatternDim, BlackLevel, WhiteLevel) | §5.2 | pending | T1 |
| P3-M4-T3 | [#112](https://github.com/emmanuel-a-otchere/AstroForge/issues/112) | Implement Bayer uncertainty prompt (telescope selection / pattern selection) | §5.2 | pending | T1 |
| P3-M4-T4 | [#113](https://github.com/emmanuel-a-otchere/AstroForge/issues/113) | Implement confidence scoring (>0.85 auto, 0.5–0.85 prompt, <0.5 assume RGB) | §5.2 | pending | T1 |

---

## Phase 4 — Ecosystem & Research

**Goal:** Plugin ecosystem, recipe gallery, platform-specific optimizations,
and experimental AI models.

**Exit criterion:** Third-party WASM plugins can add/replace pipeline stages.
Recipe gallery is browsable in-app. Platform-specific GPU backends (Metal, CUDA)
are optimized. StableSR is available as an experimental opt-in plugin.

### Milestone 4.1 — Plugin Architecture

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P4-M1-T1 | [#114](https://github.com/emmanuel-a-otchere/AstroForge/issues/114) | Define plugin API contract (stage add/replace interface) | §15 | pending | Phase 3 |
| P4-M1-T2 | [#115](https://github.com/emmanuel-a-otchere/AstroForge/issues/115) | Implement WASM plugin runtime (capability-scoped sandbox, filesystem allowlist, no network by default) | §15 | pending | T1 |
| P4-M1-T3 | [#116](https://github.com/emmanuel-a-otchere/AstroForge/issues/116) | Implement custom AI model loading (drop ONNX into `~/AstroForge/models/`) | §15 | pending | P2-M1-T2 |
| P4-M1-T4 | [#117](https://github.com/emmanuel-a-otchere/AstroForge/issues/117) | Implement export plugin interface (new output targets) | §15 | pending | T1 |
| P4-M1-T5 | [#118](https://github.com/emmanuel-a-otchere/AstroForge/issues/118) | Evaluate optional Python sidecar (user-installed, not bundled) | §15, §17 item 9 | pending | T1 |

### Milestone 4.2 — Recipe Gallery

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P4-M2-T1 | [#119](https://github.com/emmanuel-a-otchere/AstroForge/issues/119) | Decide hosting: GitHub-based repo vs. static JSON index | §17 item 3 | pending | P3-M2-T1 |
| P4-M2-T2 | [#120](https://github.com/emmanuel-a-otchere/AstroForge/issues/120) | Implement in-app recipe gallery (browsable, filterable by target/equipment/palette) | §11.3 | pending | T1 |
| P4-M2-T3 | [#121](https://github.com/emmanuel-a-otchere/AstroForge/issues/121) | Implement recipe search and filtering | §11.3 | pending | T2 |

### Milestone 4.3 — Platform Optimizations

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P4-M3-T1 | [#122](https://github.com/emmanuel-a-otchere/AstroForge/issues/122) | Optimize macOS Metal backend (GPU FFTs, AI inference) | §13 | pending | Phase 3 |
| P4-M3-T2 | [#123](https://github.com/emmanuel-a-otchere/AstroForge/issues/123) | Optimize Windows CUDA / DirectML backend | §13 | pending | Phase 3 |
| P4-M3-T3 | [#124](https://github.com/emmanuel-a-otchere/AstroForge/issues/124) | Optimize Linux Vulkan / CUDA backend | §13 | pending | Phase 3 |
| P4-M3-T4 | [#125](https://github.com/emmanuel-a-otchere/AstroForge/issues/125) | Profile and optimize memory usage for 4 GB target | §2, §12 | pending | Phase 3 |

### Milestone 4.4 — Experimental AI Models

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P4-M4-T1 | [#126](https://github.com/emmanuel-a-otchere/AstroForge/issues/126) | Integrate StableSR as experimental opt-in plugin (GPU-gated, integrity badge) | §10.3, §10.7 | pending | P4-M1-T2 |
| P4-M4-T2 | [#127](https://github.com/emmanuel-a-otchere/AstroForge/issues/127) | A/B test SwinIR vs Real-ESRGAN on real smart-telescope data | §17 item 7 | pending | P2-M1-T6 |
| P4-M4-T3 | [#128](https://github.com/emmanuel-a-otchere/AstroForge/issues/128) | Evaluate "Star Prior GAN" research for star-core restoration | §10.7 | pending | P4-M1-T2 |
| P4-M4-T4 | [#129](https://github.com/emmanuel-a-otchere/AstroForge/issues/129) | Source/build SwinIR astro fine-tuning dataset | §17 item 5 | pending | P2-M1-T5 |

### Milestone 4.5 — Accessibility & i18n

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P4-M5-T1 | [#130](https://github.com/emmanuel-a-otchere/AstroForge/issues/130) | Decide WCAG AA scope (v1 or deferred) | §17 item 12 | pending | — |
| P4-M5-T2 | [#131](https://github.com/emmanuel-a-otchere/AstroForge/issues/131) | Implement keyboard navigation and screen reader support if in scope | §17 item 12 | pending | T1 |
| P4-M5-T3 | [#132](https://github.com/emmanuel-a-otchere/AstroForge/issues/132) | Implement i18n framework and initial translations if in scope | §17 item 12 | pending | T1 |

---

## Open Decision Points

These are spec open items (§17) that gate specific tasks. They should be
resolved early to avoid blocking.

| # | Issue | Decision | Gates task(s) | Target resolution |
|---|---|---|---|---|
| 1 | [#133](https://github.com/emmanuel-a-otchere/AstroForge/issues/133) | Smart-telescope SDK integration vs. file-only | P4-M2-T1+ | Phase 4 |
| 2 | [#134](https://github.com/emmanuel-a-otchere/AstroForge/issues/134) | Live stacking preview for planetary | P3-M1-T3 | Phase 3 start |
| 3 | [#119](https://github.com/emmanuel-a-otchere/AstroForge/issues/119) | Recipe gallery hosting | P4-M2-T1 | Phase 4 start |
| 4 | [#135](https://github.com/emmanuel-a-otchere/AstroForge/issues/135) | License verification (StableSR, DIP) | P4-M4-T1, P2-M5-T1 | Phase 2 start |
| 5 | [#129](https://github.com/emmanuel-a-otchere/AstroForge/issues/129) | SwinIR fine-tuning dataset | P4-M4-T4 | Phase 4 |
| 6 | [#83](https://github.com/emmanuel-a-otchere/AstroForge/issues/83) | DIP defaults | P2-M5-T4 | Phase 2 |
| 7 | [#127](https://github.com/emmanuel-a-otchere/AstroForge/issues/127) | SwinIR vs Real-ESRGAN A/B | P4-M4-T2 | Phase 4 |
| 8 | [#73](https://github.com/emmanuel-a-otchere/AstroForge/issues/73) | Plate-solve dependency | P2-M4-T1 | Phase 2 start |
| 9 | [#118](https://github.com/emmanuel-a-otchere/AstroForge/issues/118) | Plugin runtime (WASM-only vs Python) | P4-M1-T5 | Phase 4 |
| 10 | [#94](https://github.com/emmanuel-a-otchere/AstroForge/issues/94) | Planetary memory strategy | P3-M1-T4 | Phase 3 |
| 11 | [#108](https://github.com/emmanuel-a-otchere/AstroForge/issues/108) | Auto-update + telemetry policy | P3-M3-T4, P3-M3-T5 | Phase 3 |
| 12 | [#130](https://github.com/emmanuel-a-otchere/AstroForge/issues/130) | Accessibility & i18n scope | P4-M5-T1 | Phase 4 |

---

## Changelog

| Date | Change | Author |
|---|---|---|
| 2026-08-30 | Initial project plan created from spec v1.1.0 | AstroForge |
| 2026-08-30 | All phases, milestones, tasks, and decision points created as GitHub issues (#1–#135) | AstroForge |
| 2026-09-01 | Phase 0 marked done; Phase 1 core algorithms marked done; focal length + object type auto-detection added (P1-M1-T7) | AstroForge |
| 2026-09-01 | Added Phase 1.5 — Guided Processing Train per CR AF-CR-2026-09-01-IMG-PIPELINE: 8 milestones, 43 tasks covering state machine, live preview, wizard/forge UI, AI service layer, 10-stage train, non-destructive editing, smart-telescope awareness | AstroForge |
| 2026-09-02 | Cross-linked all 51 Phase 1.5 tasks to issues #137–#187 (previously tracked as commit-message-only) | AstroForge |
