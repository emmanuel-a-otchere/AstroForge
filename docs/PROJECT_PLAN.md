# AstroForge — Living Project Plan

**Last updated:** 2026-08-30
**Current phase:** Phase 0 — Foundation & Scaffolding
**Spec version:** 1.1.0

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
The project is divided into **5 phases**, each with a clear exit criterion.
Phases map to the spec's build roadmap (§16) but break it into actionable
milestones with concrete deliverables.

| Phase | Spec mapping | Exit criterion |
|---|---|---|
| **Phase 0** — Foundation & Scaffolding | §4 Architecture, §2 Constraints | Tauri shell builds and runs on Windows + macOS; project skeleton committed |
| **Phase 1** — MVP Core Pipeline | §16 MVP | End-to-end FITS → TIFF on 4 GB machine; beginner dialog smoke test passes |
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
- **Spec ref:** the spec section it implements
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

### Milestone 0.1 — Project Skeleton & Tooling

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P0-M1-T1 | Initialize Tauri 2.x project (Rust + Svelte/SolidJS frontend) | §4 | pending | — |
| P0-M1-T2 | Configure Vite + Svelte/SolidJS with WebGPU probe and Canvas2D fallback | §4 | pending | T1 |
| P0-M1-T3 | Set up Rust workspace: `astroforge-core` (engine), `astroforge-ai` (ONNX), `astroforge-app` (Tauri) | §4 | pending | T1 |
| P0-M1-T4 | Configure CI (GitHub Actions): `cargo fmt`, `cargo clippy`, `cargo test`, `npm run build` on push | §4 | pending | T3 |
| P0-M1-T5 | Add `.gitignore`, `.editorconfig`, `rust-toolchain.toml`, `prettier` config | — | pending | T1 |
| P0-M1-T6 | Create placeholder UI: app window with "AstroForge" branding, empty workspace | §4 | pending | T2 |

### Milestone 0.2 — Core Architecture Scaffolding

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P0-M2-T1 | Define `Stage` trait and `PipelineDag` structure in `astroforge-core` | §4, §7 | pending | M1-T3 |
| P0-M2-T2 | Implement `ArtifactStore` (filesystem + metadata) with FITS/TIFF write stubs | §4, §17 | pending | M1-T3 |
| P0-M2-T3 | Set up SQLite schema for project/session state (projects, sessions, stages, checkpoints) | §14.5 | pending | M1-T3 |
| P0-M2-T4 | Implement `Orchestrator` skeleton: DAG runner with pause/resume/checkpoint stubs | §4 | pending | T1, T3 |
| P0-M2-T5 | Define IPC contract between frontend and Rust backend (Tauri commands/events) | §4 | pending | M1-T1 |
| P0-M2-T6 | Implement WebGPU capability probe with Canvas2D fallback selection | §4 | pending | M1-T2 |

### Milestone 0.3 — FITS I/O Foundation

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P0-M3-T1 | Integrate `fitsrs` / `cfitsio` bindings for FITS read/write | §4, §5 | pending | M2-T2 |
| P0-M3-T2 | Implement FITS header parser: extract `IMAGETYP`, `EXPTIME`, `FILTER`, `DATE-OBS`, `CCD-TEMP`, `BAYERPAT`, `XBAYROFF`, `YBAYROFF` | §5.1, §5.2 | pending | T1 |
| P0-M3-T3 | Implement 32-bit float image buffer type (`F32Image`) with ndarray backing | §4 | pending | M1-T3 |
| P0-M3-T4 | Write unit tests for FITS read/write round-trip with sample files | §5 | pending | T1, T3 |

---

## Phase 1 — MVP Core Pipeline

**Goal:** End-to-end deep-sky processing from FITS light+dark+flat folder to
exported 16-bit TIFF, runnable on a 4 GB machine, with beginner dialog mode.

**Exit criterion (Definition of Done — from spec §16):**
1. Drop a FITS light+dark+flat folder → exported 16-bit TIFF.
2. Kappa-sigma stack of ≥30 frames on a 4 GB machine without OOM.
3. Beginner dialog mode passes a scripted smoke test on Windows + macOS.

