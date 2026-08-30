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
| P0-M1-T1 | [#1](https://github.com/emmanuel-a-otchere/AstroForge/issues/1) | Initialize Tauri 2.x project (Rust + Svelte/SolidJS frontend) | §4 | pending | — |
| P0-M1-T2 | [#2](https://github.com/emmanuel-a-otchere/AstroForge/issues/2) | Configure Vite + Svelte/SolidJS with WebGPU probe and Canvas2D fallback | §4 | pending | T1 |
| P0-M1-T3 | [#3](https://github.com/emmanuel-a-otchere/AstroForge/issues/3) | Set up Rust workspace: `astroforge-core` (engine), `astroforge-ai` (ONNX), `astroforge-app` (Tauri) | §4 | pending | T1 |
| P0-M1-T4 | [#4](https://github.com/emmanuel-a-otchere/AstroForge/issues/4) | Configure CI (GitHub Actions): `cargo fmt`, `cargo clippy`, `cargo test`, `npm run build` on push | §4 | pending | T3 |
| P0-M1-T5 | [#5](https://github.com/emmanuel-a-otchere/AstroForge/issues/5) | Add `.gitignore`, `.editorconfig`, `rust-toolchain.toml`, `prettier` config | — | pending | T1 |
| P0-M1-T6 | [#6](https://github.com/emmanuel-a-otchere/AstroForge/issues/6) | Create placeholder UI: app window with "AstroForge" branding, empty workspace | §4 | pending | T2 |

### Milestone 0.2 — Core Architecture Scaffolding

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P0-M2-T1 | [#7](https://github.com/emmanuel-a-otchere/AstroForge/issues/7) | Define `Stage` trait and `PipelineDag` structure in `astroforge-core` | §4, §7 | pending | M1-T3 |
| P0-M2-T2 | [#8](https://github.com/emmanuel-a-otchere/AstroForge/issues/8) | Implement `ArtifactStore` (filesystem + metadata) with FITS/TIFF write stubs | §4, §17 | pending | M1-T3 |
| P0-M2-T3 | [#9](https://github.com/emmanuel-a-otchere/AstroForge/issues/9) | Set up SQLite schema for project/session state (projects, sessions, stages, checkpoints) | §14.5 | pending | M1-T3 |
| P0-M2-T4 | [#10](https://github.com/emmanuel-a-otchere/AstroForge/issues/10) | Implement `Orchestrator` skeleton: DAG runner with pause/resume/checkpoint stubs | §4 | pending | T1, T3 |
| P0-M2-T5 | [#11](https://github.com/emmanuel-a-otchere/AstroForge/issues/11) | Define IPC contract between frontend and Rust backend (Tauri commands/events) | §4 | pending | M1-T1 |
| P0-M2-T6 | [#12](https://github.com/emmanuel-a-otchere/AstroForge/issues/12) | Implement WebGPU capability probe with Canvas2D fallback selection | §4 | pending | M1-T2 |

### Milestone 0.3 — FITS I/O Foundation

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P0-M3-T1 | [#13](https://github.com/emmanuel-a-otchere/AstroForge/issues/13) | Integrate `fitsrs` / `cfitsio` bindings for FITS read/write | §4, §5 | pending | M2-T2 |
| P0-M3-T2 | [#14](https://github.com/emmanuel-a-otchere/AstroForge/issues/14) | Implement FITS header parser: extract `IMAGETYP`, `EXPTIME`, `FILTER`, `DATE-OBS`, `CCD-TEMP`, `BAYERPAT`, `XBAYROFF`, `YBAYROFF` | §5.1, §5.2 | pending | T1 |
| P0-M3-T3 | [#15](https://github.com/emmanuel-a-otchere/AstroForge/issues/15) | Implement 32-bit float image buffer type (`F32Image`) with ndarray backing | §4 | pending | M1-T3 |
| P0-M3-T4 | [#16](https://github.com/emmanuel-a-otchere/AstroForge/issues/16) | Write unit tests for FITS read/write round-trip with sample files | §5 | pending | T1, T3 |

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
| P1-M1-T1 | [#17](https://github.com/emmanuel-a-otchere/AstroForge/issues/17) | Implement folder scan: recursive directory walk, file classification via FITS headers | §5.1, §5.3 | pending | P0-M3-T2 |
| P1-M1-T2 | [#18](https://github.com/emmanuel-a-otchere/AstroForge/issues/18) | Implement auto-classification fallback (exposure-based: Bias/Dark/Flat/Light) | §5.3 | pending | T1 |
| P1-M1-T3 | [#19](https://github.com/emmanuel-a-otchere/AstroForge/issues/19) | Group lights by filter and binning | §5.3 | pending | T1 |
| P1-M1-T4 | [#20](https://github.com/emmanuel-a-otchere/AstroForge/issues/20) | Build session manifest data structure (SQLite-backed) | §5.1, §14.5 | pending | P0-M2-T3 |
| P1-M1-T5 | [#21](https://github.com/emmanuel-a-otchere/AstroForge/issues/21) | Implement "What did you shoot?" initial dialog (target name, camera type, focal length, lights-only toggle) | §5.4 | pending | P0-M2-T5 |
| P1-M1-T6 | [#22](https://github.com/emmanuel-a-otchere/AstroForge/issues/22) | Implement classification confirmation dialog with sortable override | §5.3 | pending | T2, T5 |

### Milestone 1.2 — Calibration

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P1-M2-T1 | [#23](https://github.com/emmanuel-a-otchere/AstroForge/issues/23) | Implement master dark builder (sigma-clipped median, exposure & temp scaling) | §7 Stage 4 | pending | P1-M1-T1 |
| P1-M2-T2 | [#24](https://github.com/emmanuel-a-otchere/AstroForge/issues/24) | Implement master flat builder (normalized, sigma-clipped) | §7 Stage 4 | pending | P1-M1-T1 |
| P1-M2-T3 | [#25](https://github.com/emmanuel-a-otchere/AstroForge/issues/25) | Implement master bias builder | §7 Stage 4 | pending | P1-M1-T1 |
| P1-M2-T4 | [#26](https://github.com/emmanuel-a-otchere/AstroForge/issues/26) | Implement calibration application: `(Light − MasterDark) / MasterFlat` | §7 Stage 4 | pending | T1, T2, T3 |
| P1-M2-T5 | [#27](https://github.com/emmanuel-a-otchere/AstroForge/issues/27) | Handle "lights only" path (skip dark, apply flat if present) | §7 Stage 4 | pending | T4 |
| P1-M2-T6 | [#28](https://github.com/emmanuel-a-otchere/AstroForge/issues/28) | Streaming calibration: process one frame at a time, no full-session RAM hold | §12 | pending | T4 |

### Milestone 1.3 — Registration & Stacking

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P1-M3-T1 | [#29](https://github.com/emmanuel-a-otchere/AstroForge/issues/29) | Implement star extraction (multiscale Laplacian + centroiding) | §7 Stage 6 | pending | P1-M2-T6 |
| P1-M3-T2 | [#30](https://github.com/emmanuel-a-otchere/AstroForge/issues/30) | Implement auto-reference frame selection (best FWHM + central target) | §7 Stage 6 | pending | T1 |
| P1-M3-T3 | [#31](https://github.com/emmanuel-a-otchere/AstroForge/issues/31) | Implement affine/similarity transform computation per frame | §7 Stage 6 | pending | T1, T2 |
| P1-M3-T4 | [#32](https://github.com/emmanuel-a-otchere/AstroForge/issues/32) | Implement sub-pixel cross-correlation on star cutouts | §7 Stage 6 | pending | T1 |
| P1-M3-T5 | [#33](https://github.com/emmanuel-a-otchere/AstroForge/issues/33) | Implement Kappa-Sigma clip stacking algorithm | §7 Stage 7 | pending | T3 |
| P1-M3-T6 | [#34](https://github.com/emmanuel-a-otchere/AstroForge/issues/34) | Implement stacking accumulator (streaming, bounded memory) | §7 Stage 7, §12 | pending | T5 |
| P1-M3-T7 | [#35](https://github.com/emmanuel-a-otchere/AstroForge/issues/35) | Output 32-bit float stack + weight map | §7 Stage 7 | pending | T6 |

### Milestone 1.4 — Stretching & Export

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P1-M4-T1 | [#36](https://github.com/emmanuel-a-otchere/AstroForge/issues/36) | Implement basic non-linear stretch (histogram transfer / arcsinh) | §7 Stage 11 | pending | P1-M3-T7 |
| P1-M4-T2 | [#37](https://github.com/emmanuel-a-otchere/AstroForge/issues/37) | Implement interactive histogram dialog | §7 Stage 11, §9 | pending | T1, P0-M2-T5 |
| P1-M4-T3 | [#38](https://github.com/emmanuel-a-otchere/AstroForge/issues/38) | Implement 16-bit TIFF export | §7 Stage 17 | pending | P1-M3-T7 |
| P1-M4-T4 | [#39](https://github.com/emmanuel-a-otchere/AstroForge/issues/39) | Implement processing report generation (frame stats, rejections, parameters) | §14 | pending | P1-M1-T4 |

### Milestone 1.5 — Beginner Dialog Mode & Smoke Test

| ID | Task | Spec ref | Status | Depends on |
|---|---|---|---|---|
| P1-M5-T1 | [#40](https://github.com/emmanuel-a-otchere/AstroForge/issues/40) | Implement Auto mode (defaults, no prompts) for all MVP stages | §9 | pending | M4-T3 |
| P1-M5-T2 | [#41](https://github.com/emmanuel-a-otchere/AstroForge/issues/41) | Implement beginner verbosity level (mostly Auto) | §9 | pending | T1 |
| P1-M5-T3 | [#42](https://github.com/emmanuel-a-otchere/AstroForge/issues/42) | Wire end-to-end pipeline: ingest → calibrate → register → stack → stretch → export | §7 | pending | M4-T3 |
| P1-M5-T4 | [#43](https://github.com/emmanuel-a-otchere/AstroForge/issues/43) | Write scripted smoke test: FITS folder → TIFF on Windows | §16 DoD | pending | T3 |
| P1-M5-T5 | [#44](https://github.com/emmanuel-a-otchere/AstroForge/issues/44) | Write scripted smoke test: FITS folder → TIFF on macOS | §16 DoD | pending | T3 |
| P1-M5-T6 | [#45](https://github.com/emmanuel-a-otchere/AstroForge/issues/45) | Memory test: 30-frame stack on 4 GB configuration without OOM | §16 DoD, §2 | pending | T3 |

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
| P2-M1-T1 | [#46](https://github.com/emmanuel-a-otchere/AstroForge/issues/46) | Integrate ONNX Runtime (`ort` crate) with backend auto-selection (CPU, CUDA, DirectML, CoreML) | §10 | pending | Phase 1 |
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
| P2-M6-T1 | [#84](https://github.com/emmanuel-a-otchere/AstroForge/issues/84) | Implement Confirm mode (preview + metrics, OK/Adjust/Skip) | §9 | pending | Phase 1 |
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
| P3-M1-T1 | [#91](https://github.com/emmanuel-a-otchere/AstroForge/issues/91) | Implement planetary routing (exposure < 2s + frame_count > 500) | §6.2 | pending | Phase 1 |
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
