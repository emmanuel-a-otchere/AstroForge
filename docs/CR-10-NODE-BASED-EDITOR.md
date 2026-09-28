# CR-10 Node-Based Editor

Status: Proposed
Target: AstroForge v1.5.0
Priority: High
Depends on: CR-02 (domain substrate), CR-05 (intelligent processing, owns
`pipeline-store.ts` + `NodeSidebar.svelte`), CR-08 (recipes, owns
`RecipeEditor.svelte` + `.astroforge-recipe` JSON format)
Enables: CR-11 (recipe application UX), Phase 2 deep-sky pipeline UI
revisions, Phase 4 recipe gallery UX

CR-10 establishes a user-facing node-based editor over the AstroForge
processing pipeline. It complements the existing wizard-stepper UX
(`WizardBottomSheet.svelte`) with a graph-oriented view that surfaces
the full pipeline as named, tooltipped, reorderable nodes; lets the
user insert, disable, and reorder stages; round-trips the active
graph to and from the `.astroforge-recipe` JSON format; and binds the
graph to a project plus optional per-session overrides.

⸻

## 1. Intent

AstroForge users who outgrow the wizard-stepper UX need to see the
pipeline as a graph: every stage visible at once, with tooltips that
explain what each stage does, a palette for inserting new stages,
drag-or-click reordering, and a clear mapping between the graph they
see and the recipe they share.

The node editor is not a replacement for the wizard or the
`NodeSidebar.svelte` linear DAG. It is a complementary view that
exposes the same `PipelinePlan` data through a different visual
paradigm, and binds that view to the existing recipe, comparison, and
provenance machinery.

A user can:

1. See the active pipeline as a graph of named nodes
2. Hover any node to see a tooltip describing what it does
3. Open any node to see its full parameter panel
4. Insert a new node from a categorized palette
5. Reorder, enable, disable, or remove non-required nodes
6. Save the active graph as a `.astroforge-recipe` file
7. Load a `.astroforge-recipe` file into a new graph
8. Apply the same graph to different image sets, with per-image
   execution history preserved

CR-10 does not introduce a new pipeline execution model. The
`PipelinePlan` / `PipelineStage` types in
`crates/astroforge-core/src/domain.rs:489-538` and the
`PIPELINE_STAGES` / `PipelineStageType` in
`src/lib/pipeline-store.ts:8-192` are the authoritative substrate.
CR-10 layers a node-graph UX on top.

⸻

## 2. Product Decision

### 2.1 The graph is a view, not a new execution layer

A node graph in CR-10 is a visual representation of the same
`PipelinePlan` data the wizard already manages. Every read, write,
and persistence path goes through the existing
`pipeline_plans_store.rs` and the existing `pipeline-store.ts`
abstractions. The graph is not a parallel state.

### 2.2 The wizard and the graph are mutually exclusive views, not split views

A session has one of two modes: Wizard or Node. The user toggles
between them. Internally both modes mutate the same `PipelinePlan`;
externally each mode presents a different UI. Switching modes does
not lose state.

### 2.3 Two layout modes, one data model

A top-bar toggle selects between:

- **Constrained** (default): linear vertical DAG, one node per row,
  edges drawn as straight lines. Mirrors the existing
  `NodeSidebar.svelte` behaviour.
- **Free-form**: draggable nodes on an SVG canvas, bezier wires,
  pan/zoom, snap-to-grid. The underlying `PipelineStage.sequence` is
  derived from the topological order of the edges.

Both layouts read and write the same data. The toggle is a user
preference (not a per-project setting).

### 2.4 Insert / reorder / remove are first-class operations

A user can add a new stage to the active graph (via the palette),
reorder stages (via drag in free-form, via context menu in
constrained), enable or disable any stage, and remove any
non-required stage. Required stages are protected; removal is
blocked with an inline warning.

### 2.5 Recipe round-trip is bidirectional and lossless within scope

A graph serializes to a `.astroforge-recipe` JSON via the existing
`pipeline[]` array. A recipe deserializes into a graph by validating
each referenced `stage_type` against the engine catalog and
materializing a `PipelinePlan`. The round-trip preserves every
parameter the engine understands; recipe-level metadata (name,
author, equipment_hints, integrity) is preserved on the recipe side
and surfaced in the graph's metadata panel.

### 2.6 One graph per project, per-session overrides

