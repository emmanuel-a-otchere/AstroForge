// CR-10 P1.6.3.2 — pure logic for the Node-mode stage list.
//
// Mirrors the P1.6.2.1 pattern (node-palette-logic.ts): the view model
// and the context-menu action model live in a pure module so vitest
// can pin the contract without booting Svelte. The component renders
// the derived state and dispatches the actions through the mutation
// IPCs (P1.6.3.1: set_stage_enabled / reorder_stage / remove_stage /
// update_graph).
//
// Menu actions per the plan §P1.6.3.2: Enable/Disable, Move up, Move
// down, Remove (refused for required stages — the backend rejects it,
// so the menu item is disabled). "Re-run from here" and "Reset to
// default params" are P6.1c / parameter-panel work (no IPC yet) and
// are intentionally absent rather than silently no-op.

import type { PipelinePlanDto, PipelineStageDto } from "./astroforge-api";

/** Canonical stage types whose commit is destructive (info loss or
 * irreversible pixel transform). Mirrors DESTRUCTIVE_STAGES in
 * pipeline-store.ts — which holds the USER-FACING types — mapped
 * through the §7.7.9 canonical routing: crop_rotate→crop,
 * background_extraction→background, color_calibration→color,
 * sharpen_deconvolution→detail, denoise→denoise. */
export const DESTRUCTIVE_CANONICAL_STAGES: ReadonlySet<string> = new Set([
  "crop",
  "background",
  "color",
  "detail",
  "denoise",
]);

export interface StageNodeView {
  stage_id: string;
  sequence: number;
  label: string;
  stage_type: string;
  enabled: boolean;
  required: boolean;
  /** True when the stage's canonical type is destructive — the
   * Remove action confirms through DestructiveConfirmDialog. */
  destructive: boolean;
}

/** Map the plan's stages to view models, sorted by sequence. */
export function stageNodesFromPlan(plan: PipelinePlanDto): StageNodeView[] {
  return [...plan.stages]
    .sort((a, b) => a.sequence - b.sequence)
    .map((s) => stageNodeFromDto(s));
}

function stageNodeFromDto(s: PipelineStageDto): StageNodeView {
  return {
    stage_id: s.stage_id,
    sequence: s.sequence,
    label: s.label,
    stage_type: s.stage_type,
    enabled: s.enabled,
    required: s.required,
    destructive: DESTRUCTIVE_CANONICAL_STAGES.has(s.stage_type),
  };
}

export type StageMenuActionKind =
  | "toggle-enabled"
  | "move-up"
  | "move-down"
  | "remove";

export interface StageMenuAction {
  kind: StageMenuActionKind;
  label: string;
  disabled: boolean;
}

/** The context-menu actions for a stage row, in display order, with
 * availability computed from the row's position and flags. */
export function menuActionsFor(
  stage: StageNodeView,
  index: number,
  total: number,
): StageMenuAction[] {
  return [
    {
      kind: "toggle-enabled",
      label: stage.enabled ? "Disable" : "Enable",
      disabled: false,
    },
    {
      kind: "move-up",
      label: "Move up",
      disabled: index <= 0,
    },
    {
      kind: "move-down",
      label: "Move down",
      disabled: index < 0 || index >= total - 1,
    },
    {
      kind: "remove",
      label: "Remove",
      // Required stages refuse removal at the backend too (the
      // recipe audit trail stays intact); the menu communicates
      // the rule up front.
      disabled: stage.required,
    },
  ];
}