### Milestone 1.1 — Ingest & Classification

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P1-M1-T1 | Implement folder scan: recursive directory walk, file classification via FITS headers | §5.1, §5.3 | pending | P0-M3-T2 |
| P1-M1-T2 | Implement auto-classification fallback (exposure-based: Bias/Dark/Flat/Light) | §5.3 | pending | T1 |
| P1-M1-T3 | Group lights by filter and binning | §5.3 | pending | T1 |
| P1-M1-T4 | Build session manifest data structure (SQLite-backed) | §5.1, §14.5 | pending | P0-M2-T3 |
| P1-M1-T5 | Implement "What did you shoot?" initial dialog (target name, camera type, focal length, lights-only toggle) | §5.4 | pending | P0-M2-T5 |
| P1-M1-T6 | Implement classification confirmation dialog with sortable override | §5.3 | pending | T2, T5 |

### Milestone 1.2 — Calibration

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P1-M2-T1 | Implement master dark builder (sigma-clipped median, exposure & temp scaling) | §7 Stage 4 | pending | P1-M1-T1 |
| P1-M2-T2 | Implement master flat builder (normalized, sigma-clipped) | §7 Stage 4 | pending | P1-M1-T1 |
| P1-M2-T3 | Implement master bias builder | §7 Stage 4 | pending | P1-M1-T1 |
| P1-M2-T4 | Implement calibration application: `(Light − MasterDark) / MasterFlat` | §7 Stage 4 | pending | T1, T2, T3 |
| P1-M2-T5 | Handle "lights only" path (skip dark, apply flat if present) | §7 Stage 4 | pending | T4 |
| P1-M2-T6 | Streaming calibration: process one frame at a time, no full-session RAM hold | §12 | pending | T4 |

### Milestone 1.3 — Registration & Stacking

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P1-M3-T1 | Implement star extraction (multiscale Laplacian + centroiding) | §7 Stage 6 | pending | P1-M2-T6 |
| P1-M3-T2 | Implement auto-reference frame selection (best FWHM + central target) | §7 Stage 6 | pending | T1 |
| P1-M3-T3 | Implement affine/similarity transform computation per frame | §7 Stage 6 | pending | T1, T2 |
| P1-M3-T4 | Implement sub-pixel cross-correlation on star cutouts | §7 Stage 6 | pending | T1 |
| P1-M3-T5 | Implement Kappa-Sigma clip stacking algorithm | §7 Stage 7 | pending | T3 |
| P1-M3-T6 | Implement stacking accumulator (streaming, bounded memory) | §7 Stage 7, §12 | pending | T5 |
| P1-M3-T7 | Output 32-bit float stack + weight map | §7 Stage 7 | pending | T6 |

### Milestone 1.4 — Stretching & Export

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P1-M4-T1 | Implement basic non-linear stretch (histogram transfer / arcsinh) | §7 Stage 11 | pending | P1-M3-T7 |
| P1-M4-T2 | Implement interactive histogram dialog | §7 Stage 11, §9 | pending | T1, P0-M2-T5 |
| P1-M4-T3 | Implement 16-bit TIFF export | §7 Stage 17 | pending | P1-M3-T7 |
| P1-M4-T4 | Implement processing report generation (frame stats, rejections, parameters) | §14 | pending | P1-M1-T4 |

### Milestone 1.5 — Beginner Dialog Mode & Smoke Test

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P1-M5-T1 | Implement Auto mode (defaults, no prompts) for all MVP stages | §9 | pending | M4-T3 |
| P1-M5-T2 | Implement beginner verbosity level (mostly Auto) | §9 | pending | T1 |
| P1-M5-T3 | Wire end-to-end pipeline: ingest → calibrate → register → stack → stretch → export | §7 | pending | M4-T3 |
| P1-M5-T4 | Write scripted smoke test: FITS folder → TIFF on Windows | §16 DoD | pending | T3 |
| P1-M5-T5 | Write scripted smoke test: FITS folder → TIFF on macOS | §16 DoD | pending | T3 |
| P1-M5-T6 | Memory test: 30-frame stack on 4 GB configuration without OOM | §16 DoD, §2 | pending | T3 |

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
| P2-M1-T1 | Integrate ONNX Runtime (`ort` crate) with backend auto-selection (CPU, CUDA, DirectML, CoreML) | §10 | pending | Phase 1 |
| P2-M1-T2 | Implement model registry: download, SHA-256 verify, signed manifest check | §10.8 | pending | T1 |
| P2-M1-T3 | Implement tiling inference engine (512px tiles, 64px overlap, cosine blend) | §10.4 | pending | T1 |
| P2-M1-T4 | Implement hardware probe and quality tier selection (Fast/Balanced/Research/Perceptual) | §10.5 | pending | T1 |
| P2-M1-T5 | Integrate `swinir-denoise-astro` model (Stage 13) | §10.3, §7 Stage 13 | pending | T3 |
| P2-M1-T6 | Integrate `swinir-sr-astro-2x` model (Stage 15) | §10.3, §7 Stage 15 | pending | T3 |
| P2-M1-T7 | Integrate `swin2sr-dejpeg` model (Stage 0.5) | §10.3, §7 Stage 0.5 | pending | T3 |
| P2-M1-T8 | Integrate `star-seg-v1` model (Stage 14) | §10.3, §7 Stage 14 | pending | T3 |
| P2-M1-T9 | Integrate `cloud-score-v1` model (Stage 2) | §10.3, §7 Stage 2 | pending | T3 |
| P2-M1-T10 | Integrate `color-cal-net` model (Stage 9) | §10.3, §7 Stage 9 | pending | T3 |
| P2-M1-T11 | Integrate `trail-lama-tiny` model (Stage 14) | §10.3, §7 Stage 14 | pending | T3 |
| P2-M1-T12 | Implement determinism recording (model hash, backend, tile size, seed) | §10.6 | pending | T5 |

