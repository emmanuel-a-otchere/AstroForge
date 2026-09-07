CR-05 — Intelligent Processing Workspace & Adaptive Pipeline Execution

Status: Proposed
Target: AstroForge v1.0
Priority: Critical
Depends on: CR-01, CR-02, CR-03, CR-04
Enables: CR-06, CR-07, CR-08, CR-09, CR-13, CR-14, CR-15, CR-21, CR-23

⸻

1. Intent

CR-05 transforms the processing experience from a collection of image-processing functions into an intelligent, adaptive, visual processing workflow.

The core product decision is:

The user should work with an image and its transformation, not with a technical DAG.

The underlying DAG remains fundamental to AstroForge's execution architecture, but it should be largely invisible to beginners.

The experience should therefore be:

Understand → Recommend → Preview → Process → Observe → Adjust → Continue → Review

rather than:

Configure node 1 → configure node 2 → configure node 3 → execute.

This distinction is critical to making AstroForge a standalone product.

⸻

2. Product Decision

2.1 The Pipeline is the product's processing spine

The pipeline remains a DAG internally.

But the user sees a human-readable processing journey.

For example:

```
M31 — Deep Sky OSC
✓ Calibrate
✓ Debayer
✓ Register
▶ Stack
○ Background
○ Color
○ Stretch
○ Denoise
○ Enhance
○ Export
```

Instead of:

```
ingest
quality_filter
debayer
calibration
registration
stacking
background_extraction
...
```

The latter belongs in Diagnostics / Expert mode.

⸻

3. Adaptive Pipeline Principle

The pipeline must be generated from the understanding established by CR-04.

It should not be a universal fixed workflow.

Conceptually:

```
                SESSION
                   │
                   ▼
          SESSION UNDERSTANDING
                   │
         ┌─────────┴─────────┐
         ▼                   ▼
    Target Type          Acquisition
         │                   │
         └─────────┬─────────┘
                   ▼
             PIPELINE PLAN
                   │
          ┌────────┴────────┐
          ▼                 ▼
      Required           Optional
       stages             stages
          │                 │
          └────────┬────────┘
                   ▼
              USER REVIEW
                   │
                   ▼
             EXECUTION DAG
```

Therefore a planetary dataset should not expose deep-sky processing stages simply because they exist in the engine.

⸻

4. Three Processing Modes

CR-01 established Auto / Guided / Expert.

CR-05 turns these into concrete processing behavior.

### Auto

The application makes almost all decisions.

User experience:

```
Process my data
```

AstroForge selects:

- stages;
- parameters;
- resource strategy;
- AI options;
- quality thresholds;
- checkpoints.

The user can still inspect the decisions.

### Guided

AstroForge makes recommendations but pauses at meaningful decisions.

Example:

```
Background extraction recommended

Estimated gradient detected across the image.

[Preview]
[Adjust]
[Skip]
```

### Expert

The complete pipeline becomes inspectable.

The user can modify:

- stage parameters;
- execution order where safe;
- optional stages;
- rejection thresholds;
- algorithms;
- AI models;
- tile size;
- processing precision;
- resource backend.

Expert mode must still preserve the underlying validity constraints of the DAG.

⸻

5. Processing Workspace

The workspace should follow the architecture established in CR-03.

```
┌──────────────────────────────────────────────────────────────┐
│ Project / Target              Processing: Deep-Sky OSC       │
├──────────────┬───────────────────────────────────┬───────────┤
│              │                                   │           │
│ VERSIONS     │                                   │ INTELLI-  │
│              │                                   │ GENCE     │
│ Original     │                                   │           │
│ Calibrated   │          IMAGE CANVAS             │ Analysis  │
│ Registered   │                                   │           │
│ Stacked  ◀   │                                   │ Recommend │
│ Stretched    │                                   │           │
│ Enhanced     │                                   │ Controls  │
│              │                                   │           │
├──────────────┴───────────────────────────────────┴───────────┤
│ Pipeline: ✓ Calibrate → ✓ Register → ▶ Stack → ○ Stretch    │
├──────────────────────────────────────────────────────────────┤
│ Histogram │ Zoom │ Preview │ Processing status               │
└──────────────────────────────────────────────────────────────┘
```

The image remains the dominant visual object.

⸻

6. Processing Workspace Zones

### Zone A: Version Context

Left side.

Shows meaningful image states:

```
Original
   ↓
Calibrated
   ↓
Registered
   ↓
Stacked
   ↓
Color Calibrated
   ↓
Stretched
   ↓
AI Denoised
   ↓
Enhanced
```

These are Image Versions, not raw implementation artifacts.

