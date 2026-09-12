Absolutely. CR-08 is the point where AstroForge moves from “a processing system that can produce good results” to “a system that can preserve, reproduce, learn from, and reuse successful processing workflows.”

CR-08 — Recipes, Reproducibility & Processing Provenance

Status: Proposed
Target: AstroForge v1.0
Priority: Critical
Depends on: CR-02, CR-05, CR-06, CR-07
Enables: CR-13, CR-14, CR-15, CR-16, CR-17, CR-20, CR-23, CR-25

⸻

1. Intent

CR-08 establishes Recipes and Reproducibility as a first-class capability of AstroForge.

AstroForge should not only answer:

“How do I process this image?”

It should also be able to answer:

“What exactly did I do to produce this image, why did AstroForge make those decisions, and can I reproduce or adapt the result later?”

This is essential because astrophotography is inherently iterative. Users will discover processing approaches that work particularly well for:

* a telescope;
* a camera;
* a target class;
* a filter;
* a particular acquisition environment;
* a particular integration length;
* a particular style of image.

The successful workflow should become reusable knowledge rather than disappearing into an individual project.

CR-08 therefore introduces four related concepts:

Recipe → Pipeline → Execution → Result

Where:

* Recipe = reusable processing intent.
* Pipeline = dataset-specific execution plan.
* Execution = what actually happened.
* Result = the resulting Image Version and its provenance.

⸻

2. Product Decision

2.1 Recipe is not a recorded list of UI clicks

A Recipe must represent processing intent, not an automation recording.

For example:

“Produce a natural-looking galaxy image with controlled star profiles, low background noise and preserved faint structure.”

is a valid recipe intent.

It should not merely mean:

“Set slider A to 37, slider B to 0.62, run stage X, then stage Y.”

This distinction allows AstroForge to adapt the recipe to different datasets.

⸻

2.2 Recipe versus Pipeline

This distinction becomes a fundamental AstroForge architectural invariant.

Concept	Meaning
Recipe	What the user wants
Pipeline	How AstroForge intends to achieve it for this dataset
Stage	Individual processing operation
Execution	What actually happened
Image Version	Resulting user-visible state
Provenance	Evidence of how that state was produced

Therefore:

Recipe + Session Understanding + Resource Context → Pipeline

And:

Pipeline + Execution → Image Versions

⸻

2.3 Recipes must be adaptive

A recipe should be capable of expressing:

* preferred operations;
* constraints;
* quality objectives;
* optional operations;
* AI preferences;
* resource preferences;
* target-type applicability.

It should not unnecessarily hard-code assumptions that may change between datasets.

For example:

“Use this denoise model only if estimated noise exceeds threshold X.”

is preferable to:

“Always run denoise model X.”

This preserves the intelligence established in CR-04, CR-05 and CR-06.

⸻

3. Recipe Types

AstroForge should support four principal recipe classes.

3.1 System Recipes

Bundled by AstroForge.

Examples:

* Deep Sky — Balanced
* Deep Sky — Natural
* Deep Sky — Maximum Detail
* Deep Sky — Low Noise
* Galaxy — Natural
* Nebula — Detail
* Star Cluster — Star Preservation
* Planetary — Lucky Imaging
* Lunar — High Detail

System recipes are versioned and read-only.

⸻

3.2 User Recipes

Created and maintained by the user.

Example:

“My Vespera Galaxy Workflow”

The user can modify, duplicate, export and version them.

⸻

3.3 Project Recipes

Recipes associated specifically with a project.

Useful where a user develops a workflow for a particular target.

Example:

M31 processing recipe for this project.

⸻

3.4 Imported Recipes

Recipes obtained from:

* another AstroForge installation;
* another user;
* a future recipe ecosystem;
* a project archive.

Imported recipes must be validated before becoming executable.

⸻

4. Recipe Structure

Conceptually, an AstroForge Recipe should contain:

Recipe
├── Identity
│   ├── ID
│   ├── Name
│   ├── Description
│   ├── Author
│   ├── Version
│   └── Schema Version
│
├── Applicability
│   ├── Target Types
│   ├── Acquisition Types
│   ├── Camera Characteristics
│   ├── Filter Characteristics
│   └── Dataset Constraints
│
├── Processing Intent
│   ├── Quality Objective
│   ├── Noise Preference
│   ├── Detail Preference
│   ├── Star Preference
│   ├── Color Preference
│   └── Naturalness Preference
│
├── Pipeline Intent
│   ├── Required Stages
│   ├── Optional Stages
│   ├── Stage Constraints
│   └── Ordering Constraints
│
├── AI Policy
│   ├── Allowed AI Classes
│   ├── Preferred Models
│   ├── AI Operations
│   ├── Reconstruction Policy
│   └── Resource Policy
│
├── Quality Targets
│   ├── Noise
│   ├── Sharpness
│   ├── Star Integrity
│   ├── Clipping
│   └── Background Quality
│
└── Resource Policy
    ├── Memory Preference
    ├── Performance Preference
    └── Degradation Policy

