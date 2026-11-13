// CR-10 P1.6.3.2 — pins for the stage-list view model and the
// context-menu action model (pure module; no Svelte boot).

import { describe, expect, it } from "vitest";
import {
  menuActionsFor,
  stageNodesFromPlan,
  type StageNodeView,
} from "../node-stage-list-logic";
import type { PipelinePlanDto, PipelineStageDto } from "../astroforge-api";

function stage(overrides: Partial<PipelineStageDto>): PipelineStageDto {
  return {
    stage_id: "stage_x",
    plan_id: "plan_1",
    stage_type: "stretch",
    sequence: 0,
    label: "Stretch",
    required: false,
    enabled: true,
    parameters_json: null,
    produces_image_version: true,
    undo_supported: true,
    ...overrides,
  };
}

function plan(stages: PipelineStageDto[]): PipelinePlanDto {
  return {
    plan_id: "plan_1",
    project_id: "proj_1",
    session_id: "sess_1",
    recipe_id: null,
    mode: "auto",
    target_type: "NEBULA",
    status: "ready",
    created_at: "2026-11-13T00:00:00Z",
    schema_version: 1,
    stages,
  };
}

describe("stageNodesFromPlan", () => {
  it("sorts by sequence regardless of submission order", () => {
    const nodes = stageNodesFromPlan(
      plan([
        stage({ stage_id: "b", sequence: 2, stage_type: "denoise", label: "Denoise" }),
        stage({ stage_id: "a", sequence: 0, stage_type: "crop", label: "Crop" }),
        stage({ stage_id: "c", sequence: 1, stage_type: "stretch", label: "Stretch" }),
      ]),
    );
    expect(nodes.map((n) => n.stage_id)).toEqual(["a", "c", "b"]);
  });

  it("flags destructive canonical types", () => {
    const nodes = stageNodesFromPlan(
      plan([
        stage({ stage_id: "crop", stage_type: "crop" }),
        stage({ stage_id: "stretch", stage_type: "stretch" }),
        stage({ stage_id: "color", stage_type: "color" }),
        stage({ stage_id: "stack", stage_type: "stack" }),
        stage({ stage_id: "detail", stage_type: "detail" }),
      ]),
    );
    const byId = Object.fromEntries(nodes.map((n) => [n.stage_id, n]));
    expect(byId["crop"].destructive).toBe(true);
    expect(byId["color"].destructive).toBe(true);
    expect(byId["detail"].destructive).toBe(true);
    expect(byId["stretch"].destructive).toBe(false);
    expect(byId["stack"].destructive).toBe(false);
  });
});

describe("menuActionsFor", () => {
  const row = (over: Partial<StageNodeView> = {}): StageNodeView => ({
    stage_id: "s",
    sequence: 1,
    label: "Stretch",
    stage_type: "stretch",
    enabled: true,
    required: false,
    destructive: false,
    ...over,
  });

  it("offers toggle with the opposite label", () => {
    const actions = menuActionsFor(row({ enabled: true }), 1, 3);
    const toggle = actions.find((a) => a.kind === "toggle-enabled");
    expect(toggle?.label).toBe("Disable");
    expect(toggle?.disabled).toBe(false);

    const off = menuActionsFor(row({ enabled: false }), 1, 3);
    expect(off.find((a) => a.kind === "toggle-enabled")?.label).toBe("Enable");
  });

  it("disables move-up on the first row and move-down on the last", () => {
    const first = menuActionsFor(row(), 0, 3);
    expect(first.find((a) => a.kind === "move-up")?.disabled).toBe(true);
    expect(first.find((a) => a.kind === "move-down")?.disabled).toBe(false);

    const last = menuActionsFor(row(), 2, 3);
    expect(last.find((a) => a.kind === "move-up")?.disabled).toBe(false);
    expect(last.find((a) => a.kind === "move-down")?.disabled).toBe(true);
  });

  it("disables Remove for required stages", () => {
    const actions = menuActionsFor(row({ required: true }), 1, 3);
    expect(actions.find((a) => a.kind === "remove")?.disabled).toBe(true);
    const optional = menuActionsFor(row({ required: false }), 1, 3);
    expect(optional.find((a) => a.kind === "remove")?.disabled).toBe(false);
  });

  it("returns the actions in display order", () => {
    const kinds = menuActionsFor(row(), 1, 3).map((a) => a.kind);
    expect(kinds).toEqual(["toggle-enabled", "move-up", "move-down", "remove"]);
  });
});
