CR-02 — Project, Session & Artifact Architecture

Status: Proposed
Target: AstroForge v1.0
Priority: Critical / Foundational
Depends on: CR-01 — Product & UX Foundation
Enables: CR-03 onward, particularly the Studio shell, processing workspace, AI enhancement, recipes, comparison, recovery, and standalone execution.

CR-02 establishes the persistent domain model of AstroForge. The key decision is that AstroForge must not treat a folder of images as the application state. A folder is merely an input source.

The application needs a durable Project → Session → Source → Pipeline Run → Artifact → Image Version → Export model so that processing can be resumed, inspected, compared, reproduced, and safely modified.

The existing specification already establishes SQLite for project/session state and a filesystem artifact store, while the processing pipeline is explicitly designed around independently rerunnable DAG stages, previews, quality metrics, checkpoints, pause/resume and rollback.

⸻

1. Intent

CR-02 introduces the canonical persistence and lifecycle model for AstroForge.

It must answer:

* What is an AstroForge Project?
* What constitutes an astrophotography Session?
* How are original files represented?
* How are intermediate processing products stored?
* How does a pipeline run relate to an image?
* How are AI operations recorded?
* How can the user return to an earlier result?
* How can AstroForge resume after interruption?
* How can a result be reproduced?
* How can multiple acquisition sessions contribute to one final image?
* How can the application remain functional completely offline?

Fundamental product decision

AstroForge Project is the durable unit of work; the filesystem folder is not.

This is essential for transforming AstroForge from a processing interface into a standalone image-processing system.

⸻

2. Product Decision

2.1 The AstroForge domain hierarchy

The canonical model shall be:

ASTROFORGE
│
├── Project
│   │
│   ├── Target
│   │
│   ├── Session
│   │   ├── Source Assets
│   │   ├── Capture Metadata
│   │   └── Classification
│   │
│   ├── Recipe
│   │
│   ├── Pipeline Runs
│   │   ├── Stage Runs
│   │   └── Checkpoints
│   │
│   ├── Artifacts
│   │   ├── Intermediate
│   │   ├── Preview
│   │   └── Final
│   │
│   ├── Image Versions
│   │
│   ├── AI Operations
│   │
│   └── Exports

This creates a clean separation between:

what the user owns
→ Project

what was captured
→ Session

what was imported
→ Source Asset

what AstroForge did
→ Pipeline Run / Stage Run

what was produced
→ Artifact

what the user sees as an image state
→ Image Version

how it was produced
→ Recipe + provenance

what left AstroForge
→ Export

⸻

3. Core Domain Objects

3.1 Project

A Project is the primary persistent AstroForge workspace.

Example:

M42 — Orion Nebula

A Project may contain:

* one target
* multiple capture sessions
* multiple nights
* multiple filters
* multiple processing attempts
* multiple recipes
* multiple image versions
* multiple exports

This is important because astrophotography processing frequently evolves over time.

For example:

M42 Project
Session 01 — 2026-01-12
  120 × 10s OSC
Session 02 — 2026-01-15
  180 × 10s OSC
Session 03 — 2026-01-20
  Ha / OIII
                    ↓
        Combined Processing
                    ↓
      M42 — Version 7

Project properties

Minimum:

project_id
name
description
target_id
created_at
updated_at
application_version
schema_version
status
active_session_id
active_image_version_id
project_root

Optional target information:

target_name
target_type
ra
dec
constellation
catalog_identifiers

⸻

4. Target

A Target represents the astronomical subject being processed.

Examples:

M42
NGC 7000
Jupiter
Moon
M31
IC 434

Target should be separated from Session because a user may photograph the same object over multiple nights.

Target model

target_id
name
object_type
ra
dec
catalog_ids
description

object_type:

deep_sky
planet
lunar
solar
unknown

The target-type detection described in the current specification therefore becomes project/session metadata, rather than merely a transient UI decision.

⸻

5. Session

5.1 Definition

A Session represents a coherent acquisition set.

For example:

