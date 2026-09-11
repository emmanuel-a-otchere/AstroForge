CR-04 — Intelligent Import, Session Understanding & Target Detection

Status: Partial — Target Detection (P4, PR #253) + Session Grouping (P5, PR #301) + Capture Analysis (P6, PR #302) + Narrowband Detection (P7, PR #303) + IPC layer (P8, PR #304) shipped; P9 (Import Review UI) pending
Target: AstroForge v1.0
Priority: Critical
Depends on: CR-01, CR-02, CR-03
Enables: CR-05, CR-06, CR-09, CR-13, CR-14, CR-15, CR-18, CR-23

> **Implementation note (2026-09-11):** The CR-04 tranche
> shipped target_detection (FITS OBJECT keyword + filename +
> directory pattern + ~50-target catalog) behind PR #253 plus
> earlier slices in the same family. The P5/P6/P7 follow-on
> slices (session_grouping / capture_analysis / narrowband) are
> scoped in the CR-04 plan but not yet shipped. See
> `docs/plans/2026-09-06-cr04-intelligent-import/PLAN.md` for the
> tranche plan and `CHANGELOG.md` for per-slice entries.

1. Intent

CR-04 establishes Import Intelligence as the first substantive intelligence layer of AstroForge.

The fundamental product decision is:

AstroForge must understand what the user captured before deciding how to process it.

Import therefore cannot be treated as a conventional file-picker operation. The user should be able to point AstroForge at a folder containing raw telescope data and have the application determine:

* what files are present;
* which files are usable;
* what each file represents;
* whether files belong together;
* what target was photographed;
* what acquisition characteristics can be inferred;
* whether the dataset is deep-sky or planetary/lunar;
* whether calibration data exists;
* whether Bayer information is available or needs detection;
* whether narrowband data is present;
* and what processing route is most appropriate.

This is particularly important because the current repository already contains dedicated core capabilities such as bayer_detection.rs, calibration.rs, debayer.rs, and artifact/database infrastructure.

CR-04 therefore turns those capabilities into a coherent user-facing ingestion intelligence system, rather than exposing them as isolated technical functions.

⸻

2. Product Decision

2.1 Import becomes an intelligence pipeline

The canonical flow is:

Drop / Select Data

↓

Scan

↓

Identify

↓

Classify

↓

Group

↓

Understand

↓

Validate

↓

Confirm

↓

Create Session

↓

Recommend Pipeline

The user should not have to manually establish all of these relationships.

2.2 AstroForge should infer first and ask second

The application should make intelligent assumptions where evidence is strong.

For example:

382 Light frames detected
12 Dark frames detected
Seestar OSC data detected
Bayer pattern: RGGB — high confidence
Target: M31 / Andromeda Galaxy
Acquisition appears to be deep-sky

Recommended workflow: Deep-Sky OSC
Calibration → Debayer → Registration → Stacking → Background → Color → Stretch → AI Enhancement

The user should be able to simply select:

Accept Recommendation

rather than configuring every stage.

2.3 Uncertainty must be explicit

AstroForge must never disguise an inference as known metadata.

Every important inference has:

Observation → Evidence → Confidence → Decision

For example:

Target type: Deep-Sky
Confidence: High
Evidence: 30–120 s exposures, 400+ frames, star-field characteristics.

Versus:

Target type: Planetary/Lunar
Confidence: Low
Evidence: short exposures and high frame count.
Please confirm.

This becomes the foundational contract for later AI intelligence.

⸻

3. Import Intelligence Model

CR-04 introduces an Import Intelligence Layer between the filesystem and the Project/Session model.

                 USER DATA
                    │
                    ▼
             ┌─────────────┐
             │ File Scanner│
             └──────┬──────┘
                    │
                    ▼
          ┌────────────────────┐
          │ Metadata Extraction│
          └─────────┬──────────┘
                    │
          ┌─────────▼──────────┐
          │ File Classification│
          └─────────┬──────────┘
                    │
          ┌─────────▼──────────┐
          │ Dataset Grouping   │
          └─────────┬──────────┘
                    │
          ┌─────────▼──────────┐
          │ Target Detection   │
          └─────────┬──────────┘
                    │
          ┌─────────▼──────────┐
          │ Capture Analysis   │
          └─────────┬──────────┘
                    │
          ┌─────────▼──────────┐
          │ Pipeline Recommend.│
          └─────────┬──────────┘
                    │
                    ▼
              USER CONFIRMATION
                    │
                    ▼
             ASTROFORGE SESSION

The key architectural distinction is:

Scanning discovers files. Import Intelligence constructs meaning.

⸻

4. File Discovery

AstroForge must initially support the formats established by the product specification:

* FITS
* PNG
* JPEG/JPG
* DNG
* TIFF
* associated calibration data where applicable.

Discovery should recursively scan the selected source according to user configuration.

Each discovered file receives an initial state:

* discovered
* supported
* unsupported
* unreadable
* duplicate
* candidate
* excluded

Import must never modify the source file.

This follows CR-02’s immutable Source Asset principle.

⸻

5. Metadata Extraction

For each usable asset AstroForge should extract whatever metadata is available.

FITS

Potential metadata includes:

* OBJECT
* DATE-OBS
* EXPTIME
* FILTER
* XBINNING / YBINNING
* CCD dimensions
* BITPIX
* image dimensions
* Bayer-related metadata
* telescope/instrument information
* camera information
* focal length where available
* gain/offset where available
* temperature where available.

DNG / TIFF / JPEG / PNG

Extract:

* dimensions;
* bit depth;
* EXIF;
* camera information;
* acquisition date;
* color information;
* embedded profiles;
* orientation;
* Bayer information where available.

Metadata should be preserved as part of the Source Asset record rather than discarded after ingestion.

⸻

6. Frame Classification

AstroForge should classify frames into semantic astrophotography roles.

Primary classes

Class	Meaning
Light	Science/exposure image containing the astronomical target
Dark	Dark-current calibration frame
Flat	Optical/vignetting/dust calibration frame
Bias	Readout-offset calibration frame
Unknown	Cannot confidently classify
Unsupported	Format/content cannot be processed
Invalid	File exists but cannot be reliably read

Classification should use a combination of:

1. filename conventions;
2. directory structure;
3. FITS/EXIF metadata;
4. exposure characteristics;
5. image dimensions;
6. image statistics;
7. temporal relationships;
8. filter/binning relationships;
9. user-provided information.

No single heuristic should be treated as authoritative.

⸻

7. Bayer Pattern Intelligence

The repository already contains a dedicated Bayer detection implementation, making this an important integration point for CR-04.

CR-04 formalizes its role.

Possible states:

Bayer Pattern
│
├── Explicit metadata
│      └── RGGB
│
├── Strong inferred detection
│      └── RGGB / BGGR / GRBG / GBRG
│
├── Weak inference
│      └── Requires confirmation
│
└── No Bayer pattern
       └── Monochrome / already debayered

The UI should never simply say:

Bayer = RGGB

It should say:

Bayer pattern: RGGB
Detected from image metadata
Confidence: High

or:

Bayer pattern: RGGB
Inferred from image structure
Confidence: Medium
[Change]

This distinction becomes important for deterministic reproducibility.

⸻

8. Dataset Grouping

Files should not simply become one large session.

AstroForge should establish logical groups using:

* target;
* acquisition date/time;
* instrument;
* filter;
* binning;
* image dimensions;
* exposure characteristics;
* directory relationships.

For example:

M31
│
├── Session — 2026-08-14
│   ├── Lights — L
│   ├── Darks
│   └── Flats
│
├── Session — 2026-08-15
│   ├── Lights — L
│   ├── Darks
│   └── Flats
│
└── Session — 2026-08-16
    ├── Lights — Ha
    └── Lights — OIII

This directly implements the distinction introduced in CR-02:

Target ≠ Session ≠ Source Asset.

Multiple acquisition sessions can contribute to the same astronomical target.

⸻

9. Target Detection

Target identification should use multiple signals.

Evidence hierarchy

Highest confidence

1. Explicit FITS OBJECT.
2. Explicit user metadata.
3. Recognizable acquisition metadata.
4. Directory/project naming.
5. Filename patterns.

Secondary intelligence

6. Plate solving, when available.
7. Astronomical coordinate inference.
8. Image-based astronomical recognition, where supported.

Target detection should produce:

Target
Name: M31
Type: Galaxy
RA: ...
DEC: ...
Confidence: High
Evidence:
  • FITS OBJECT = M31
  • Consistent filenames

Importantly, target recognition and plate solving are related but distinct.

A future plate-solving stage can establish coordinates even when the target name is unknown.

⸻

10. Deep-Sky vs Planetary/Lunar Detection

This becomes a critical routing decision.

AstroForge should evaluate:

Deep-sky indicators

* longer exposure;
* many subframes;
* stellar field;
* nebular/galactic morphology;
* calibration-frame availability;
* tracking characteristics;
* relatively stable framing.

Planetary/lunar indicators

* very high frame count;
* short exposures;
* compact bright subject;
* planetary disk morphology;
* lunar limb/features;
* video-like acquisition;
* rapid frame-to-frame variation.

The result should be expressed as a recommendation:

Likely Deep-Sky — 94% confidence

or:

Likely Planetary/Lunar — 61% confidence
Dataset characteristics are ambiguous.

If confidence is insufficient, AstroForge must ask.

Critical rule

AstroForge must never silently route an ambiguous dataset into an irreversible processing strategy.

⸻

11. Narrowband Detection

CR-04 should also identify likely narrowband acquisition.

Signals include:

* FILTER metadata;
* filenames;
* repeated target/session structure;
* filter names such as Ha, Hα, OIII, SII;
* channel-specific acquisition groups.

Example:

Target: NGC 7000
Detected:
  Ha   — 124 frames
  OIII — 108 frames
Suggested composition:
  HOO

The system should recommend, not silently impose, the final composition.

This feeds directly into CR-14 — Narrowband Studio.

⸻

12. Import Review UI

The import experience should be a guided intelligence dialog rather than a conventional file browser.

Step 1 — Add Data

┌───────────────────────────────────────┐
│           ADD ASTRO DATA              │
│                                       │
│   Drop a folder or files here         │
│                                       │
│          [ Choose Folder ]            │
│                                       │
│   FITS • DNG • TIFF • PNG • JPEG      │
└───────────────────────────────────────┘

⸻

Step 2 — Scanning

Analyzing your data…
✓ 462 files discovered
✓ 462 readable
✓ Metadata extracted
✓ 438 Light frames
✓ 18 Dark frames
✓ 6 Flat frames
Analyzing image characteristics…

The user should not see raw technical logs.

⸻

Step 3 — Understanding

WHAT DID YOU CAPTURE?
Target
M31 — Andromeda Galaxy
Confidence: High
Capture type
Deep-Sky
Confidence: High
Camera
OSC
Confidence: High
Bayer
RGGB
Confidence: High
Calibration
Dark + Flat detected
Narrowband
None detected

⸻

Step 4 — Recommendation

ASTROFORGE RECOMMENDS
Deep-Sky OSC Workflow
Calibration
        ↓
Debayer
        ↓
Registration
        ↓
Stacking
        ↓
Background correction
        ↓
Color calibration
        ↓
Stretch
        ↓
AI denoise
        ↓
Detail enhancement
[ Accept & Continue ]
[ Review Settings ]

This is the first point at which AstroForge begins to feel like an intelligent astrophotography application, rather than a collection of image-processing algorithms.

⸻

13. Ambiguity UX

If AstroForge cannot establish a reliable conclusion, the UI should be concise.

Example:

We need your help

These 1,240 frames could be either planetary or deep-sky data.

What were you photographing?

○ Deep-Sky
○ Planet / Moon
○ I’m not sure

[Continue]

The application should avoid presenting the user with a dozen technical questions.

⸻

14. Manual Override

Every automated decision must be overridable.

For example:

Target Type
● Deep-Sky
○ Planetary / Lunar
Detected: Deep-Sky
Confidence: Medium
Why?
Long exposures and star-field characteristics.
[ Change ]

If changed:

User override:
TargetType = PlanetaryLunar
Reason = User selected
Timestamp = ...

This override becomes part of provenance.

⸻

15. CR-02 Data Model Extension

CR-04 extends the project model with import intelligence entities.

import_run

id
project_id
source_location
started_at
completed_at
status
application_version
engine_version

import_asset

id
import_run_id
source_asset_id
discovery_state
readability_state
classification
classification_confidence
metadata_status

asset_observation

id
asset_id
observation_type
value
source
confidence
created_at

Examples:

bayer_pattern = RGGB
source = image_analysis
confidence = 0.96

classification_evidence

id
asset_id
signal
value
weight

session_detection

id
target_candidate
session_candidate
confidence
evidence

user_override

id
entity_type
entity_id
field
previous_value
new_value
reason
timestamp

The exact schema should be reconciled with the existing database implementation rather than blindly creating duplicate persistence structures.

⸻

16. Import State Machine

CR-04 should establish a deterministic state machine:

DISCOVERING
     │
     ▼
SCANNING
     │
     ▼
EXTRACTING_METADATA
     │
     ▼
CLASSIFYING
     │
     ▼
GROUPING
     │
     ▼
ANALYZING
     │
     ▼
RECOMMENDING
     │
     ├──────────────┐
     ▼              ▼
CONFIRMATION     AMBIGUOUS
     │              │
     │              ▼
     │          USER_DECISION
     │              │
     └───────┬──────┘
             ▼
       SESSION_CREATED
             │
             ▼
      READY_FOR_PROCESSING

Failure at any stage should support:

* retry;
* skip affected assets;
* inspect;
* cancel;
* resume.

⸻

17. Error Handling

Import errors must be isolated rather than catastrophic.

For example:

7 files could not be read

AstroForge imported the remaining 455 files successfully.

The affected files appear to be corrupted or incomplete.

[Review Files] [Continue]

Not:

FITSIO ERROR: status=123

Technical diagnostics can be available under Diagnostics → Details.

⸻

18. Duplicate Detection

CR-02 established content hashing as an identity mechanism.

CR-04 should use that capability to detect:

* exact duplicates;
* duplicate files with different names;
* repeated imports of the same source;
* potentially equivalent assets where metadata differs.

Example:

24 duplicate files detected

They already exist in this project.

[Ignore duplicates] [Import as separate references]

This prevents accidental double-stacking.

⸻

19. Recommendation Contract

Import Intelligence should produce a machine-readable recommendation.

Conceptually:

ImportUnderstanding
 ├── Target
 ├── TargetType
 ├── AcquisitionMode
 ├── CameraProfile
 ├── BayerPattern
 ├── FrameGroups
 ├── CalibrationAvailability
 ├── FilterGroups
 ├── SessionGroups
 ├── QualityIndicators
 ├── Confidence
 ├── Evidence
 ├── Warnings
 └── RecommendedRecipe

This object becomes the bridge between:

CR-04 → CR-05 Processing Workspace

and ultimately:

Import Intelligence → DAG construction.

⸻

20. AI Boundary

CR-04 should initially favor deterministic heuristics.

The intelligence hierarchy should be:

Level 1 — Metadata

Fastest and most trustworthy.

Level 2 — Deterministic image analysis

Statistics, dimensions, exposure characteristics, Bayer analysis, etc.

Level 3 — Computer vision / AI

Used when deterministic evidence is insufficient.

Level 4 — User confirmation

The final authority when ambiguity remains.

This establishes a crucial AstroForge principle:

AI assists interpretation; it does not replace provenance or user authority.

⸻

21. Architecture Impact

CR-04 introduces an explicit boundary:

┌───────────────────────────────────────────┐
│                AstroForge UI              │
└───────────────────┬───────────────────────┘
                    │
                    ▼
┌───────────────────────────────────────────┐
│          Import Intelligence API          │
└───────────────────┬───────────────────────┘
                    │
       ┌────────────┼────────────┐
       ▼            ▼            ▼
   Scanner      Metadata      Analysis
                  Engine        Engine
       │            │            │
       └────────────┼────────────┘
                    ▼
              Understanding
                    │
                    ▼
               Session Model
                    │
                    ▼
              Recipe Selector
                    │
                    ▼
               Pipeline DAG

This is preferable to having the UI directly invoke:

scan()
detect_bayer()
classify()
...

The UI should consume semantic results.

⸻

22. Implementation Map

The current repository has a three-crate structure under crates/, including astroforge-core, astroforge-app, and astroforge-ai.

CR-04 should therefore be mapped approximately as follows.

astroforge-core

Existing relevant capabilities include:

* bayer_detection.rs
* calibration.rs
* debayer.rs
* artifact.rs
* db.rs

These should be integrated rather than duplicated.

New/expanded conceptual modules:

import.rs
import_scan.rs
metadata.rs
classification.rs
frame_classification.rs
session_detection.rs
target_detection.rs
capture_analysis.rs
import_recommendation.rs
import_provenance.rs

astroforge-app

Expose semantic operations such as:

start_import
get_import_progress
get_import_understanding
confirm_import
override_import_classification
create_session_from_import
get_pipeline_recommendation

astroforge-ai

Later AI-backed operations can implement:

target recognition
capture-type classification
astronomical object classification
advanced image-quality assessment

without contaminating the deterministic core.

⸻

23. Test Strategy

CR-04 requires a dedicated fixture corpus.

Dataset categories

1. OSC FITS deep-sky.
2. OSC FITS with explicit Bayer metadata.
3. OSC data requiring Bayer inference.
4. PNG/JPEG smart-telescope exports.
5. DNG raw captures.
6. Dark/flat/bias sets.
7. Mixed folders.
8. Multiple acquisition nights.
9. Narrowband Hα/OIII/SII.
10. Planetary datasets.
11. Lunar datasets.
12. Ambiguous datasets.
13. Corrupted files.
14. Unsupported files.
15. Duplicate imports.
16. Missing metadata.
17. misleading filenames.
18. inconsistent directory structures.

Required testing dimensions

Functional

Can the system correctly discover and classify assets?

Determinism

Does identical input produce identical analysis?

Robustness

Does one malformed file avoid destroying the import?

Explainability

Can every significant inference show its evidence?

Persistence

Does the Import Run survive application restart?

Override

Can users change an incorrect inference?

Regression

Do improvements to classification avoid breaking known datasets?

⸻

24. Acceptance Criteria

CR-04 is complete only when:

Import

* [ ]	User can import a folder without external preprocessing.
* [ ]	Supported formats are detected automatically.
* [ ]	Unsupported files are isolated.
* [ ]	Corrupt files are isolated.
* [ ]	Source assets remain immutable.
* [ ]	Duplicate assets are detected.

Understanding

* [ ]	Metadata is extracted where available.
* [ ]	Light/Dark/Flat/Bias classification is performed.
* [ ]	Bayer pattern is detected where relevant.
* [ ]	Confidence accompanies inferred values.
* [ ]	Evidence is retained.
* [ ]	Target candidates can be established.
* [ ]	Deep-sky vs planetary/lunar routing is performed.
* [ ]	Narrowband groups can be detected.

Session

* [ ]	Assets are grouped into coherent Sessions.
* [ ]	Sessions can belong to a Target.
* [ ]	Multiple sessions can contribute to one Target.
* [ ]	Manual corrections are persisted.

Recommendation

* [ ]	AstroForge generates a recommended processing route.
* [ ]	Recommendation is explainable.
* [ ]	User can accept or modify it.
* [ ]	Recommendation becomes the basis for pipeline creation.

Recovery

* [ ]	Import can be interrupted.
* [ ]	Import can resume.
* [ ]	Import failure does not corrupt the Project.
* [ ]	Restart preserves import state.

Standalone

* [ ]	No Python installation is required.
* [ ]	No external astrophotography software is required.
* [ ]	No manual FITS preparation is required.
* [ ]	Technical errors are translated into actionable UI.
* [ ]	Core import intelligence works offline.

⸻

25. ADRs Introduced by CR-04

ADR-04.1 — Import Is an Intelligence Stage

Import is not merely file acquisition. It establishes the semantic context required for processing.

ADR-04.2 — Evidence Before Inference

Every important automated conclusion must have observable evidence.

ADR-04.3 — Confidence Is First-Class

Inference without confidence is insufficient for an intelligent application.

ADR-04.4 — User Override Is Authoritative

When the user explicitly overrides an inference, that decision becomes the active project state and is recorded in provenance.

ADR-04.5 — Deterministic First

Metadata and deterministic image analysis precede AI interpretation whenever practical.

ADR-04.6 — No Silent Routing on Ambiguity

AstroForge must request user confirmation when confidence is inadequate for safe pipeline selection.

⸻

26. Definition of Done

The definitive CR-04 scenario is:

Launch AstroForge → Create/Open Project → Drop a raw telescope folder → AstroForge scans it → identifies files → extracts metadata → classifies frames → detects Bayer characteristics → identifies target/session relationships → determines likely acquisition type → detects calibration/narrowband characteristics → explains its conclusions → recommends the appropriate processing workflow → user accepts or corrects the interpretation → AstroForge creates a persistent Session and processing recommendation → Project can be closed and reopened with the entire interpretation intact.

At this point AstroForge has crossed an important product boundary:

It no longer merely accepts images. It understands the dataset.

⸻

Strategic consequence

CR-01 established how AstroForge should feel.

CR-02 established what AstroForge must remember.

CR-03 established where the user works.

CR-04 establishes how AstroForge understands what the user has given it.

That makes CR-05 the natural next step:

CR-05 — Intelligent Processing Workspace & Adaptive Pipeline Execution

CR-05 should take the Session Understanding + Recommendation produced here and turn it into the actual image-processing experience, including the human-readable pipeline, live previews, execution controls, checkpoints, adaptive parameter recommendations, progress, recovery, and creation of Image Versions.