The actual persisted schema should be versioned independently from the application.

⸻

5. Recipe Versioning

Recipe evolution must not invalidate historical results.

For example:

Galaxy Natural
    v1.0
    v1.1
    v1.2

A historical Pipeline Run must retain the exact recipe version used.

If the user modifies the recipe, AstroForge creates a new version rather than silently changing history.

Required properties

Every execution records:

* Recipe ID
* Recipe version
* Recipe schema version
* Recipe content hash

Therefore AstroForge can establish:

“This image was generated using Recipe X, version 1.3, with recipe hash Y.”

⸻

6. Reproducibility Model

AstroForge should distinguish exact reproducibility from material reproducibility.

Exact reproducibility

Possible when:

* same source assets;
* same application/engine version;
* same recipe;
* same models;
* same parameters;
* same backend;
* same precision;
* same random seed;
* same execution configuration.

⸻

Material reproducibility

The output may differ slightly because of:

* different hardware;
* different floating-point implementation;
* different GPU backend;
* updated libraries;
* nondeterministic acceleration;
* model changes.

AstroForge should therefore never falsely claim:

“This will produce the exact same pixels.”

Instead it should record the conditions required for reproducibility and explain deviations.

⸻

7. Provenance Model

Provenance is not merely a log file.

It is a structured relationship between:

Source Assets
      ↓
Session
      ↓
Recipe
      ↓
Pipeline
      ↓
Stage Executions
      ↓
AI Operations
      ↓
Image Versions
      ↓
Comparison / Decision
      ↓
Export

This creates a traceable transformation graph.

⸻

8. Processing Provenance

Every meaningful Image Version should be able to answer:

What?

What operation produced this image?

From what?

Which Image Version or source assets were used?

With what?

Which parameters, models and recipe were used?

Why?

What processing decision led to the operation?

When?

When was it executed?

Where?

Which execution backend/hardware was used?

With what result?

What quality measurements resulted?

⸻

9. AI Provenance

CR-08 extends the AI provenance requirements introduced in CR-06.

Every AI operation records at minimum:

AI Operation ID
Input Image Version
Output Image Version
Operation Type
Model ID
Model Version
Model Hash
Parameters
Mask ID
Inference Backend
Precision
Tile Configuration
Deterministic Class
Random Seed
Application Version
Engine Version
Timestamp
Execution Metrics
Quality Validation

This is especially important for AI super-resolution and perceptual enhancement.

A future user should be able to determine:

“Was this detail actually present in the source, or was it reconstructed by the AI model?”

⸻

10. Recipe Editor

The Recipe Editor should support progressive complexity.

Beginner

Recipe Name
Target Type
Processing Style
○ Natural
○ Balanced
○ Detail
○ Low Noise
AI Enhancement
○ Off
○ Conservative
○ Recommended
○ Advanced

⸻

Guided

Expose:

* processing objectives;
* stage inclusion;
* AI preferences;
* quality targets;
* optional operations.

⸻

Expert

Expose:

* stage constraints;
* parameter ranges;
* execution conditions;
* model selection;
* model versions;
* masks;
* thresholds;
* ordering constraints;
* resource policies;
* reproducibility controls.

⸻

11. Recipe Application Flow

The user should experience:

Select Recipe
      ↓
AstroForge evaluates Session
      ↓
Check Recipe Applicability
      ↓
Adapt Recipe
      ↓
Show Changes
      ↓
Generate Pipeline
      ↓
Preview
      ↓
Run

Critically, AstroForge should explain adaptation.

Example:

Recipe adapted

Your recipe normally applies deconvolution before enhancement.
AstroForge detected elevated noise in this dataset and recommends denoising first.

Reason: High noise may amplify deconvolution artifacts.

[Accept Adaptation] [Keep Recipe Order]

This preserves user authority while allowing intelligence.

⸻

12. Recipe Applicability

Before execution, AstroForge evaluates:

* target type;
* image characteristics;
* acquisition mode;
* available calibration;
* filter configuration;
* camera characteristics;
* image dimensions;
* dataset quality;
* available resources;
* installed AI models.

