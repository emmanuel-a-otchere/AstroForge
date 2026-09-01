# Change Request: AF-CR-2026-09-01-IMG-PIPELINE

**Title:** Implement Guided Multi-Mode Image Processing Train with Non-Destructive Editing and Backend AI Support  
**Priority:** High  
**Target Release:** Next major iteration of AstroForge  
**Requester:** Product / Architecture (informed by AstroWizard changelog synthesis)  
**Date:** 2026-09-01  

## 1. Summary
Introduce a structured, interactive image processing train (pipeline) that converts raw or stacked smart-telescope imagery into publication-ready deep-sky images. The pipeline must support fully non-destructive editing, expose backend AI assistance for every major capability, and offer three explicit user-selectable operating modes:

- **Automagic** – fully AI-driven, one-click or minimal-input path.
- **Automagic Expert** – AI proposes and applies optimisations while exposing interactive dialogs and controls so the user can accept, refine, or selectively re-apply changes equally across selected features.
- **Pure Expert** – AI disabled; every parameter and sub-step is fully exposed at maximum granularity.

## 2. Background & Motivation
AstroWizard's rapid evolution demonstrated that users of smart-telescope data value:
- A clear, ordered process train rather than a free-form toolbox.
- Live, truthful previews that never "jump" after noise or stretch operations.
- Non-destructive multi-save / Undo-after-save behaviour.
- Data-type-aware guidance (OSC / dual-band / mono) without nagging.
- Exact, reversible algorithmic steps (especially stretch and star handling).
- Graceful orchestration of external or backend AI engines with clear free/paid paths and fallbacks.

## 3. Scope
**In scope**
- Definition and implementation of the core image process train.
- Non-destructive editing model (history, layers/masks, parameter stacks).
- Backend AI service layer that can support every pipeline stage.
- Three selectable operating modes with clear UI/UX and behaviour differences.
- Algorithmic nuances required for high-quality deep-sky results (stretch, gradient, colour, sharpen, denoise, star separation/replace, narrowband mixing).
- Smart-telescope and mono/dual-band awareness.
- Live preview system with stable statistics and real-pixel zoom/pan.

**Out of scope (for this CR)**
- Full stacking engine (may be a future sibling or hand-off interface).
- Cloud processing or multi-user collaboration.
- New proprietary neural-network training (leverage or wrap existing engines where possible).

## 4. Functional Requirements

### 4.1 Image Process Train (Core Pipeline)
The pipeline shall be a strictly ordered sequence of stages. Each stage produces a reversible, versioned result that can be inspected, adjusted, or bypassed according to the active mode.

Recommended canonical stages:

1. **Ingest & Analyse** – Load FITS/TIFF/XISF (and common smart-telescope formats). Auto-detect camera type, filter set (OSC / dual-band / mono Ha/OIII/SII/LRGB), bit depth, linear vs stretched state, and basic statistics. Produce a data-type declaration used by later guidance.
2. **Framing / Crop / Rotate** – Interactive free-select crop with live rotation, aspect-ratio presets, and meridian-flip awareness. Must be an explicit editing decision, never a silent auto-crop of the final result.
3. **Gradient / Background Extraction**
4. **Colour Calibration / Balance** (bounded corrections; dual-band and mono-aware)
5. **Sharpen / Deconvolution**
6. **Denoise**
7. **Stretch** – Data-anchored "Deep" engine that solves its own black/white points and strength; multi-preview grid (Soft / Normal / Aggressive / Deep / Deep-keep-colours / Custom); "Keep this look" one-click commit of the current preview.
8. **Star Handling** – Separation -> independent editing of starless and stars layers -> exact or soft replace with live strength and colour-boost controls. Mathematically exact star layers required so replace is lossless.
9. **Creative / Final Polish** – Curves (including saturation channel and colour-family targeting), one-click colour-transmutation "spells" with editable recipes, narrowband palette mixes, final tone and detail adjustments.
10. **Export** – Multi-format (FITS master, TIFF, JPEG, starless, stars-only), non-destructive (session continues after save), with clear success/failure messaging.

Each stage must emit:
- A versioned intermediate result.
- A human-readable receipt / log entry.
- Updated image statistics used by the live preview system.

### 4.2 Non-Destructive Editing
- Full history stack with unlimited Undo/Redo (including after export).
- Parameter sets and masks stored separately from pixel data so any earlier stage can be revisited and re-executed without destroying later work (or with explicit "re-apply from here" choice).
- Ability to save multiple versions / formats without terminating the session.
- "Hold to Compare" and side-by-side original vs current views at any stage.
- Explicit warning when re-running an already-applied AI or irreversible-looking step.
- Crop, stretch, and star-replace operations must be reversible to the exact pre-operation state.

