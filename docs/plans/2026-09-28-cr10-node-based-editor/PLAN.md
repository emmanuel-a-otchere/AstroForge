# CR-10 Implementation Plan: Node-Based Editor

**Date:** 2026-09-28
**Status:** Proposed (docs-only slice landed; P1 onwards pending)
**Source CR:** [`../../CR-10-NODE-BASED-EDITOR.md`](../../CR-10-NODE-BASED-EDITOR.md)
**Spec carrier:** [`../../specs/AstroForge_Spec_v1.4.0.md`](../../specs/AstroForge_Spec_v1.4.0.md) §7.7 + Delta from 1.4.0
**Strategy:** P0 docs-only (this PR), then four sub-milestones (1.6.1 through 1.6.4) decomposed into one-task-per-IPC slices per Decision D-CR-10-10. Each slice is one PR-sized tranche; each tranche is independently reviewable and CI-green before the next starts.

## Why a phased rollout

CR-10 is the largest UX CR to date in the AstroForge programme: a new visual surface (free-form canvas), a new data type (Graph + Edge + the per-stage primitives), three new engine stages (`crop`, `star_handling`, `creative_polish`), a generated tooltip manifest, a Graph ↔ Recipe JSON round-trip, and an extension to the existing Wizard / Node toggle. A big-bang landing would be unreviewable and would block CR-11 (recipe application UX) for the duration. The phased structure keeps each PR small enough to reason about end-to-end.

## Current state (as of 0ff0655, CR-08 Slice I)

What already exists:

- `crates/astroforge-core/src/ai_boundary.rs`: `AiBoundaryLabel::for_stage_type` with the locked-test enumeration of 10 stage types (`all_eight_classical_stage_types_are_not_ai`, lines 158-167).
- `crates/astroforge-core/src/pipeline_plan/dispatch.rs`: `StageHandler` trait, 8 handler structs (`StackHandler`, `CalibrateHandler`, `StretchHandler`, `DenoiseHandler`, `DebayerHandler`, `RegisterHandler`, `BackgroundHandler`, `ExportHandler`) with documented `*Params` structs.
- `crates/astroforge-core/src/domain.rs:489-538`: `PipelinePlan` + `PipelineStage` types carrying `stage_id`, `stage_type`, `sequence`, `label`, `required`, `enabled`, `parameters_json`, `produces_image_version`, `undo_supported`.
- `crates/astroforge-core/src/domain.rs:540-577`: `StageExecution` keyed by `plan_id + stage_id + attempt` for per-image-set history.
- `crates/astroforge-core/src/pipeline_plans_store.rs`: SQLite persistence for `PipelinePlan` + `PipelineStage`.
- `src/lib/pipeline-store.ts:8-22`: `PipelineStageType` union of 12 user-facing types.
- `src/lib/pipeline-store.ts:131-192`: `PIPELINE_STAGES` array of `StageDefinition { type, label, description, defaultParams }`.
- `src/components/NodeSidebar.svelte` (326 lines): vertical SVG-card DAG with straight `<line>` connectors, status colors, context menu (Re-run from here).
- `src/components/ParameterSidebar.svelte` (297 lines): full parameter panel for the selected node, writes through `onParamsChange` → `App.handleParamsChange` → `updateNodeParams`.
- `src/components/CompareWorkspace.svelte` (501+ lines): side-by-side A/B comparison of two Image Versions, per-version execution history.
- `src/components/RecipeEditor.svelte` (1080 lines): Beginner + Guided tier; Expert tier is a "ProfileManager" stub (lines 42-45) awaiting node-graph integration.

What does NOT yet exist:

- A canonical `NodeCatalog` type in `crates/astroforge-core` (the 12-to-10 taxonomy mapping is not yet a first-class Rust type; the §7.7.9 mapping table lives in prose only).
- A generated tooltip manifest at `crates/astroforge-core/src/node_catalog.json`.
- A node palette UI (`NodePalette.svelte`).
- A free-form draggable canvas mode (the existing `NodeSidebar.svelte` is constrained-only).
- Pipeline mutation IPCs (`update_graph`, `insert_stage`, `remove_stage`, `reorder_stage`).
- A Graph ↔ Recipe JSON round-trip module.
- The three new engine stages (`crop`, `star_handling`, `creative_polish`): referenced by the user-facing taxonomy but not implemented as `StageHandler` structs.
- A Wizard/Node mode toggle in the top navigation bar.