The result should be:

Compatible

Recipe can be used directly.

Adaptable

Recipe can be used with documented changes.

Partially compatible

Some stages must be skipped or substituted.

Incompatible

Recipe cannot safely be applied.

⸻

13. Recipe Diff

When applying a recipe to a new dataset, AstroForge should optionally show:

Recipe adaptation

Recipe Intent	Dataset Reality	AstroForge Decision
Calibration	No dark frames	Skip dark calibration
Denoise	High noise	Enable denoise
Deconvolution	Low star quality	Reduce strength
AI SR	12 MP output	Available
Star enhancement	Dense star field	Conservative

This becomes an important trust mechanism.

⸻

14. Recipe Save from Successful Processing

CR-08 should make successful processing reusable.

After a strong result:

Save as Recipe

AstroForge asks:

Save this workflow as a reusable recipe?

The user can select:

* processing stages;
* parameter choices;
* AI operations;
* enhancement order;
* masks;
* quality objectives;
* applicability.

The recipe should be generated from intent + execution knowledge, not merely a raw serialized pipeline.

⸻

15. UI Specification

Recipe Library

The global application navigation gains:

Recipes

The Recipe Library contains:

* System;
* My Recipes;
* Project Recipes;
* Imported;
* Recently Used.

Each recipe card displays:

* name;
* target type;
* style;
* version;
* AI usage;
* applicability;
* last used;
* provenance status.

⸻

Recipe Detail

┌──────────────────────────────────────────────┐
│ Galaxy — Natural                    v1.3     │
│                                              │
│ Natural galaxy processing with preserved     │
│ star profiles and controlled noise.          │
│                                              │
│ Target       Galaxy                          │
│ AI           Conservative                    │
│ Style        Natural                         │
│                                              │
│ Processing Intent                            │
│ ● Preserve faint structures                  │
│ ● Control noise                              │
│ ● Preserve stars                             │
│                                              │
│ [Apply Recipe] [Duplicate] [Edit]            │
└──────────────────────────────────────────────┘

⸻

16. Processing Workspace Integration

CR-05 should gain:

Save as Recipe

from the processing workspace.

After a successful run:

Processing Complete
Your image has been processed successfully.
[Review Result]
[Compare]
[Save as Recipe]
[Export]

The recipe creation flow should automatically inherit relevant processing information.

⸻

17. Provenance Viewer

Every Image Version should expose:

View Provenance

with a human-readable summary:

Final Image
Created from:
  Stacked Image v2.1
Recipe:
  Galaxy Natural v1.3
Processing:
  Background extraction
  Color calibration
  Stretch
  Noise reduction
  Star enhancement
AI:
  Super Resolution — Model X v2.1
  Deterministic class: Perceptual
Quality:
  Noise       ↓ 31%
  Sharpness   ↑ 14%
  Clipping    unchanged
Execution:
  CPU + integrated GPU

Expert mode can expand this into technical provenance.

⸻

18. Provenance Graph

Advanced users should be able to visualize:

Raw Frames
   │
   ├── Calibration
   │
   ├── Registration
   │
   └── Quality Filtering
          │
       Stacked v1
          │
       Stretch
          │
       Image v2
        /     \
       /       \
  AI Denoise   Natural
     │          │
   v3           v4
     \          /
      Comparison
          │
       Preferred
          │
        Final

This directly supports the branching model established by CR-07.

⸻

19. Recipe Import/Export

AstroForge should define a portable recipe format.

Conceptually:

.afrecipe

Properties:

* human-readable;
* schema-versioned;
* hashable;
* portable;
* path-independent;
* platform-independent;
* validation-friendly.

A recipe must never contain executable arbitrary code.

It describes processing intent and parameters only.

⸻

20. Recipe Security

Imported recipes must be treated as untrusted data.

AstroForge must validate:

* schema;
* supported stages;
* parameter ranges;
* model references;
* resource requirements;
* dependencies;
* version compatibility.

A recipe cannot:

* execute arbitrary shell commands;
* access arbitrary filesystem paths;
* install software;
* execute Python;
* invoke external processes outside approved AstroForge capabilities.

This is particularly important because recipes may eventually be shared through an ecosystem.

⸻

21. Data Model

CR-08 introduces or formalizes:

recipe
recipe_version
recipe_stage
recipe_parameter
recipe_constraint
recipe_ai_policy
recipe_quality_target
recipe_resource_policy
pipeline_plan
pipeline_execution
provenance_record
provenance_edge
reproducibility_record
execution_environment
recipe_application
recipe_adaptation

