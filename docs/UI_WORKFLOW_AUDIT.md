# AstroForge UI Workflow Audit

**Date:** 2026-09-10
**Baseline:** `origin/main` after CR-05 P6.1b (`aec7974`)
**Scope:** All user-reachable application screens, menus, workspace tabs, dialogs, controls, and integration boundaries.

## Executive summary

No: the UI is not fully done and integrated.

The main Studio workspaces are present as components and the CR-05 Process
workflow has substantial real integration. However, the application still has
user-reachable placeholder shells and browser-mode placeholder data layers.
Several application navigation targets are declared but are not rendered as
real screens, and some controls are intentionally presentational placeholders.
The current state is a credible integrated vertical slice, not a complete
product UI.

The most important distinction is:

- **Implemented and integrated:** Home/Projects shell, project context, Studio
  shell, Overview, Import, Process, Enhance, Compare, Export component routes,
  CR-05 plan creation/execution/preview/recovery/timeline, and core Tauri IPC
  paths.
- **Present but not complete:** Recipes, AI Models, Settings, Help application
  destinations; Overview's non-process checklist state; browser fallback data;
  some Process recovery actions; and the final end-to-end visual/DoD evidence.
- **Not yet proven:** complete cross-screen navigation parity, no-placeholder
  production path, and full fixture-backed UI regression coverage.

## Methodology

This audit reads the UI along four axes:

1. **State machine:** application navigation, Studio viewport, project context,
   workspace stores, pipeline stores, and recovery state.
2. **Layout shells:** ApplicationShell, StudioShell, workspace components, and
   fallback screens.
3. **Component tree:** `App.svelte` → application shell/Studio shell → screen
   components → domain controls and dialogs.
4. **Data flow:** Tauri commands, Svelte stores, browser fallbacks, local state,
   and derived state.

Searches were performed against the current `origin/main` source. This is a
source/integration audit, not a visual screenshot review; it identifies what
is wired by code and what remains placeholder or unverified.

## Component and screen inventory

### Application-level navigation

`src/state/application.ts:11-17` defines six application targets:
Home, Projects, Recipes, AI Models, Settings, and Help. The navigation metadata
is at `src/state/application.ts:36-63`.

The root application composition is in `src/App.svelte:27-32` and renders the
six Studio workspace components: Overview, Import, Process, Enhance, Compare,
and Export. `src/App.svelte:153-170` selects among those workspace screens.

### Studio workspaces

The following components exist:

- `OverviewWorkspace.svelte`
- `ImportWorkspace.svelte`
- `ProcessWorkspace.svelte`
- `EnhanceWorkspace.svelte`
- `CompareWorkspace.svelte`
- `ExportWorkspace.svelte`

The Studio tab rail is generated from `STUDIO_NAV_ITEMS` in
`src/components/StudioShell.svelte:71-96`, which is the correct single list
for those six workspace tabs.

### Supporting controls and dialogs

The repository contains real components for project creation/deletion,
classification, Bayer prompting, destructive confirmation, profiles,
recommendations, stage cards, timeline, recovery, gallery, version timeline,
export, and compare surfaces. Presence does not by itself prove complete
integration; findings below distinguish reachable real behavior from
presentational or fallback behavior.

## State-machine audit

### Application navigation state

`applicationNavTarget` is a typed writable store
(`src/state/application.ts:69-87`). The target enum includes destinations
that need application-shell rendering, but the current root composition is
primarily Studio workspace routing. A declared target without a corresponding
real screen is a navigation contract gap.

### Studio state

`studioViewport` owns the selected Studio view and project-open state. The
Studio shell reads it at `src/components/StudioShell.svelte:45-68` and changes
views at `src/components/StudioShell.svelte:34-42`. This is structurally sound:
the tab rail and active workspace derive from the same store.

### Project context

`projectContext` is closed from StudioShell at
`src/components/StudioShell.svelte:38-42`. The audit should preserve the
existing close path, but a complete workflow test must verify that all child
stores reset or rehydrate when the project changes.

### Pipeline state

CR-05 state is split across `activePlan`, `stageExecutions`, processing metrics,
timeline, and control actions. The Process controls refresh metrics and
 timeline after recovery actions, which is the correct projection pattern.
The remaining concern is not the runner itself; it is that non-Process screens
still use older or placeholder data paths.

## Layout-shell audit

### ApplicationShell

The shell is real for the currently implemented application surfaces, but
`App.svelte:182-187` explicitly renders a placeholder screen for unsupported
application routing. This is user-reachable if the application target changes
to an unimplemented destination.

### StudioShell

The shell, header, close action, tab rail, active-tab state, and workspace
slot are integrated. The fallback at `StudioShell.svelte:110-116` is still a
placeholder path and should be unreachable for the six known Studio views, but
it remains a production-visible dead-end if state or deep-link data is invalid.

### Workspace screens