M42
 └── Session: 2026-09-05 Seestar S50
      ├── Lights
      ├── Darks
      └── Flats

A Project can contain multiple Sessions.

This distinction is important:

Project = what I am creating.
Session = what I captured.

⸻

5.2 Session metadata

session_id
project_id
name
capture_start
capture_end
source_location
camera_profile
telescope_profile
site_profile
target_detection
processing_status
created_at
updated_at

Capture metadata can include:

camera
sensor
resolution
pixel_size
focal_length
gain
offset
temperature
binning
filter
exposure
date_obs
dithering

The values should be normalized where possible while preserving original FITS metadata.

⸻

6. Source Asset

A Source Asset is an immutable original input.

Examples:

light_001.fits
light_002.fits
dark_001.fits
flat_001.fits
capture_001.dng

The original file must never be modified by processing.

Key rule

Source Assets are immutable.

AstroForge may copy, reference, index, or hash them, but the processing engine must never overwrite them.

⸻

6.1 Source identity

Each imported file should receive:

asset_id
content_hash
original_filename
original_path
file_size
format
mime_type
created_at
imported_at
session_id

Plus astronomical metadata:

frame_type
exposure
filter
binning
width
height
bit_depth
bayer_pattern
camera
date_obs
ra
dec

⸻

7. Content Hashing

AstroForge should use content hashing to identify source and derived artifacts.

Example:

SHA-256

This enables:

* duplicate detection
* safe caching
* artifact reuse
* integrity verification
* crash recovery
* reproducibility

Example:

source hash
    ↓
8e4a...f921

If the same file is imported again, AstroForge can identify it as an existing asset rather than processing it twice.

⸻

8. Artifact

An Artifact is any persistent output generated by AstroForge.

Examples:

debayered.fit
master_dark.fit
registered_001.fit
stacked.fit
background_corrected.fit
stretched.tif
star_mask.fit
denoised.tif
super_resolved.tif

An artifact is not necessarily a final image.

⸻

8.1 Artifact categories

SOURCE
DERIVED
PREVIEW
MASK
METADATA
QUALITY_METRIC
CHECKPOINT
EXPORT

The artifact model should contain:

artifact_id
artifact_hash
artifact_type
format
path
size
created_at
producer_stage
pipeline_run_id
parent_artifact_ids
width
height
channels
bit_depth
color_space
linear_or_nonlinear

⸻

9. Image Version

This is one of the most important concepts introduced by CR-02.

An Image Version represents a meaningful visual state of the user’s image.

For example:

V0 — Imported
V1 — Calibrated
V2 — Registered
V3 — Stacked
V4 — Background Corrected
V5 — Stretched
V6 — AI Denoised
V7 — Star Enhanced
V8 — Super Resolution
V9 — Final

The user should be able to move between these versions without destroying the underlying artifacts.

Critical distinction

An Artifact is an implementation object.

An Image Version is a user-facing product object.

The UI should therefore say:

Version 8 — AI Enhanced

rather than exposing:

artifact_7d8f4b3.onnx_stage_output.bin

⸻

10. Non-Destructive Processing Model

AstroForge should use a copy-forward model, not destructive editing.

Original
   │
   ▼
Calibrated
   │
   ▼
Registered
   │
   ▼
Stacked
   │
   ▼
Stretched
   │
   ├──────────────► Version A
   │
   ▼
AI Denoised
   │
   ▼
AI Enhanced
   │
   ▼
Final

If the user dislikes AI enhancement:

AI Enhanced
     ↓
Discard version
     ↓
return to Stretched

The original data remains untouched.

⸻

11. Pipeline Run

A Pipeline Run represents one execution of a processing recipe.

Example:

Run #12
Recipe:
Deep Sky Auto v1
Input:
Session 2026-09-05
Result:
Version 9

A Pipeline Run records:

run_id
project_id
session_ids
recipe_id
started_at
completed_at
status
application_version
engine_version
hardware_profile
execution_mode
input_artifacts
output_artifacts

Possible states:

queued
running
paused
completed
failed
cancelled
recovering