Core relationships

Recipe
  1 ─── N RecipeVersion
RecipeVersion
  1 ─── N RecipeStage
RecipeVersion
  1 ─── N RecipeApplication
RecipeApplication
  1 ─── 1 PipelinePlan
PipelinePlan
  1 ─── N StageExecution
StageExecution
  1 ─── N ProvenanceRecord
ProvenanceRecord
  ─── ImageVersion

⸻

22. Semantic API

Conceptual application commands:

create_recipe
get_recipe
list_recipes
create_recipe_version
update_recipe
duplicate_recipe
delete_recipe
validate_recipe
check_recipe_applicability
adapt_recipe
preview_recipe
apply_recipe
save_pipeline_as_recipe
save_processing_as_recipe
import_recipe
export_recipe
get_recipe_provenance
get_image_provenance
get_reproducibility_report
get_execution_environment
compare_recipe_versions

The exact naming should follow the repository’s existing command conventions.

⸻

23. Events

Introduce events such as:

RecipeCreated
RecipeUpdated
RecipeVersionCreated
RecipeValidated
RecipeApplicabilityEvaluated
RecipeAdaptationProposed
RecipeAdaptationAccepted
RecipeApplied
RecipeImportStarted
RecipeImportCompleted
RecipeImportFailed
RecipeExported
PipelineSavedAsRecipe
ProvenanceCreated
ReproducibilityRecordCreated

⸻

24. Architecture Impact

CR-08 establishes the following architecture:

                ┌──────────────┐
                │    Recipe    │
                └──────┬───────┘
                       │
                       ▼
             Session Understanding
                       │
                       ▼
                Recipe Adaptation
                       │
                       ▼
                Pipeline Generator
                       │
                       ▼
                  DAG Runner
                       │
          ┌────────────┼────────────┐
          ▼            ▼            ▼
       Stage 1      Stage 2       AI Ops
          │            │            │
          └────────────┼────────────┘
                       ▼
                  Image Version
                       │
                       ▼
                   Provenance
                       │
                       ▼
                    Export

CR-08 therefore becomes the bridge between processing execution and reusable processing knowledge.

⸻

25. Implementation Map

Conceptual implementation areas:

crates/
├── astroforge-core/
│   ├── recipe/
│   │   ├── recipe.rs
│   │   ├── version.rs
│   │   ├── validation.rs
│   │   ├── applicability.rs
│   │   ├── adaptation.rs
│   │   └── serialization.rs
│   │
│   └── provenance/
│       ├── record.rs
│       ├── graph.rs
│       └── reproducibility.rs
│
├── astroforge-persistence/
│   ├── recipe/
│   ├── provenance/
│   └── migrations/
│
├── astroforge-pipeline/
│   ├── recipe_adapter.rs
│   └── recipe_generator.rs
│
├── astroforge-ai/
│   └── provenance/
│
└── astroforge-app/
    ├── recipe_commands.rs
    ├── recipe_events.rs
    ├── recipe_workspace.rs
    └── provenance_workspace.rs

These are conceptual boundaries; existing repository conventions should take precedence over creating unnecessary new crates.

⸻

26. Performance Considerations

Recipes themselves are lightweight.

The expensive concern is reproducibility metadata and provenance.

Therefore:

* store metadata in SQLite;
* store large artifacts on filesystem;
* hash rather than duplicate source data;
* reference existing Image Versions;
* do not embed images inside recipes;
* do not duplicate AI models inside projects;
* maintain model identity through registry references and hashes.

⸻

27. Standalone Readiness

CR-08 reinforces the standalone requirement substantially.

A portable AstroForge project should retain enough information to explain:

“How was this image produced?”

without requiring:

* Internet access;
* external processing software;
* Python;
* external FITS tools;
* external AI tools.

The project should therefore contain or reference:

Project
├── Recipe references
├── Recipe versions
├── Pipeline executions
├── Image versions
├── AI provenance
├── Processing parameters
├── Model identity
├── Source hashes
└── Reproducibility information

CR-11’s offline-first requirements ensure these remain available offline.

⸻

28. Acceptance Criteria

CR-08 is complete when:

Recipe Management

* [ ]	User can create a Recipe.
* [ ]	User can edit and version a Recipe.
* [ ]	User can duplicate a Recipe.
* [ ]	User can delete/archive a user Recipe.
* [ ]	System Recipes are protected from modification.
* [ ]	Recipes have schema versions and content hashes.

Recipe Application

