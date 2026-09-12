CR-07 — Image Review, Comparison & Decision Workspace

Status: Proposed
Target: AstroForge v1.0
Priority: Critical
Depends on: CR-01, CR-02, CR-03, CR-04, CR-05, CR-06
Enables: CR-08, CR-13, CR-14, CR-15, CR-19, CR-23, CR-25

⸻

1. Intent

CR-07 establishes the Image Review, Comparison & Decision Workspace as a first-class part of AstroForge.

CR-05 and CR-06 deliberately introduce multiple image transformations:

* processing stages;
* alternative pipeline configurations;
* AI enhancements;
* different enhancement strengths;
* masks;
* branches;
* perceptual versus deterministic processing.

AstroForge therefore needs a dedicated mechanism for answering:

Which result is actually better, and why?

The objective is not to automate artistic judgment. It is to give the user a rigorous environment in which visual quality, measurable image characteristics, processing provenance, and astronomical integrity can be evaluated together.

The core workflow becomes:

Select → Compare → Inspect → Measure → Assess → Decide → Promote

⸻

2. Product Decision

The Image Version established in CR-02 becomes the central comparison object.

AstroForge must allow users to compare:

* any Image Version within a project;
* versions from different branches;
* pipeline outputs;
* AI-enhanced outputs;
* recipe variants;
* historical versions;
* imported/reference images where supported.

The comparison system must never destroy or overwrite the alternatives being evaluated.

⸻

3. Core Principle

AstroForge measures image characteristics; the user decides what constitutes the better image.

This distinction is critical.

For example:

Version A
Noise:        Low
Detail:       Moderate
Stars:        Natural
Clipping:     Low
Version B
Noise:        Very Low
Detail:       High
Stars:        Slightly artificial
Clipping:     Moderate

AstroForge should not simply declare:

Version B is better.

Instead:

Version B has lower noise and higher measured local detail, but increased clipping and a higher AI-artifact risk.

The final decision belongs to the astrophotographer.

⸻

4. Review Experience

CR-07 introduces a dedicated Compare workspace within the CR-03 Studio.

┌─────────────────────────────────────────────────────┐
│ Project / Target                    Compare          │
├──────────────┬─────────────────────────┬────────────┤
│ VERSION TREE │                         │ ASSESSMENT │
│              │                         │            │
│ v18          │       IMAGE CANVAS      │ Metrics    │
│ v19          │                         │            │
│ v20 ●        │       A ↔ B             │ Provenance │
│ v21          │                         │ Warnings   │
│              │                         │            │
├──────────────┴─────────────────────────┴────────────┤
│ Histogram │ Zoom │ Blink │ Split │ Difference       │
└─────────────────────────────────────────────────────┘

The image remains the dominant element.

⸻

5. Comparison Modes

CR-07 should provide a common comparison framework.

5.1 Side-by-Side

Two versions displayed simultaneously.

Useful for:

* overall composition;
* color;
* star field;
* global processing differences.

⸻

5.2 Split View

A movable divider separates two versions.

┌─────────────────────┬─────────────────────┐
│                     │                     │
│     Version A       │      Version B      │
│                     │                     │
└─────────────────────┴─────────────────────┘
                      ▲
                   divider

The divider should support smooth movement.

⸻

5.3 Blink

Rapid alternation between versions.

Especially useful for identifying:

* star movement;
* registration differences;
* faint structure changes;
* halos;
* sharpening artifacts.

⸻

5.4 Difference View

Calculate/display an image-space difference.

Potential modes:

* absolute difference;
* signed difference;
* amplified difference;
* structural difference.

Difference visualization should be clearly labeled as an analytical visualization, not an astronomical image.

⸻

5.5 Overlay

One version rendered above another with adjustable opacity.

Useful for:

* registration;
* crop alignment;
* star deformation;
* structural changes.

⸻

6. Synchronized Navigation

When comparing versions, AstroForge should synchronize:

* zoom;
* pan;
* cursor location;
* selected region;
* histogram selection;
* pixel inspection;
* mask visualization where compatible.

For example:

Zoom into M51’s core on Version A.

The same location should automatically appear on Version B.

This removes unnecessary navigation friction.

⸻

7. Comparison Scope

Users should be able to compare:

Whole image

Global assessment.

Selected region

For example:

* galaxy core;
* nebula;
* star field;
* lunar crater;
* planetary surface.

Specific feature

Where AstroForge has semantic understanding.

Example:

Compare:
☑ Whole Image
☐ Nebula
☐ Stars
☐ Background
☐ Galaxy Core

This integrates directly with CR-06’s region-aware enhancement architecture.

⸻

8. Image Metrics

CR-07 should expose objective measurements where technically meaningful.

Potential metrics include:

Noise

* luminance noise;
* chrominance noise;
* regional noise.

