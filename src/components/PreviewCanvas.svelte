<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { WebGLRenderer, type PreviewParams, type ViewportState } from "../lib/gl-renderer";
  import { previewStore } from "../lib/preview-store";
  import { previewToImageData } from "../lib/pipeline";

  export let params: PreviewParams = {
    blackPoint: 0,
    midtones: 0.25,
    highlights: 1,
    strength: 0,
    scnrStrength: 0,
    scnrMethod: 0,
  };
  export let renderMode: "identity" | "mtf" | "scnr" | "difference" | "composite" = "mtf";
  export let compareMode: boolean = false;
  export let imageData: ImageData | null = null;
  export let floatData: { width: number; height: number; data: Float32Array } | null = null;
  /** Optional session ID — when set, the canvas pulls the matching preview from previewStore. */
  export let sessionId: string | null = null;

  let canvas: HTMLCanvasElement;
  let renderer: WebGLRenderer | null = null;
  let container: HTMLDivElement;
  let isPanning = false;
  let panStart = { x: 0, y: 0 };
  let viewport: ViewportState = { zoom: 1, panX: 0, panY: 0 };
  let showOriginal = false;

  function handleResize() {
    if (!renderer || !container) return;
    const rect = container.getBoundingClientRect();
    renderer.resize(rect.width, rect.height);
    renderer.refit();
    renderer.render(renderMode);
  }

  /**
   * Begin a gesture (pan / wheel) by switching to the reduced-resolution
   * drawing buffer. Spec: P1.5-M2-T8 / #152 — preview at half-res during
   * drag, full res on rest. The buffer flips back to full-res after the
   * debounce window expires (see `endGesture`).
   */
  function beginGesture() {
    if (!renderer) return;
    renderer.setReducedResolution(true);
  }

  /** End a gesture: schedule a full-resolution re-render after a short
   *  debounce. If a new gesture begins before the debounce fires, it
   *  re-cancels itself via `beginGesture` -> `setReducedResolution(true)`
   *  and the timer is implicitly replaced on the next `requestDebouncedRender`. */
  function endGesture() {
    if (!renderer) return;
    renderer.setReducedResolution(false);
    renderer.requestDebouncedRender(renderMode, 150);
  }

  function handleMouseDown(e: MouseEvent) {
    isPanning = true;
    panStart = { x: e.clientX, y: e.clientY };
    beginGesture();
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isPanning || !renderer) return;
    const dx = (e.clientX - panStart.x) / canvas.width;
    const dy = (e.clientY - panStart.y) / canvas.height;
    viewport.panX += dx * 0.5;
    viewport.panY -= dy * 0.5;
    renderer.setViewport(viewport);
    renderer.render(renderMode);
    panStart = { x: e.clientX, y: panStart.y };
    panStart = { x: e.clientX, y: e.clientY };
  }

  function handleMouseUp() {
    if (!isPanning) return;
    isPanning = false;
    endGesture();
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    if (!renderer) return;
    // Each wheel tick begins a brief gesture: reduced-res for the move,
    // full-res 150 ms after the user stops scrolling.
    beginGesture();
    const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
    viewport.zoom = Math.max(0.05, Math.min(50, viewport.zoom * factor));
    renderer.setViewport(viewport);
    renderer.render(renderMode);
    endGesture();
  }

  function handleDoubleClick() {
    if (!renderer) return;
    viewport = { zoom: 1, panX: 0, panY: 0 };
    renderer.setViewport(viewport);
    renderer.refit();
    renderer.render(renderMode);
  }

  function handleCompareDown() {
    showOriginal = true;
    if (renderer) renderer.render("identity");
  }

  function handleCompareUp() {
    showOriginal = false;
    if (renderer) renderer.render(renderMode);
  }

  $: if (renderer && params) {
    renderer.setParams(params);
    renderer.requestDebouncedRender(renderMode);
  }

  $: if (renderer && imageData) {
    renderer.setImageFromImageData(imageData);
    renderer.refit();
    renderer.render(renderMode);
  }

  $: if (renderer && floatData) {
    renderer.setImageData(floatData.width, floatData.height, floatData.data);
    renderer.refit();
    renderer.render(renderMode);
  }

  // Subscribe to the preview store so a freshly-completed pipeline run
  // in ManifestReview feeds straight into the canvas without prop-drilling.
  $: if (renderer && sessionId && $previewStore?.sessionId === sessionId) {
    const data = previewToImageData($previewStore.preview);
    renderer.setImageFromImageData(data);
    renderer.refit();
    renderer.render(renderMode);
  }

  $: if (renderer && compareMode !== undefined) {
    renderer.render(showOriginal ? "identity" : renderMode);
  }

  onMount(() => {
    try {
      renderer = new WebGLRenderer(canvas);
      handleResize();
      const ro = new ResizeObserver(handleResize);
      ro.observe(container);
      return () => ro.disconnect();
    } catch (e) {
      console.error("WebGL init failed:", e);
    }
  });

  onDestroy(() => {
    renderer?.destroy();
  });
</script>

<div class="preview-container" bind:this={container}>
  <canvas
    bind:this={canvas}
    on:mousedown={handleMouseDown}
    on:mousemove={handleMouseMove}
    on:mouseup={handleMouseUp}
    on:mouseleave={handleMouseUp}
    on:wheel={handleWheel}
    on:dblclick={handleDoubleClick}
    class="preview-canvas"
  ></canvas>

  <button
    class="compare-btn"
    on:mousedown={handleCompareDown}
    on:mouseup={handleCompareUp}
    on:mouseleave={handleCompareUp}
    on:touchstart={handleCompareDown}
    on:touchend={handleCompareUp}
    type="button"
    title="Hold to compare with original"
  >
    Hold to Compare
  </button>

  <div class="viewport-info">
    Zoom: {(viewport.zoom * 100).toFixed(0)}%
  </div>
</div>

<style>
  .preview-container {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--surface-container-lowest);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .preview-canvas {
    display: block;
    cursor: grab;
    width: 100%;
    height: 100%;
  }

  .preview-canvas:active {
    cursor: grabbing;
  }

  .compare-btn {
    position: absolute;
    bottom: 1rem;
    left: 50%;
    transform: translateX(-50%);
    padding: 0.375rem 1rem;
    background: var(--overlay-preview-bg);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-size: 0.75rem;
    font-weight: 500;
    cursor: pointer;
    user-select: none;
    backdrop-filter: blur(8px);
    transition: background 0.15s ease;
  }

  .compare-btn:hover {
    background: var(--overlay-preview-bg-hover);
  }

  .compare-btn:active {
    background: var(--accent);
    color: var(--bg-primary);
  }

  .viewport-info {
    position: absolute;
    top: 0.75rem;
    right: 0.75rem;
    padding: 0.25rem 0.625rem;
    background: var(--overlay-preview-bg);
    color: var(--text-muted);
    border-radius: var(--radius-default);
    font-size: 0.6875rem;
    font-variant-numeric: tabular-nums;
    backdrop-filter: blur(8px);
  }
</style>