⸻

12. Stage Run

Each DAG node becomes a persistent Stage Run.

For example:

Run #12
01 ingest              ✓
02 quality_filter      ✓
03 debayer             ✓
04 calibration         ✓
05 cosmetic            ✓
06 registration        ✓
07 stacking            ✓
08 background          ✓
09 color               ✓
11 stretching         ✓
13 denoise             ✓
14 star enhancement   ▶
15 super resolution    —

The current specification explicitly defines the pipeline as a DAG whose stages produce previews and quality metrics and can be independently rerun. CR-02 turns that conceptual requirement into persistent state.

⸻

13. Stage Checkpoints

Every expensive or meaningful stage should support checkpoint persistence.

Example:

registration
     ↓
checkpoint
     ↓
stacking

If AstroForge crashes during stacking:

Application restart
       ↓
Recover Project
       ↓
Detect incomplete Run
       ↓
Restore last checkpoint
       ↓
Resume stacking

The user should not have to restart from ingestion.

⸻

14. Recipe

A Recipe describes how AstroForge should process an image.

Example:

Deep Sky — Auto

or:

M42 — HOO — Low Noise

Recipe contains:

recipe_id
name
version
target_type
pipeline_graph
parameters
enabled_stages
disabled_stages
AI_model_selections
created_at
updated_at

Recipes must be independent of a particular execution.

Therefore:

Recipe
   ↓
Pipeline Run
   ↓
Artifacts

not:

Recipe = Pipeline Run

This distinction is essential for reproducibility.

⸻

15. AI Operation

AI processing requires additional provenance.

For every AI operation AstroForge should record:

operation_id
stage_run_id
model_id
model_version
model_hash
runtime
backend
precision
parameters
seed
deterministic
experimental
input_artifact
output_artifact

For example:

AI Operation
Model:
SwinIR Astro Denoise
Version:
1.2.0
Runtime:
ONNX Runtime
Backend:
CoreML
Precision:
INT8
Mode:
Deterministic
Seed:
N/A

For an experimental generative/perceptual model:

Mode:
Experimental / Perceptual
Seed:
482901
Model:
StableSR
Model Version:
...

This directly supports the specification’s requirement that deterministic processing be the default and stochastic/generative processing be explicitly labeled and seed-recorded.

⸻

16. Provenance Graph

AstroForge should maintain a provenance relationship:

Source
  │
  ▼
Artifact A
  │
  ▼
Artifact B
  │
  ▼
Artifact C
  │
  ├── AI Operation
  │
  ▼
Image Version
  │
  ▼
Export

This allows the user to ask:

“How was this image produced?”

and AstroForge can answer:

Final Image
 ├─ Super Resolution
 │   └─ SwinIR Astro SR 2×
 ├─ Star Enhancement
 │   └─ Star Segmentation v1
 ├─ Noise Reduction
 │   └─ SwinIR Astro Denoise
 ├─ Stretch
 │   └─ GHS
 ├─ Color Calibration
 ├─ Background Extraction
 └─ Kappa-Sigma Stack

This becomes the foundation for a future Processing History UI.

⸻

17. Persistence Architecture

The specification already establishes:

SQLite index + filesystem artifact store.

CR-02 formalizes the division of responsibility.

SQLite owns

Projects
Targets
Sessions
Source Assets
Artifacts
Image Versions
Recipes
Pipeline Runs
Stage Runs
AI Operations
Exports
Settings
Indexes
Relationships
Statuses
Metadata

Filesystem owns

FITS
TIFF
XISF
PNG
JPEG
DNG
preview images
masks
checkpoints
model files
logs

SQLite should not contain large image payloads.

⸻

18. Proposed Project Storage Layout

A project should have a self-contained application-managed directory:

AstroForge/
└── Projects/
    └── M42-Orion-Nebula/
        │
        ├── project.db
        │
        ├── project.json
        │
        ├── sources/
        │   ├── <hash>.fits
        │   └── <hash>.fits
        │
        ├── artifacts/
        │   ├── <hash>.fits
        │   ├── <hash>.tif
        │   └── <hash>.xisf
        │
        ├── previews/
        │
        ├── checkpoints/
        │
        ├── recipes/
        │
        ├── exports/
        │
        ├── logs/
        │
        └── cache/

