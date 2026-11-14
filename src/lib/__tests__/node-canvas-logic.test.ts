// CR-10 P1.6.3.3 — geometry pins for the free-form canvas helper
// (ADR-0018: hand-rolled SVG; these tests lock the math the
// component renders).

import { describe, expect, it } from "vitest";
import {
  bezierPath,
  clampNodePos,
  clampZoom,
  defaultLayout,
  NODE_HEIGHT,
  NODE_WIDTH,
  ROW_GAP,
  screenToWorld,
  viewTransform,
  type Point,
} from "../node-canvas-logic";

const stage = (id: string) => ({
  stage_id: id,
  label: id,
  stage_type: "stretch",
  enabled: true,
});

describe("defaultLayout", () => {
  it("stacks unarranged nodes vertically in sequence order", () => {
    const nodes = defaultLayout([stage("a"), stage("b"), stage("c")], {});
    expect(nodes[0].y).toBeLessThan(nodes[1].y);
    expect(nodes[1].y).toBeLessThan(nodes[2].y);
    expect(nodes[1].y - nodes[0].y).toBe(NODE_HEIGHT + ROW_GAP);
    expect(nodes.every((n) => n.x === nodes[0].x)).toBe(true);
  });

  it("keeps user-arranged positions and does not shift later rows", () => {
    const positions = { b: { x: 300, y: 40 } };
    const nodes = defaultLayout([stage("a"), stage("b"), stage("c")], positions);
    expect(nodes[1].x).toBe(300);
    expect(nodes[1].y).toBe(40);
    // "c" takes row 1 (the first free row), not row 2 — arranged
    // nodes do not consume a default row.
    expect(nodes[2].y).toBe(nodes[0].y + NODE_HEIGHT + ROW_GAP);
  });
});

describe("bezierPath", () => {
  it("runs from the source's bottom-center to the target's top-center", () => {
    const [a, b] = defaultLayout([stage("a"), stage("b")], {});
    const d = bezierPath(a, b);
    expect(d).toContain(`M ${a.x + NODE_WIDTH / 2} ${a.y + NODE_HEIGHT}`);
    expect(d.endsWith(`${b.x + NODE_WIDTH / 2} ${b.y}`)).toBe(true);
  });
});

describe("view + pointer math", () => {
  it("viewTransform carries pan and zoom in SVG order", () => {
    const pan: Point = { x: 12, y: 34 };
    expect(viewTransform(pan, 1.25)).toBe("translate(12 34) scale(1.25)");
  });

  it("screenToWorld inverts the view transform", () => {
    const pan: Point = { x: 40, y: 80 };
    const zoom = 1.5;
    const world = screenToWorld(100, 160, pan, zoom);
    expect(world.x).toBeCloseTo((100 - 40) / 1.5);
    expect(world.y).toBeCloseTo((160 - 80) / 1.5);
  });

  it("clamps zoom to the legible band", () => {
    expect(clampZoom(0.1)).toBe(0.5);
    expect(clampZoom(1)).toBe(1);
    expect(clampZoom(9)).toBe(2);
  });
});

describe("clampNodePos", () => {
  it("keeps nodes at least half-visible on every edge", () => {
    expect(clampNodePos(-999, -999, 800, 600).x).toBe(-NODE_WIDTH / 2);
    expect(clampNodePos(5000, 5000, 800, 600).x).toBe(800 - NODE_WIDTH / 2);
    expect(clampNodePos(10, 10, 800, 600)).toEqual({ x: 10, y: 10 });
  });
});
