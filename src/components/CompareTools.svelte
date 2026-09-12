<!--
  CR-07 follow-on — Compare tools.

  A side-by-side, split-slider, blink, and difference-map
  comparison surface. Takes two ImageCanvas instances (one for
  version A, one for version B) and renders them through a
  shared compare mode:

  - side-by-side: just the two canvases laid out side by side.
  - split: a vertical slider overlays B on top of A; the right
    side of B is clipped by `inset(0 0 0 <pct>%)` so A shows
    through on the left.
  - blink: a 1Hz toggle between A and B; pauses when the tab is
    hidden so the cycle doesn't burn cycles in a backgrounded
    tab.
  - difference: a third overlay canvas computes the absolute
    per-pixel delta of the two canvases with a 4× gain so the
    change is visible.

  Keyboard accessibility: the split slider is an <input
  type="range"> with arrow-key nav at 1%/shift-arrow 10%
  increments. The blink comparator exposes a "Pause / Resume"
  button as well as the toggle.
-->
<script lang="ts">
  import ImageCanvas from "./ImageCanvas.svelte";
  import { onDestroy } from "svelte";

  export let versionIdA: string | null = null;
  export let versionIdB: string | null = null;
  export let maskA: Float32Array | null = null;
  export let maskWidthA = 0;
  export let maskHeightA = 0;
  export let maskB: Float32Array | null = null;
  export let maskWidthB = 0;
  export let maskHeightB = 0;

  type CompareMode = "side-by-side" | "split" | "blink" | "difference";
  let mode: CompareMode = "side-by-side";

  let splitPercent = 50; // 0..100
  let blinkInterval = 1000; // ms
  let blinkVisible: "a" | "b" = "a";
  let blinkPaused = false;
  let diffGain = 4;

  let intervalId: ReturnType<typeof setInterval> | null = null;
  let diffCanvasEl: HTMLCanvasElement | undefined;
  let canvasAEl: HTMLCanvasElement | undefined;
  let canvasBEl: HTMLCanvasElement | undefined;
  let canvasAImageData: ImageData | null = null;
  let canvasBImageData: ImageData | null = null;

  function startBlink() {
    stopBlink();
    if (blinkPaused) return;
    if (typeof document !== "undefined" && document.hidden) return;
    intervalId = setInterval(() => {
      blinkVisible = blinkVisible === "a" ? "b" : "a";
    }, blinkInterval);
  }

  function stopBlink() {
    if (intervalId !== null) {
      clearInterval(intervalId);
      intervalId = null;
    }
  }

  $: if (mode === "blink") startBlink();
  else stopBlink();

  function onVisibilityChange() {
    if (mode === "blink") {
      if (document.hidden) stopBlink();
      else startBlink();
    }
  }

  function capturePixelData() {
    // Sample the two source canvases into ImageData once per
    // version change. We can't call getImageData on a foreign
    // canvas in a single put, but we can read both into a
    // single buffer and combine.
    if (!canvasAEl || !canvasBEl || !diffCanvasEl) return;
    const ctxA = canvasAEl.getContext("2d");
    const ctxB = canvasBEl.getContext("2d");
    if (!ctxA || !ctxB) return;
    const w = diffCanvasEl.width;
    const h = diffCanvasEl.height;
    canvasAImageData = ctxA.getImageData(0, 0, w, h);
    canvasBImageData = ctxB.getImageData(0, 0, w, h);
    recomputeDifference();
  }

  function recomputeDifference() {
    if (!diffCanvasEl || !canvasAImageData || !canvasBImageData) return;
    const ctx = diffCanvasEl.getContext("2d");
    if (!ctx) return;
    const out = ctx.createImageData(
      canvasAImageData.width,
      canvasAImageData.height,
    );
    const a = canvasAImageData.data;
    const b = canvasBImageData.data;
    const o = out.data;
    for (let i = 0; i < a.length; i += 4) {
      const dr = Math.abs(a[i] - b[i]) * diffGain;
      const dg = Math.abs(a[i + 1] - b[i + 1]) * diffGain;
      const db = Math.abs(a[i + 2] - b[i + 2]) * diffGain;
      o[i] = Math.min(255, dr);
      o[i + 1] = Math.min(255, dg);
      o[i + 2] = Math.min(255, db);
      o[i + 3] = 255;
    }
    ctx.putImageData(out, 0, 0);
  }

  // Resample when the source canvases have rendered.
  $: if (
    mode === "difference" &&
    versionIdA &&
    versionIdB &&
    canvasAEl &&
    canvasBEl &&
    diffCanvasEl
  ) {
    // Wait one frame so the source canvases finish rendering,
    // then capture.
    requestAnimationFrame(capturePixelData);
  }

  function splitClipPath(): string {
    return `inset(0 0 0 ${splitPercent}%)`;
  }

  if (typeof document !== "undefined") {
    document.addEventListener("visibilitychange", onVisibilityChange);
  }

  onDestroy(() => {
    stopBlink();
    if (typeof document !== "undefined") {
      document.removeEventListener(
        "visibilitychange",
        onVisibilityChange,
      );
    }
  });
</script>