All six Studio workspace components are mounted by `App.svelte`, but not all
content is equally integrated. `OverviewWorkspace.svelte:11` documents
placeholder data, while `CompareWorkspace.svelte:79-91` contains canvas
placeholder regions. These are not merely CSS labels: they represent missing
image-data integration or missing visual output.

## Component-tree audit

The top-level chain is:

```text
App.svelte
└── ApplicationShell / StudioShell
    ├── Studio header + tab rail
    └── selected workspace
        ├── domain panel(s)
        ├── stage/recommendation controls
        └── dialogs / timeline / export / compare surfaces
```

The strongest integrated branch is Process:

```text
ProcessWorkspace
└── ProcessingControls
    ├── AutoPlanPanel
    ├── IntelligencePanel / RecommendationCard
    ├── ProcessingTimeline
    ├── ErrorRecoveryPanel
    └── ExpertDagView / StageCard
```

The weak branch is application-level secondary navigation: the target list
exists, but a complete target-to-screen component tree is not evident for
Recipes, AI Models, Settings, and Help.

## Data-flow audit

### Canonical backend paths

CR-05 plan creation, stage execution, preview gating, resource detection,
metrics, timeline, retry, and skip use typed IPC wrappers in
`src/lib/astroforge-api.ts` and corresponding Rust commands. This is the
correct architecture: durable state belongs to the backend/domain stores.

### Browser fallback paths

The frontend deliberately has browser-mode fallbacks. Examples include:

- `src/lib/gallery.ts:9` and `src/lib/gallery.ts:156-169`
- `src/lib/profile-store.ts:16-17` and `src/lib/profile-store.ts:227-234`
- `src/state/versions.ts:104-110` and `src/state/versions.ts:164-203`

Those paths are useful for development, but they can make a screen appear
integrated while bypassing persistence. Production acceptance must exercise the
Tauri path, not only Vite/browser mode.

### Overview status derivation

`src/state/workspace.ts:115-126` still returns `import`, `analyze`, `review`, and
`export` as pending while deriving only the process status from real plan or
legacy run data. This means the Overview checklist cannot become fully green
from the current state model even when the corresponding workflows have run.

## Gap taxonomy

### Category 1 — Mock or placeholder shells (HIGH)

**Finding C1.1:** `App.svelte:182-187` explicitly renders a placeholder for
application destinations not handled by the current composition. This is a
reachable dead-end, not just a comment.

**Finding C1.2:** `StudioShell.svelte:110-116` contains a fallback placeholder,
and `OverviewWorkspace.svelte:11` documents deterministic placeholder content.
The shell is real, but the content contract is not uniformly real.

### Category 2 — Parallel switches for the same concept (MEDIUM)

**Finding C2.1:** Application-level targets are represented by
`applicationNavTarget`, while Studio views use `studioViewport`. These are
legitimate separate scopes, but the root must clearly arbitrate which one is
visible; otherwise a target can change without a rendered destination.

**Finding C2.2:** Legacy wizard/pipeline state and CR-05 plan state coexist.
`src/state/workspace.ts:9-19` documents fallback from CR-05 plan executions to
legacy pipeline runs. This is a compatibility bridge, but it creates two
possible sources for process status until the legacy path is retired.

### Category 3 — Rehydration gaps (MEDIUM)

**Finding C3.1:** `workspaceState.load()` asynchronously refreshes the most
recent plan's stage executions at `src/state/workspace.ts:58-85`, but the
refresh is fire-and-forget. A screen can render its initial empty state before
plan execution rows arrive, without an explicit loading state for that branch.

**Finding C3.2:** The browser fallback stores and Tauri stores have different
rehydration semantics. `versions.ts:104-110` loads synthetic data in browser
mode, while production uses IPC. A complete test must verify project-open,
project-switch, and project-close across both paths.

### Category 4 — Dual source of truth (MEDIUM)

**Finding C4.1:** Process status can come from either CR-05 plan executions or
legacy pipeline runs (`workspace.ts:115-126`). This is an intentional fallback,
but it must be removed or made explicit once every project has a CR-05 plan.

**Finding C4.2:** Preview/recommendation state is represented in component-level
reactivity and pipeline-plan stores. The audit should verify that Apply,
Undo/Redo, and project switching always re-read the store value rather than
retaining stale local copies.

### Category 5 — Back-navigation state leakage (LOW)

**Finding C5.1:** `StudioShell.closeProject()` calls both
`studioViewport.closeProject()` and `projectContext.close()` at
`StudioShell.svelte:38-42`, but the audit found no single centralized reset
for every workspace-specific store. This is a likely stale-state risk during
project switching.

**Finding C5.2:** Browser fallback stores intentionally retain in-memory data
for development. If those stores are reused across project sessions, close/open
must clear project-keyed values explicitly.

### Category 6 — Single-layer validation (LOW)

**Finding C6.1:** Import has real backend scanning and classification paths, but
acceptance should test malformed files and partial decode failures, not only
file extensions. The decoder policy is log-and-skip for individual failures;
zero decoded frames must remain a visible error.

