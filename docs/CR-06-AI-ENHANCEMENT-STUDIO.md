CR-06 — AI Enhancement Studio & Intelligent Image Revamp

Status: P1–P7 + P5.1 landed (PRs #292–#299 + P5.1 PR pending); spec bump 1.2.0 → 1.3.0
Target: AstroForge v1.3.0
Priority: Critical
Depends on: CR-01, CR-02, CR-03, CR-04, CR-05
Enables: CR-07, CR-08, CR-09, CR-13, CR-14, CR-15, CR-16, CR-17, CR-19, CR-21, CR-23

> **Implementation note (2026-09-11):** All seven CR-06 slices
> shipped in the AI Enhancement Studio tranche (P1 data model +
> provenance + safety classification, P2 image analysis, P3
> recommendation engine, P4 enhancement operations + stack +
> Studio shell, P5 region-aware masks, P6 quality gates +
> branching UX polish, P7 audit + §40 DoD test + spec bump)
> behind PRs #292–#299. The M9 audit records 34 of 39 §35
> criteria shipped, 5 partial (Zone B canvas + real ONNX
> inference), 0 missing — concentrated in CR-07 / P5.1 territory
> rather than CR-06 itself. Spec bumped from 1.1.0 to 1.2.0
> (backward-compatible minor). See
> `docs/plans/2026-09-10-cr06-ai-enhancement-studio/PLAN.md` and
> `docs/M9_AUDIT.md`.

⸻

1. Intent

CR-06 establishes the AI Enhancement Studio as one of AstroForge's defining product capabilities.

The objective is not simply:

"Apply AI Enhance."

It is:

Understand the image → identify what can be improved → explain what AI recommends → preview the transformation → let the user control it → create a new, traceable image version.

AI therefore becomes an intelligent image-revamp layer operating on top of the deterministic astrophotography pipeline established by CR-05.

The resulting experience should allow a beginner to accept an intelligent enhancement recommendation while giving an advanced astrophotographer precise control over what AI changes, where it changes it, which model performs the operation, and how the result differs from the source.

⸻

2. Product Decision

AI Enhancement is a Studio, not a button

AstroForge shall treat AI enhancement as a structured workspace:

Analyze → Recommend → Preview → Adjust → Apply → Compare → Continue

The user should always understand:

1. What AstroForge detected
2. What it recommends
3. Why it recommends it
4. What model/operation will be used
5. What is expected to change
6. Whether the operation is deterministic or perceptual/generative
7. What the resulting image version represents

⸻

3. Core Product Principle

The central CR-06 principle is:

AI may enhance an astronomical image, but it must never silently redefine the astronomical evidence represented by that image.

This is particularly important for:

* super-resolution
* detail enhancement
* star reconstruction
* deconvolution
* inpainting
* generative restoration

AstroForge must distinguish between:

Evidence-preserving enhancement

Operations intended to improve representation of existing information.

Examples:

* denoising
* controlled deconvolution
* color correction
* star reduction
* background correction
* sharpening
* artifact suppression

Perceptual enhancement

Operations that infer or reconstruct information.

Examples:

* AI super-resolution
* learned detail reconstruction
* generative restoration
* aggressive texture reconstruction

The second category must be explicitly identified to the user.

⸻

4. AI Enhancement Experience

The AI Enhancement Studio follows the CR-01 UX spine:

Understand → Recommend → Preview → Adjust → Apply → Compare → Reuse

The user enters Enhance with an existing Image Version.

Example:

Stacked + Stretched Image
          │
          ▼
     AI Analysis
          │
          ├── Noise: High
          ├── Stars: Slightly bloated
          ├── Background: Moderate gradient
          ├── Nebula detail: Good
          ├── Core clipping: Low
          └── Fine detail: Moderate
          │
          ▼
   Recommended Enhancement
          │
          ├── Denoise
          ├── Background cleanup
          ├── Star refinement
          └── Detail enhancement
          │
          ▼
        Preview
          │
          ▼
      User Decision
          │
          ▼
    New Image Version

⸻

5. AI Image Understanding

Before recommending an enhancement, AstroForge should construct an Image Intelligence Profile.

This is distinct from the target/session understanding performed by CR-04.

CR-04 asks:

What did you shoot?

CR-06 asks:

What is happening in this image, and what can be improved?

⸻

5.1 Image Intelligence Profile

The analysis should identify, where technically feasible:

Astronomical structures

* stars
* star fields
* nebula
* galaxy
* globular clusters
* open clusters
* planetary/lunar structures
* bright cores
* faint structures
* dust regions
* emission regions

Image characteristics

* luminance distribution
* background level
* gradients
* noise
* chromatic noise
* local contrast
* sharpness
* blur
* star eccentricity
* star size/FWHM
* clipping
* saturation
* dynamic range
* color balance
* local detail
* ringing
* halos
* compression artifacts

Defects

* hot pixels
* residual calibration artifacts
* satellite trails
* airplane trails
* stacking artifacts
* registration artifacts
* edge artifacts
* interpolation artifacts
* walking noise
* background residuals

⸻

6. AI Analysis Output

The AI analysis should produce structured observations rather than an opaque score.

Example:

Image Intelligence
Background
━━━━━━━━━━━━━━━━
Moderate gradient detected
Confidence: 94%
Noise
━━━━━━━━━━━━━━━━
Moderate luminance noise
Confidence: 91%
Stars
━━━━━━━━━━━━━━━━
Stars slightly bloated
Confidence: 87%
Nebula
━━━━━━━━━━━━━━━━
Strong faint structure preserved
Confidence: 82%
Clipping
━━━━━━━━━━━━━━━━
Minor highlight clipping
Confidence: 96%

This establishes the CR-01 intelligence contract:

Observation → Evidence → Confidence → Recommendation → User Decision

⸻

7. Recommendation Engine

The analysis feeds an AI Enhancement Recommendation Engine.

Example:

Recommended Enhancement
1. Background cleanup
   High confidence
2. Luminance denoise
   High confidence
3. Star refinement
   Medium confidence
4. Local detail enhancement
   Medium confidence
Estimated processing time: 38 sec
Estimated memory: 1.9 GB

The recommendation should also explain sequencing.

For example:

Denoising is recommended before detail enhancement because enhancing the current image first would amplify existing noise.

This makes AstroForge behave like an astrophotography assistant, rather than a collection of AI filters.

⸻

8. AI Enhancement Operations

CR-06 establishes a common framework for AI-powered operations.

8.1 AI Denoising

Capabilities may include:

* luminance denoise
* chrominance denoise
* structure-preserving denoise
* adaptive denoise
* tile-based inference

Controls:

* strength
* detail preservation
* chroma/luma balance
* protected regions
* preview scale

⸻

8.2 AI Deconvolution

Purpose:

Recover apparent spatial detail from known/estimated blur.

Controls may include:

* strength
* iteration count
* PSF/profile
* star protection
* ringing suppression

AI should not replace conventional deconvolution where deterministic algorithms are more appropriate.

⸻

8.3 Star Enhancement

Potential operations:

* star sharpening
* star size normalization
* star profile refinement
* star color recovery
* star separation from background

The operation should understand that stars are astronomical objects, not generic image textures.

⸻

8.4 Star Reduction

Capabilities:

* star-size reduction
* star-field attenuation
* selective star reduction
* star/background separation

Critically, star reduction should use segmentation/masks rather than indiscriminately applying blur or morphological operations.

⸻

8.5 AI Super-Resolution

Super-resolution is a particularly important CR-06 capability.

AstroForge should support:

* 2× enhancement
* potentially higher factors where resource constraints permit
* tiled inference
* overlap blending
* edge-safe processing

But the UI must clearly identify:

Perceptual Reconstruction

rather than presenting the result as objectively recovered sensor information.

Example:

AI Super Resolution
2×
This model reconstructs fine image detail
based on learned visual patterns.
⚠ Perceptual enhancement
Not guaranteed to represent additional
captured astronomical information.

This distinction is central to AstroForge's trust model.

⸻

9. AI Inpainting & Cosmetic Repair

AI inpainting can be used for controlled removal of:

* residual sensor defects
* small cosmetic artifacts
* isolated stacking defects
* unwanted non-astronomical artifacts

However, astronomical structures should be protected.

Therefore:

AI inpainting must never operate unrestricted across an astrophotography image by default.

The user should define or approve the affected region.

⸻

10. Background Intelligence

AI-assisted background analysis should identify:

* gradients
* uneven illumination
* light-pollution patterns
* residual calibration gradients
* color gradients

The enhancement system can recommend:

Background issue detected.
Recommended:
Background model → Adaptive
Strength → Moderate
Protected structures → Nebula + Galaxy

This is preferable to simply applying global background subtraction.

⸻

11. Region-Aware Enhancement

One of CR-06's most important capabilities is localized AI enhancement.

The user should be able to work with semantic regions:

IMAGE
│
├── Stars
├── Background
├── Nebula
├── Galaxy
├── Core
├── Faint structures
└── User mask

An enhancement can therefore target:

Denoise
    ↓
Background only
Sharpen
    ↓
Nebula only
Star reduction
    ↓
Stars only
Detail enhancement
    ↓
Galaxy structure only

This substantially improves quality compared with global AI processing.

⸻

12. Masking Model

CR-06 should introduce a standardized AI mask abstraction.

Masks may be:

Automatic

Generated from AI segmentation.

Parametric

Generated from astrophotography properties.

Examples:

* star brightness
* star size
* luminance
* saturation

User-defined

Brush, gradient, polygon, radial, etc.

Composite

Combination of automatic and user-defined masks.

Example:

Nebula Mask
       +
Exclude Stars
       +
Protect Core
       =
Enhancement Region

⸻

13. AI Enhancement Workspace

The CR-03 three-zone Studio layout remains the foundation.

Zone A — Image / Version Context

Displays:

* current Image Version
* source version
* enhancement history
* branches
* masks
* operation stack

Example:

FINAL
 │
 ├── AI Detail
 ├── Star Reduction
 ├── Denoise
 └── Stretched

⸻

Zone B — Image Canvas

Primary workspace.

Capabilities:

* zoom
* pan
* fit
* 1:1
* before/after
* split comparison
* blink
* difference
* clipping
* histogram
* mask overlay
* region inspection

The image remains the dominant UI element.

⸻

Zone C — AI Intelligence Panel

Example:

AI ANALYSIS
Noise             HIGH
Stars             BLOATED
Background        GRADIENT
Detail            GOOD
────────────────────
RECOMMENDED
✓ Denoise
✓ Background cleanup
✓ Star refinement
○ Detail enhancement
────────────────────
[ Preview All ]

⸻

14. Enhancement Operation Card

Each AI operation should use a common interaction model.

┌──────────────────────────────┐
│ AI Denoise                   │
│                              │
│ Recommended                  │
│ Confidence: 91%              │
│                              │
│ Strength          ●──────    │
│ Detail Preserve   ───●──     │
│                              │
│ Region: Entire Image         │
│                              │
│ Deterministic                │
│ CPU • ~18 sec • ~1.2 GB      │
│                              │
│ [ Preview ]     [ Apply ]    │
└──────────────────────────────┘

⸻

15. Preview Is Mandatory for High-Impact AI

AI operations should support a preview before committing.

The preview should allow:

Before / After

Original | AI Result

Split

Adjustable divider.

Blink

Rapid alternation.

Difference

Visualize changes.

Region preview

Compare only selected region.

⸻

16. AI Safety Classification

Every AI operation must have a classification.

Class A — Deterministic

Expected to produce reproducible output.

Examples:

* deterministic denoise
* segmentation
* conventional deconvolution
* masking

Class B — Learned / Perceptual

Model-based reconstruction with potentially inferred detail.

Examples:

* super-resolution
* learned sharpening
* learned restoration

Class C — Generative

May synthesize information not directly present in the source.

These should be opt-in only.

The UI should never hide the classification.

⸻

17. Hallucination / Fabrication Safeguards

AstroForge needs a strong provenance boundary.

For potentially reconstructive operations:

Before application

Display:

This operation may reconstruct visual detail that was not explicitly resolved in the source image.

After application

The resulting Image Version should carry:

Enhancement:
AI Super Resolution
Classification:
Perceptual
Source:
Image Version 17
Model:
AstroForge-SR-2x
Model Hash:
...
Seed:
...
Backend:
CPU
Parameters:
...

This makes the distinction between:

"better represented"

and

"inferred/reconstructed"

auditable.

⸻

18. AI Model Selection

Users should not normally need to select models manually.

Auto mode:

AstroForge Recommendation
        ↓
Best compatible model
        ↓
Available hardware
        ↓
Memory budget
        ↓
Image characteristics
        ↓
Inference

Expert mode can expose:

* model
* version
* quantization
* backend
* precision
* tile size
* overlap
* CPU/GPU/NPU

This integrates directly with CR-16 — AI Model Hub UX.

⸻

19. Resource Intelligence

CR-06 must integrate with CR-09 and CR-21.

Before inference:

AI Denoise
Model: AF-Denoise INT8
Resolution: 6240 × 4160
Estimated:
Memory: 1.4 GB
Time: ~24 sec
Available:
Memory budget: 4.8 GB
✓ Safe to run

If resources are insufficient:

This enhancement exceeds the current
memory budget.
AstroForge recommends:
✓ Smaller tile size
✓ INT8 model
✓ Preview resolution
[ Use Recommended Settings ]

The user should not be forced to manually understand VRAM/RAM allocation.

⸻

20. Tile-Based AI Inference

For the 4–8 GB target, AI processing must support:

* tiled inference
* overlap regions
* streaming
* bounded memory
* temporary artifact handling
* preview resolution
* model unloading
* cache control

The engine should estimate memory before starting.

No operation should unexpectedly consume the entire machine's memory.

⸻

21. AI Operation Provenance

Every AI operation becomes a first-class provenance object.

Minimum fields:

AI Operation
├── operation_id
├── source_image_version
├── output_image_version
├── operation_type
├── model_id
├── model_version
├── model_hash
├── parameters
├── mask
├── backend
├── precision
├── tile_configuration
├── deterministic_class
├── seed
├── application_version
├── engine_version
├── execution_timestamp
└── resource_metrics

This directly extends the provenance architecture established in CR-02.

⸻

22. AI Enhancement Stack

AstroForge should support an enhancement stack rather than isolated operations.

Example:

Image Version 42
       │
       ▼
AI Background Cleanup
       │
       ▼
AI Denoise
       │
       ▼
Star Refinement
       │
       ▼
Local Detail Enhancement
       │
       ▼
AI Super Resolution
       │
       ▼
Image Version 47

Each operation remains independently identifiable.

The user can:

* reorder where technically valid
* disable
* re-preview
* remove
* branch
* compare
* revert

⸻

23. Intelligent Ordering

AstroForge should detect problematic operation sequences.

For example:

⚠ Recommendation
Detail enhancement is currently positioned
before denoising.
This may amplify existing noise.
Recommended order:
Denoise → Detail Enhancement

This is a major differentiator.

The system is not merely executing commands; it understands processing dependencies and image consequences.

⸻

24. Branching

AI experimentation must not destroy the primary image.

Example:

                    Stretched
                       │
             ┌─────────┴─────────┐
             │                   │
          Natural              Dramatic
             │                   │
        AI Denoise          AI Denoise
             │                   │
       Star Refinement      Super Resolution

Both branches remain available to CR-07 Compare.

⸻

25. Image Version Semantics

AI operations must create meaningful versions.

Bad:

output_1234.tif

Good:

v12 — AI Denoised
v13 — Star Reduced
v14 — Detail Enhanced
v15 — 2× AI Super Resolution

The user can therefore understand the evolution of the image without understanding internal artifacts.

⸻

26. AI Enhancement Data Model

CR-06 extends CR-02 with entities conceptually equivalent to:

image_analysis
image_region
ai_recommendation
ai_operation
ai_model_reference
ai_mask
enhancement_stack
enhancement_preview

image_analysis

Stores the intelligence profile.

image_region

Represents detected or user-defined semantic regions.

ai_recommendation

Stores recommendation + confidence + evidence.

ai_operation

Stores execution provenance.

ai_model_reference

References the exact model used.

ai_mask

Stores mask provenance and parameters.

enhancement_stack

Represents ordered AI transformations.

enhancement_preview

Stores temporary/non-final preview artifacts.

⸻

27. Recommendation Contract

The AI recommendation API should produce structured information similar to:

Recommendation {
    operation
    reason
    confidence
    evidence[]
    affected_regions[]
    expected_effect
    risk_level
    model_candidates[]
    recommended_model
    estimated_runtime
    estimated_memory
    classification
}

This prevents the UI from interpreting opaque AI output.

⸻

28. Semantic API Boundary

Conceptual commands:

analyze_image
get_image_analysis
get_image_regions
get_ai_recommendations
preview_ai_operation
apply_ai_operation
create_ai_mask
update_ai_mask
remove_ai_operation
reorder_ai_operations
create_enhancement_branch
compare_ai_result
get_ai_operation_provenance
get_ai_resource_estimate

Events:

AIAnalysisStarted
AIAnalysisCompleted
AIRecommendationCreated
AIOperationPreviewStarted
AIOperationPreviewCompleted
AIOperationStarted
AIOperationProgress
AIOperationCompleted
AIOperationFailed
AIMaskCreated
ImageVersionCreated
EnhancementBranchCreated

⸻

29. Architecture

CR-06 sits above the existing processing engine:

                 AstroForge UI
                      │
              AI Enhancement Studio
                      │
              AI Intelligence API
                      │
       ┌──────────────┼──────────────┐
       │              │              │
 Image Analysis   Recommendation   AI Operation
       │              │              │
       └──────────────┼──────────────┘
                      │
                  Model Hub
                      │
               ONNX Runtime
                      │
       ┌──────────────┼──────────────┐
       │              │              │
      CPU           GPU/NPU       Backend
                      │
                 AI Artifact
                      │
               Image Version
                      │
                 Provenance

⸻

30. Relationship to CR-05

CR-05 establishes:

How AstroForge processes the dataset.

CR-06 establishes:

How AstroForge intelligently improves the resulting image.

The distinction is important.

CR-05
Dataset
  ↓
Astrophotography Pipeline
  ↓
Scientifically/technically processed image
  ↓
CR-06
AI Image Understanding
  ↓
Enhancement Recommendation
  ↓
AI Enhancement
  ↓
Publication-ready image

AI should not become an uncontrolled replacement for the astrophotography processing pipeline.

⸻

31. Relationship to CR-07

CR-06 generates multiple meaningful image versions.

CR-07 will provide the dedicated comparison experience.

Therefore:

CR-06 creates the alternatives.

CR-07 helps the user judge them.

⸻

32. Relationship to CR-16

CR-06 consumes models.

CR-16 manages them.

Therefore:

CR-16
Model Hub
   │
   ├── Model availability
   ├── Compatibility
   ├── Version
   ├── License
   ├── Hardware suitability
   └── Model health
          │
          ▼
CR-06
AI Enhancement Studio

The enhancement workspace should not become a model-management interface.

⸻

33. Repository Implementation Map

The implementation should integrate with the existing AstroForge architecture rather than create a parallel processing system.

Conceptual additions:

crates/
  astroforge-ai/
    analysis/
    segmentation/
    recommendations/
    enhancement/
    inference/
    provenance/
  astroforge-core/
    image_analysis/
    masks/
    enhancement/
    quality/
  astroforge-persistence/
    ai_operations/
    ai_models/
    image_analysis/
    recommendations/
  astroforge-app/
    ai_commands/
    ai_events/
    enhancement_workspace/

Existing image-processing capabilities should be reused where appropriate rather than duplicated.

The precise paths should follow the repository's actual module conventions during implementation.

⸻

34. AI Model Requirements

Models integrated into AstroForge should provide metadata including:

Model ID
Model Version
Model Hash
License
Task
Input Format
Output Format
Supported Precision
Supported Backend
Minimum Memory
Recommended Memory
Tile Support
Determinism
Perceptual Classification
Expected Runtime

A model that cannot satisfy the provenance requirements should not be silently treated as a normal production model.

⸻

35. Acceptance Criteria

CR-06 is complete when:

AI analysis

* [ ]	AstroForge can analyze an Image Version.
* [ ]	Image characteristics are identified.
* [ ]	Relevant astronomical regions can be identified.
* [ ]	Confidence is exposed.
* [ ]	Observations have supporting evidence.

Recommendations

* [ ]	AI recommendations are generated from analysis.
* [ ]	Recommendations explain their rationale.
* [ ]	Recommendations include confidence.
* [ ]	Resource estimates are available.
* [ ]	Operation ordering can be recommended.

Enhancement

* [ ]	AI denoising works through the unified AI framework.
* [ ]	AI/detail enhancement can be invoked.
* [ ]	Star enhancement/reduction is supported where models permit.
* [ ]	Super-resolution can operate through tiled inference.
* [ ]	Controlled inpainting/cosmetic correction is supported where models permit.
* [ ]	Operations can use masks.

UX

* [ ]	AI Enhancement Studio is image-first.
* [ ]	Preview exists before high-impact AI application.
* [ ]	Before/after comparison is available.
* [ ]	Split comparison is available.
* [ ]	AI operations are understandable without technical knowledge.
* [ ]	Expert controls are available through progressive disclosure.

Trust

* [ ]	Deterministic/perceptual/generative classifications are visible.
* [ ]	Perceptual operations are explicitly labeled.
* [ ]	Generative operations are opt-in.
* [ ]	Model identity is recorded.
* [ ]	Model hash is recorded.
* [ ]	Parameters are recorded.
* [ ]	Seed is recorded where applicable.
* [ ]	Input/output Image Versions are recorded.
* [ ]	AI provenance survives application restart/export.

Resource management

* [ ]	Memory usage is estimated.
* [ ]	Tile processing is supported where required.
* [ ]	Operations respect configured resource limits.
* [ ]	Unsafe model configurations are rejected or adapted.

Non-destructive operation

* [ ]	Original source is never modified.
* [ ]	AI creates new Image Versions.
* [ ]	AI branches are supported.
* [ ]	Previous versions remain recoverable.

Standalone

* [ ]	AI functionality works without external applications.
* [ ]	Missing optional models produce actionable guidance.
* [ ]	Backend selection is automatic in normal mode.
* [ ]	No Python/CLI/model-path knowledge is required from normal users.

⸻

36. Test Strategy

Unit tests

Test:

* image-analysis metrics
* segmentation
* recommendation logic
* confidence calculation
* model compatibility
* resource estimation
* mask composition
* operation ordering
* provenance serialization

Integration tests

Test:

Image
 ↓
Analysis
 ↓
Recommendation
 ↓
Preview
 ↓
AI Operation
 ↓
Image Version
 ↓
Provenance

Model tests

Each bundled model should have:

* known input
* expected output characteristics
* determinism test where applicable
* memory test
* tile/non-tile equivalence tolerance
* failure handling

Visual regression

Maintain reference astronomical datasets for:

* nebula
* galaxy
* star cluster
* lunar
* planetary
* narrowband
* noisy data
* poor-quality data

Compare:

* histogram
* SNR
* star metrics
* structural metrics
* artifact generation
* clipping
* color shifts

⸻

37. AI Quality Gates

An AI operation should not automatically be accepted merely because inference succeeded.

Post-operation validation should examine:

* unexpected clipping
* excessive noise amplification
* star artifacts
* halos
* ringing
* false structures
* color shifts
* edge artifacts
* segmentation leakage
* excessive smoothing

Example:

AI Result Review
✓ Noise reduced
✓ Background preserved
✓ Stars preserved
⚠ Moderate star halo increase detected
Recommendation:
Reduce enhancement strength from 72 → 54

This creates a second intelligence loop:

AI enhancement → quality measurement → validation → recommendation

⸻

38. ADRs

CR-06 should introduce at least the following Architecture Decision Records.

ADR-06.1 — AI Enhancement as a First-Class Studio

AI enhancement is a dedicated user experience rather than a generic image filter.

ADR-06.2 — Evidence-Preserving AI by Default

Default AI operations should prioritize preservation of captured astronomical information.

ADR-06.3 — Perceptual Enhancement Must Be Explicit

Operations capable of reconstructing information must be visibly classified.

ADR-06.4 — AI Operations Are Non-Destructive

Every AI operation produces a new Image Version.

ADR-06.5 — AI Recommendations Require Explainability

Recommendations must expose observation, evidence and confidence.

ADR-06.6 — AI Must Be Region-Aware

Where technically feasible, AI enhancement should operate on semantic regions and masks.

ADR-06.7 — Model Identity Is Part of Provenance

A model cannot be treated as an interchangeable black box.

ADR-06.8 — Resource Constraints Are Part of Model Selection

The optimal model is not simply the highest-quality model; it must be compatible with the available hardware and memory budget.

ADR-06.9 — AI Output Requires Quality Validation

Successful inference does not equal successful astrophotographic enhancement.

⸻

39. Standalone Readiness Impact

CR-06 significantly advances the standalone product definition.

Without CR-06:

AstroForge is an intelligent astrophotography processing application.

With CR-06:

AstroForge becomes an intelligent astrophotography image-editing and enhancement studio capable of understanding an image and actively assisting the user in transforming it.

The distinction is fundamental.

The application should feel like:

"AstroForge understands my image and helps me make it better."

rather than:

"AstroForge contains a collection of AI filters."

⸻

40. Definition of Done

A user should be able to:

1. Open a processed Image Version.
2. Enter Enhance.
3. AstroForge analyzes the image.
4. AstroForge identifies stars, background, structures, noise and defects.
5. AstroForge explains its findings.
6. AstroForge recommends an enhancement sequence.
7. The user accepts the recommendation.
8. AstroForge selects appropriate models automatically.
9. Resource requirements are checked.
10. The user previews the result.
11. The user adjusts strength or regions.
12. AstroForge applies the enhancement.
13. A new Image Version is created.
14. Full AI provenance is recorded.
15. The result can be compared against the source.
16. The user can branch and try an alternative enhancement.
17. The user can undo/revert without destroying prior work.
18. The final result can proceed to Export.
19. Restarting AstroForge preserves the entire enhancement history.

⸻

41. Strategic Position of CR-06

The first six CRs now form a coherent product architecture:

CR	Product question	Result
CR-01	How should AstroForge feel?	Product & UX foundation
CR-02	What does AstroForge remember?	Project, Session & Artifact architecture
CR-03	Where does the user work?	Application Shell & Studio
CR-04	What did the user shoot?	Intelligent Import & Target Understanding
CR-05	How should it process it?	Intelligent Processing Workspace
CR-06	How should it make the image better?	AI Enhancement Studio

And the emerging product loop is now:

             ┌──────────────────────┐
             │       PROJECT        │
             └──────────┬───────────┘
                        │
                     IMPORT
                        │
                        ▼
                  UNDERSTAND
                        │
                        ▼
                   PROCESS
                        │
                        ▼
                    REVIEW
                        │
                        ▼
                  AI ANALYZE
                        │
                        ▼
                  RECOMMEND
                        │
                        ▼
                   ENHANCE
                        │
                        ▼
                   COMPARE
                        │
                        ▼
                    EXPORT
                        │
                        ▼
                    REUSE

CR-06 is therefore the point where AstroForge's AI proposition becomes tangible. It establishes the principle that AI is not merely an enhancement algorithm embedded in the engine; it is an interpretable intelligence layer around the image, capable of understanding image characteristics, proposing interventions, validating outcomes, and maintaining provenance.

The natural next step is CR-07 — Image Review, Comparison & Decision Workspace, because CR-05 and CR-06 now generate multiple Image Versions and branches, and the product needs a dedicated mechanism for the user to judge which transformation is actually better.