This distinction from CR-02 must remain intact.

### Zone B: Image Canvas

The center remains dominant.

Required capabilities:

- fit to screen;
- zoom;
- pan;
- 1:1;
- pixel inspection;
- histogram;
- clipping indicators;
- before/after;
- split comparison;
- blink;
- mask visualization;
- crop;
- rotate;
- image statistics.

The canvas should update through non-destructive previews before expensive operations are committed.

### Zone C: Intelligence & Controls

Right-hand contextual panel.

It should change according to the active stage.

Example:

```
Stacking

STACKING
462 Light frames
Quality analysis
★★★★★ Excellent
Recommended:
Weighted average
Expected improvement:
Signal ↑
Noise ↓
[ Preview ]
Advanced
▸ Rejection
▸ Weighting
▸ Normalization
```

The user sees the reason for the recommendation, not merely a parameter value.

⸻

7. Stage Model

Every user-visible processing stage should have a consistent conceptual structure:

```
Stage
 ├── Purpose
 ├── Input
 ├── Recommendation
 ├── Confidence
 ├── Preview
 ├── Controls
 ├── Expected result
 ├── Resource estimate
 └── Apply
```

For example:

```
Noise Reduction

AstroForge detected residual chromatic noise in the background.

Recommended strength: Moderate
Confidence: High

[Preview] [Adjust] [Apply]
```

⸻

8. Pipeline Stage States

Every stage should have a visible state.

- ○ Not started
- ◌ Ready
- ▶ Running
- ✓ Completed
- ⏸ Paused
- ⚠ Needs attention
- ✕ Failed
- ↻ Recovering

The underlying engine may have richer states, but the UI should translate them into meaningful concepts.

⸻

9. Pipeline Execution

Execution should support:

### Start

Start the complete recommended workflow.

### Pause

Pause at the next safe checkpoint.

### Resume

Continue from the latest valid checkpoint.

### Cancel

Stop execution while preserving completed work.

### Retry

Retry failed stages.

### Skip

Skip optional stages where safe.

### Re-run

Re-execute a stage against an appropriate prior Image Version.

⸻

10. Checkpoint Strategy

CR-02 established checkpoints as a core persistence mechanism.

CR-05 turns that into visible behavior.

For example:

```
AstroForge is processing your image

Stacking 381 / 462
Estimated remaining: 1m 42s

✓ Calibration
✓ Debayer
✓ Registration
▶ Stacking
```

If the application closes unexpectedly:

```
Your previous processing session was recovered.

AstroForge found a valid checkpoint after registration.

[Resume Processing]
[Start Over]
```

The user should never need to understand checkpoint files.

⸻

11. Preview vs Full Processing

This is a major usability requirement.

Expensive processing should generally support a preview path.

For example:

```
FULL IMAGE
     │
     ▼
Representative Region
     │
     ▼
Preview Processing
     │
     ▼
User Evaluation
     │
     ▼
Full Resolution Processing
```

The preview should be explicitly labeled:

```
Preview — reduced resolution
```

The user can then determine whether a processing choice is visually beneficial before committing significant compute time.

⸻

12. Adaptive Processing

CR-05 introduces an important distinction between:

static parameters and adaptive parameters.

Example:

Instead of:

```
Noise reduction = 0.35
```

AstroForge may determine:

```
Noise profile detected
Background SNR estimated
Star density estimated
Nebula density estimated
Recommended noise reduction:
Moderate
Reason:
High background noise with relatively strong nebular signal.
```

The actual numerical controls can remain available in Guided / Expert mode.

⸻

13. Quality Feedback Loop

Processing should not be blind execution.

After important stages AstroForge should evaluate the resulting image.

Conceptually:

```
PROCESS
   ↓
MEASURE
   ↓
COMPARE
   ↓
ASSESS
   ↓
RECOMMEND NEXT ACTION
```

Example:

```
Stack complete

417 / 462 frames retained.

Image quality improved by removing 45 low-quality frames.

Signal-to-noise improvement: significant.

Next recommended step: Background correction.
```

This creates the foundation for genuinely intelligent processing.

⸻

14. Quality Metrics

CR-05 should establish a standard internal quality-metric framework.

Potential metrics include:

- frame sharpness;
- star eccentricity;
- star count;
- FWHM;
- background variance;
- signal-to-noise estimation;
- saturation;
- clipping;
- gradient strength;
- noise estimate;
- registration quality;
- rejected-frame ratio.

These metrics should primarily drive decisions, rather than overwhelm the user.

Expert users can inspect them.

⸻

15. Intelligent Frame Rejection

The quality-filter stage should be presented as a meaningful decision.