However, the user should not need to understand this structure.

It is an implementation detail.

⸻

19. External Source Files

AstroForge should support two source strategies.

Managed

AstroForge copies source files into its project storage.

Advantages:

* portable project
* reliable recovery
* self-contained project
* easier backup

Referenced

AstroForge keeps the original files in their existing location.

Advantages:

* avoids duplicating large datasets
* useful for large telescope captures

The Project should record:

source_mode =
    managed
    referenced

If a referenced source disappears, AstroForge should report:

Source unavailable

rather than silently failing.

⸻

20. Portable Project Principle

A major standalone-product objective should be:

A project should be transferable between AstroForge installations.

For example:

AstroForge Project
       ↓
External SSD
       ↓
Another computer
       ↓
Import Project
       ↓
Continue processing

This is particularly important for astrophotographers who may process captures on a desktop while acquiring images elsewhere.

⸻

21. Project State Machine

Project state:

NEW
 │
 ▼
IMPORTED
 │
 ▼
ANALYZING
 │
 ▼
READY
 │
 ▼
PROCESSING
 │
 ├── PAUSED
 │
 ├── FAILED
 │
 └── RECOVERING
 │
 ▼
PROCESSED
 │
 ▼
ENHANCING
 │
 ▼
REVIEW
 │
 ▼
EXPORTED

Importantly, EXPORTED must not be terminal.

The user can return to:

REVIEW
   ↓
ENHANCE
   ↓
EXPORT

as many times as required.

⸻

22. UI Consequence

CR-02 directly changes the UI established in CR-01.

The Project workspace should expose:

Project Header

M42 — Orion Nebula
Deep Sky · OSC
3 Sessions · 482 Frames

Project Workspace

Overview | Import | Process | Enhance | Compare | Export

Processing status

✓ Imported
✓ Analyzed
✓ Calibrated
✓ Stacked
✓ Stretched
● AI Enhancement
○ Export

Current image

Large image canvas.

Version history

V0 Original
V1 Calibrated
V2 Stacked
V3 Stretched
V4 AI Enhanced
V5 Final

This gives the user a visual mental model of where the image is in its evolution.

⸻

23. Project Overview Screen

The Overview should answer four questions immediately:

1. What am I working on?

M42 — Orion Nebula

2. What did I capture?

3 sessions
482 lights
24 darks
12 flats
OSC

3. What has AstroForge done?

Stacked
Background corrected
Color calibrated
Stretched

4. What should I do next?

Recommended:
Enhance Image
AI analysis:
• Noise is moderate
• Stars are slightly bloated
• Nebulosity has strong detail potential

That is far more aligned with AstroForge’s purpose than presenting system telemetry as the primary interface.

⸻

24. Version Timeline UI

The image-version model should become a major UX feature.

Example:

ORIGINAL
   │
   ▼
CALIBRATED
   │
   ▼
STACKED
   │
   ▼
STRETCHED
   │
   ├─────────────┐
   ▼             ▼
AI DENOISED   ORIGINAL STRETCH
   │
   ▼
AI ENHANCED
   │
   ▼
FINAL

Users should be able to click any version and inspect it.

⸻

25. Compare Must Be Version-Aware

CR-02 establishes the data foundation for CR-07.

The comparison engine should eventually support:

Version 3 vs Version 7

or:

Before AI vs After AI

or:

Recipe A vs Recipe B

without reprocessing the source data.

⸻

26. Delete Semantics

Deletion needs strict rules.

Source

DELETE

requires explicit confirmation.

Artifact

Can be garbage-collected if no longer referenced.

Image Version

Can be hidden/deleted from the user timeline, but its artifacts should only be removed if no other object references them.

Export

Can be deleted without affecting processing history.

Original source

Should have the strongest protection.

⸻