### Milestone 2.2 — Remaining Deep-Sky Stages

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P2-M2-T1 | Implement quality filter stage (FWHM, eccentricity, star count, SNR, background, cloud score; auto-reject worst 15%) | §7 Stage 2 | pending | P2-M1-T9 |
| P2-M2-T2 | Implement debayer stage (VNG + bilinear, camera white balance from metadata) | §7 Stage 3 | pending | Phase 1 |
| P2-M2-T3 | Implement cosmetic correction (hot/cold pixel detection, sigma-clip, interpolation) | §7 Stage 5 | pending | P1-M2-T6 |
| P2-M2-T4 | Implement background extraction (2D polynomial/spline model, nebulosity mask) | §7 Stage 8 | pending | P1-M3-T7 |
| P2-M2-T5 | Implement color calibration (white-balance via reference stars or user-picked neutral region) | §7 Stage 9 | pending | P2-M1-T10 |
| P2-M2-T6 | Implement crop and rotate (manual crop, rotate to cardinal, edge removal) | §7 Stage 10 | pending | P1-M4-T1 |
| P2-M2-T7 | Implement star segmentation and enhancement (star/background layers, color boost, bloat reduction) | §7 Stage 14 | pending | P2-M1-T8 |
| P2-M2-T8 | Implement final detail enhancement (multi-scale unsharp mask, local contrast) | §7 Stage 16 | pending | P1-M4-T1 |
| P2-M2-T9 | Implement full export (TIFF 16/32-bit, PNG, JPEG, XISF with history, sidecar JSON) | §7 Stage 17 | pending | P1-M4-T3 |

### Milestone 2.3 — Narrowband Composition

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P2-M3-T1 | Implement narrowband detection (≥2 light groups with Ha/OIII/SII filter names) | §7.6 | pending | P1-M1-T3 |
| P2-M3-T2 | Implement channel extraction from OSC (Ha→Red, SII→Red, OIII→Blue+Green) | §7.6 | pending | P2-M2-T2 |
| P2-M3-T3 | Implement inter-filter group registration | §7..6 | pending | P1-M3-T3 |
| P2-M3-T4 | Implement composition palettes (HOO, SHO, HSO, Custom) | §7.6 | pending | T3 |
| P2-M3-T5 | Implement SCNR (Subtractive Color Noise Reduction) per palette | §7 Stage 9.5 | pending | T4 |
| P2-M3-T6 | Implement channel ratio normalization (Ha:OIII balance) | §7 Stage 9.5 | pending | T4 |

### Milestone 2.4 — Plate Solving

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P2-M4-T1 | Evaluate plate-solve dependency: bundle ASTAP vs. online astrometry.net vs. defer | §17 item 8 | pending | — |
| P2-M4-T2 | Integrate ASTAP binary (bundled, offline star catalogs) | §7 Stage 6.5 | pending | T1 |
| P2-M4-T3 | Implement WCS output to FITS header | §7 Stage 6.5 | pending | T2 |
| P2-M4-T4 | Implement auto-crop to subject using WCS | §7 Stage 10 | pending | T3, P2-M2-T6 |
| P2-M4-T5 | Implement annotated star map overlay | §7 Stage 6.5 | pending | T3 |
| P2-M4-T6 | Implement photometric calibration anchoring (APASS/Gaia) | §7 Stage 9 | pending | T3, P2-M2-T5 |
| P2-M4-T7 | Implement graceful failure (skip if solve fails, registration still works) | §7 Stage 6.5 | pending | T2 |

