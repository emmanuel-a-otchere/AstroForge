// ADR-0018: CR-10 §7.7 Free-form canvas library
// decision
//
// Status: Proposed (lands via PR #410, CR-10
// Slice P1.6.1.1)
// Date: 2026-09-29
// Resolves: OD-CR-10-2
//
// Context. CR-10 establishes a node-based editor
// over the AstroForge processing pipeline. The
// editor has two layout modes: a constrained
// vertical DAG (the existing `NodeSidebar.svelte`
// pattern, 326 lines, straight `<line>`
// connectors) and a free-form draggable canvas
// for advanced users (P1.6.3 work).
//
// The free-form canvas needs:
// 1. Pan + zoom via mouse wheel + drag.
// 2. Drag-to-move nodes with position
//    interpolation.
// 3. Drag-to-connect (click a node handle, drag
//    to another node's handle, drop to create
//    an edge).
// 4. Multi-select with marquee.
// 5. Per-node tooltips + parameter panel.
//
// OD-CR-10-2 asks: adopt `@xyflow/svelte` (the
// Svelte port of React Flow, a mature graph
// library) or hand-roll a small SVG helper
// matching the existing `NodeSidebar.svelte`
// pattern?
//
// Decision. **Hand-roll a small SVG helper.** The
// AstroForge graph is bounded (a single project
// graph is at most ~20 stages; the spike benchmark
// at 200 nodes is a 10x ceiling, not a target).
// `@xyflow/svelte` would add ~30 KB gz to the
// production bundle for CRUD + pan/zoom + selection
// primitives we would otherwise reuse from the
// existing constrained DAG. The marginal complexity
// cost (learning the library's node/edge/handle
// abstractions, working around Svelte 5 reactivity
// differences, debugging pan/zoom edge cases)
// outweighs the marginal development-time savings.
//
// The spike's benchmark (`src/lib/__tests__/spike-node-renderer-benchmark.test.ts`)
// shows the hand-rolled approach meets the 60 Hz
// budget comfortably:
//
// | N nodes | hand-rolled render time (jsdom) | @xyflow/svelte estimate |
// |---|---|---|
// | 50 | 9.0 ms/frame | ~12 ms/frame (incl. d3-zoom bookkeeping) |
// | 100 | 9.2 ms/frame | ~13 ms/frame |
// | 200 | 18.1 ms/frame | ~22 ms/frame |
// | mutation cycle (clear+render 100) | 8.9 ms | ~14 ms |
//
// jsdom is ~5x slower than Chromium for SVG element
// creation; the Chromium-realistic numbers are ~2 ms
// per 100 nodes for the hand-rolled approach, well
// inside the 16.6 ms / 60 Hz budget.
//
// Consequences.
//
// 1. **Bundle stays small.** No new production
//    dependency. The free-form canvas reuses the
//    `pipeline-store.ts` `PipelineNode` +
//    `PipelineEdge` types and the
//    `NodeSidebar.svelte` SVG-card pattern.
// 2. **Reactivity is consistent.** The hand-rolled
//    helper uses Svelte 5 reactive statements the
//    same way `NodeSidebar.svelte` does. No new
//    mental model for store-to-canvas reactivity.
// 3. **Pan/zoom + marquee are the cost.** A small
//    `panZoom.ts` helper (~80 lines) provides wheel
//    zoom + drag-pan + scale-clamping. Marquee
//    selection is ~40 lines of pointer-event
//    bookkeeping. Drag-to-connect is ~60 lines of
//    pointer-event bookkeeping + a temporary edge
//    overlay. Total: ~180 lines of new code in P1.6.3.
// 4. **Future escape hatch.** If the user base
//    outgrows the constrained-but-free-form model
//    (e.g. wants minimap, autolayout, or 500+
//    nodes), revisiting `@xyflow/svelte` is a
//    localized swap of the canvas component. The
//    `PipelineNode` + `PipelineEdge` types stay.
//    ADR-0018 marks this as a one-way door that
//    is cheap to reverse for the next 18 months.
//
// Alternatives considered.
//
// - **`@xyflow/svelte`**: mature, well-documented,
//    full-featured. Cost: ~30 KB gz production bundle,
//    ~2 weeks of learning curve + Svelte 5 reactivity
//    workarounds, and a dependency on a third-party
//    library whose release cadence we don't control.
//    The library's strengths (minimap, autolayout,
//    1000+ node support) are not needed for the
//    AstroForge use case at current scale.
// - **Konva / Fabric / PixiJS**: 2D-canvas-based,
//    not SVG. Pros: better performance at >500 nodes.
//    Cons: harder to test (no DOM), worse a11y (no
//    inspectable text), and the existing
//    `NodeSidebar.svelte` is SVG. Mixing canvas +
//    SVG in the same project would double the
//    rendering-cost surface.
// - **WebGL graph renderer (e.g. `sigma.js`)**: not
//    warranted; the AstroForge graph is small enough
//    that SVG meets the 60 Hz budget.
//
// Open follow-ons.
//
// - **P1.6.1.2**: NodeCatalog Rust type +
//    generated manifest (the substrate that the
//    free-form canvas consumes).
// - **P1.6.3.3**: free-form canvas implementation
//    (the `panZoom.ts` helper + drag-to-connect +
//    marquee). Estimated ~180 LOC.
// - **P1.6.4.5**: acceptance pass + audit refresh
//    (this ADR gets re-stated with the actual
//    in-production numbers from the canvas).