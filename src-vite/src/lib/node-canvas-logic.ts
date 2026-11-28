// CR-10 P1.6.3.3 — pure geometry for the free-form node canvas.
//
// Per the P1.6.1.1 decision (ADR-0018) the canvas is a hand-rolled
// SVG helper: no third-party graph library, drag + bezier + pan/zoom
// computed here. The pipeline graph is a linear sequence (bounded ~20
// stages), so the default layout is a vertical column and wires
// connect consecutive stages in sequence order.

export interface CanvasNode {
  stage_id: string;
  label: string;
  stage_type: string;
  enabled: boolean;
  /** User-arranged position (canvas coordinates). Absent = the
   * default layout position. */
  x: number;
  y: number;
}

export interface Point {
  x: number;
  y: number;
}

/** Node card size (canvas units — the view transform scales it). */
export const NODE_WIDTH = 170;
export const NODE_HEIGHT = 52;
/** Vertical gap between rows in the default layout. */
export const ROW_GAP = 34;
/** Canvas margin for the default column. */
export const LAYOUT_MARGIN = 40;

/**
 * The default free-form layout: a vertical column in sequence order.
 * Nodes without a user-arranged position get `colX` + row spacing;
 * arranged nodes keep their saved position.
 */
export function defaultLayout(
  stages: { stage_id: string; label: string; stage_type: string; enabled: boolean }[],
  positions: Record<string, Point>,
  colX = LAYOUT_MARGIN,
): CanvasNode[] {
  let row = 0;
  return stages.map((s) => {
    const saved = positions[s.stage_id];
    const node: CanvasNode = {
      ...s,
      x: saved?.x ?? colX,
      y: saved?.y ?? LAYOUT_MARGIN + row * (NODE_HEIGHT + ROW_GAP),
    };
    if (!saved) row += 1;
    return node;
  });
}

/**
 * The cubic bezier wire between two consecutive nodes: leaves the
 * source's bottom-center and enters the target's top-center, with
 * control points at the vertical midpoint so short gaps stay smooth
 * and long gaps curve gently.
 */
export function bezierPath(from: CanvasNode, to: CanvasNode): string {
  const x1 = from.x + NODE_WIDTH / 2;
  const y1 = from.y + NODE_HEIGHT;
  const x2 = to.x + NODE_WIDTH / 2;
  const y2 = to.y;
  const mid = (y1 + y2) / 2;
  return `M ${x1} ${y1} C ${x1} ${mid}, ${x2} ${mid}, ${x2} ${y2}`;
}

/** SVG transform for the current pan/zoom view. */
export function viewTransform(pan: Point, zoom: number): string {
  return `translate(${pan.x} ${pan.y}) scale(${zoom})`;
}

/** Convert a screen (viewport) point to canvas coordinates. */
export function screenToWorld(px: number, py: number, pan: Point, zoom: number): Point {
  return {
    x: (px - pan.x) / zoom,
    y: (py - pan.y) / zoom,
  };
}

/** Zoom clamps — 0.5x to 2x keeps node text legible at both ends. */
export function clampZoom(zoom: number): number {
  return Math.min(2, Math.max(0.5, zoom));
}

/**
 * Keep a node's position inside the canvas bounds (with the node's
 * own size accounted for): never fully off-canvas.
 */
export function clampNodePos(x: number, y: number, boundsW: number, boundsH: number): Point {
  return {
    x: Math.min(Math.max(x, -NODE_WIDTH / 2), Math.max(boundsW - NODE_WIDTH / 2, 0)),
    y: Math.min(Math.max(y, -NODE_HEIGHT / 2), Math.max(boundsH - NODE_HEIGHT / 2, 0)),
  };
}