### Milestone 2.5 — DIP Optional Stages

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P2-M5-T1 | Implement DIP deconvolution (zero-shot, PSF as forward operator) | §7 Stage 12, §10.7 | pending | P2-M1-T1 |
| P2-M5-T2 | Implement DIP denoise (linear data, blended in linear space) | §7 Stage 13, §10.7 | pending | P2-M1-T1 |
| P2-M5-T3 | Implement DIP inpaint (high-quality trail removal) | §7 Stage 14, §10.7 | pending | P2-M1-T1 |
| P2-M5-T4 | Implement DIP defaults (iteration count, early-stopping heuristic) | §17 item 6 | pending | T1 |

### Milestone 2.6 — Dialog Modes & Session Management

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P2-M6-T1 | Implement Confirm mode (preview + metrics, OK/Adjust/Skip) | §9 | pending | Phase 1 |
| P2-M6-T2 | Implement Manual mode (full parameter panel) | §9 | pending | T1 |
| P2-M6-T3 | Implement Intermediate and Expert verbosity levels | §9 | pending | T1, T2 |
| P2-M6-T4 | Implement before/after slider, histogram overlay, "revert to auto," "save as preset" | §9 | pending | T1 |
| P2-M6-T5 | Implement project/session persistence (SQLite: DAG state, checkpoint refs) | §14.5 | pending | P0-M2-T3 |
| P2-M6-T6 | Implement crash recovery (detect interrupted project on launch, offer resume) | §14.5 | pending | T5 |
| P2-M6-T7 | Implement checkpointing (intermediate FITS per stage, crash-safe) | §12 | pending | T5 |

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
| P3-M1-T1 | Implement planetary routing (exposure < 2s + frame_count > 500) | §6.2 | pending | Phase 1 |
| P3-M1-T2 | Implement feature tracking / limb detection for registration | §8 | pending | P1-M3-T1 |
| P3-M1-T3 | Implement lucky imaging: rank by sharpness, stack best 10–30% | §8 | pending | T2 |
| P3-M1-T4 | Implement streaming two-pass rank/select for 50,000-frame ingest | §8, §17 item 10 | pending | T3 |
| P3-M1-T5 | Implement planetary drizzle | §8 | pending | T3 |
| P3-M1-T6 | Implement planetary stretching (aggressive contrast for surface detail) | §8 | pending | P1-M4-T1 |
| P3-M1-T7 | Implement planetary sharpening (aggressive unsharp / wavelet) | §8 | pending | T6 |
| P3-M1-T8 | Implement lunar HDR merge (exposure sets) | §8 | pending | T6 |
| P3-M1-T9 | Implement DIP-coadd for multi-frame lucky-imaging restoration | §8, §16 v2 | pending | P2-M5-T1 |

### Milestone 3.2 — Recipe System

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P3-M2-T1 | Define and implement recipe JSON schema (v1.0) | §11.1 | pending | Phase 2 |
| P3-M2-T2 | Implement recipe export (sanitized: strip paths, GPS, machine info) | §11.2 | pending | T1 |
| P3-M2-T3 | Implement recipe import (validate compatibility, check required models) | §11.3 | pending | T1 |
| P3-M2-T4 | Implement recipe application (set parameters, prompt model download) | §11.3, §11.4 | pending | T3 |
| P3-M2-T5 | Implement integrity badge (tag exports/recipes with perceptual model usage) | §10.6 | pending | T1 |

### Milestone 3.3 — Cross-Platform Packaging

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P3-M3-T1 | Configure Windows packaging (.msix) with optional CUDA installer | §13 | pending | Phase 2 |
| P3-M3-T2 | Configure macOS packaging (.dmg, Apple Silicon native, notarization) | §13 | pending | Phase 2 |
| P3-M3-T3 | Configure Linux packaging (.AppImage + .deb + .rpm + Flatpak) | §13 | pending | Phase 2 |
| P3-M3-T4 | Implement Tauri updater with signed payloads | §13 | pending | T1, T2 |
| P3-M3-T5 | Implement opt-in crash-reporting and telemetry (configured on first launch) | §13 | pending | T1 |