**Finding C6.2:** Compare and export should validate that selected versions and
artifacts still exist before rendering or writing. Placeholder canvas regions
make this boundary difficult to distinguish in UI-only testing.

### Category 7 — Fixed-width / fixed-position risks (LOW)

**Finding C7.1:** The Studio tab rail is a single horizontal list
(`StudioShell.svelte:71-96`). Narrow viewport behavior needs an explicit
responsive test; no source-only audit can prove that six tabs remain usable.

**Finding C7.2:** Compare uses side-by-side canvas regions
(`CompareWorkspace.svelte:79-91`). Two-up rendering needs viewport constraints
and overflow testing on compact windows.

### Category 8 — Decorative UI elements (LOW)

**Finding C8.1:** SaveIndicator and resource/backend badges communicate state,
but acceptance must confirm they reflect persisted backend state rather than
only local optimistic state.

**Finding C8.2:** The AI-boundary badge is now backed by execution provenance,
but the audit should verify that unavailable model/backend states disable or
explain the action rather than functioning as decoration.

## Integrated versus incomplete matrix

| Area | Status | Evidence | Conclusion |
|---|---|---|---|
| Home / Projects | Integrated | `App.svelte`, HomeScreen, ProjectsScreen | Real shell and project entry path, with fallback data in browser mode |
| Studio shell | Integrated | `StudioShell.svelte:45-116` | Header, close, tabs, active view are wired |
| Overview | Partial | `OverviewWorkspace.svelte:11`, `workspace.ts:115-126` | Layout exists; several checklist/data paths remain placeholder/pending |
| Import | Partial-to-integrated | `ImportWorkspace.svelte`, import IPC | Real scan/classification path exists; malformed/partial flows need DoD coverage |
| Process | Integrated vertical slice | Process controls, runner IPC, preview/recovery/timeline | Strongest branch; some presentational recovery actions remain |
| Enhance | Partial | `EnhanceWorkspace.svelte`, versions fallback | Screen exists; backend completeness and persistent data path need verification |
| Compare | Partial | `CompareWorkspace.svelte:79-91` | Selection UI exists; canvas regions are placeholders |
| Export | Partial | `ExportWorkspace.svelte`, export IPC | Export path exists; full multi-format/session coverage is not proven |
| Recipes | Incomplete | target metadata only | No complete application screen integration found |
| AI Models | Incomplete | target metadata only | No complete application screen integration found |
| Settings | Incomplete | target metadata only | No complete application screen integration found |
| Help | Incomplete | target metadata only | No complete application screen integration found |

## Recommendations

| # | Recommendation | Effort | Impact | Blocks |
|---|---|---:|---|---|
| R1 | Implement or explicitly gate Recipes, AI Models, Settings, and Help targets; remove the generic placeholder route from production navigation. | M | High | Complete application shell |
| R2 | Replace Overview's pending placeholders with real import/analyze/review/export state derivations. | M | High | Honest project completion status |
| R3 | Finish Compare artifact/image rendering and replace canvas placeholders with real version-backed output. | M | High | Trustworthy review workflow |
| R4 | Centralize project-open/project-close rehydration and reset for all workspace stores. | M | Medium | Reliable project switching |
| R5 | Separate or retire legacy pipeline fallback once CR-05 plans cover all supported projects. | M | Medium | Single source of truth |
| R6 | Add a production-path UI DoD suite covering drop folder → plan → preview → process → recover → compare → export. | L | High | Release confidence |
| R7 | Replace intentional Process presentational placeholders (`adjustProcessing`, `contactSupport`) with real flows or clearly disable them. | S–M | Medium | No dead controls |
| R8 | Add responsive and accessibility verification for six-tab Studio navigation and two-up Compare layout. | S | Medium | Usability on real windows |

## Recommended next tranche

The highest-value next step is **R1 + R2**: finish the application navigation
contract and make Overview truthful. R3 is the next user-facing visual gap.
R4 and R5 should follow before calling the full UI integrated.

## Suggested acceptance checklist

- Every application nav target lands on a real screen or is visibly disabled with
  an explanation; no generic placeholder route is reachable in production.
- Every Studio tab renders persisted project data or an honest empty state.
- Project close/open resets all project-scoped stores and rehydrates the new
  project before controls become actionable.
- Overview status is derived from import, analysis, process, review, and export
  domain state rather than hardcoded pending values.
- Compare renders selected persisted artifacts, including missing-artifact and
  incompatible-version errors.
- Process recovery actions either execute real workflows or are disabled; no
  dead-looking buttons.
- The critical path passes in the Tauri runtime, not only browser fallback mode.
- Responsive and keyboard navigation checks pass for the Studio rail, dialogs,
  menus, and Compare layout.

## Conclusion

The answer is **not yet**: the UI is substantially integrated for the core
Studio/Process vertical slice, but not all screens, menus, and elements are
complete. The source contains explicit placeholders and incomplete application
navigation. We should treat the product as **core workflow integrated, shell and
secondary surfaces incomplete** until R1–R3 and the DoD suite are addressed.
