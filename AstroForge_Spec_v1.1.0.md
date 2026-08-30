# AstroForge — Complete Product Specification
**Version:** 1.1 (consolidated, enhanced)
**One-liner:** A cross-platform app that turns raw smart-telescope data into
publication-ready deep-sky images through an interactive, AI-augmented pipeline.

---

## 1. Vision & Goals

AstroForge ingests raw astrophotography data — OSC (One-Shot Color) FITS, PNG, JPG,
or DNG from smart telescopes — and automatically delivers a high-definition,
processed image. The user is guided through an interactive workflow via dialogs and
toggles, but the default path is "one-click magic" for beginners while exposing
advanced controls for experts.

**Core promises**
- **Zero-to-hero pipeline:** raw folder → finished image, no external tools.
- **Interactive by default:** every major stage surfaces a dialog with sensible
  defaults and expert toggles.
- **AI-augmented at key stages:** denoising, super-resolution, star segmentation,
  inpainting — all within a 4–8 GB RAM budget.
- **Trustworthy output:** deterministic by default; generative/"perceptual" models
  are opt-in and clearly labeled.
- **Extensible:** plugin architecture for AI models, calibration frames, and export.

---

## 2. Design Principles & Constraints

| Constraint | Implication |
|---|---|
| **Free** | All bundled models need permissive licenses; no paywalled stages. |
| **4–8 GB RAM target** | Models must be small, quantized (INT8), and tile-based. |
| **Consistent output** | Feed-forward deterministic models in the default path; stochastic/generative models are opt-in and seed-recorded. |
| **Cross-platform first** | Single architecture, then per-OS optimization. |
| **OSC smart-telescope focus** | Auto-detect raw Bayer; support narrowband composition; no mono-camera assumption. |
| **Offline-first** | Core pipeline runs with no network; optional online services (plate solve, recipe gallery) degrade gracefully. |

---

## 3. Target Users & Scenarios

- **Beginner smart-telescope owner** (Seestar, Unistellar, Stellina, Vespera, Dwarf):
  drops a folder, clicks "Process," gets a great image with minimal prompts.
- **Intermediate enthusiast:** adds narrowband filters, wants HOO/SHO composition
  and control over stretching/denoising.
- **Advanced user:** wants planetary/lunar lucky-imaging, deconvolution, and
  reproducible shareable recipes.

---

## 4. High-Level Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                   UI Layer (Tauri 2.x)                       │
│  Wizard dialogs · Live preview · Histogram · Star map        │
│  WebGPU with Canvas2D fallback                                │
└──────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                 Orchestrator (DAG Runner)                    │
│  Runs the pipeline as a directed acyclic graph of stages.    │
│  Supports pause/resume, checkpoint, rollback, preview.       │
│  Project/session state persisted in SQLite.                   │
└──────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
┌──────────────┐   ┌──────────────────┐   ┌─────────────────┐
│  Core Engine │   │   AI Model Hub   │   │ Plugin Registry │
│ (Rust)       │   │ (ONNX Runtime)   │   │ (WASM / Python)  │
└──────────────┘   └──────────────────┘   └─────────────────┘
        │                     │                     │
        └─────────────────────┼─────────────────────┘
                              ▼
                   ┌──────────────────────┐
                   │   Artifact Store     │
                   │  fits / tif / xisf   │
                   └──────────────────────┘