Example:

```
Frame Quality

AstroForge found 45 frames with significantly lower quality.

Main issues:
• Tracking variation
• Elongated stars
• Excessive background noise

Recommended:
Keep 417 / 462 frames

[Preview Result]
[Review Frames]
[Change Threshold]
```

This is far better than presenting a raw sigma-rejection parameter.

⸻

16. Processing Decisions Should Be Reversible

AstroForge must not make destructive edits to the source.

The conceptual model remains:

```
Source
  │
  ▼
Version A
  │
  ▼
Version B
  │
  ▼
Version C
```

If the user dislikes Version C:

```
Return to Version B
```

The system should not need to recompute everything before B if valid artifacts / checkpoints still exist.

⸻

17. Branching

CR-05 should support pipeline branches.

Example:

```
                 Stacked
                    │
          ┌─────────┴─────────┐
          ▼                   ▼
     Natural Color       Artistic Color
          │                   │
          ▼                   ▼
     Denoise A            Denoise B
          │                   │
          └─────────┬─────────┘
                    ▼
                 Compare
```

This is particularly important for AI enhancement.

The user should be able to experiment without destroying the primary processing lineage.

⸻

18. Processing Recipes

CR-05 consumes the Recipe concept from CR-02.

A recipe should define:

```
Recipe
 ├── Target Type
 ├── Acquisition Type
 ├── Required Stages
 ├── Optional Stages
 ├── Parameters
 ├── AI Operations
 ├── Resource Policy
 └── Validation Rules
```

But the recipe should be adaptive.

For example:

```
Deep-Sky OSC — Balanced
```

could become:

```
Calibration
Debayer
Quality Filter
Registration
Stack
Background
Color
Stretch
Denoise
Detail Enhancement
```

while:

```
Deep-Sky OSC — High Quality
```

could enable additional processing.

⸻

19. Recipe vs Pipeline

This distinction needs to be explicit.

Recipe: what AstroForge intends to do.

Pipeline: the concrete execution plan for this dataset.

Example:

```
Recipe:
Deep-Sky OSC / Balanced
        ↓ adaptation
Dataset:
M31 / 462 frames / 3 calibration groups
        ↓
Concrete Pipeline:
Calibrate → Debayer → Reject 45 → Register
→ Stack → Background → Color → Stretch
→ Denoise → Detail
```

This separation is essential for reproducibility.

⸻

20. Processing Intelligence Contract

CR-04 established:

```
Observation → Evidence → Confidence → Decision
```

CR-05 extends this to:

```
Observation → Evidence → Recommendation → Preview → User Decision → Execution → Measurement → Next Recommendation
```

This becomes one of AstroForge's most important architectural patterns.

⸻

21. Resource-Aware Execution

Processing decisions should consider available hardware.

The user should not have to manually choose:

- CPU;
- CUDA;
- DirectML;
- CoreML;
- OpenVINO;
- tile sizes;
- thread counts.

Instead:

```
AstroForge optimized processing for this device.
```

Advanced users can inspect:

```
Execution
CPU
GPU
Memory
Tile size
Precision
Backend
```

This connects directly to CR-09.

⸻

22. Memory Safety

Because AstroForge targets approximately 4–8 GB RAM systems, CR-05 must treat memory management as a product concern rather than merely an engineering optimization.

Large images should support:

- tiled processing;
- streaming;
- memory budgeting;
- intermediate artifact disposal;
- bounded concurrency;
- preview-resolution execution.

The application should avoid the classic failure mode:

```
User selects a large dataset → RAM exhaustion → application disappears.
```

Instead:

```
This operation requires more memory than is currently available.

AstroForge will process it in smaller tiles.

Estimated additional time: ~2m.

[Continue]
```

⸻

23. AI Boundary

CR-05 does not make every processing operation AI-driven.

AI should be introduced where it adds measurable value.

Examples:

- intelligent frame quality assessment;
- adaptive noise estimation;
- parameter recommendation;
- star detection;
- background analysis;
- enhancement recommendation.

Generative / perceptual operations remain explicitly identified.

Example:

```
AI Enhancement

Model: AstroForge Detail v1.2
Type: Perceptual enhancement
Deterministic: Yes
Seed: N/A
```

This operation may alter structures beyond conventional image processing.

This anticipates CR-06.

⸻

24. Pipeline Visualization

The pipeline should have a compact visual representation.

Beginner:

```
CALIBRATE → STACK → COLOR → STRETCH → ENHANCE
```

Guided:

```
✓ Calibration
   ↓
✓ Registration
   ↓
▶ Stacking
   ↓
○ Background
   ↓
○ Color
   ↓
○ Stretch
   ↓
○ Enhancement
```