* [ ]	User can apply a Recipe to a Session.
* [ ]	AstroForge evaluates applicability.
* [ ]	AstroForge can adapt a Recipe.
* [ ]	Adaptations are explicitly shown.
* [ ]	User can accept or reject adaptations.
* [ ]	Applied Recipe generates a dataset-specific Pipeline.

Reproducibility

* [ ]	Every Pipeline Run records Recipe identity/version.
* [ ]	Source asset hashes are retained.
* [ ]	Application and engine versions are retained.
* [ ]	AI model versions/hashes are retained.
* [ ]	Backend/precision are retained.
* [ ]	Seeds are retained where applicable.
* [ ]	Reproducibility conditions are recorded.

Provenance

* [ ]	Every Image Version has provenance.
* [ ]	Provenance links source → processing → AI → result.
* [ ]	Provenance survives application restart.
* [ ]	Provenance survives project migration.
* [ ]	AI operations are identifiable.
* [ ]	Users can inspect provenance without entering Expert mode.

Import/Export

* [ ]	Recipes can be exported.
* [ ]	Recipes can be imported.
* [ ]	Imported recipes are validated.
* [ ]	Invalid recipes cannot execute.
* [ ]	Recipes contain no arbitrary executable code.

⸻

29. Test Strategy

Unit Tests

Test:

* recipe validation;
* schema migration;
* versioning;
* hashing;
* applicability;
* adaptation;
* parameter constraints;
* provenance relationships;
* reproducibility records.

Integration Tests

Test:

Create Recipe
    ↓
Apply Recipe
    ↓
Generate Pipeline
    ↓
Execute
    ↓
Create Image Version
    ↓
Persist Provenance
    ↓
Restart Application
    ↓
Read Provenance

Reproducibility Tests

Run identical:

Source + Recipe + Model + Configuration

and verify materially equivalent output.

Test differences between:

* CPU;
* GPU;
* different precision;
* different model versions;
* deterministic/non-deterministic execution.

Security Tests

Attempt to import recipes containing:

* invalid stages;
* invalid parameters;
* filesystem references;
* executable payloads;
* unsupported model references.

All must fail safely.

⸻

30. Architectural Decision Records

ADR-08.1 — Recipe Represents Intent

Recipes represent processing intent rather than UI actions or implementation details.

ADR-08.2 — Recipe and Pipeline Are Distinct

A Recipe is reusable; a Pipeline is dataset-specific.

ADR-08.3 — Recipe Adaptation Must Be Explicit

AstroForge may intelligently adapt a Recipe, but must disclose meaningful changes.

ADR-08.4 — Historical Execution Is Immutable

Changing a Recipe must never rewrite historical processing.

ADR-08.5 — Provenance Is First-Class Data

Processing history is structured application data, not merely a log.

ADR-08.6 — AI Identity Is Part of Provenance

AI-generated results must retain model identity and execution characteristics.

ADR-08.7 — Reproducibility Is Qualified

AstroForge records whether an operation is exactly reproducible, materially reproducible, or inherently variable.

ADR-08.8 — Recipes Cannot Execute Arbitrary Code

Recipes are declarative processing specifications.

ADR-08.9 — Provenance Is Human-Readable

Technical provenance must have both user and Expert representations.

⸻

31. Definition of Done

CR-08 is complete when a user can:

Process an astrophotography dataset → obtain a successful Image Version → inspect exactly how it was produced → save the workflow as a Recipe → apply that Recipe to another dataset → allow AstroForge to adapt it intelligently → review those adaptations → execute it → compare the result → retain complete provenance → export the Recipe and resulting image independently.

The application must preserve the entire history even after restart, migration or offline operation.

⸻

32. Strategic Outcome

CR-08 changes AstroForge from a processing application into a processing knowledge system.

The progression now becomes:

CR-01 — Define the experience
CR-02 — Define what AstroForge remembers
CR-03 — Define where the user works
CR-04 — Understand the astrophotography dataset
CR-05 — Intelligently process the dataset
CR-06 — Intelligently revamp the image
CR-07 — Compare, measure and decide
CR-08 — Capture, reproduce and reuse what worked

That creates an important higher-level loop:

UNDERSTAND → RECOMMEND → PROCESS → ENHANCE → COMPARE → DECIDE → SAVE KNOWLEDGE → REUSE

This is particularly important for AstroForge’s long-term differentiation: the application should progressively become better at translating an astrophotographer’s intent into a high-quality result without turning the user into a software engineer.

Next logical CR: CR-09 — Resource, Hardware & Execution Intelligence, which makes the same Recipe/Pipeline capable of intelligently adapting to the actual machine while preserving the user’s processing intent.