### Milestone 3.4 — PNG/JPG/DNG Bayer Detection

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P3-M4-T1 | Implement statistical Bayer detection (autocorrelation, green variance, camera signature DB) | §5.2 | pending | Phase 1 |
| P3-M4-T2 | Implement DNG parser (TIFF tags: CFAPattern, CFARepeatPatternDim, BlackLevel, WhiteLevel) | §5.2 | pending | T1 |
| P3-M4-T3 | Implement Bayer uncertainty prompt (telescope selection / pattern selection) | §5.2 | pending | T1 |
| P3-M4-T4 | Implement confidence scoring (>0.85 auto, 0.5–0.85 prompt, <0.5 assume RGB) | §5.2 | pending | T1 |

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
| P4-M1-T1 | Define plugin API contract (stage add/replace interface) | §15 | pending | Phase 3 |
| P4-M1-T2 | Implement WASM plugin runtime (capability-scoped sandbox, filesystem allowlist, no network by default) | §15 | pending | T1 |
| P4-M1-T3 | Implement custom AI model loading (drop ONNX into `~/AstroForge/models/`) | §15 | pending | P2-M1-T2 |
| P4-M1-T4 | Implement export plugin interface (new output targets) | §15 | pending | T1 |
| P4-M1-T5 | Evaluate optional Python sidecar (user-installed, not bundled) | §15, §17 item 9 | pending | T1 |

### Milestone 4.2 — Recipe Gallery

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P4-M2-T1 | Decide hosting: GitHub-based repo vs. static JSON index | §17 item 3 | pending | P3-M2-T1 |
| P4-M2-T2 | Implement in-app recipe gallery (browsable, filterable by target/equipment/palette) | §11.3 | pending | T1 |
| P4-M2-T3 | Implement recipe search and filtering | §11.3 | pending | T2 |

### Milestone 4.3 — Platform Optimizations

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P4-M3-T1 | Optimize macOS Metal backend (GPU FFTs, AI inference) | §13 | pending | Phase 3 |
| P4-M3-T2 | Optimize Windows CUDA / DirectML backend | §13 | pending | Phase 3 |
| P4-M3-T3 | Optimize Linux Vulkan / CUDA backend | §13 | pending | Phase 3 |
| P4-M3-T4 | Profile and optimize memory usage for 4 GB target | §2, §12 | pending | Phase 3 |

### Milestone 4.4 — Experimental AI Models

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P4-M4-T1 | Integrate StableSR as experimental opt-in plugin (GPU-gated, integrity badge) | §10.3, §10.7 | pending | P4-M1-T2 |
| P4-M4-T2 | A/B test SwinIR vs Real-ESRGAN on real smart-telescope data | §17 item 7 | pending | P2-M1-T6 |
| P4-M4-T3 | Evaluate "Star Prior GAN" research for star-core restoration | §10.7 | pending | P4-M1-T2 |
| P4-M4-T4 | Source/build SwinIR astro fine-tuning dataset | §17 item 5 | pending | P2-M1-T5 |

### Milestone 4.5 — Accessibility & i18n

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P4-M5-T1 | Decide WCAG AA scope (v1 or deferred) | §17 item 12 | pending | — |
| P4-M5-T2 | Implement keyboard navigation and screen reader support if in scope | §17 item 12 | pending | T1 |
| P4-M5-T3 | Implement i18n framework and initial translations if in scope | §17 item 12 | pending | T1 |

---

## Open Decision Points

These are spec open items (§17) that gate specific tasks. They should be
resolved early to avoid blocking.

| # | Decision | Gates task(s) | Target resolution |
|---|---|---|---|
| 1 | Smart-telescope SDK integration vs. file-only | P4-M2-T1+ | Phase 4 |
| 2 | Live stacking preview for planetary | P3-M1-T3 | Phase 3 start |
| 3 | Recipe gallery hosting | P4-M2-T1 | Phase 4 start |
| 4 | License verification (StableSR, DIP) | P4-M4-T1, P2-M5-T1 | Phase 2 start |
| 5 | SwinIR fine-tuning dataset | P4-M4-T4 | Phase 4 |
| 6 | DIP defaults | P2-M5-T4 | Phase 2 |
| 7 | SwinIR vs Real-ESRGAN A/B | P4-M4-T2 | Phase 4 |
| 8 | Plate-solve dependency | P2-M4-T1 | Phase 2 start |
| 9 | Plugin runtime (WASM-only vs Python) | P4-M1-T5 | Phase 4 |
| 10 | Planetary memory strategy | P3-M1-T4 | Phase 3 |
| 11 | Auto-update + telemetry policy | P3-M3-T4, P3-M3-T5 | Phase 3 |
| 12 | Accessibility & i18n scope | P4-M5-T1 | Phase 4 |

---

## Changelog

| Date | Change | Author |
|---|---|---|
| 2026-08-30 | Initial project plan created from spec v1.1.0 | AstroForge |
