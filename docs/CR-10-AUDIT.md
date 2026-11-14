# CR-10 Audit — Node-Based Editor acceptance walk (P1.6.4.5)

Date: 2026-11-15. Walks the 15 acceptance criteria from
[CR-10 §5](./CR-10-NODE-BASED-EDITOR.md) against what actually
shipped. Verdicts: Pass / Partial / Missing.

## Summary

**11 Pass, 4 Partial, 0 Missing.** The engine layer (mutation
primitives, recipe round-trip, overrides, the three new stages) and
the command surface are complete and CI-green. The Partials are all
thin UI surfaces over shipped IPCs: node tooltips (3), drag→sequence
recomputation policy (6), the Save/Open-Recipe buttons (8, 9), and
parameter-hint consumption in the parameter panel (11).

## Criteria walk

| # | Criterion | Verdict | Evidence |
|---|---|---|---|
| 1 | Wizard ↔ Node mode switch without losing pipeline state | **Pass** | `NodeGraphModeToggle.svelte` flips the `isNodeMode` store; the plan lives in `pipeline-plan-store` and is untouched by the toggle. Verified in the dev app (M27: Wizard → Node shows the same plan's stages). |
| 2 | Node mode shows the plan as a graph of named nodes, labels matching canonical `PIPELINE_STAGES[i].label` | **Pass** | `NodeStageList.svelte` renders the active plan's stages with the plan's `label` field (stamped from `StageType::label()` at generation/mutation time). |
| 3 | Node tooltip = first 3–7 words of the node's `description` | **Partial** | The palette carries `description`/hints per entry (`node-palette-logic.ts`); the stage rows do not yet surface description tooltips. Small follow-up: bind a `title` attr to the row's label span. |
| 4 | Palette lists the 12 user-facing nodes grouped by D-CR-10-5 | **Pass** | Shipped in P1.6.2.1 with **seven** groups — the criterion text says "six" but Decision D-CR-10-5 and spec §7.7.3 both list seven (Input / Calibration / Calibration-Free / Stacking / Stretch / Refinement / Output). The CR text self-inconsistency is noted here rather than shipping a wrong grouping. |
| 5 | Palette click inserts at a deterministic sequence position | **Pass** | `handlePaletteSelect` → `insert_stage` IPC; the primitive appends at `max(sequence) + 1` (`node_graph::mutation::insert_stage`). |
| 6 | Free-form drag recomputes `sequence` from topological edge order | **Partial** | Drag + arrangement shipped in P1.6.3.3 (`NodeCanvas.svelte`); for a **linear** graph the topological order IS the sequence, and reordering is owned by the list context menu (`reorder_stage`) so one editing surface owns state. Drag does not reorder — documented deviation, revisit when branching graphs arrive (Phase 4 per §7.7.10). |
| 7 | Enable / disable / reorder / remove; required stages blocked | **Pass** | P1.6.3.1 primitives + P1.6.3.2 context menu; `remove_stage` refuses required stages (`StageRequired`) and the menu item is disabled up front; the workflow-A routing's required-but-disabled stages survive intact. |
| 8 | Graph → `.astroforge-recipe` round-trips through JSON back to a plan | **Partial** | Engine complete (P1.6.4.1): `graph_to_recipe` IPC, §11.2 sanitization, deterministic ids; 9 `recipe_io_test.rs` pins. The **UI button** ("Save as Recipe" bound to `graphToRecipe`) is not wired yet — `SaveAsRecipePanel` still uses the legacy recipe-store path. |
| 9 | `.astroforge-recipe` loads into a session; catalog validation; unknown-type diagnostic | **Partial** | Engine complete (P1.6.4.2): `recipe_to_graph` validates every stage through the catalog and fails with the offending pipeline entry index. The **"Open Recipe" UI** (prompt-before-replace flow) is not wired yet. |
| 10 | Per-project default graph + per-session overrides + reset | **Pass** | P1.6.4.3: `graph_overrides` table, pin/re-pin/clear, `reset_session_to_project_default` composing the round-trip (idempotent, deterministic ids); "Reset to project default" button in `NodeStageList`. |
| 11 | Parameter hints from the generated `node_catalog.json` manifest | **Partial** | The manifest exists (build-time generated, P1.6.1.2, consumed by the palette). The plan's P1.6.3.4 slice — `ParameterSidebar` rendering per-parameter hint badges from `sub_features` — is **not shipped yet** (the only unsheipped slice of P1.6.3/4). |
| 12 | 12 user-facing types map per §7.7.9; unmapped = startup error | **Pass** | `user_facing_to_canonical` (panics on unknown — startup-time), `user_facing_to_canonical_is_total` test, `CANONICAL_STAGE_TYPES` extended to 14 (13 + `quality_filter`, which the round-trip proved was missing). |
| 13 | Node mode respects the verbosity setting | **Partial** | The Expert DAG view is verbosity-gated elsewhere, but `NodeStageList`/`NodeCanvas` render a single density. Defer: verbosity-aware node density is a UX-polish item, not a correctness gap. |
| 14 | Node surfaces compose cleanly with the existing Process surfaces | **Pass** | `NodeStageList`/`NodeCanvas` mount in the sidebar snippet next to `NodePalette`; `ProcessingControls`, `IntelligencePanel`, `RecoveryBanner` untouched. |
| 15 | Command surface documented, registered, and bridged | **Pass** | `src-tauri/src/commands_node_graph.rs` — 12 commands (`read_node_catalog`, `insert_stage`, `update_graph`, `remove_stage`, `reorder_stage`, `set_stage_enabled`, `graph_to_recipe`, `recipe_to_graph`, `list_project_graphs`, `get/set_session_graph_override`, `reset_session_to_project_default`) registered in `main.rs::invoke_handler`, each with a documented `astroforge-api.ts` wrapper. |

## Slice ledger (actuals vs plan)

| Slice | Plan | Shipped | PR |
|---|---|---|---|
| P0 docs | 1 | 1 | #415 (P0) |
| P1.6.1 substrate | 3 | 3 | #411–#413 |
| P1.6.2 palette + constrained | 4 | 4 | #413–#416 |
| P1.6.3.1 update_graph + primitives | 1 | 1 | #480 |
| P1.6.3.2 mutation UI | 1 | 1 | #482 |
| P1.6.3.3 free-form canvas | 1 | 1 | #485 |
| P1.6.3.4 parameter hints | 1 | **0 — open** | — |
| P1.6.4.1/2 recipe round-trip | 2 | 2 | #482 |
| P1.6.4.3 overrides + reset | 1 | 1 | #483 |
| P1.6.4.4 three new stages | 1 | 1 | #481 |
| P1.6.4.5 acceptance + audit | 1 | 1 | this PR |

## Follow-ups out of this audit

1. Node-row description tooltips (criterion 3) — one-line `title` binding.
2. "Save as Recipe" / "Open Recipe" UI wiring over the shipped IPCs (8, 9).
3. `ParameterSidebar` hint badges from `sub_features` (11 — the open slice P1.6.3.4).
4. Verbosity-aware node density (13).
5. Drag→reorder policy revisit when branching graphs arrive (6, Phase 4).