### 4.3 Backend AI Support & Operating Modes
A unified backend AI service layer shall be available to every pipeline stage. The layer must support:
- Local or remote inference.
- Graceful degradation / CPU fallback.
- Clear status and error reporting to the UI.
- Optional free-path engines alongside higher-quality paid/accelerated engines.

**Mode definitions:**

| Mode | AI Behaviour | User Interaction | Granularity |
|------|--------------|------------------|-------------|
| **Automagic** | Fully autonomous. AI selects and applies optimal parameters for the detected data type with minimal or zero user input. | Single "Process" or per-stage "Auto" buttons. Progress and final result only. | Hidden. User sees only high-level stage status. |
| **Automagic Expert** | AI analyses the image, proposes parameter sets and optimisations, and can apply them. User retains the ability to accept, reject, or refine any proposal through interactive dialogs and live controls. Changes can be applied equally (batch) to selected features or stages. | AI suggestions appear in dialogs with live preview. User can adjust sliders/curves inside the suggestion, then apply to current stage or propagate to matching features. "Apply equally to selected" control. | Medium - key parameters and masks exposed inside AI-driven dialogs. |
| **Pure Expert** | AI completely disabled / ignored. | Every control, sub-parameter, mask, and intermediate buffer is exposed. Manual sequencing of sub-steps allowed. | Maximum - full algorithmic surface area visible and editable. |

Mode state must be persisted with the session and clearly indicated in the UI at all times. Switching modes mid-session shall offer the choice to keep current pixel state or re-process from a chosen stage under the new mode.

### 4.4 Algorithmic Nuances (Mandatory Implementation Guidance)
- **Stretch** - Prefer data-anchored solvers over simple histogram pushes. Provide both channel-equalising and colour-preserving Deep variants. Multi-preview grid with synced zoom is required so users can choose by eye.
- **Preview stability** - Denoise and sharpen must not alter the display stretch statistics; the preview "aim" remains constant so the image does not appear to brighten or change character after noise reduction.
- **Star maths** - Separation must produce exact complementary layers so replace (Exact or Soft blend) is mathematically lossless. Strength and colour controls operate on the returned stars only.
- **Colour** - Auto-balance corrections must be bounded. Dual-band and mono data receive specialised treatment and clear labelling.
- **Honesty** - Every failure, warning (hot pixels, blank frames, already-applied steps, imperfect alignment, linear data, etc.) and data-type decision must surface a clear, actionable message. No silent auto-crops or hidden range changes.
- **Performance** - Heavy stages (curves, narrowband, stretch commits) run multi-core / off-main-thread. Full-resolution preview renders only when the user rests input.
- **Smart-telescope awareness** - Header and filename dialects for common devices (Seestar, Dwarf family, etc.) and filter naming conventions must be recognised and used for guidance and calibration decisions.

## 5. Non-Functional Requirements
- Cross-platform parity (Windows, macOS Apple Silicon + Intel, Linux).
- Live preview remains responsive on modest hardware; AI stages show measured progress and estimated time.
- Session state (including mode, history, and all intermediate results) must survive crashes via autosave with explicit "keep or discard" after successful export.
- Clear separation between free and accelerated AI paths with transparent messaging.

## 6. Acceptance Criteria
1. User can complete a full process train from load to multi-format export in all three modes.
2. Switching between modes mid-session behaves as specified and never silently loses work.
3. Undo/Redo works across every stage, including after export, and restores exact prior pixel + parameter state.
4. Live previews remain statistically stable across denoise/sharpen and support real-pixel zoom/pan/refit.
5. Stretch stage offers a multi-preview grid and data-anchored Deep options; "Keep this look" commits the exact viewed result.
6. Star separation + replace is mathematically exact (verifiable by difference maps).
7. Automagic Expert mode surfaces interactive dialogs that allow selective or equal application of AI proposals.
8. Pure Expert mode exposes every control with no AI influence.
9. All warnings, receipts, and data-type decisions are visible and actionable.
10. Pipeline respects smart-telescope and mono/dual-band metadata for guidance and processing decisions.

## 7. Risks & Mitigations
- **AI quality variance** -> Provide free-path engines + clear quality indicators; allow Pure Expert escape hatch.
- **Performance on large frames** -> Mandatory multi-core / streaming design; progressive preview.
- **Mode confusion** -> Persistent mode indicator, confirmation on switch, and mode-specific tooltips.
- **Scope creep** -> Strict adherence to the ordered train; additional creative tools only inside stage 9.

## 8. Implementation Notes
- Treat the process train as a state machine with versioned artefacts rather than a simple linear script.
- Design the AI service interface so any stage can request analysis, parameter suggestion, or full execution, then receive a structured proposal that Automagic Expert can surface.
- Prioritise the Stretch and Star Handling stages early-these historically produce the highest impact and the most user feedback.
- Instrument every stage with the honesty/receipt pattern from day one.