27. Artifact Garbage Collection

Because astrophotography datasets can become very large, AstroForge needs an artifact lifecycle.

Example:

Artifact A
   ↑
Version 2

If Version 2 is deleted:

Artifact A
   ↓
unreferenced

AstroForge may eventually mark it:

eligible_for_cleanup

but should not immediately delete it.

The application should eventually provide:

Storage Management

with:

Project size: 84.2 GB
Sources:       51.3 GB
Artifacts:     29.4 GB
Previews:       1.2 GB
Cache:          2.3 GB
Potential cleanup:
17.8 GB

This becomes important for the 4–8 GB RAM / local-storage-oriented architecture.

⸻

28. Crash Recovery

This is a mandatory requirement.

If AstroForge terminates unexpectedly:

Application starts
       ↓
Load Projects
       ↓
Detect interrupted Pipeline Run
       ↓
Validate checkpoints
       ↓
Present:
"AstroForge recovered an interrupted process."
Resume
Restart Stage
Discard Run

The user should never have to understand what a corrupted DAG node is.

⸻

29. Transactional Artifact Creation

Artifact creation should be atomic.

Recommended pattern:

write temporary artifact
        ↓
validate
        ↓
calculate hash
        ↓
atomic rename
        ↓
commit SQLite record

Never:

create database record
        ↓
start writing image
        ↓
application crashes
        ↓
database points to nonexistent image

The persistence layer must prevent this class of corruption.

⸻

30. Schema Versioning

The database must contain:

schema_version

Example:

AstroForge Schema
v1

Future releases:

v1 → v2
v2 → v3

Migrations must be automatic.

The user should see:

Updating AstroForge Project…

not:

ALTER TABLE pipeline_runs...

⸻

31. Application Version vs Processing Version

These must be separately recorded.

Application

AstroForge 1.0.0

Processing Engine

Engine 1.3.0

Recipe

Deep Sky Auto 1.1

AI Model

SwinIR Astro Denoise 1.2

Model hash

sha256:...

This allows AstroForge to determine whether an old project can be reproduced exactly.

⸻

32. Reproducibility Contract

For every final image, AstroForge should be able to reconstruct:

Input Assets
      +
Recipe
      +
Parameters
      +
Application Version
      +
Engine Version
      +
Model Versions
      +
Model Hashes
      +
Hardware Backend
      +
Random Seeds
      +
Pipeline Graph
      =
Processing Provenance

This becomes one of AstroForge’s strongest differentiators.

⸻

33. What the User Should See

The complexity above should remain mostly invisible.

The user sees:

M42 — Final Image

and can expand:

How AstroForge created this

482 frames
      ↓
Quality filtering
      ↓
Calibration
      ↓
Registration
      ↓
Kappa-Sigma stacking
      ↓
Background correction
      ↓
Color calibration
      ↓
GHS stretch
      ↓
AI denoise
      ↓
Star enhancement
      ↓
2× super-resolution

Advanced users can expand each stage.

Beginners do not have to.

⸻

34. Data Model

A conceptual relational model:

PROJECT
  1 ─────── N SESSION
  │
  ├──────── 1 TARGET
  │
  ├──────── N RECIPE
  │
  ├──────── N PIPELINE_RUN
  │              │
  │              └── N STAGE_RUN
  │                       │
  │                       └── N AI_OPERATION
  │
  ├──────── N ARTIFACT
  │
  ├──────── N IMAGE_VERSION
  │
  └──────── N EXPORT
SESSION
  │
  └── N SOURCE_ASSET
PIPELINE_RUN
  │
  └── N STAGE_RUN
           │
           ├── input artifacts
           └── output artifacts
IMAGE_VERSION
  │
  └── primary artifact

⸻

35. Recommended SQLite Entities

Initial schema should include approximately:

projects
targets
sessions
source_assets
camera_profiles
telescope_profiles
recipes
recipe_versions
pipeline_runs
stage_runs
artifacts
artifact_relationships
image_versions
ai_operations
quality_metrics
exports
checkpoints
project_events
schema_migrations