## Slice decomposition

CR-10 decomposes into **5 PRs** per the AstroForge CR-slicing convention: P0 docs-only, then four sub-milestones (1.6.1 through 1.6.4) each decomposed into one-task-per-IPC slices per Decision D-CR-10-10.

### P0: Documentation (this PR, lands first)

Scope:

- `docs/specs/AstroForge_Spec_v1.4.0.md`: append §7.7 Node-Based Editor Surface in the body (between §7.6 and §8) + a "Delta from 1.4.0" section at the foot linking to the CR.
- `docs/CR-10-NODE-BASED-EDITOR.md`: new standalone CR document.
- `docs/plans/2026-09-28-cr10-node-based-editor/PLAN.md`: this file.
- `docs/PROJECT_PLAN.md`: new Phase 1.6 (between Phase 1.5 and Phase 2) with 4 sub-milestones.
- `docs/specs/SPEC_INDEX.md`: add a "Forward-look slice (proposed, this PR)" entry pointing to CR-10.
- `CHANGELOG.md`: add a Phase 1.6 entry.

Verification gates (per `astroforge-cr-slices` P0 docs-only exception):

- `cargo +stable fmt --all -- --check` clean (no Rust changes; gate runs as a no-op)
- `npm run check` 0 errors (no Svelte changes; gate runs as a no-op)
- `bash scripts/mvp_smoke.sh` not gated for P0 (no behavior change)
- Manual diff review against the spec carrier's "Versioning" section to confirm the carrier pattern is followed

### P1.6.1: Substrate (3 slices)

Goal: lock the data model, the generated manifest, and the Tauri command surface. No UI changes in this milestone.

#### P1.6.1.1: Library / framework spike