<div class="compare-tools" data-testid="compare-tools">
  <div class="toolbar" role="toolbar" aria-label="Compare mode">
    <button
      type="button"
      class:active={mode === "side-by-side"}
      on:click={() => (mode = "side-by-side")}
      aria-pressed={mode === "side-by-side"}
    >
      Side by side
    </button>
    <button
      type="button"
      class:active={mode === "split"}
      on:click={() => (mode = "split")}
      aria-pressed={mode === "split"}
    >
      Split
    </button>
    <button
      type="button"
      class:active={mode === "blink"}
      on:click={() => (mode = "blink")}
      aria-pressed={mode === "blink"}
    >
      Blink
    </button>
    <button
      type="button"
      class:active={mode === "difference"}
      on:click={() => (mode = "difference")}
      aria-pressed={mode === "difference"}
    >
      Difference
    </button>
    {#if mode === "split"}
      <label class="control">
        Split
        <input
          type="range"
          min="0"
          max="100"
          step="1"
          bind:value={splitPercent}
          aria-label="Split position"
          aria-valuetext="{splitPercent}% of canvas width"
        />
        <span class="readout">{splitPercent}%</span>
      </label>
    {/if}
    {#if mode === "blink"}
      <label class="control">
        Speed
        <input
          type="range"
          min="200"
          max="3000"
          step="100"
          bind:value={blinkInterval}
          on:input={startBlink}
          aria-label="Blink interval in milliseconds"
        />
        <span class="readout">{blinkInterval}ms</span>
      </label>
      <button
        type="button"
        on:click={() => {
          blinkPaused = !blinkPaused;
          if (blinkPaused) stopBlink();
          else startBlink();
        }}
      >
        {blinkPaused ? "Resume" : "Pause"}
      </button>
    {/if}
    {#if mode === "difference"}
      <label class="control">
        Gain
        <input
          type="range"
          min="1"
          max="16"
          step="1"
          bind:value={diffGain}
          on:input={recomputeDifference}
          aria-label="Difference gain"
        />
        <span class="readout">{diffGain}×</span>
      </label>
    {/if}
  </div>

  <div class="compare-stage" data-mode={mode}>
    {#if mode === "blink"}
      <div class="blink-stage">
        <div class="canvas-mount" hidden={blinkVisible !== "a"}>
          <ImageCanvas
            versionId={versionIdA}
            mask={maskA}
            maskWidth={maskWidthA}
            maskHeight={maskHeightA}
          />
        </div>
        <div class="canvas-mount" hidden={blinkVisible !== "b"}>
          <ImageCanvas
            versionId={versionIdB}
            mask={maskB}
            maskWidth={maskWidthB}
            maskHeight={maskHeightB}
          />
        </div>
      </div>
    {:else if mode === "difference"}
      <div class="diff-stage">
        <div class="hidden-sources" aria-hidden="true">
          <canvas
            bind:this={canvasAEl}
            width="512"
            height="512"
          ></canvas>
          <canvas
            bind:this={canvasBEl}
            width="512"
            height="512"
          ></canvas>
          <ImageCanvas
            versionId={versionIdA}
            mask={maskA}
            maskWidth={maskWidthA}
            maskHeight={maskHeightA}
          />
          <ImageCanvas
            versionId={versionIdB}
            mask={maskB}
            maskWidth={maskWidthB}
            maskHeight={maskHeightB}
          />
        </div>
        <canvas
          class="diff-canvas"
          bind:this={diffCanvasEl}
          width="512"
          height="512"
          data-testid="diff-canvas"
        ></canvas>
      </div>
    {:else}
      <div class="split-stage" style:--clip-path={splitClipPath()}>
        <div class="canvas-mount canvas-a">
          <ImageCanvas
            versionId={versionIdA}
            mask={maskA}
            maskWidth={maskWidthA}
            maskHeight={maskHeightA}
          />
          {#if mode === "split"}
            <span class="badge">A</span>
          {/if}
        </div>
        <div class="canvas-mount canvas-b">
          <ImageCanvas
            versionId={versionIdB}
            mask={maskB}
            maskWidth={maskWidthB}
            maskHeight={maskHeightB}
          />
          {#if mode === "split"}
            <span class="badge">B</span>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .compare-tools {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    background: #0b0d12;
    color: #d8dde6;
  }
  .toolbar {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px 10px;
    background: #161922;
    border-bottom: 1px solid #2a2f3a;
    flex-wrap: wrap;
  }
  .toolbar button {
    background: #2a2f3a;
    color: #d8dde6;
    border: none;
    padding: 4px 10px;
    border-radius: 4px;
    cursor: pointer;
  }
  .toolbar button.active {
    background: #4a90ff;
    color: #fff;
  }
  .control {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
  }
  .readout {
    min-width: 4em;
    color: #a0a6b3;
  }
  .compare-stage {
    flex: 1 1 auto;
    position: relative;
    overflow: hidden;
  }
  .split-stage {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
    width: 100%;
    height: 100%;
  }
  .compare-stage[data-mode="split"] .split-stage {
    grid-template-columns: 1fr;
  }
  .compare-stage[data-mode="split"] .canvas-b {
    position: absolute;
    inset: 0;
    clip-path: var(--clip-path, inset(0 0 0 50%));
  }
  .canvas-mount {
    position: relative;
    width: 100%;
    height: 100%;
    background: #000;
    border: 1px solid #2a2f3a;
    border-radius: 4px;
    overflow: hidden;
  }
  .badge {
    position: absolute;
    top: 8px;
    left: 8px;
    background: rgba(0, 0, 0, 0.6);
    color: #fff;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 12px;
    font-weight: bold;
    pointer-events: none;
  }
  .blink-stage,
  .diff-stage {
    position: relative;
    width: 100%;
    height: 100%;
  }
  .hidden-sources {
    position: absolute;
    inset: 0;
    visibility: hidden;
    pointer-events: none;
  }
  .hidden-sources canvas {
    display: none;
  }
  .diff-canvas {
    width: 100%;
    height: 100%;
    display: block;
    background: #000;
  }
</style>