project_events

This is worth introducing now.

It creates an append-oriented history:

PROJECT_CREATED
SESSION_IMPORTED
SOURCE_IMPORTED
ANALYSIS_COMPLETED
RECIPE_SELECTED
PIPELINE_STARTED
STAGE_COMPLETED
AI_OPERATION_APPLIED
VERSION_CREATED
EXPORT_CREATED

This gives AstroForge an auditable application history without requiring the UI to reconstruct events from image files.

⸻

36. Architecture Impact

CR-02 introduces a dedicated persistence/domain layer.

Recommended conceptual architecture:

                 ┌─────────────────────┐
                 │      UI / Svelte     │
                 └──────────┬──────────┘
                            │
                     Tauri Commands
                            │
                 ┌──────────▼──────────┐
                 │ Application Services│
                 └──────────┬──────────┘
                            │
             ┌──────────────┼──────────────┐
             ▼              ▼              ▼
        Project        Pipeline        Artifact
        Service        Service         Service
             │              │              │
             └──────────────┼──────────────┘
                            ▼
                   Domain / Persistence
                     ┌────────────┐
                     │   SQLite   │
                     └─────┬──────┘
                           +
                   ┌────────────┐
                   │ Filesystem │
                   └────────────┘

The DAG runner should operate against the domain model, not directly against UI state.

⸻

37. Implementation Direction in AstroForge

The current repository already has a Rust workspace structure under crates/, alongside the product specification and project documentation.

CR-02 should therefore introduce or consolidate domain responsibilities along these lines:

crates/
├── astroforge-core/
│   ├── project
│   ├── target
│   ├── session
│   ├── asset
│   ├── artifact
│   ├── image_version
│   └── provenance
│
├── astroforge-persistence/
│   ├── sqlite
│   ├── migrations
│   └── repositories
│
├── astroforge-pipeline/
│   ├── run
│   ├── stage
│   ├── checkpoint
│   └── recovery
│
└── astroforge-ai/
    └── provenance

The exact crate names should follow the existing repository conventions rather than creating duplicate architectural concepts.

Frontend

The frontend should consume project state through application commands rather than maintaining its own authoritative project database.

Conceptually:

UI state
   ↓
Tauri command
   ↓
Application service
   ↓
Domain
   ↓
SQLite / Artifact Store

⸻

38. UI State vs Persistent State

A critical rule:

UI state

Temporary:

selected panel
zoom
histogram visibility
comparison mode
sidebar width
current tool

Persistent project state

Durable:

image versions
pipeline status
recipe
parameters
artifacts
AI operations
exports
session metadata
processing history

The UI must never be the authoritative source for the latter.

⸻

39. Acceptance Criteria

CR-02 is complete when:

Project

* [ ]	User can create a Project.
* [ ]	Project persists after application restart.
* [ ]	Project has a stable unique ID.
* [ ]	Project can contain multiple Sessions.
* [ ]	Project can be renamed without affecting artifacts.

Session

* [ ]	User can import a Session.
* [ ]	Session metadata persists.
* [ ]	Multiple capture sessions can belong to one Project.
* [ ]	Session classification persists.

Source Assets

* [ ]	Original files are immutable.
* [ ]	Each source has a stable identity/hash.
* [ ]	Duplicate imports are detected.
* [ ]	Missing referenced files are detected explicitly.

Artifacts

* [ ]	Every processing output has a persistent identity.
* [ ]	Artifact lineage is retained.
* [ ]	Artifacts are never silently overwritten.
* [ ]	Artifact integrity can be validated.

Pipeline

* [ ]	Pipeline Run is persistent.
* [ ]	Stage Runs are persistent.
* [ ]	Pipeline state survives application restart.
* [ ]	Checkpoints can be restored.
* [ ]	Failed stages can be rerun without necessarily rerunning earlier stages.

Image Versions

* [ ]	Meaningful image states are represented as versions.
* [ ]	Users can return to previous versions.
* [ ]	Versions are non-destructive.
* [ ]	Comparison can reference any retained version.