```

**Tech stack**
- **Runtime:** Tauri 2.x — Rust backend + native WebView (~5 MB shell vs
  Electron's ~150 MB, critical on 4 GB machines).
- **Image math:** Rust `ndarray` + C++ bindings; FITS I/O via `fitsrs`/`cfitsio`.
- **AI inference:** ONNX Runtime (`ort` crate) — CPU, CUDA, DirectML, CoreML,
  OpenVINO backends auto-selected.
- **Frontend:** Svelte/SolidJS. Live previews via WebGPU with a Canvas2D
  fallback (downscaled tiled preview). Probe at startup; degrade gracefully.
- **Storage:** local SQLite index + filesystem artifact store.
- **App updates:** Tauri built-in updater; signed payloads.

---

## 5. Input Handling

### 5.1 Ingest Modes
| Mode | Description |
|---|---|
| **Folder scan** | Recursively scan a directory; classify via FITS headers (`IMAGETYP`, `EXPTIME`, `FILTER`, `DATE-OBS`, `CCD-TEMP`). |
| **Session import** | Import pre-organized sessions (NINA, SGPro, APT exports). |
| **Drag & drop** | Drop a mixed bag; auto-sort into Light/Dark/Flat/Bias. |
| **OSC PNG/JPG/DNG** | Treat as potentially-raw; run Bayer detection (§5.2). |

### 5.2 File Format Support & Bayer Detection
Smart telescopes output varied formats. **Never assume an image is debayered.**

| Format | Likely content | Detection |
|---|---|---|
| **FITS** | Usually debayered, sometimes raw Bayer | Check `BAYERPAT`, `XBAYROFF`, `YBAYROFF`; else statistical test |
| **DNG** | Raw Bayer with CFA metadata | Parse TIFF tags: `CFAPattern`, `CFARepeatPatternDim`, `BlackLevel`, `WhiteLevel` |
| **PNG** | Processed RGB *or* raw Bayer | Statistical Bayer detection |
| **JPG** | Usually RGB, rarely grayscale Bayer | Grayscale + Bayer pattern check |
| **TIFF** | Either | CFA tags, else statistical |

**Statistical Bayer detection (when metadata absent)**
1. Grayscale/single-channel check (candidate for raw Bayer).
2. 2×2 autocorrelation at offsets (1,0), (0,1), (1,1) — Bayer shows strong 2-pixel periodicity.
3. Green-channel variance test — green occupies 50% of the grid.
4. Known-camera signature DB (e.g., Seestar S50 = IMX462, RGGB, 1920×1080).
5. Confidence: >0.85 auto-proceed; 0.5–0.85 prompt; <0.5 assume RGB.

**Bayer prompt (when uncertain)**
> "This file may be raw Bayer data. Is your telescope: [Seestar S50] [Unistellar
> eVscope] [Stellina] [Vespera] [Dwarf] [Other]?"
> If Other: "Select pattern: [RGGB] [BGGR] [GRBG] [GBRG] [Auto-detect]"

### 5.3 Auto-Classification
1. Read FITS `IMAGETYP` if present.
2. Fallback: exposure 0s→Bias, dark-cap short→Dark, uniform stats→Flat, else Light.
3. Group Lights by **filter** and **binning**.
4. Surface classification in a confirmation dialog.

### 5.4 Initial Dialog — "What did you shoot?"
- Target name (optional)
- Camera type: OSC / smart telescope
- Telescope focal length (for plate solving)
- "I only have lights" toggle (skip calibration gracefully)
- "Include dithering info" toggle

---

## 6. Target-Type Detection & Routing

### 6.1 Heuristics
| Signal | Deep-Sky | Planetary / Lunar |
|---|---|---|
| Exposure time | > 10s | < 2s (often ms) |
| Frame count | 10–500 | 1,000–50,000 |
| Frame size | Full sensor | Small ROI / crop |
| Calibration frames | Usually present | Rarely |
| File naming | Sequential, gaps | Burst / video |
| FITS `OBJECT` | Nebula/galaxy | "Jupiter", "Moon" |

### 6.2 Routing Logic
```
IF exposure < 2s AND frame_count > 500:   → PLANETARY
ELIF exposure > 10s AND frame_count < 500: → DEEP_SKY
ELSE:                                      → prompt user
```

### 6.3 Ambiguity Prompt
> "This session could be deep-sky or planetary/lunar. Which? [Deep-sky]
> [Planet/Moon] [Not sure]"

### 6.4 Pipeline Differences (see §8 for the planetary branch)

---

## 7. Deep-Sky Pipeline Stages (The DAG)

Each stage is a node emitting a **preview**, a **quality metric**, and is
independently re-runnable. AI stages are marked with their model.

### Stage 0.5 — `compression_cleanup`  *[swin2sr-dejpeg]*
For JPEG/PNG inputs: remove blocking/compression artifacts *before* calibration.
Only runs when input is lossy-compressed.

### Stage 1 — `ingest`
Load files, parse headers, build session manifest. Confirm classification, flag
anomalies (clouds, guiding errors via FWHM spikes).

### Stage 2 — `quality_filter`  *[cloud-score-v1]*
Per-frame metrics: FWHM, eccentricity, star count, SNR, background, cloud score.
Auto-reject outside percentiles (default worst 15%). Sortable override dialog.

### Stage 3 — `debayer`  *(OSC only)*
Algorithms: **VNG** (quality) or **bilinear** (speed). Apply camera white balance
from metadata. For narrowband-captured OSC, debayer now; channel extraction later.

### Stage 4 — `calibration`
- Build master dark (sigma-clipped median, scaled by exposure & temp).
- Build master flat (normalized, sigma-clipped). Build master bias if present.
- Apply: `(Light − MasterDark) / MasterFlat`.
- OSC PNG without darks: skip dark, still apply flat.

### Stage 5 — `cosmetic_correction`
Detect hot/cold pixels via sigma-clip vs local neighborhood; interpolate
(bilinear/8-neighbor median). Optional small denoising CNN for stubborn defects.

### Stage 6 — `registration`
Star extraction (multiscale Laplacian + centroiding). Affine/similarity transform
per frame vs auto-picked reference (best FWHM + central target). Optional
distortion correction for wide-field. Sub-pixel cross-correlation on star cutouts.

### Stage 6.5 — `plate_solve`  *(optional, recommended)*
Local solver via ASTAP (bundled, ~50 MB, offline star catalogs) or astrometry.net
(online fallback). Output WCS in FITS header; enables auto-crop to subject,
annotated star map, and photometric calibration anchoring.
- Offline-first: bundle ASTAP + compact star index (G17–G18, ~400 MB optional download).
- Failure mode: skip gracefully; registration still works on star matching.

### Stage 7 — `stacking`
Algorithms: Average, Median, **Kappa-Sigma clip** (default), Linear Fit Clip,
**Drizzle** (for dithered data). Per-channel stacking for OSC.
Output: 32-bit float linear + weight map.

### Stage 7.5 — `narrowband_compose`  *(if multi-filter detected)*
See §7.6 below.

### Stage 8 — `background_extraction`
Model background as 2D polynomial/spline; subtract to neutralize gradients.
Mask to preserve large-scale nebulosity.

### Stage 9 — `color_calibration`  *[color-cal-net]*
White-balance via reference star colors or user-picked neutral region. Optional
photometric calibration against APASS/Gaia (requires plate-solved WCS from Stage 6.5).

### Stage 9.5 — `narrowband_color_correction`  *(narrowband only)*
- SCNR (Subtractive Color Noise Reduction): remove green/magenta bias per palette.
- Optional channel ratio normalization (e.g., Ha:OIII balance).

### Stage 10 — `crop_and_rotate`
- Auto-crop to subject (from plate solve) or manual crop; rotate to cardinal;
  remove stacking edges/weight-map borders.

### Stage 11 — `stretching`
Multi-pass non-linear: **GHS** on luminance → **masked stretch** to protect cores
→ **AutoDev** local contrast. Luminance/chrominance split. Interactive histogram.

### Stage 12 — `deconvolution`  *(optional)*  *[DIP-deconv option]*
Richardson-Lucy or van Cittert with PSF from stars, conservative iterations.
**DIP-deconv:** zero-shot alternative using PSF as forward operator.

### Stage 13 — `noise_reduction`  *[swinir-denoise-astro primary]*
- **Primary:** SwinIR denoiser (fine-tuned), applied via luminance mask.
- **Fallback:** wavelet/MMT for low-memory.
- **Optional:** DIP-denoise for linear data / unusual sensors, blended in linear
  space (e.g., 0.55 denoised + 0.45 original).

### Stage 14 — `star_segmentation_and_enhancement`  *[star-seg-v1]*
Segment stars from extended emission via CNN.
- Star layer: resize, color boost, reduce bloat, remove satellite trails.
- Background layer: contrast/structure, saturation.
- **Trail inpaint:** `trail-lama-tiny` (fast) or DIP-inpaint (high-quality).
Recombine.

### Stage 15 — `ai_super_resolution`  *[swinir-sr-astro-2x primary]*
- **Primary:** SwinIR SR 2×, applied after stretching.
- **Experimental:** StableSR (GPU-gated, integrity badge) — see §10.
Star-profile-preserving. Scale 2×/4× dialog.

### Stage 16 — `final_detail_enhancement`
Multi-scale unsharp mask, local contrast enhancement on midtones, optional
structure transfer from high-frequency layer.

### Stage 17 — `export`
TIFF (16/32-bit), PNG, JPEG, XISF (with history). Embed processing history as
XMP/sidecar JSON. Optional web gallery. Screen/print sharpening.

### 7.6 Narrowband → RGB Composition
**Detection:** ≥2 light groups with known narrowband names (Ha, OIII, SII) →
narrowband branch.

**Channel extraction (OSC + narrowband)**
| Filter | Primary channel | Notes |
|---|---|---|
| **Ha** | Red | Red holds nearly all Ha signal |
| **SII** | Red | Same as Ha; distinguish by filter name |
| **OIII** | Blue + Green | OIII at 495.9/500.7 nm hits blue & green |
| **Dual-band** | Red + Blue/Green | Split after debayer |

**Composition palettes**
| Palette | R | G | B | Use |
|---|---|---|---|---|
| **HOO** | Ha | OIII | OIII | 2-filter natural |
| **SHO** (Hubble) | SII | Ha | OIII | 3-filter false color |
| **HSO** | Ha | SII | OIII | Alt false color |
| **Custom** | pick | pick | pick | Advanced |

**Flow:** stack per filter group → extract channel → register groups to each other
→ combine to RGB → continue standard post-processing.

---

## 8. Planetary / Lunar Pipeline Variant

Same DAG skeleton, different parameters per stage:

| Stage | Deep-Sky | Planetary |
|---|---|---|
| Calibration | Full dark/flat/bias | Skip or optional dark |
| Registration | Star alignment | Feature tracking / limb detection |
| Stacking | Kappa-sigma, all frames | **Lucky imaging:** rank by sharpness, stack best 10–30% |
| Drizzle | Optional | **Strongly recommended** |
| Stretching | Gentle, preserve faint | Aggressive contrast for surface detail |
| Star handling | Remove/enhance stars | **No star removal** (planet is target) |
| Sharpening | Conservative | Aggressive unsharp / wavelet |
| AI SR | Optional | **Recommended** |
| **Co-add** | — | **DIP-coadd** (multi-frame lucky-imaging restoration) |

**Planetary memory strategy:** 50,000-frame ingest streams frames through a
two-pass rank/select: pass 1 computes per-frame sharpness (streaming, O(1) memory);
pass 2 re-reads only the selected top percentile for stacking. No full-frame-set
is held in RAM.

**Lunar-specific:** high dynamic range → optional HDR merge of exposure sets; no
atmospheric dispersion correction.

---

## 9. Interactive Dialog System

Every stage runs in one of three modes:
- **Auto** — recommended defaults, no prompt.
- **Confirm** — preview + metrics, "OK / Adjust / Skip."
- **Manual** — full parameter panel.

**Global verbosity:**
- 🟢 Beginner — mostly Auto.
- 🟡 Intermediate — Confirm on major stages.
- 🔴 Expert — Manual everywhere.

Dialogs support before/after slider, histogram overlay, "revert to auto," and
"save as preset."

---

## 10. AI Model Hub

### 10.1 Memory Budget (4–8 GB system)
- OS + app: ~1.5 GB · Image data (4K, 32-bit RGB): ~200 MB · Buffers: ~500 MB
- **Available for AI: ~1–2 GB**

### 10.2 Selection Criteria
< 500 MB inference · INT8 quantized · tiled inference · deterministic (default) ·
ONNX Runtime · permissive license for free distribution.

### 10.3 Model Registry
| Model | Task | Stage | Quantized size | Default? |
|---|---|---|---|---|
| `swinir-denoise-astro` | Denoising | 13 | ~15 MB | ✅ Primary |
| `swinir-sr-astro-2x` | Super-resolution | 15 | ~15 MB | ✅ Primary |
| `swin2sr-dejpeg` | JPEG/PNG artifact cleanup | 0.5 | ~15 MB | ✅ (lossy inputs) |
| `star-seg-v1` | Star/background mask | 14 | ~12 MB | ✅ |
| `cloud-score-v1` | Frame QC | 2 | ~8 MB | ✅ |
| `trail-lama-tiny` | Fast trail inpaint | 14 | ~30 MB | ✅ Primary |
| `DIP` (code-only) | Deconv / linear denoise / inpaint / planetary co-add | 12,13,14, planetary | ~0 MB (runtime ~0.5–1 GB tiled) | ⚙️ Optional slow path |
| `StableSR` | Perceptual SR | 15b | ~5+ GB VRAM | 🚫 Experimental, GPU-gated |
| `color-cal-net` | Photometric calibration | 9 | ~2 MB | ✅ |

Base install ≈ 90 MB; additional models download on-demand.

### 10.4 Tiling Strategy
```
tile_size = 512 (256 for SR); overlap = 64
for each tile:
    pad by overlap → infer → blend via cosine ramp
