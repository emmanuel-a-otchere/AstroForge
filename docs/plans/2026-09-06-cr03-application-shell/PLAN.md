# CR-03 — Implementation Plan

Companion to `docs/CR-03-APPLICATION-SHELL-STUDIO.md`. Phased PR-by-PR, no big-bang.

## Constraints

- **No big-bang.** Six PRs (P0..P6), each independently reviewable, each green before the next starts.
- **Additive until P3.** Phases P1 and P2 coexist with the existing `AppShell` + `currentMode (a/b/c/d)` wizard pattern. P3 introduces the Studio shell swap; P1+P2 don't.
- **Strict separation of concerns:**
  - Application shell (left nav, header) = P1
  - Project workspace (project list, project create/open) = P2
  - Studio shell (project header, tab bar) = P3
  - Workspace content (the tabs themselves) = P4
  - Intelligence UX (versions, recommendations) = P5
  - Lifecycle UX (errors, keyboard, window close) = P6
- **CR-04 boundary.** Import workspace tab is shell-only in P4; intelligent ingest lands in CR-04.
- **Visitor-facing docs.** Each phase PR adds a short "what this phase means for a visitor" section to the orientation doc (`docs/ORIENTATION.md`). P0 lands the orientation doc itself.
- **README pointer block.** P0 adds a CR pointer block to `README.md`.

## Existing code constraints (don't break)

- `src/App.svelte` (776 lines) — wizard orchestrator; per-step `currentMode (a/b/c/d)`. Coexists with the new shell.
- `src/components/AppShell.svelte` — wrapper used by App.svelte. Untouched in P1+P2; replaced by P3.
- `src/lib/layout-mode.ts` — drives `currentMode`. Untouched in P1+P2; P3 deprecates the wizard mode swap in favor of project workspace.
- `src/lib/pipeline-store.ts` — wizard pipeline state. Untouched in P1+P2; P3 wires it to the project context.
- `src-tauri/src/commands_project.rs` + `src/lib/astroforge-api.ts` — the CR-02.6 IPC surface (12 commands). P2 wires `projectList / projectCreate / projectOpen / projectGet` to the Projects screen. P5 wires `pipelineRunList / pipelineRunListStages / pipelineRunFindInterrupted` to the version timeline + recovery UX.
- `crates/astroforge-core/src/{domain, domain_store, project, pipeline_run}.rs` — durable substrate. Read-only for CR-03; CR-04 may extend.

## Phases

### P0 — CR-03 doc + plan + orientation + README pointer (#241)

- Save CR-03 to `docs/CR-03-APPLICATION-SHELL-STUDIO.md` (this PR).
- Add `docs/ORIENTATION.md` — visitor-facing orientation covering CR-01/02/03 in plain language, with cross-references to the formal CR docs.
- Add a CR pointer block to `README.md` linking to the three CR docs.
- Add `docs/plans/2026-09-06-cr03-application-shell/PLAN.md` (this file).
- No code changes.

### P1 — Application shell + Home workspace

Implements CR-03 §4 (Application Shell header), §5 (Shell Design Principle), §6 (Home), §26 (Empty States for Home).

New files:

- `src/components/ApplicationShell.svelte` — left nav (Home / Projects / Recipes / AI Models / Settings / Help), persistent header with project identity (filled in P3), responsive collapse for narrow widths.
- `src/components/HomeScreen.svelte` — the §6 Home screen: New Project / Open Project / Import Session / Resume Processing + recent projects list.
- `src/state/application.ts` — Svelte store for the active application navigation target (home / projects / recipes / ai-models / settings / help). P3 adds the Studio overlay.

Behavior:

- Renders inside the existing `App.svelte` root, alongside the wizard mode swap (coexistence).
- The left-nav items route to their respective screens.
- "New Project" and "Open Project" open a placeholder modal in P1 (real flow in P2).
- "Recent projects" is empty in P1; reads through `projectList` in P2.
- Responsive: left nav becomes a collapsible drawer below 1024 px.
- Header shows "AstroForge" and an empty project slot ("No project open") until P3.

### P2 — Projects workspace + create/open flow

Implements CR-03 §7 (Projects), §26 (No Projects empty state).

New files:

- `src/components/ProjectsScreen.svelte` — project grid (thumbnail / target / type / session count / processing state / last modified / latest image version).
- `src/components/CreateProjectDialog.svelte` — form for name + target type; calls `projectCreate`.
- `src/components/OpenProjectDialog.svelte` — slug picker; calls `projectOpen`.

Wiring:

- `projectList()` populates the grid.
- `projectCreate({ name, targetId, applicationVersion })` creates the project; on success routes to P3's Studio entry (placeholder for P2 — the new project doesn't open yet).
- `projectOpen(slug)` opens the project; on success transitions `application.ts` to a `studio-active` state (the actual Studio shell lands in P3).
- `projectDelete(slug)` is wired with the §26 confirm gate.
- `projectArchive(slug)` is wired.

Untouched in P2: the studio entry itself, the wizard mode swap. P2 lands the data flow but the route lands in P3.

### P3 — Studio entry + Overview workspace + persistent project context

Implements CR-03 §4 (Studio shell header swap), §8 (Project Overview), §9 (Persistent Project Context), §33 (Navigation Rules 1–6).

New files:

- `src/components/StudioShell.svelte` — replaces the per-step `<AppShell>` swap inside `App.svelte` with the Studio shell: header (← Projects / project name / save state / processing state) + tab bar (Overview / Import / Process / Enhance / Compare / Export) + content area.
- `src/components/StudioOverview.svelte` — the §8 overview: project identity, image canvas placeholder, processing checklist, "Continue Processing" / "Enhance Image" CTAs.
- `src/state/project-context.ts` — the active project identity (project_id, slug, status, application_version). Read-only for the UI; written by `projectOpen`.

Behavior:

- When `application.ts` enters `studio-active`, `App.svelte` renders `<StudioShell>` instead of `<AppShell>` + wizard.
- Header reads from `project-context.ts`. Save state derives from `project-status` (`Active`/`Archived`/`Exported`).
- Overview's processing checklist is a static derivation in P3 (calibrated / stacked / stretched / enhanced from the project's pipeline runs; placeholder when none). P5 wires real version lineage.
- Navigation Rule 3 (processing does not force the user to remain on the Process screen) is implemented in P3.
- Navigation Rule 6 (destructive actions always require explicit confirmation) is implemented at the `delete_project` IPC boundary.

Deprecation in P3: the wizard mode swap is no longer the primary navigation. The wizard remains functional for the demo flow but the new Studio shell is the canonical surface for new projects. Full wizard retirement is not in CR-03 scope; CR-04 or later CR handles it.

### P4 — Import / Process / Enhance / Compare / Export workspace tabs

Implements CR-03 §10–§23 — the workspace content. The shell exists in P3; this phase fills it.

New files (one per workspace tab):

- `src/components/studio/ImportTab.svelte` — workspace-level UX for picking a folder, dropping files, surfacing the first-class session summary. The intelligent ingest pipeline (Bayer detection, classification, routing) belongs to CR-04; this tab is the user-facing surface for it.
- `src/components/studio/ProcessTab.svelte` — §14 human-readable timeline + §15 stage interaction. Reads `pipeline_run_list(project_id)` + `pipeline_run_list_stages(run_id)`. §16 progress UX reads from the §39 event model in P5.
- `src/components/studio/EnhanceTab.svelte` — §18 AI recommendations layout. The recommendation *content* lands in P5; this PR provides the layout shell.
- `src/components/studio/CompareTab.svelte` — §22 side-by-side / split / blink / difference. Reads two `ImageVersion` selections; P5 wires real version lineage.
- `src/components/studio/ExportTab.svelte` — §23 Export workspace. Calls the existing `export_multi_format` IPC; does not introduce a new export pipeline.
- `src/components/studio/tabs.ts` — shared tab definition (id / label / icon / disabled state).

Behavior:

- Studio tab bar enables / disables tabs based on project state: Overview always-on; Process / Enhance disabled until Import produces a session; Compare disabled until at least two image versions exist; Export disabled until at least one image version exists.
- Each tab implements the §27 responsive behavior: collapse to drawers on narrow widths.
- §11 image canvas remains visually dominant: tabs may not force the canvas to shrink below 60% of the workspace width.
- §25 error UX: technical errors are caught and surfaced as plain-language messages (e.g., "AI denoising could not run" rather than `ORT_ERROR_INVALID_GRAPH`).