AI

* [ ]	Model identity is recorded.
* [ ]	Model version is recorded.
* [ ]	Model hash is recorded.
* [ ]	Backend/runtime is recorded.
* [ ]	Deterministic status is recorded.
* [ ]	Seed is recorded when applicable.

Reproducibility

* [ ]	A final image has complete processing provenance.
* [ ]	Recipe version is recorded.
* [ ]	Parameters are recorded.
* [ ]	Application/engine versions are recorded.

Standalone operation

* [ ]	A project works without network access.
* [ ]	No external database is required.
* [ ]	No Python installation is required for normal operation.
* [ ]	No manual model-path configuration is required.
* [ ]	Project state is restored after application restart.

⸻

40. CR-02 Non-Goals

CR-02 should not attempt to implement:

* full image-processing algorithms
* AI models themselves
* advanced histogram UI
* complete recipe marketplace
* cloud synchronization
* telescope control
* mission monitoring
* observatory management
* astrophotography acquisition control

Those belong to later CRs.

CR-02 establishes the persistent substrate on which those capabilities can safely operate.

⸻

41. Migration Strategy

Do not attempt a large rewrite of AstroForge in one step.

Recommended sequence:

CR-02.1 — Domain types

Introduce:

Project
Target
Session
SourceAsset
Artifact
ImageVersion
Recipe
PipelineRun
StageRun
AIOperation
Export

CR-02.2 — SQLite persistence

Implement repositories and migrations.

CR-02.3 — Artifact Store

Implement:

hash
write
read
validate
delete

CR-02.4 — Project lifecycle

Implement:

Create
Open
Close
Rename
Archive
Delete
Recover

CR-02.5 — Pipeline persistence

Connect DAG execution to:

PipelineRun
StageRun
Checkpoint
Artifact

CR-02.6 — UI integration

Replace transient UI state with persistent Project state.

⸻

42. CR-02 Definition of Done

The CR should be considered architecturally complete when AstroForge can perform this sequence:

Launch AstroForge
       ↓
Create Project
       ↓
"M42 — Orion Nebula"
       ↓
Import folder
       ↓
Create Session
       ↓
Register Source Assets
       ↓
Analyze
       ↓
Create Recipe
       ↓
Start Pipeline Run
       ↓
Execute DAG
       ↓
Persist every Stage Run
       ↓
Create Image Versions
       ↓
Apply AI enhancement
       ↓
Record AI provenance
       ↓
Export Final Image
       ↓
Close AstroForge
       ↓
Restart AstroForge
       ↓
Open M42
       ↓
All history remains
       ↓
Continue / compare / re-edit / export

That sequence is the real CR-02 success criterion.

⸻

43. Strategic Consequence for AstroForge

CR-01 defined how AstroForge should feel.

CR-02 defines what AstroForge actually owns.

This is a major architectural transition:

BEFORE
Files → UI → Processing → Output
AFTER
                 ┌───────────────┐
                 │    Project    │
                 └───────┬───────┘
                         │
       ┌─────────────────┼─────────────────┐
       ▼                 ▼                 ▼
   Sessions           Recipes          History
       │                 │                 │
       ▼                 ▼                 ▼
   Sources          Pipeline Runs      Versions
                         │                 │
                         ▼                 ▼
                      Stages           Comparison
                         │
                         ▼
                      Artifacts
                         │
                         ▼
                     AI / Engine
                         │
                         ▼
                       Export

This is what makes AstroForge capable of becoming a standalone, resumable, reproducible astrophotography intelligence and enhancement platform, rather than simply a graphical front-end around a sequence of image-processing functions.

Next CR

The natural next step is CR-03 — AstroForge Application Shell & Studio Workspace.

CR-03 should take the Project/Session model established here and turn it into the actual executable application’s information architecture: Home → Projects → Project Overview → Import → Process → Enhance → Compare → Export, including the responsive layout, persistent project context, image canvas hierarchy, navigation, status model, and transition from the current repository UI into the CR-01/CR-02 product architecture.