```
Keeps peak memory bounded; handles images larger than RAM.

### 10.5 Quality Ladder & Hardware Gating
| Tier | Denoise | SR | Extras | Hardware |
|---|---|---|---|---|
| **Fast** | U-Net fallback | ESRGAN-x2 | — | 4 GB, CPU |
| **Balanced** (default) | SwinIR | SwinIR 2× | — | 8 GB, CPU/iGPU |
| **Research fidelity** | SwinIR + DIP blend | SwinIR 2× | DIP-deconv, DIP-coadd | 8 GB+, patience |
| **Perceptual max** (experimental) | SwinIR | StableSR | integrity badge | dGPU ≥ 8 GB VRAM |

Probe hardware at startup; select tier accordingly.

### 10.6 Determinism & Reproducibility Policy
- Feed-forward models (SwinIR, LaMa, seg): deterministic given weights + backend;
  record model hash, backend, tile size.
- DIP: record seed, iteration count, early-stopping rule, blend ratio.
- StableSR: record seed + fidelity; auto-tag exports/recipes with integrity badge.

### 10.7 Model Notes (rationale)
- **SwinIR** (Apache 2.0): single backbone for denoise + SR + JPEG cleanup;
  windowed attention scales linearly with tile → ideal for small RAM.
- **Deep Image Prior:** zero trained weights; per-image optimization; validated in
  astronomy (lucky imaging, multi-frame restoration). Slow → master-image only.
- **StableSR:** ~8.9 GB VRAM per 512px tile, ~12 GB for 4K even tiled; slow;
  diffusion hallucination risk → experimental opt-in only.
- **GFPGAN:** excluded — face-only prior (StyleGAN2), wrong domain. Conceptual
  takeaway: a future "Star Prior GAN" for star-core restoration (v2+ research).

### 10.8 Model Supply-Chain Integrity
- Models fetched over HTTPS from a pinned registry; SHA-256 verified on download.
- Registry manifest signed; client rejects mismatched hashes.
- Recipes reference models by name+version+hash; apply fails fast on mismatch.
- No model is executed from an untrusted path without explicit user approval.

---

## 11. Collaboration & Recipe Sharing

**Decision: sidecar JSON supports shareable recipes.**

### 11.1 Recipe Format (sanitized, portable)
```json
{
  "recipe_version": "1.0",
  "app_version": "1.1.0",
  "name": "HOO Narrowband - M42",
  "author": "optional_username",
  "target_type": "deep_sky",
  "equipment_hints": { "camera": "Seestar S50", "filters": ["Ha","OIII"] },
  "pipeline": [
    { "stage": "calibration", "params": { "dark_scale": 1.0, "flat_normalize": true } },
    { "stage": "stacking", "params": { "algorithm": "kappa_sigma", "sigma": 3.0 } },
    { "stage": "ai_super_resolution", "params": { "model": "swinir-sr-astro-2x@1.0", "scale": 2 } }
  ],
  "model_versions": { "swinir-denoise-astro": "1.0", "swinir-sr-astro-2x": "1.0" },
  "integrity": { "perceptual_models_used": false }
}
```

### 11.2 Stripped Before Sharing
File paths, GPS/timestamps, machine-specific info (GPU model, OS).

### 11.3 Sharing Mechanisms
1. **Export/Import:** `.astroforge-recipe` JSON file.
2. **In-app Recipe Gallery:** browsable, filterable by target/equipment/palette.
3. **Apply:** check required models (prompt download), validate compatibility, apply.

### 11.4 Privacy & Safety
Recipes only set parameters (read-only intent); model refs by name+version; no
auto-download of third-party models without confirmation.

---

## 12. Performance & Resource Strategy

- **Streaming:** one frame at a time through calibration → registration → stacking
  accumulator (no full-session RAM hold).
- **GPU:** FFTs (registration), AI inference, drizzle.
- **Checkpointing:** intermediate FITS per stage (crash-safe).
- **Parallelism:** per-frame ops in a thread pool.
- **Progress:** accurate ETA from per-stage timing history.

---

## 13. Platform & Optimization Sub-Plan

**Primary:** Tauri 2.x (Rust + WebView).

| Platform | GPU backend | AI backend | Packaging |
|---|---|---|---|
| **macOS** | Metal | CoreML → ONNX CPU | .dmg, Apple Silicon native, notarized |
| **Windows** | DirectML / CUDA | DirectML / CUDA / OpenVINO | .msix, optional CUDA installer |
| **Linux** | Vulkan / CUDA | CUDA / OpenVINO / CPU | .AppImage + .deb + .rpm + Flatpak |

**Build order:** Windows + macOS first (largest smart-telescope base), then Linux.

**App updates:** Tauri built-in updater with signed payloads; crash-reporting and
telemetry are opt-in only, configured on first launch.

---

## 14. Output & Deliverables

- Final image (TIFF/PNG/JPEG)
- Processing report (PDF/HTML): frame stats, rejections, parameters
- Sidecar JSON with full DAG + params (reproducible, shareable)
- Optional: layered PSD/XCF (star / background / enhancement layers)
- Optional: web gallery with zoom/pan

### 14.5 Project & Session Management
- A "project" = session manifest + DAG state + checkpoint refs in SQLite.
- Resume re-loads the DAG graph and last completed stage; previews restored from cache.
- History list of recent projects; per-project processing report and recipe.
- Crash recovery: on launch, detect interrupted project and offer resume.

---

## 15. Extensibility

- **Plugin API:** WASM (preferred) or sandboxed Python plugins add/replace stages.
  - WASM path: zero added runtime, ~5 MB shell preserved, deterministic, cross-platform.
  - Python path: optional sidecar via user-installed Python (not bundled); for power users only.
  - All plugins run in a capability-scoped sandbox (filesystem allowlist, no network by default).
- **Custom AI models:** drop ONNX into `~/AstroForge/models/`.
- **Export plugins:** new output targets (e.g., AstroBin upload).

---

## 16. Build Roadmap

### MVP (6–8 weeks)
**Scope guard:** MVP targets FITS-only ingest, OSC debayer, single-filter deep-sky.
PNG/JPG/DNG Bayer detection, narrowband, and planetary move to v1.
- Ingest with FITS Bayer detection
- Deep-sky routing (planetary stub)
- Calibration (dark/flat)
- Registration + Kappa-Sigma stacking
- Basic stretch + export · Beginner dialog mode

**MVP Definition of Done**
- End-to-end: drop a FITS light+dark+flat folder → exported 16-bit TIFF.
- Kappa-sigma stack of ≥30 frames on a 4 GB machine without OOM.
- Beginner dialog mode passes a scripted smoke test on Windows + macOS.

### v1 (+8 weeks)
- PNG/JPG/DNG Bayer detection
- Narrowband detection + HOO/SHO composition
- Planetary/lunar pipeline (lucky imaging, drizzle)
- Plate solving (ASTAP) + auto-crop
- Background extraction, color calibration, SCNR
- Star segmentation + enhancement · **SwinIR denoise + SR + de-JPEG**
- DIP optional stages (deconv + inpaint)
- Intermediate/Expert modes · checkpointing · project resume · basic recipe export/import

### v2 (+8 weeks)
- DIP-coadd for planetary branch
- Satellite trail removal (LaMa)
- Recipe gallery · platform optimizations (Metal, CUDA)
- Plugin API (WASM first) · DNG support
- StableSR experimental plugin (GPU-gated)
- Evaluate "Star Prior GAN" research

---

## 17. Open Items & Next Steps

1. **Smart-telescope SDK integration:** pull from telescope apps (Seestar/Unistellar
   APIs) vs. process exported files only?
2. **Live stacking preview** for planetary (live vs. post-stack)?
3. **Recipe gallery hosting:** GitHub-based repo vs. static JSON index (low cost)?
4. **License verification:** SwinIR confirmed Apache 2.0; verify StableSR & DIP repo
   licenses before bundling.
5. **SwinIR astro fine-tuning dataset:** source/build training data.
6. **DIP defaults:** iteration count + early-stopping heuristic.
7. **SwinIR vs Real-ESRGAN A/B** on real smart-telescope data before final commit.
8. **Plate-solve dependency decision:** bundle ASTAP vs. online astrometry.net vs. defer.
9. **Plugin runtime decision:** WASM-only vs. optional Python sidecar (impacts shell size).
10. **Planetary memory:** 50,000-frame ingest strategy — streaming rank/select without holding all frames.
11. **App auto-update mechanism** (Tauri updater) and crash-reporting/telemetry opt-in policy.
12. **Accessibility (WCAG AA) and i18n** scope — in scope for v1 or deferred?

---

*End of specification.*