A project carries one default graph (the project's
`PipelinePlan`). Each session can override the graph with its own
`PipelinePlan`. Re-applying the project's default is a one-click
"Reset to project default" action. Per-image-set execution history
is preserved per `StageExecution` in
`crates/astroforge-core/src/domain.rs:540`.

### 2.7 Tooltips come from Rust doc-comments, not hand-curated JSON

The canonical source for per-parameter hints is the doc-comments on
the Rust `*Params` structs (`CalibrateParams`, `StretchParams`,
`DenoiseParams`, etc.) in
`crates/astroforge-core/src/pipeline_plan/dispatch.rs`. A build-time
generator emits a typed JSON manifest that the frontend reads via a
new IPC. Hand-curated hints drift; generated hints stay in sync with
the engine.

⸻

## 3. Scope

### 3.1 In Scope

- The 12 user-facing node types listed in `PIPELINE_STAGES`
  (`src/lib/pipeline-store.ts:131-192`) plus their engine mappings
  per §7.7.9 of the spec
- A node palette grouped by six engine categories (Input,
  Calibration, Calibration-Free, Stacking, Stretch, Refinement,
  Output) per Decision D-CR-10-5
- A constrained layout (default) and a free-form layout (draggable
  SVG canvas with bezier wires) per Decision D-CR-10-4
- Per-stage mutation: enable, disable, insert, reorder, remove
  (non-required only) per Decision D-CR-10-9
- Graph ↔ `.astroforge-recipe` JSON round-trip per Decision D-CR-10-7
- Per-project default graph + per-session overrides per Decision
  D-CR-10-6
- Tooltip source generated from Rust `*Params` doc-comments per
  Decision D-CR-10-8
- Existing `NodeSidebar.svelte` and `ParameterSidebar.svelte`
  surfaces extended (not replaced) for node-graph presentation

### 3.2 Out of Scope

- Free-form graph editing with arbitrary user-defined node types
- Custom node authoring (a user cannot define a new `stage_type`;
  the catalog is closed)
- Real-time collaborative graph editing (multiple users editing one
  graph simultaneously)
- Version-controlled graph branches (linear history lives at
  `.astroforge-recipe` v1; branching is a Phase 4 follow-on)
- Drag-to-insert from the palette (click-to-insert only in v1.5.0;
  drag is a follow-on)
- New pipeline execution semantics (the existing
  `PipelineRunner::with_handlers` path remains authoritative)
- New AI or model-registry work (CR-10 does not add models)

⸻

## 4. UX Specification

### 4.1 Top-bar toggle

A toggle in the top navigation bar switches between Wizard mode and
Node mode. The toggle persists across sessions for the current
project. Switching modes preserves all graph state; only the visual
layout and input controls change.

### 4.2 Layout-mode toggle

Within Node mode, a secondary toggle selects Constrained vs
Free-form layout. The toggle persists for the current user (across
projects) in the same location as the existing verbosity preference
(`Verbosity` in `src/lib/ipc.ts:26`).

### 4.3 Node card surface

Each node card shows:

- Icon (from the set in §7.7.2)
- Label (from `PIPELINE_STAGES[i].label`)
- Status LED (from `NodeStatus` enum in `pipeline-store.ts:40`)
- AI badge when `AiBoundaryLabel::for_stage_type(stage_type).uses_ai`
  is true
- Hover tooltip: first 3 to 7 words of `description`
- Click target: opens the parameter panel for the node
- Right-click target: opens the context menu (Re-run from here,
  Disable, Insert before, Insert after, Move up, Move down, Reset
  to default params, Remove [disabled for required stages])

### 4.4 Palette surface

The palette is a left-sidebar panel listing the 12 nodes grouped by
the six engine categories. Each entry shows icon + label + a
one-line hint. Click inserts the node into the active graph at the
end of the sequence (constrained) or at the current cursor position
(free-form). Search field at the top filters by label or hint
substring.

### 4.5 Parameter panel surface

The parameter panel (extending the existing `ParameterSidebar.svelte`)
shows the full parameter set for the selected node. Each parameter
has:

- Label
- Hint (from the generated `sub_features` manifest)
- Control (slider, numeric input, dropdown, checkbox, depending on
  the parameter's type)
- Default-value badge on hover

### 4.6 Recipe import / export

Two toolbar buttons:

- "Save as Recipe" opens a modal for recipe name + author +
  equipment_hints + target_type, then serializes the active graph
  to `.astroforge-recipe` JSON and saves via the existing
  `recipe_save` IPC.
- "Open Recipe" opens a file picker, deserializes the recipe, and
  prompts the user to either replace the active graph or create a
  new session.

### 4.7 Context-sensitive warnings

The existing `DestructiveConfirmDialog.svelte` is shown when:

- The user enables a stage whose type is in
  `DESTRUCTIVE_STAGES` (`pipeline-store.ts:28-35`)
- The user re-runs any stage whose type is in `DESTRUCTIVE_STAGES`
- The user removes a stage that has produced an Image Version

⸻

## 5. Acceptance Criteria

The CR is closed when every item below is `Pass`:

1. A user can switch a session from Wizard mode to Node mode and
   back without losing any pipeline state.
2. The Node mode shows the active `PipelinePlan` as a graph of
   named nodes; each node's label matches the canonical
   `PIPELINE_STAGES[i].label`.
3. Hovering any node shows a tooltip whose text is the first 3 to 7
   words of the node's `description`.
4. The palette lists the 12 user-facing nodes, grouped into the six
   engine categories per Decision D-CR-10-5.
5. Clicking a palette entry inserts the corresponding node into the
   active graph at a deterministic sequence position.
6. In Free-form layout, the user can drag any node to a new canvas
   position; the `PipelineStage.sequence` is recomputed from the
   topological order of the edges.
7. The user can enable, disable, reorder, or remove any non-required
   stage; required stages cannot be removed and removal is blocked
   with an inline warning.
8. A graph serializes to a `.astroforge-recipe` JSON file via
   "Save as Recipe"; the resulting JSON round-trips through
   `Recipe::from_json` and back into a `PipelinePlan`.
9. A `.astroforge-recipe` JSON file loads into a new session via
   "Open Recipe"; every referenced `stage_type` is validated against
   the engine catalog; unknown types surface as a diagnostic.
10. Each project carries one default graph; each session can
    override the graph; "Reset to project default" restores the
    project graph in the session.
11. Tooltip content for parameter hints comes from a generated JSON
    manifest (`crates/astroforge-core/src/node_catalog.json`)
    derived from the Rust `*Params` doc-comments; no hand-curated
    hints exist.
12. The 12 user-facing node types map to engine-dispatched
    `stage_type` strings per §7.7.9; the mapping is enforced at
    startup by `NodeCatalog::resolve`; an unmapped type is a
    startup-time error.
13. The Node mode respects the existing verbosity setting
    (`Verbosity` in `src/lib/ipc.ts`): Expert users see the full
    parameter panel and tooltips; Intermediate users see a curated
    subset; Beginner users see only labels and status.
14. The NodeSidebar.svelte / ParameterSidebar.svelte /
    NodePalette.svelte surfaces compose cleanly with the existing
    ProcessingControls, IntelligencePanel, RecoveryBanner, and
    CompareWorkspace surfaces in ProcessWorkspace.
15. The Tauri command surface for node-graph operations is
    documented at `src-tauri/src/commands_node_graph.rs` and
    registered in `main.rs::invoke_handler`; each command has a
    matching entry in `src/lib/astroforge-api.ts`.

⸻

## 6. New Stages

CR-10 adds three engine stages that the node-graph UX requires but
the existing wizard does not:

| stage_type | Purpose | Maps to user-facing |
|---|---|---|
| `crop` | Crop and rotate the active image; parameters include rotation, aspect ratio, focal-point selection | `crop_rotate` |
| `star_handling` | Separate stars from extended emission, edit layers independently, recombine with replaceStrength + colorBoost | `star_handling` |
| `creative_polish` | Curves, colour transmutation, narrowband palette mixes | `creative_polish` |

These stages are added to the `StageHandler` registry in
`crates/astroforge-core/src/pipeline_plan/dispatch.rs` and to the
`PIPELINE_STAGES` array in
`src/lib/pipeline-store.ts:131-192`. Each gets a
`*Params` struct in the dispatcher with doc-comments that drive the
generated tooltip manifest per Decision D-CR-10-8.

⸻

## 7. Implementation Locations

```
crates/
├── astroforge-core/
│   ├── node_catalog.rs              (NEW: catalog, taxonomy mapping, generated manifest)
│   ├── node_graph/
│   │   ├── mod.rs                   (NEW)
│   │   ├── graph.rs                 (NEW: Graph, Node, Edge types)
│   │   ├── mutation.rs              (NEW: insert / remove / reorder / enable / disable)
│   │   └── recipe_io.rs             (NEW: Graph ↔ Recipe JSON)
│   ├── pipeline_plan/
│   │   └── dispatch.rs              (EXTEND: add CropHandler, StarHandlingHandler, CreativePolishHandler + their *Params structs)
│   └── pipeline_plans_store.rs      (EXTEND: single update_graph write path; per-stage primitives are in-process only)
├── astroforge-ai/                  (no change: CR-10 does not add models)
└── (no new crates)

src-tauri/
├── src/
│   ├── commands_node_graph.rs       (NEW: get_node_catalog, update_graph, graph_to_recipe, recipe_to_graph, list_project_graphs, get_session_graph_override, set_session_graph_override, reset_session_to_project_default)
│   └── main.rs                      (EXTEND: register commands_node_graph in invoke_handler)

src/
├── lib/
│   ├── node-graph-store.ts          (NEW: nodes, edges, activeNodeId, layoutMode, mode toggle, mutation primitives)
│   └── astroforge-api.ts            (EXTEND: typed wrappers for each new command)
├── components/
│   ├── NodePalette.svelte           (NEW: palette grouped by 6 categories, search field, click-to-insert)
│   ├── NodeSidebar.svelte           (EXTEND: add free-form layout mode, palette toggle, drag-to-reorder, edge rendering)
│   ├── ParameterSidebar.svelte      (EXTEND: consume generated sub_features manifest, show hint per param)
│   └── ProcessWorkspace.svelte      (EXTEND: mount NodePalette + mode toggle when in Node mode)
└── App.svelte                       (EXTEND: Wizard/Node mode toggle, persistence)

tests/
├── crates/astroforge-core/tests/
│   ├── node_catalog_test.rs         (NEW: taxonomy mapping, generated manifest round-trip)
│   ├── node_graph_mutation_test.rs  (NEW: insert / remove / reorder / enable / disable)
│   └── recipe_io_test.rs            (NEW: Graph ↔ Recipe JSON round-trip; unknown stage_type diagnostic)
└── (no new tests in src-tauri per the P0/P1 contract; behavioral tests live in astroforge-core)
```

The full PR-by-PR breakdown is in
[`plans/2026-09-28-cr10-node-based-editor/PLAN.md`](plans/2026-09-28-cr10-node-based-editor/PLAN.md).

⸻

## 8. Open Decisions

| # | Issue | Status | Resolution target |
|---|---|---|---|
| OD-CR-10-1 | Palette UI affordance: icon + label only, or icon + label + per-node 2-line hint visible at rest? | Open | UX review at M1.6.2 |
| OD-CR-10-2 | Free-form canvas library: roll a small SVG helper, or adopt `@xyflow/svelte` (svelte-flow)? | Open | Slice 1.6.1.1 spike |
| OD-CR-10-3 | Should the NodePalette live in a permanent left sidebar, or be a collapsible drawer? | Open | UX review at M1.6.2 |
| OD-CR-10-4 | When a session overrides the project default graph, is the override visible in the Node view, or only via "Reset to project default"? | Open | UX review at M1.6.3 |
| OD-CR-10-5 | Should the AI badge be a permanent chip on every AI node card, or only on hover? | Open | UX review at M1.6.2 |

⸻

## 9. Related Work

- CR-05 Intelligent Processing Workspace (Implements through P6, P7
  audit landed; owns `pipeline-store.ts` and `NodeSidebar.svelte`)
- CR-08 Recipes, Reproducibility & Processing Provenance (Implements
  through §28; owns `RecipeEditor.svelte` and the
  `.astroforge-recipe` JSON format)
- PROFILE_PIPELINES_PLAN.md D-4 (the `color_wb` + `color_scnr` split,
  landed in CR-10 §7.7.9)

⸻

## 10. Carrier

This document is the canonical CR for the node-based editor work. It
is carried into `docs/specs/AstroForge_Spec_v1.4.0.md` as §7.7
Node-Based Editor Surface and as a "Delta from 1.4.0" section at the
foot of that file. The implementation plan lives at
[`docs/plans/2026-09-28-cr10-node-based-editor/PLAN.md`](plans/2026-09-28-cr10-node-based-editor/PLAN.md).
The project plan gains a new Phase 1.6 between Phase 1.5 (Guided
Processing Train) and Phase 2 (Full Deep-Sky Pipeline).

End of CR.