Sharpness

* estimated FWHM;
* local sharpness;
* edge response.

Stars

* star count;
* star size;
* eccentricity;
* FWHM distribution;
* saturation;
* star-to-background contrast.

Background

* mean background;
* variance;
* gradient strength;
* color gradient.

Dynamic range

* black clipping;
* highlight clipping;
* saturation percentage.

Signal

* estimated SNR;
* local SNR;
* structural contrast.

AI quality

Where applicable:

* segmentation confidence;
* artifact indicators;
* reconstruction risk;
* model confidence.

⸻

9. Metrics Must Be Contextual

A metric should never be presented without explaining what it means.

For example:

FWHM
3.1 px
Lower generally indicates tighter stars,
but values depend on acquisition and
processing conditions.

Likewise:

Higher local contrast does not necessarily mean more real astronomical detail.

This prevents users from optimizing the wrong metric.

⸻

10. Delta Analysis

When comparing two versions, AstroForge should calculate meaningful differences.

Example:

                    A          B       Δ
Noise              18.2       12.1    -33%
Star FWHM           3.4        3.1     -9%
Clipping            0.3%       1.1%   +267%
Background gradient 8.2        5.7     -30%

The delta should identify direction:

* improved;
* degraded;
* unchanged;
* inconclusive.

⸻

11. Quality Assessment

AstroForge should optionally provide an Assessment Summary.

Example:

Assessment
✓ Noise substantially reduced
✓ Background gradient reduced
✓ Star profiles preserved
⚠ Highlight clipping increased
⚠ Moderate AI detail artifacts detected
Overall:
Strong improvement with minor trade-offs.

This is an assessment—not an automatic artistic verdict.

⸻

12. Astronomical Integrity Checks

This is particularly important because of CR-06.

Review should detect potential degradation of astronomical information.

Possible checks:

* faint structure suppression;
* star disappearance;
* artificial stars;
* star shape changes;
* halos;
* ringing;
* excessive smoothing;
* color anomalies;
* clipped galaxy cores;
* nebula structure amplification;
* AI reconstruction artifacts.

Example:

Potential information loss: faint outer nebula structure is less visible in Version B.

This is more valuable than a generic “quality score.”

⸻

13. AI-Aware Comparison

When one version contains AI processing, the comparison should identify it.

Example:

Version A
────────────
Deterministic pipeline
Version B
────────────
AI Denoise
AI Detail Enhancement
AI Super Resolution 2×
⚠ Perceptual reconstruction present

The user can then make an informed comparison.

⸻

14. Provenance Panel

The user should be able to inspect exactly how each version was produced.

Example:

VERSION B
Source:
v18 — Stretched
Processing:
✓ Background Extraction
✓ Color Calibration
✓ AI Denoise
✓ Star Reduction
✓ AI Detail
Models:
AF-Denoise 1.2
AF-Star 1.0
AF-Detail 2.1
Recipe:
Natural Deep Sky v1.4

Advanced provenance can expose:

* parameters;
* model hashes;
* backend;
* seed;
* execution time;
* application version.

This connects CR-07 directly to CR-02 and CR-08.

⸻

15. Version Tree

Comparison must not depend on a flat list of files.

The user should see the transformation graph.

                    v12
                     │
                 Stretched
                     │
             ┌───────┴───────┐
             │               │
            v13             v14
        Natural AI       Aggressive AI
             │               │
            v15             v16
          Final A          Final B

Selecting two nodes automatically prepares them for comparison.

⸻

16. Comparison Sets

Users should be able to save a comparison set.

Example:

M42 Final Candidates
A — Natural
B — AI Enhanced
C — High Detail
D — Narrowband Blend

This allows a user to return later without reconstructing the comparison.

⸻

17. Decision State

An Image Version can have a decision state:

WORKING
CANDIDATE
PREFERRED
FINAL
REJECTED
REFERENCE

These are metadata states.

Rejected does not mean deleted.

⸻

18. Promotion Model

A user can promote a version:

Candidate
    ↓
Preferred
    ↓
Final

Promotion should preserve the complete history.

Example:

Version 27 promoted to Final.

No earlier version is deleted.

⸻

19. Compare → Continue Workflow

The review workspace must not become a dead end.

After choosing a version:

Compare
   │
   ├── Continue Enhancing
   ├── Create Branch
   ├── Mark Preferred
   └── Export

Example:

Version 24 is preferred, but highlights need additional adjustment.

The user should be able to return directly to Enhance.

⸻

20. Intelligent Recommendation After Comparison

AstroForge can optionally identify actionable trade-offs.

Example:

Comparison Insight
Version B has:
✓ Lower noise
✓ Better star separation
⚠ Increased highlight clipping
Recommended next step:
Reduce stretch highlights before
finalizing Version B.

