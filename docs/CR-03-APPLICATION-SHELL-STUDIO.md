# CR-03 — AstroForge Application Shell & Studio Workspace

Status: Proposed
Target: AstroForge v1.0
Priority: Critical
Depends on: CR-01, CR-02
Primary objective: Transform the current application into an image-first astrophotography studio shell built around the persistent Project/Session/Artifact model.

## Relationship to CR-01 and CR-02

The first three CRs now form a coherent foundation:

```
CR-01 PRODUCT & UX FOUNDATION
  defines
            ▼
How AstroForge should work
            ▼
CR-02 PROJECT / SESSION / ARTIFACT ARCHITECTURE
  defines
            ▼
What AstroForge owns and remembers
            ▼
CR-03 APPLICATION SHELL & STUDIO
  defines
            ▼
How the user interacts with it
```

Together: CR-01 defines the experience. CR-02 defines the state. CR-03 defines the interface.

The next major CR should therefore move from structure into actual astrophotography workflow intelligence.

CR-04 — Intelligent Import, Session Understanding & Target Detection takes the current specification's ingest, Bayer detection, frame classification, session grouping, and deep-sky/planetary routing logic and turns them into the first genuinely intelligent user experience: Drop Folder → Detect → Classify → Explain → Confirm → Build Session → Recommend Pipeline. That is the point where AstroForge begins to demonstrate its intelligence before a single processing stage is executed.

## Architectural Decision Records

ADR-03.1 — Image-first workspace. The image canvas is the primary visual object. Reason: AstroForge is an astrophotography editing/enhancement application, not an infrastructure or monitoring application.

ADR-03.2 — Application vs Studio navigation. Separate global application navigation from Project Studio navigation. Reason: Prevents navigation complexity and maintains project context.

ADR-03.3 — Contextual controls. Controls appear according to the current processing/enhancement context. Reason: Prevents a professional feature set from overwhelming beginners.

ADR-03.4 — Progressive disclosure. Auto → Guided → Expert. Reason: Same engine, different cognitive burden.

ADR-03.5 — AI as intelligence. AI is embedded throughout the workflow rather than isolated as a standalone processing mode. Reason: AstroForge's differentiation is not merely access to AI models; it is intelligent application of those models to astrophotography.

## Inter-CR Boundaries

CR-03's Import workspace tab is the first user-facing surface that exercises the underlying ingest pipeline. The actual ingest intelligence (Bayer detection, frame classification, session grouping, deep-sky/planetary routing) belongs to CR-04. CR-03 ships the *workspace tab + UI surface*; CR-04 brings the intelligence behind it. This is an explicit boundary; no work overlaps.

Note: per the user's scope decision for CR-03, the workspace-level Import UX (file selection, folder drop, first-class session summary) is in scope here. The intelligent classification, recommendation, and pipeline-routing engines land in CR-04.

## Phase Map (cross-reference)

The implementation plan lives at `docs/plans/2026-09-06-cr03-application-shell/PLAN.md`. The phase map:

- P0 — this document + plan + orientation harvest + README pointer (#241)
- P1 — application shell + Home workspace (CR-03 §4, §6)
- P2 — Projects workspace + create/open flow (CR-03 §7, §8)
- P3 — Studio entry + Overview workspace + persistent project context (CR-03 §4 shell swap, §8, §9)
- P4 — Import / Process / Enhance / Compare / Export workspace tabs (CR-03 §10–§23)
- P5 — Image version timeline + AI-as-intelligence UX (CR-03 §19–§21)
- P6 — Window lifecycle + error UX + keyboard interaction (CR-03 §25, §28, §30)

## Acceptance Criteria (verbatim from §40)

### Application

- AstroForge launches into a coherent application shell.
- Home, Projects, Recipes, AI Models, Settings and Help are accessible.
- Application navigation is separate from Project navigation.
- Opening a Project enters Studio context.

### Project

- Project identity remains visible while working.
- Project state is persistent.
- Project save state is visible.
- Multiple sessions are represented correctly.

### Studio

- Overview, Import, Process, Enhance, Compare and Export exist as coherent workspace states.
- Image canvas is visually dominant.
- Controls are contextual.
- Panels can collapse.

### Processing

- Pipeline stages are human-readable.
- Progress is visible.
- Pause/resume is supported by the architecture.
- Recovery state can be surfaced.

### AI

- AI recommendations are contextual to the current image.
- AI operations identify their model.
- Deterministic/experimental status is visible.
- Preview-before-apply is supported.

### Versions

- Image versions are visible.
- Users can navigate between versions.
- Version selection changes the displayed image.
- Versions can feed Compare.

### Export

- User can export the selected image version.
- Export does not destroy the Project state.
- Processing history can be included.

### Standalone

- Normal workflow does not require terminal interaction.
- No external database configuration is required.
- No manual AI runtime configuration is required.
- Application can function offline for core processing.

## Definition of Done (§41)

A user should be able to launch AstroForge and experience the complete flow:

```
ASTROFORGE → HOME → [New Project] → M42 — Orion Nebula → IMPORT
→ ANALYZE → PROCESS → [Auto | Guided] → REVIEW → ENHANCE
→ AI Recommendations → PREVIEW → APPLY → NEW IMAGE VERSION
→ COMPARE → EXPORT
```

And after restarting AstroForge:

```
Launch → Projects → M42 → Everything remains → Continue working
```

That is the fundamental CR-03 outcome.

## Cross-references

- CR-01 — Product & UX Foundation (lives at `docs/specs/`)
- CR-02 — Project, Session & Artifact Architecture (`docs/CR-02-PROJECT-SESSION-ARTIFACT-ARCHITECTURE.md`)
- CR-02 implementation plan (`docs/plans/2026-09-06-cr02-domain-model/PLAN.md`) — for the durable substrate that CR-03 surfaces
- CR-02.6 scaffold (`src-tauri/src/commands_project.rs` + `src/lib/astroforge-api.ts`) — the §37 Tauri boundary surface that CR-03 consumes