### P5 — Image version timeline + AI-as-intelligence UX

Implements CR-03 §19–§21.

New files:

- `src/components/VersionTimeline.svelte` — §21 timeline panel. Reads `pipeline_run_list_stages(run_id)` to derive image versions. Selecting a version changes the canvas image.
- `src/components/RecommendationPanel.svelte` — §19 AI recommendation UI: Observation → Recommendation → Confidence → Preview → User Decision.
- `src/components/AiDisclosure.svelte` — §20 compact AI disclosure: model name, version, determinism, precision, runtime. "Experimental" badge where applicable.
- `src/lib/version-store.ts` — Svelte store for the active image version within a project.

Behavior:

- The version timeline lives in the bottom panel of the Studio shell.
- Selecting a version routes through `version-store.ts`; the canvas reads from this store.
- Recommendations are computed locally from the latest image version's metrics (placeholder heuristic in P5; full intelligence is a future CR).
- §17 processing recovery UX: `pipeline_run_find_interrupted()` runs on Studio entry; if a run is in `Running` or `Queued` state, the recovery dialog offers "Resume" / "Restart Stacking" / "Cancel Run".

### P6 — Window lifecycle + error UX + keyboard interaction

Implements CR-03 §25 (Error UX), §28 (Keyboard), §30 (Window Lifecycle), §29 (Project Persistence Indicator).

New files:

- `src/components/WindowCloseDialog.svelte` — §30 lifecycle: processing active → "Continue in Background" / "Pause and Save" / "Cancel Processing".
- `src/lib/keyboard-shortcuts.ts` — §28 shortcuts (Space / F / 1 / B / Tab / H / Z / Esc / Ctrl+Z / Ctrl+S). Tab-collapses panels. Esc cancels operation. Ctrl+S saves the active project.
- `src/components/SaveIndicator.svelte` — §29 save state: ✓ Saved / ● Saving… / ⚠ Save failed. Derives from the project-context store + IPC command status.

Behavior:

- Keyboard shortcuts are scoped to the Studio context only (no global hijack of the OS).
- The window-close dialog intercepts `beforeunload` when a `Running` pipeline is in flight.
- Save indicator uses Tauri events from `commands_project.ts` (P5+) or a periodic IPC poll in P6 (whichever is cheaper).

## Dependencies

- All phases depend on CR-02's durable substrate (already shipped).
- P1..P6 depend on each other linearly (P1 → P2 → P3 → P4 → P5 → P6).
- P5 requires the version timeline data path which is in `crates/astroforge-core/src/pipeline_run.rs` (already shipped in CR-02.5).
- CR-04 (Intelligent Import) begins after P4 lands; P4's ImportTab provides the workspace surface CR-04 will hook into.

## Risks

- **Wizard deprecation timing.** P3 deprecates the wizard mode swap but doesn't retire it. If a CR-03 acceptance criterion depends on the wizard being gone, that CR will close when the wizard is finally retired (likely CR-05 or CR-06).
- **Image canvas dominance is hard to enforce.** §11 requires the canvas to never be permanently overwhelmed. Implementing this in Svelte/CSS needs careful responsive design; P4 may need iteration.
- **AI recommendations as content (not shell).** P4 ships the recommendation layout; P5 ships the actual recommendation generation. If the heuristic in P5 turns out to be too thin, a future CR may need to extend the recommendation engine.
- **Cross-platform keyboard handling.** §28 shortcuts are stated but the cross-platform semantics (especially Tab on macOS for system traversal) need care. P6 documents the platform-specific behavior in code.

## Verification

Every PR must pass:

- `cargo fmt --all`
- `cargo clippy --all-targets --no-deps -- -D warnings`
- `cargo test --workspace` — all green
- `npm run check` — 0 errors (22 pre-existing warnings unchanged)
- `npm run build` — clean
- 6/6 GitHub Actions CI checks green
- Manual smoke on the desktop dev server for any shell/UX change

## Status

| Phase | Status | Reference |
|-------|--------|-----------|
| P0 | done | #241 |
| P1 | done | #242 |
| P2 | done | #243 |
| P3 | done | #244 |
| P4a | done | #245 |
| P4b | done | #246 |
| P5  | in_progress | this PR (versions + AI recs + Compare picker + Export picker) |
| P6  | pending | save indicator + error UX + keyboard |