This creates the CR-05/06 feedback loop:

Process → Measure → Compare → Assess → Recommend → Improve

⸻

21. No “AI Winner” by Default

AstroForge should not automatically label an image:

“AI Winner”

unless the user explicitly asks for automated ranking.

Even then, the output should be framed as:

Recommended based on selected quality criteria

rather than objective truth.

⸻

22. Optional Quality Profiles

Advanced comparison can allow the user to select priorities.

Natural

Prioritize:

* information preservation;
* low artifacts;
* realistic stars.

Detail

Prioritize:

* apparent fine detail;
* structure;
* sharpness.

Clean

Prioritize:

* noise reduction;
* background smoothness.

Publication

Balance:

* detail;
* noise;
* color;
* clipping;
* artifacts.

The user should understand that changing the profile changes the evaluation criteria.

⸻

23. Expert Comparison

Expert mode can expose:

* pixel values;
* histogram statistics;
* FWHM distributions;
* star eccentricity;
* noise maps;
* masks;
* difference amplification;
* clipping masks;
* channel statistics;
* local measurements.

This should remain progressive disclosure, consistent with CR-01.

⸻

24. Beginner Comparison

For beginners, the experience should remain extremely simple:

Compare Results
Natural        AI Enhanced
─────────      ────────────
Which do you prefer?
[ Natural ]    [ AI Enhanced ]

AstroForge can provide a concise explanation beneath each:

Lower noise, natural star profiles.

More apparent detail, perceptual reconstruction.

⸻

25. Comparison Data Model

CR-07 should extend CR-02 with concepts such as:

comparison_session
comparison_item
comparison_region
comparison_metric
comparison_delta
quality_assessment
image_decision
comparison_set

comparison_session

A user’s active comparison activity.

comparison_item

References an Image Version.

comparison_region

Defines the comparison area.

comparison_metric

Stores measured characteristics.

comparison_delta

Stores differences between versions.

quality_assessment

Stores analytical observations.

image_decision

Stores Preferred/Final/etc.

comparison_set

Reusable collection of candidate versions.

⸻

26. Semantic API

Conceptual application commands:

create_comparison
add_comparison_version
remove_comparison_version
set_comparison_mode
set_comparison_region
get_comparison_metrics
get_metric_delta
get_quality_assessment
get_version_provenance
set_image_decision
promote_image_version
create_comparison_set
save_comparison_set
create_branch_from_version

Events:

ComparisonCreated
ComparisonVersionAdded
ComparisonModeChanged
ComparisonAnalysisStarted
ComparisonAnalysisCompleted
QualityAssessmentCreated
ImageDecisionChanged
ImageVersionPromoted
ComparisonSetSaved
BranchCreated

⸻

27. Architecture

CR-07 sits between image generation and final decision:

                 CR-05
               PROCESSING
                   │
                   ▼
              Image Versions
                   │
                 CR-06
              AI ENHANCEMENT
                   │
                   ▼
          Multiple Image Versions
                   │
                   ▼
                 CR-07
        REVIEW / COMPARE / ASSESS
                   │
             ┌─────┼─────┐
             │     │     │
          Metrics Visual Provenance
             │     │     │
             └─────┼─────┘
                   ▼
                DECISION
                   │
          ┌────────┼────────┐
          ▼        ▼        ▼
       Enhance   Branch    Export

⸻

28. Implementation Map

Conceptual modules:

crates/
  astroforge-core/
    comparison/
      comparison.rs
      metrics.rs
      difference.rs
      assessment.rs
      regions.rs
  astroforge-persistence/
    comparison/
      repository.rs
      migrations.rs
  astroforge-app/
    comparison_commands.rs
    comparison_events.rs
    comparison_workspace.rs

Existing image-analysis and quality infrastructure from CR-05/06 should be reused.

The exact implementation paths should follow the current repository structure rather than introducing unnecessary parallel modules.

⸻

29. Performance Requirements

Comparison should avoid unnecessarily duplicating large images in memory.

Where possible:

* use existing artifacts;
* use lower-resolution previews for overview comparison;
* stream high-resolution regions;
* cache generated difference images;
* release inactive comparison buffers;
* use GPU/WebGPU acceleration where available;
* fall back to Canvas2D/CPU.

This is important under the CR-09 4–8 GB resource constraint.

⸻

30. Export From Comparison

The comparison workspace should support:

Export selected version

Normal export flow.

Export comparison

Optional:

* side-by-side JPEG/PNG;
* before/after;
* annotated comparison.

Export analytical report

Optional advanced output:

Comparison Report
─────────────────
Source Versions
Metrics
Differences
AI Operations
Quality Warnings
Decision
Provenance

The analytical report should never replace the original high-bit-depth artifacts.

⸻

31. Acceptance Criteria

Core comparison