Expert: expose the actual DAG.

```
quality_filter
      │
      ▼
   debayer
      │
      ▼
 calibration
      │
      ▼
 registration
      │
      ▼
   stacking
   /      \
background color
     \      /
      stretch
         │
      denoise
         │
       stars
         │
         ▼
      export
```

⸻

25. Processing Timeline

The version timeline and pipeline timeline should work together.

Example:

```
10:31  Import
10:33  Calibration complete
10:36  Registration complete
10:41  Stack created
10:42  Background correction
10:44  Color calibrated
10:46  Stretch applied
10:49  AI denoise
10:51  Detail enhancement
```

Selecting an event should reveal the corresponding Image Version.

This is critical for reproducibility.

⸻

26. Data Model Extensions

CR-05 extends CR-02.

### `pipeline_plan`

```
id
session_id
recipe_id
target_type
mode
status
created_at
```

### `pipeline_stage`

```
id
pipeline_plan_id
stage_type
sequence
required
enabled
parameters
```

### `stage_execution`

```
id
pipeline_stage_id
input_version_id
output_artifact_id
status
started_at
completed_at
resource_usage
error
```

### `quality_metric`

```
id
stage_execution_id
metric_type
value
unit
scope
```

### `processing_decision`

```
id
stage_execution_id
decision_type
observation
evidence
recommendation
confidence
user_decision
```

### `preview_run`

```
id
stage_execution_id
source_version_id
preview_artifact_id
parameters
created_at
```

This establishes a traceable relationship between:

Recommendation → Preview → Decision → Execution → Result.

⸻

27. UI-to-Engine API

The application boundary should remain semantic.

Prefer:

```
create_pipeline_plan()
get_pipeline_plan()
preview_stage()
apply_stage()
pause_pipeline()
resume_pipeline()
cancel_pipeline()
retry_stage()
skip_stage()
create_branch()
get_processing_metrics()
get_recommendation()
```

Avoid exposing implementation-level commands such as:

```
execute_node_17()
run_stack_kernel()
call_onnx_model()
```

The latter creates a brittle UI / engine coupling.

⸻

28. Error and Recovery UX

Every processing failure should answer four questions:

1. What happened?
2. What was preserved?
3. What can AstroForge do next?
4. What can the user do?

Example:

```
Stacking could not complete

AstroForge successfully preserved the calibrated and registered data.

The available memory became insufficient while building the stack.

Recommended: retry using tiled processing.

[Retry Optimized]
[Adjust Processing]
[Cancel]
```

This is standalone-product behavior.

⸻

29. Acceptance Criteria

### Pipeline

- [ ] Pipeline is generated from Session Understanding.
- [ ] User can inspect the recommended workflow.
- [ ] Pipeline is represented in human-readable terms.
- [ ] Expert mode exposes the underlying DAG.
- [ ] Required / optional stages are distinguishable.
- [ ] Invalid stage configurations are prevented.

### Execution

- [ ] Pipeline can start.
- [ ] Pipeline can pause.
- [ ] Pipeline can resume.
- [ ] Pipeline can cancel safely.
- [ ] Failed stages can be retried.
- [ ] Optional stages can be skipped.
- [ ] Checkpoints enable recovery.

### Image Versions

- [ ] Meaningful processing stages create Image Versions.
- [ ] Versions are non-destructive.
- [ ] Previous versions remain accessible.
- [ ] Branching is supported.
- [ ] Versions connect to their provenance.

### Intelligence

- [ ] Recommendations include evidence.
- [ ] Confidence is shown where relevant.
- [ ] User decisions are recorded.
- [ ] Post-processing quality is evaluated.
- [ ] Next-stage recommendations can adapt to results.

### Preview

- [ ] Expensive stages can use previews where technically possible.
- [ ] Preview results are clearly distinguished from full-resolution results.
- [ ] User can compare preview outcomes before committing.

### Performance

- [ ] Processing respects memory budgets.
- [ ] Large datasets can use tiled / streamed execution.
- [ ] Resource utilization is observable.
- [ ] Processing can continue safely without requiring external tools.

### Standalone

- [ ] Beginner can process a supported dataset without understanding the DAG.
- [ ] User does not need to configure runtime backends.
- [ ] Errors are actionable.
- [ ] Processing state survives restart.
- [ ] Results remain available after application restart.

⸻

30. Test Strategy

CR-05 needs more than unit testing.

### Pipeline-generation tests

Given:

```
Deep-sky + OSC + calibration + no narrowband
```

verify that the appropriate pipeline is generated.

Given:

```
Planetary + high-frame-count
```

verify that the planetary branch is selected.

### Execution tests

Test:

- start;
- pause;
- resume;
- cancellation;
- retry;
- crash recovery;
- checkpoint recovery;
- partial failure.

### Visual regression

Maintain reference datasets and compare:

- histograms;
- clipping;
- star preservation;
- background;
- noise;
- output dimensions;
- numerical tolerances.

### Resource tests

Run datasets against:

- low-memory configuration;
- normal desktop;
- GPU-enabled machine;
- CPU-only machine.

The application must degrade gracefully.

⸻

31. Implementation Map

The repository already has the core processing implementation under `astroforge-core`, with modules covering capabilities such as calibration, debayering, background processing, color calibration, cosmetic correction, cropping and detail enhancement.

CR-05 should orchestrate those existing capabilities rather than reimplement them.

Conceptually:

```
crates/
├── astroforge-core/
│   ├── pipeline/
│   │   ├── plan.rs
│   │   ├── stage.rs
│   │   ├── execution.rs
│   │   ├── preview.rs
│   │   ├── checkpoint.rs
│   │   ├── recovery.rs
│   │   └── adaptive.rs
│   │
│   ├── processing/
│   │   └── existing processing modules
│   │
│   ├── quality/
│   │   ├── metrics.rs
│   │   ├── frame_quality.rs
│   │   └── image_quality.rs
│   │
│   └── provenance/
│
├── astroforge-ai/
│   ├── recommendations/
│   ├── quality/
│   └── inference/
│
└── astroforge-app/
    ├── pipeline_commands
    ├── processing_events
    └── workspace_state
```

The exact module placement should follow the repository's existing conventions rather than introducing unnecessary restructuring.

⸻

32. Important Architectural Rule

There is one rule made explicit in the v1.0 specification:

The Processing Workspace must never become a visual representation of the internal implementation architecture.

The user does not care that AstroForge internally has:

```
Stage 6
Stage 6.5
Stage 7
Stage 7.5
```

They care that:

```
My image is being aligned, stacked, cleaned and improved.
```

The DAG exists to make the product reliable, resumable and extensible.

It should not dictate the UX.

⸻

33. ADRs Introduced by CR-05

### ADR-05.1: Human Workflow Over Technical DAG

The UI represents meaningful astrophotography operations; the DAG remains an execution abstraction.

### ADR-05.2: Pipeline Generated From Dataset Understanding

Processing plans are derived from CR-04 Session Understanding and are therefore dataset-aware.

### ADR-05.3: Preview Before Expensive Commitment

Where technically feasible, users should be able to evaluate an operation before full-resolution execution.

### ADR-05.4: Processing Is Non-Destructive

Every meaningful transformation produces a traceable Image Version.

### ADR-05.5: Processing Is Measurable

The system evaluates important processing outcomes and can adapt subsequent recommendations.

### ADR-05.6: Resource Constraints Are First-Class

The engine must adapt execution to available hardware rather than expecting users to configure compute resources.

### ADR-05.7: AI Recommendations Are Explainable

AI may recommend or adapt processing, but recommendations remain attributable and inspectable.

⸻

34. Definition of Done

The definitive CR-05 scenario is:

User opens a Session → AstroForge presents its recommended processing workflow → user accepts it → AstroForge generates the concrete execution pipeline → the user can preview meaningful operations → processing begins → progress is visible → checkpoints are created → intermediate Image Versions appear → AstroForge evaluates results → subsequent recommendations adapt where appropriate → user can pause / resume / retry / branch → the final image is available as a versioned result with complete provenance.

The user never needs to know how the underlying DAG works.

⸻

35. Strategic Position of CR-05

The first five CRs now form a coherent product architecture:

```
CR-01
PRODUCT + UX FOUNDATION
        │
        ▼
CR-02
PROJECT + STATE + PROVENANCE
        │
        ▼
CR-03
APPLICATION + STUDIO WORKSPACE
        │
        ▼
CR-04
IMPORT + DATASET UNDERSTANDING
        │
        ▼
CR-05
INTELLIGENT PROCESSING
        │
        ▼
CR-06
AI ENHANCEMENT
```

CR-04 makes AstroForge intelligent about the input.

CR-05 makes AstroForge intelligent about the processing.

CR-06 should make AstroForge intelligent about the image itself and how it can be enhanced. CR-06 is not "add AI denoise." It establishes the AI Enhancement Studio: image analysis, star / nebula / background understanding, enhancement recommendations, deterministic versus perceptual AI operations, model selection, localized enhancement, preview, explainability, safeguards against hallucinated astronomical structures, and full AI provenance.