Status: ✅ Shipped (PR #410, commit TBD; lands in CR-10 Slice P1.6.1.1).

Goal: pick the free-form canvas library (open decision OD-CR-10-2).

Options evaluated:

- Roll a small SVG helper (zero deps, hand-rolled drag + bezier)
- `@xyflow/svelte` (svelte-flow; ~30 KB gzipped; mature)
- `rete.js` v2 (canvas-based; would need a Svelte adapter — eliminated early)

Decision: hand-roll a small SVG helper. ADR-0018 records the decision. Benchmark at 50/100/200 nodes in `src/lib/__tests__/spike-node-renderer-benchmark.test.ts` shows the hand-rolled approach meets the 60 Hz budget comfortably (jsdom cost is ~5x Chromium; Chromium-realistic numbers are ~2 ms per 100 nodes). The AstroForge pipeline graph is bounded (~20 stages for a single project; the spike's 200-node ceiling is a 10x safety margin). `@xyflow/svelte`'s strengths (minimap, autolayout, 1000+ node support) are not needed at current scale; the marginal complexity cost (Svelte 5 reactivity workarounds + dependency on a third-party library) outweighs the marginal development-time savings.

#### P1.6.1.2: NodeCatalog data type + generated manifest

Goal: introduce `NodeCatalog` as a first-class Rust type and emit the tooltip manifest at build time.

Implementation:

- New `crates/astroforge-core/src/node_catalog.rs` with `NodeCatalog`, `NodeDefinition`, `SubFeature`, `NodeTaxonomyMap` types.
- `build.rs` script that parses the `*Params` doc-comments from `dispatch.rs` and emits `crates/astroforge-core/src/node_catalog.json`.
- The 12 user-facing node types → 10 engine-dispatched mapping per §7.7.9.
- Unit tests for `NodeCatalog::resolve(user_facing_type) -> engine_type` covering every entry in the mapping table.
- Integration test that the generated JSON parses cleanly and round-trips through `serde_json`.

Files touched:

- `crates/astroforge-core/src/node_catalog.rs` (new)
- `crates/astroforge-core/src/lib.rs` (`pub mod node_catalog;`)
- `crates/astroforge-core/build.rs` (new)
- `crates/astroforge-core/tests/node_catalog_test.rs` (new)

#### P1.6.1.3: Tauri command surface for read-only access

Goal: expose `get_node_catalog` to the frontend so the palette can render.

Implementation:

- New `src-tauri/src/commands_node_graph.rs` with the `get_node_catalog` command (returns the generated JSON + the taxonomy map).
- Register the command in `main.rs::invoke_handler`.
- Frontend wrapper in `src/lib/astroforge-api.ts`.

Files touched:

- `src-tauri/src/commands_node_graph.rs` (new)
- `src-tauri/src/main.rs` (register command)
- `src/lib/astroforge-api.ts` (wrapper)

### P1.6.2: Palette + Constrained Layout (4 slices)

Goal: ship the NodePalette component + extend `NodeSidebar.svelte` for the constrained layout, plus the Wizard/Node mode toggle.

#### P1.6.2.1: `NodePalette.svelte` skeleton + IPC

Goal: render the palette grouped by the six engine categories.

Implementation:

- New `src/components/NodePalette.svelte` consuming `get_node_catalog`.
- Six collapsible groups (Input / Calibration / Calibration-Free / Stacking / Stretch / Refinement / Output).
- Search field at the top filtering by label or hint substring.
- Click-to-insert stub (does not yet write to the graph; logs the selected type).

Files touched:

- `src/components/NodePalette.svelte` (new)

#### P1.6.2.2: Palette mount + search field wiring

Goal: mount `NodePalette` in `ProcessWorkspace.svelte` and wire the search filter.

Implementation:

- `ProcessWorkspace.svelte` mounts `NodePalette` when the session is in Node mode (read from a new `nodeGraphMode` writable store).
- Search field filters the palette entries client-side.

Files touched:

- `src/components/ProcessWorkspace.svelte` (mount palette)
- `src/lib/node-graph-store.ts` (new: `nodeGraphMode` writable)

#### P1.6.2.3: Mode toggle (Wizard / Node)

Goal: top-bar toggle between Wizard and Node modes.

Implementation:

- `App.svelte` adds the toggle to the existing top navigation bar.
- Persistence: `localStorage` key `astroforge.session.mode` (default `wizard`).
- The toggle writes to `nodeGraphMode`; `ProcessWorkspace` reacts.

Files touched:

- `src/App.svelte` (toggle + persistence)

#### P1.6.2.4: Constrained layout + per-node tooltip wiring

Goal: `NodeSidebar.svelte` shows the active graph in constrained mode, with tooltips sourced from the generated manifest.

Implementation:

- Extend `NodeSidebar.svelte` to read the active `PipelinePlan` and render each `PipelineStage` as a node card with the existing status LED + label.
- Hover tooltip reads the first 3 to 7 words of `description` from the catalog.
- Insert button on each palette entry writes through a new `insert_stage` IPC (registered in this slice).

Files touched:

- `src/components/NodeSidebar.svelte` (extend)
- `src/components/NodePalette.svelte` (wire insert button)
- `src-tauri/src/commands_node_graph.rs` (add `insert_stage`)
- `src-tauri/src/main.rs` (register)
- `src/lib/astroforge-api.ts` (wrapper)
- `crates/astroforge-core/src/node_graph/mutation.rs` (new)
- `crates/astroforge-core/src/pipeline_plans_store.rs` (extend)
- `crates/astroforge-core/tests/node_graph_mutation_test.rs` (new)

### P1.6.3: Mutation + Free-Form Layout (4 slices)

Goal: ship per-stage mutation (enable / disable / reorder / remove) and the free-form draggable canvas mode.

#### P1.6.3.1: `update_graph` IPC + mutation primitives

Goal: a single write-path IPC for graph state, with in-process primitives for testing.

Implementation:

- New IPC `update_graph(plan_id, graph_state)` that takes the full graph state (nodes + edges + sequence + enabled flags) and returns the persisted `PipelinePlan`.
- `mutation.rs` exposes `insert_stage`, `remove_stage`, `reorder_stage`, `set_stage_enabled` as in-process primitives consumed by tests and the IPC.

Files touched:

- `src-tauri/src/commands_node_graph.rs` (add `update_graph`)
- `crates/astroforge-core/src/node_graph/mutation.rs` (primitives)

#### P1.6.3.2: Mutation UI in NodeSidebar

Goal: enable / disable / reorder / remove through the constrained layout.

Implementation:

- Right-click context menu in `NodeSidebar.svelte` adds Re-run from here, Disable, Insert before, Insert after, Move up, Move down, Reset to default params, Remove (disabled for required stages).
- Each action writes through `update_graph`.
- `DestructiveConfirmDialog.svelte` fires when the action targets a `DESTRUCTIVE_STAGES` entry.

Files touched:

- `src/components/NodeSidebar.svelte` (context menu)
- `src/components/DestructiveConfirmDialog.svelte` (extend if needed)

#### P1.6.3.3: Free-form canvas + layout-mode toggle

Goal: ship the draggable SVG canvas mode with bezier wires + pan/zoom.

Implementation:

- Extend `NodeSidebar.svelte` with the free-form mode (using the library/framework chosen in P1.6.1.1).
- Layout-mode toggle in the top bar (same location as the verbosity preference).
- `PipelineStage.sequence` is derived from the topological order of the edges on every layout change.

Files touched:

- `src/components/NodeSidebar.svelte` (free-form mode)
- `src/App.svelte` (layout-mode toggle + persistence)

#### P1.6.3.4: Parameter panel consumes generated sub_features

Goal: `ParameterSidebar.svelte` shows hint per parameter, sourced from the generated manifest.

Implementation:

- `ParameterSidebar.svelte` reads the selected node's `sub_features` from the catalog and renders a hint badge per parameter.
- The hint is a 5 to 12 word summary derived from the doc-comment.

Files touched:

- `src/components/ParameterSidebar.svelte` (extend)

### P1.6.4: Recipe Round-Trip + Per-Image-Set + Three New Stages (5 slices)

Goal: ship Graph ↔ Recipe JSON round-trip, per-project default graph + per-session overrides, and the three new engine stages (`crop`, `star_handling`, `creative_polish`).

#### P1.6.4.1: `graph_to_recipe` IPC + serialization

Goal: serialize the active `PipelinePlan` to `.astroforge-recipe` JSON.

Implementation:

- New IPC `graph_to_recipe(plan_id)` returning the JSON string.
- `crates/astroforge-core/src/node_graph/recipe_io.rs` (new): `Graph::to_recipe_json()` + sanitize per §11.2.

Files touched:

- `crates/astroforge-core/src/node_graph/recipe_io.rs` (new)
- `src-tauri/src/commands_node_graph.rs` (add `graph_to_recipe`)
- `src/lib/astroforge-api.ts` (wrapper)
- `crates/astroforge-core/tests/recipe_io_test.rs` (new)

#### P1.6.4.2: `recipe_to_graph` IPC + deserialization

Goal: parse a `.astroforge-recipe` JSON into a `PipelinePlan`.

Implementation:

- New IPC `recipe_to_graph(recipe_json)` returning the materialized `PipelinePlan`.
- Unknown `stage_type` values surface as a diagnostic with the offending entry's index.
- The IPC does NOT auto-apply; the user is prompted to either replace the active graph or create a new session.

Files touched:

- `crates/astroforge-core/src/node_graph/recipe_io.rs` (extend)
- `src-tauri/src/commands_node_graph.rs` (add `recipe_to_graph`)

#### P1.6.4.3: Per-project default + per-session overrides

Goal: a project carries one default graph; each session can override.

Implementation:

- New IPCs `list_project_graphs`, `get_session_graph_override`, `set_session_graph_override`, `reset_session_to_project_default`.
- "Reset to project default" toolbar button in `NodeSidebar.svelte`.

Files touched:

- `src-tauri/src/commands_node_graph.rs` (add 4 IPCs)
- `src/components/NodeSidebar.svelte` (toolbar button)

#### P1.6.4.4: Three new engine stages: `crop`, `star_handling`, `creative_polish`

Goal: implement the three new `StageHandler` structs required by the user-facing taxonomy.

Implementation:

- `crates/astroforge-core/src/pipeline_plan/dispatch.rs`: add `CropHandler`, `StarHandlingHandler`, `CreativePolishHandler` + their `*Params` structs with documented fields.
- Register the three new types in `PIPELINE_STAGES` and the `PipelineStageType` union.
- Update `node_catalog.rs` with the three new mappings.

Files touched:

- `crates/astroforge-core/src/pipeline_plan/dispatch.rs` (3 handlers + 3 *Params)
- `src/lib/pipeline-store.ts` (extend `PIPELINE_STAGES` + `PipelineStageType`)
- `crates/astroforge-core/src/node_catalog.rs` (extend mapping table)

#### P1.6.4.5: Acceptance pass + audit refresh

Goal: walk the 15 acceptance criteria from CR-10 §5 and produce the CR-10-AUDIT.md doc.

Implementation:

- For each acceptance criterion, mark Pass / Fail / Partial / Missing.
- For Partial / Missing, propose the follow-on slice or carry-over to a future CR.

Files touched:

- `docs/CR-10-AUDIT.md` (new)
- `docs/specs/SPEC_INDEX.md` (extend)

## Total PR count

| Milestone | Slices | PRs |
|---|---|---|
| P0 (docs-only) | 1 | 1 |
| P1.6.1 (substrate) | 3 | 3 |
| P1.6.2 (palette + constrained) | 4 | 4 |
| P1.6.3 (mutation + free-form) | 4 | 4 |
| P1.6.4 (recipe + per-image-set + 3 new stages) | 5 | 5 |
| **Total** | **17** | **17** |

## Estimated scope

Per the CR-09 implementation-plan conventions, slices are sized at ~200-500 net lines each. The 17-slices total lands roughly:

- 2,500 LOC for `crates/astroforge-core/src/node_catalog.rs` + `node_graph/` + the 3 new `StageHandler` structs (~1,200 net lines)
- 1,800 LOC for `src-tauri/src/commands_node_graph.rs` + `main.rs` registration (~400 net lines)
- 3,200 LOC for `src/lib/node-graph-store.ts` + 4 component extensions (~1,500 net lines)
- 1,500 LOC for tests in `crates/astroforge-core/tests/` (~900 net lines)
- 1,200 LOC for docs (CR-10 + PLAN + AUDIT + spec delta)

**Total: ~10,200 LOC over 17 PRs.**

## Verification gates (per slice)

Per the `astroforge-cr-slices` shared gates:

- `cargo +stable fmt --all -- --check` clean
- `cargo +stable clippy --workspace --all-targets -- -D warnings` clean
- `cargo +stable test --workspace` (or `--test <file>` for slice-local)
- `npm run check` 0 errors
- `npm run build` succeeded
- `bash scripts/mvp_smoke.sh tests/fixtures/sample-session` passed

**Local src-tauri build is gated on `javascriptcoregtk-4.1` not being installed in this environment; CI provides the system library and is the authoritative build gate for the Tauri binary.**

## Pitfalls (carried from `astroforge-cr-slices`)

- Match the Svelte event syntax to the file being edited (some files use `on:click`, others `onclick`). Pre-emptive check: `grep -nE 'on:click|onclick' src/components/<file>.svelte` before adding a new handler.
- Test the IPC boolean contract on the store layer, not the IPC. New behavioral tests live in `crates/astroforge-core/tests/`, not `src-tauri/src/commands_node_graph.rs` (the latter is CI-dead).
- `DomainStore::record_event` takes `payload_json` as the 3rd argument, not `session_id`. Any new IPC that records `GraphUpdated` events follows the existing pattern.
- A Tauri command can join across multiple `State` types (`State<'_, ProjectState>` + `State<'_, RecipeState>`); the shape that works when the first leg returns `serde_json::Value` is round-tripping `serde_json::from_value(serde_json::json!({"field": value.get("field")}))` to keep the typed surface at the second leg.
- SQLite has no `ALTER TABLE ADD COLUMN IF NOT EXISTS`: gate every additive migration with `pragma_table_info` in `pipeline_plans_store.rs::run_migrations`.

End of plan.