* [ ]	Any compatible Image Versions can be compared.
* [ ]	Side-by-side works.
* [ ]	Split view works.
* [ ]	Blink works.
* [ ]	Overlay works.
* [ ]	Difference view works.
* [ ]	Synchronized zoom/pan works.
* [ ]	Regional comparison works.

Analysis

* [ ]	Relevant image metrics are available.
* [ ]	Metrics can be compared between versions.
* [ ]	Deltas are calculated.
* [ ]	Quality warnings can be surfaced.
* [ ]	Astronomical integrity checks can be surfaced.
* [ ]	AI processing is identified.

Provenance

* [ ]	Processing history is accessible.
* [ ]	AI model information is accessible.
* [ ]	Recipe information is accessible.
* [ ]	Image Version ancestry is visible.
* [ ]	Provenance remains intact after comparison.

Decisions

* [ ]	Version can be marked Candidate.
* [ ]	Version can be marked Preferred.
* [ ]	Version can be marked Final.
* [ ]	Version can be rejected without deletion.
* [ ]	Preferred/Final state survives restart.
* [ ]	User can continue editing from a selected version.
* [ ]	Branching remains non-destructive.

UX

* [ ]	Image remains dominant.
* [ ]	Beginner mode is simple.
* [ ]	Advanced metrics use progressive disclosure.
* [ ]	Comparison does not expose internal DAG complexity.
* [ ]	Comparison remains usable offline.

⸻

32. Test Strategy

Visual regression

Maintain known reference datasets and verify:

* split alignment;
* blink consistency;
* difference rendering;
* overlay accuracy;
* histogram consistency;
* zoom synchronization.

Metric validation

Validate metrics against controlled datasets with known:

* noise;
* blur;
* clipping;
* star eccentricity;
* background gradients.

Version integrity

Verify that:

Version A
+
Version B
+
Comparison

does not modify either artifact.

AI comparison tests

Verify that AI-derived versions correctly expose:

* model;
* model version;
* model hash;
* classification;
* parameters;
* provenance.

Performance tests

Test comparison with:

* 4K images;
* 8K images;
* 16/32-bit data;
* multiple simultaneous versions;
* large projects;
* limited RAM.

⸻

33. ADRs

ADR-07.1 — Image Version Is the Comparison Primitive

Comparison operates on meaningful Image Versions rather than arbitrary filesystem files.

ADR-07.2 — Visual Comparison Is Primary

Quantitative metrics support visual judgment; they do not replace it.

ADR-07.3 — Metrics Are Contextual

No metric should be interpreted as an absolute measure of astrophotographic quality.

ADR-07.4 — Comparison Is Non-Destructive

Reviewing or rejecting an image never deletes or mutates its source version.

ADR-07.5 — Provenance Is Always Available

Users can trace every candidate back through its processing and AI history.

ADR-07.6 — AI Processing Is Explicit in Comparison

Perceptual or generative transformations cannot be hidden from the comparison context.

ADR-07.7 — Decision Is User-Owned

AstroForge can provide analysis and recommendations but does not silently determine artistic preference.

ADR-07.8 — Comparison Is a Feedback Loop

Review can generate recommendations for additional processing or enhancement.

⸻

34. Definition of Done

A user can:

1. Process a Session.
2. Generate multiple Image Versions.
3. Apply different AI enhancements.
4. Create branches.
5. Open Compare.
6. Select any candidate versions.
7. Compare them side-by-side, split, blink, overlay or difference.
8. Zoom into identical regions.
9. Inspect histograms and relevant metrics.
10. Review AI/provenance information.
11. See potential quality or astronomical-integrity issues.
12. Select a preferred candidate.
13. Continue enhancing that candidate or export it.
14. Preserve every alternative version.
15. Reopen the project later with all comparison and decision history intact.

⸻

35. Strategic Outcome

CR-07 closes a major gap in the AstroForge product loop.

Without CR-07:

AstroForge can create many good images.

With CR-07:

AstroForge helps the user understand the consequences of different processing decisions and choose the result that best meets their intent.

The resulting intelligence loop is now:

         UNDERSTAND
              │
              ▼
          RECOMMEND
              │
              ▼
           PROCESS
              │
              ▼
          ENHANCE
              │
              ▼
          COMPARE
              │
              ▼
           MEASURE
              │
              ▼
           ASSESS
              │
              ▼
           DECIDE
              │
        ┌─────┼─────┐
        ▼     ▼     ▼
     Refine Branch Export
        │
        └──────► REPEAT

This is important to the overall AstroForge concept: the product is not simply a linear image-processing pipeline. It is an intelligent, iterative image-revision system.

That makes CR-08 — Recipes, Reproducibility & Processing Provenance the logical next layer: turning the successful decisions made in CR-05–07 into reusable, reproducible astrophotography workflows.