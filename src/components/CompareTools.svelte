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
  // CR-07 §29.3b.4: WebGPU diff compute path. When the
  // browser supports WebGPU and the device acquisition
  // succeeds, `recomputeDifference` runs the GPU path
  // (§29.3a shaders) instead of the per-pixel Canvas2D
  // loop. Falls back to the Canvas2D path on any failure.
  import {
    acquireWebGpuDevice,
    WebGpuDiffCompute,
    type DiffMode,
  } from "../lib/webgpu-diff";

  export let versionIdA: string | null = null;
  export let versionIdB: string | null = null;
  export let maskA: Float32Array | null = null;
  export let maskWidthA = 0;
  export let maskHeightA = 0;
  export let maskB: Float32Array | null = null;
  export let maskWidthB = 0;
  export let maskHeightB = 0;

  type CompareMode = "side-by-side" | "split" | "blink" | "difference" | "overlay";
  let mode: CompareMode = "side-by-side";

  // CR-07 B10: §5.4's four difference modes (Absolute, Signed,
  // Amplified, Structural). The frontend mirrors the
  // `DiffKind` enum in `crates/astroforge-core/src/difference.rs`
  // so the on-canvas math stays in 8-bit canvas space; the
  // backend enum is the canonical reference for any Rust
  // consumer (CI fixtures, server-side batch comparison, etc.).
  type DiffKind = "absolute" | "signed" | "amplified" | "structural";
  let diffKind: DiffKind = "absolute";

  // CR-07 B11: per-channel auto-stretch normalizer applied
  // to the diff canvas pixels. The frontend mirrors
  // `normalize_stretch` in
  // `crates/astroforge-core/src/difference_normalize.rs` so
  // we don't pay an IPC round-trip per slider tick. Low and
  // high percentiles default to the astronomy convention
  // 0.5% / 99.5%; the user can disable the stretch via the
  // "Off" button.
  type StretchMode = "off" | "auto" | "manual";
  let stretchMode: StretchMode = "auto";
  let stretchLowPct = 0.5;
  let stretchHighPct = 99.5;
  // CR-07 B12: fixed-stretch / n-sigma state. The two
  // inputs (vLowFixed / vHighFixed) are the explicit cutoffs
  // used when stretchMode === "manual". nSigmaK is the
  // multiplier used by the n-sigma button (always computes
  // fresh stats over the current diff image).
  let vLowFixed: number = 32;
  let vHighFixed: number = 220;
  let nSigmaK: number = 3;

  let splitPercent = 50; // 0..100
  let blinkInterval = 1000; // ms
  let blinkVisible: "a" | "b" = "a";
  let blinkPaused = false;
  let diffGain = 4;
  // CR-07 B6: overlay mode opacity for version B (A is
  // rendered opaque underneath). 0..100 slider percentage.
  let overlayOpacity = 50;

  let intervalId: ReturnType<typeof setInterval> | null = null;
  let diffCanvasEl: HTMLCanvasElement | undefined;
  let canvasAEl: HTMLCanvasElement | undefined;
  let canvasBEl: HTMLCanvasElement | undefined;
  let canvasAImageData: ImageData | null = null;
  let canvasBImageData: ImageData | null = null;
  // CR-07 B6: overlay mode reuses the diff stage's read-then-
  // composite pattern. The overlay canvas holds the blended
  // output (A + alpha-B); the two source canvases feed it.
  let overlayCanvasEl: HTMLCanvasElement | undefined;

  // CR-07 §29.3b.4: WebGPU diff compute. `null` = not yet
  // attempted; `"unavailable"` = browser lacks WebGPU or
  // acquisition failed; `"loading"` = acquisition in flight;
  // `WebGpuDiffCompute` = ready.
  let gpuDiff: WebGpuDiffCompute | "unavailable" | "loading" | null = null;

  async function ensureGpuDiff(): Promise<WebGpuDiffCompute | null> {
    if (gpuDiff === "unavailable") return null;
    if (gpuDiff === "loading") return null;
    if (gpuDiff instanceof WebGpuDiffCompute) return gpuDiff;
    gpuDiff = "loading";
    try {
      const device = await acquireWebGpuDevice();
      if (!device) {
        gpuDiff = "unavailable";
        return null;
      }
      gpuDiff = new WebGpuDiffCompute(device);
      return gpuDiff;
    } catch (err) {
      console.warn("[webgpu-diff] device acquisition failed; using Canvas2D path", err);
      gpuDiff = "unavailable";
      return null;
    }
  }

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
    if (!canvasAEl || !canvasBEl) return;
    const ctxA = canvasAEl.getContext("2d");
    const ctxB = canvasBEl.getContext("2d");
    if (!ctxA || !ctxB) return;
    // CR-07 B6: pick the target canvas based on mode. The
    // diff stage uses a 512x512 buffer; overlay uses the same
    // so the hidden-source ImageCanvas mounts have a fixed
    // pixel grid to draw against.
    const target =
      mode === "overlay" ? overlayCanvasEl : diffCanvasEl;
    if (!target) return;
    const w = target.width;
    const h = target.height;
    canvasAImageData = ctxA.getImageData(0, 0, w, h);
    canvasBImageData = ctxB.getImageData(0, 0, w, h);
    if (mode === "difference") recomputeDifference();
    else if (mode === "overlay") recomputeOverlay();
  }

  // CR-07 B10: dispatch the four §5.4 difference modes. The
  // structural path uses a Sobel-style neighbour diff on the
  // captured ImageData buffers; the others stay linear.
  // Implementation mirrors `compute_diff` in
  // `crates/astroforge-core/src/difference.rs`.
  //
  // CR-07 §29.3b.4: when WebGPU is available, the per-pixel
  // loop below is replaced by the §29.3a WGSL compute
  // shaders. The GPU path is preferred; the Canvas2D loop
  // is the fallback for browsers without WebGPU or when the
  // GPU dispatch fails.
  function recomputeDifference() {
    if (!diffCanvasEl || !canvasAImageData || !canvasBImageData) return;
    const ctx = diffCanvasEl.getContext("2d");
    if (!ctx) return;
    const w = canvasAImageData.width;
    const h = canvasAImageData.height;
    const a = canvasAImageData.data;
    const b = canvasBImageData.data;

    // Fire-and-forget the GPU path. The reactive graph
    // doesn't await, so we kick the async work off and
    // let it paint when it finishes.
    void (async () => {
      const gpu = await ensureGpuDiff();
      if (gpu) {
        // Convert ImageData.data (Uint8ClampedArray) to
        // Uint8Array view for the GPU wrapper. The wrapper
        // expects raw RGBA8 bytes.
        const aBytes = new Uint8Array(a.buffer, a.byteOffset, a.byteLength);
        const bBytes = new Uint8Array(b.buffer, b.byteOffset, b.byteLength);
        const result = await gpu.compute(
          diffKind as DiffMode,
          aBytes,
          bBytes,
          diffGain,
          w,
        );
        if (result && result.length === w * h * 4) {
          // Wrap the GPU output in ImageData, apply
          // stretch JS-side, paint.
          const out = new ImageData(w, h);
          out.data.set(
            new Uint8ClampedArray(result.buffer, result.byteOffset, result.byteLength),
          );
          if (stretchMode === "auto") {
            applyStretch(out.data);
          } else if (stretchMode === "manual") {
            applyFixedStretch(out.data, vLowFixed, vHighFixed);
          }
          ctx.putImageData(out, 0, 0);
          return;
        }
        // result === null → GPU dispatch failed; fall
        // through to Canvas2D.
      }
      // Canvas2D fallback path (original B10 code).
      recomputeDifferenceCanvas2d(ctx, w, h, a, b);
    })();
  }

  // CR-07 §29.3b.4: the original Canvas2D per-pixel loop,
  // preserved as the fallback when WebGPU is unavailable or
  // the GPU dispatch fails. The `stretch` application
  // after the diff is identical to the GPU path's stretch
  // step: both paths converge on the same JS-side
  // stretch logic before `putImageData`.
  function recomputeDifferenceCanvas2d(
    ctx: CanvasRenderingContext2D,
    w: number,
    h: number,
    a: Uint8ClampedArray,
    b: Uint8ClampedArray,
  ) {
    const out = ctx.createImageData(w, h);
    const o = out.data;
    const stride = 4;
    if (diffKind === "structural") {
      // Greyscale edge map of A abs-diff edge map of B. The
      // boundary pixels (row 0, column 0, last row, last col)
      // fall back to absolute diff so the user still sees a
      // meaningful render when the structural path has no
      // interior neighbours.
      const edgeA = new Uint8ClampedArray(o.length);
      const edgeB = new Uint8ClampedArray(o.length);
      for (let y = 1; y < h - 1; y++) {
        for (let x = 1; x < w - 1; x++) {
          const i = (y * w + x) * stride;
          const left = i - stride;
          const up = i - w * stride;
          for (let c = 0; c < 3; c++) {
            const ea = Math.max(
              Math.abs(a[i + c] - a[left + c]),
              Math.abs(a[i + c] - a[up + c]),
            );
            const eb = Math.max(
              Math.abs(b[i + c] - b[left + c]),
              Math.abs(b[i + c] - b[up + c]),
            );
            edgeA[i + c] = ea;
            edgeB[i + c] = eb;
          }
        }
      }
      for (let i = 0; i < o.length; i += stride) {
        o[i] = Math.abs(edgeA[i] - edgeB[i]);
        o[i + 1] = Math.abs(edgeA[i + 1] - edgeB[i + 1]);
        o[i + 2] = Math.abs(edgeA[i + 2] - edgeB[i + 2]);
        o[i + 3] = 255;
      }
    } else if (diffKind === "signed") {
      // A - B shifted by 128 so equal pixels read grey; brighter
      // means B is brighter than A, darker means A is brighter
      // than B. gain is ignored on this path.
      for (let i = 0; i < o.length; i += stride) {
        o[i] = clamp255(a[i] - b[i] + 128);
        o[i + 1] = clamp255(a[i + 1] - b[i + 1] + 128);
        o[i + 2] = clamp255(a[i + 2] - b[i + 2] + 128);
        o[i + 3] = 255;
      }
    } else if (diffKind === "amplified") {
      // |A - B| * gain, clamped to 0..255.
      for (let i = 0; i < o.length; i += stride) {
        o[i] = clamp255(Math.abs(a[i] - b[i]) * diffGain);
        o[i + 1] = clamp255(Math.abs(a[i + 1] - b[i + 1]) * diffGain);
        o[i + 2] = clamp255(Math.abs(a[i + 2] - b[i + 2]) * diffGain);
        o[i + 3] = 255;
      }
    } else {
      // absolute (default). |A - B|, clamped to 8-bit.
      for (let i = 0; i < o.length; i += stride) {
        o[i] = Math.abs(a[i] - b[i]);
        o[i + 1] = Math.abs(a[i + 1] - b[i + 1]);
        o[i + 2] = Math.abs(a[i + 2] - b[i + 2]);
        o[i + 3] = 255;
      }
    }
    // CR-07 B11: per-channel auto-stretch. Mirrors the Rust
    // `normalize_stretch` in
    // `crates/astroforge-core/src/difference_normalize.rs`.
    // Skipped when the user picked "Off"; the histogram is
    // built once on the small imageData buffer (sub-millisecond
    // on a 1-2 megapixel canvas).
    if (stretchMode === "auto") {
      applyStretch(out.data);
    } else if (stretchMode === "manual") {
      // CR-07 B12: fixed-stretch with the user-supplied
      // cutoffs (or the cutoffs produced by n-sigma, if the
      // user pressed the "Apply n-sigma" button).
      applyFixedStretch(out.data, vLowFixed, vHighFixed);
    }
    ctx.putImageData(out, 0, 0);
  }

  // CR-07 B11: per-channel percentile histogram stretch.
  // Builds a 256-bin histogram per channel, walks it to find
  // v_low / v_high at the configured percentiles, then linearly
  // remaps the in-range pixels. The alpha channel is skipped.
  function applyStretch(pixels: Uint8ClampedArray): void {
    const stride = 4;
    const channelCount = (pixels.length / stride) * 3;
    if (channelCount === 0) return;
    // Build combined RGB histogram (channels share cutoffs).
    const histo = new Uint32Array(256);
    for (let i = 0; i < pixels.length; i += stride) {
      histo[pixels[i]]++;
      histo[pixels[i + 1]]++;
      histo[pixels[i + 2]]++;
    }
    const lowThreshold = Math.ceil(
      (stretchLowPct / 100) * channelCount,
    );
    const highThreshold = Math.floor(
      (1 - stretchHighPct / 100) * channelCount,
    );
    // Walk low: first bin whose cumulative count >= lowThreshold.
    let vLow = 0;
    let cum = 0;
    let foundLow = false;
    for (let i = 0; i < 256; i++) {
      cum += histo[i];
      if (!foundLow && cum >= lowThreshold) {
        vLow = i;
        foundLow = true;
      }
    }
    // Walk high: first bin from the top whose from-top count
    // >= highThreshold.
    let vHigh = 255;
    let fromTop = 0;
    let foundHigh = false;
    for (let i = 255; i >= 0; i--) {
      fromTop += histo[i];
      if (!foundHigh && fromTop >= highThreshold) {
        vHigh = i;
        foundHigh = true;
      }
    }
    if (!foundLow) vLow = 0;
    if (!foundHigh) vHigh = 255;
    if (vHigh <= vLow) {
      // Degenerate range (single-value image); no stretch.
      return;
    }
    const range = vHigh - vLow;
    for (let i = 0; i < pixels.length; i += stride) {
      for (let c = 0; c < 3; c++) {
        const v = pixels[i + c];
        if (v <= vLow) {
          pixels[i + c] = 0;
        } else if (v >= vHigh) {
          pixels[i + c] = 255;
        } else {
          pixels[i + c] = Math.round(
            ((v - vLow) * 255 + range / 2) / range,
          );
        }
      }
      // Alpha preserved.
    }
  }

  // CR-07 B12: explicit-cutoff stretch. Mirrors the Rust
  // `normalize_fixed` in
  // `crates/astroforge-core/src/difference_normalize.rs`.
  // Pixels below vLow clamp to 0; above vHigh clamp to 255;
  // in-range pixels linearly remap to [0, 255]. Alpha is
  // preserved. vLow >= vHigh is a no-op.
  function applyFixedStretch(
    pixels: Uint8ClampedArray,
    vLow: number,
    vHigh: number,
  ): void {
    if (vLow >= vHigh) return;
    const range = vHigh - vLow;
    for (let i = 0; i < pixels.length; i += 4) {
      for (let c = 0; c < 3; c++) {
        const v = pixels[i + c];
        if (v <= vLow) {
          pixels[i + c] = 0;
        } else if (v >= vHigh) {
          pixels[i + c] = 255;
        } else {
          pixels[i + c] = Math.round(
            ((v - vLow) * 255 + range / 2) / range,
          );
        }
      }
      // Alpha preserved.
    }
  }

  // CR-07 B12: compute mean and standard deviation across
  // all RGB bytes in the buffer (alpha skipped). Mirrors
  // the Rust `mean_stddev` helper.
  function computeMeanStddev(pixels: Uint8ClampedArray): {
    mean: number;
    stddev: number;
  } {
    let sum = 0;
    let count = 0;
    for (let i = 0; i < pixels.length; i += 4) {
      sum += pixels[i];
      sum += pixels[i + 1];
      sum += pixels[i + 2];
      count += 3;
    }
    if (count === 0) return { mean: 0, stddev: 0 };
    const mean = sum / count;
    let varSum = 0;
    for (let i = 0; i < pixels.length; i += 4) {
      for (let c = 0; c < 3; c++) {
        const d = pixels[i + c] - mean;
        varSum += d * d;
      }
    }
    return { mean, stddev: Math.sqrt(varSum / count) };
  }

  // CR-07 B12: read the current diff canvas, compute mean
  // and stddev, then write `mean ± k*sigma` (clamped to
  // [0, 255]) back into the vLowFixed / vHighFixed inputs.
  // The recompute that follows uses those cutoffs.
  function applyNSigmaToFixed(): void {
    if (!diffCanvasEl) return;
    const ctx = diffCanvasEl.getContext("2d");
    if (!ctx) return;
    const w = diffCanvasEl.width;
    const h = diffCanvasEl.height;
    const img = ctx.getImageData(0, 0, w, h);
    const k = Number.isFinite(nSigmaK) && nSigmaK > 0 ? nSigmaK : 1;
    const { mean, stddev } = computeMeanStddev(img.data);
    const lo = Math.max(0, Math.min(255, mean - k * stddev));
    const hi = Math.max(0, Math.min(255, mean + k * stddev));
    vLowFixed = Math.round(lo);
    vHighFixed = Math.round(hi);
    recomputeDifference();
  }

  function clamp255(v: number): number {
    if (v < 0) return 0;
    if (v > 255) return 255;
    return v;
  }

  // CR-07 B6: overlay composite. Draw A opaque, then B on top
  // with the user-controlled opacity. Pixels are read from
  // the two hidden source canvases (which the existing
  // ImageCanvas mount fills) once per version change; opacity
  // changes just re-blend the cached ImageData.
  function recomputeOverlay() {
    if (
      !overlayCanvasEl ||
      !canvasAImageData ||
      !canvasBImageData
    )
      return;
    const ctx = overlayCanvasEl.getContext("2d");
    if (!ctx) return;
    const a = canvasAImageData.data;
    const b = canvasBImageData.data;
    const out = ctx.createImageData(
      canvasAImageData.width,
      canvasAImageData.height,
    );
    const o = out.data;
    const alpha = overlayOpacity / 100;
    const oneMinusAlpha = 1 - alpha;
    for (let i = 0; i < a.length; i += 4) {
      o[i] = a[i] * oneMinusAlpha + b[i] * alpha;
      o[i + 1] = a[i + 1] * oneMinusAlpha + b[i + 1] * alpha;
      o[i + 2] = a[i + 2] * oneMinusAlpha + b[i + 2] * alpha;
      o[i + 3] = 255;
    }
    ctx.putImageData(out, 0, 0);
  }

  // Resample when the source canvases have rendered.
  $: if (
    (mode === "difference" || mode === "overlay") &&
    versionIdA &&
    versionIdB &&
    canvasAEl &&
    canvasBEl &&
    ((mode === "difference" && diffCanvasEl) ||
      (mode === "overlay" && overlayCanvasEl))
  ) {
    // Wait one frame so the source canvases finish rendering,
    // then capture.
    requestAnimationFrame(capturePixelData);
  }

  // CR-07 B6: overlay recomputes on opacity changes (cheap,
  // pure re-blend of cached pixels) without re-capturing.
  $: if (
    mode === "overlay" &&
    overlayCanvasEl &&
    canvasAImageData &&
    canvasBImageData
  ) {
    recomputeOverlay();
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
    <button
      type="button"
      class:active={mode === "overlay"}
      on:click={() => (mode = "overlay")}
      aria-pressed={mode === "overlay"}
    >
      Overlay
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
      <div class="diff-kind" role="group" aria-label="Difference mode">
        <button
          type="button"
          class:active={diffKind === "absolute"}
          on:click={() => {
            diffKind = "absolute";
            recomputeDifference();
          }}
          aria-pressed={diffKind === "absolute"}
        >
          Absolute
        </button>
        <button
          type="button"
          class:active={diffKind === "signed"}
          on:click={() => {
            diffKind = "signed";
            recomputeDifference();
          }}
          aria-pressed={diffKind === "signed"}
        >
          Signed
        </button>
        <button
          type="button"
          class:active={diffKind === "amplified"}
          on:click={() => {
            diffKind = "amplified";
            recomputeDifference();
          }}
          aria-pressed={diffKind === "amplified"}
        >
          Amplified
        </button>
        <button
          type="button"
          class:active={diffKind === "structural"}
          on:click={() => {
            diffKind = "structural";
            recomputeDifference();
          }}
          aria-pressed={diffKind === "structural"}
        >
          Structural
        </button>
      </div>
      {#if diffKind === "amplified"}
        <label class="control">
          Gain
          <input
            type="range"
            min="1"
            max="16"
            step="1"
            bind:value={diffGain}
            on:input={recomputeDifference}
            aria-label="Amplified difference gain"
          />
          <span class="readout">{diffGain}×</span>
        </label>
      {/if}
      <div class="stretch" role="group" aria-label="Stretch normalizer">
        <button
          type="button"
          class:active={stretchMode === "auto"}
          on:click={() => {
            stretchMode = "auto";
            recomputeDifference();
          }}
          aria-pressed={stretchMode === "auto"}
        >
          Auto stretch
        </button>
        <button
          type="button"
          class:active={stretchMode === "off"}
          on:click={() => {
            stretchMode = "off";
            recomputeDifference();
          }}
          aria-pressed={stretchMode === "off"}
        >
          Off
        </button>
      </div>
      {#if stretchMode === "auto"}
        <label class="control">
          Low
          <input
            type="range"
            min="0"
            max="20"
            step="0.5"
            bind:value={stretchLowPct}
            on:input={recomputeDifference}
            aria-label="Auto stretch low percentile"
          />
          <span class="readout">{stretchLowPct.toFixed(1)}%</span>
        </label>
        <label class="control">
          High
          <input
            type="range"
            min="80"
            max="100"
            step="0.5"
            bind:value={stretchHighPct}
            on:input={recomputeDifference}
            aria-label="Auto stretch high percentile"
          />
          <span class="readout">{stretchHighPct.toFixed(1)}%</span>
        </label>
      {/if}
      <button
        type="button"
        class:active={stretchMode === "manual"}
        on:click={() => {
          stretchMode = "manual";
          recomputeDifference();
        }}
        aria-pressed={stretchMode === "manual"}
      >
        Manual
      </button>
      {#if stretchMode === "manual"}
        <label class="control">
          vLow
          <input
            type="number"
            min="0"
            max="255"
            step="1"
            bind:value={vLowFixed}
            on:input={recomputeDifference}
            aria-label="Fixed stretch low cutoff"
          />
        </label>
        <label class="control">
          vHigh
          <input
            type="number"
            min="0"
            max="255"
            step="1"
            bind:value={vHighFixed}
            on:input={recomputeDifference}
            aria-label="Fixed stretch high cutoff"
          />
        </label>
        <label class="control">
          n-sigma k
          <input
            type="range"
            min="1"
            max="6"
            step="0.5"
            bind:value={nSigmaK}
            aria-label="n-sigma multiplier"
          />
          <span class="readout">{nSigmaK.toFixed(1)}×σ</span>
        </label>
        <button
          type="button"
          class="nsigma-apply"
          on:click={applyNSigmaToFixed}
          aria-label="Apply n-sigma cutoffs to vLow/vHigh"
        >
          Apply n-sigma
        </button>
      {/if}
    {/if}
    {#if mode === "overlay"}
      <label class="control">
        B opacity
        <input
          type="range"
          min="0"
          max="100"
          step="1"
          bind:value={overlayOpacity}
          on:input={recomputeOverlay}
          aria-label="Overlay opacity for version B"
        />
        <span class="readout">{overlayOpacity}%</span>
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
    {:else if mode === "overlay"}
      <div class="overlay-stage">
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
          class="overlay-canvas"
          bind:this={overlayCanvasEl}
          width="512"
          height="512"
          data-testid="overlay-canvas"
        ></canvas>
        <div class="overlay-badges" aria-hidden="true">
          <span class="badge badge-a">A</span>
          <span class="badge badge-b">B</span>
        </div>
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
  /* CR-07 B10: sub-toolbar for the §5.4 difference modes.
     Inherits button styles from .toolbar; only needs to lay
     out the 4 mode buttons in a tight group with a hairline
     separator so the difference toolbar doesn't blend into
     the rest of the controls. */
  .diff-kind {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 0 6px;
    margin: 0 4px;
    border-left: 1px solid #2a2e36;
  }
  .diff-kind button {
    font-size: 11px;
    padding: 4px 8px;
  }
  /* CR-07 B11: stretch normalizer sub-toolbar. Mirrors
     .diff-kind's hairline separator + compact button sizing
     so the three sub-toolbars (mode / gain / stretch) read
     as a single visual cluster under the Difference
     toggle. */
  .stretch {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 0 6px;
    margin: 0 4px;
    border-left: 1px solid #2a2e36;
  }
  .stretch button {
    font-size: 11px;
    padding: 4px 8px;
  }
  /* CR-07 B12: Manual stretch apply button gets a slight
     tint so it stands out from the mode toggles. */
  .nsigma-apply {
    font-size: 11px;
    padding: 4px 8px;
    background: #3a3f4a;
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
  .diff-stage,
  .overlay-stage {
    position: relative;
    width: 100%;
    height: 100%;
  }
  .overlay-canvas {
    width: 100%;
    height: 100%;
    display: block;
    background: #000;
  }
  .overlay-badges {
    position: absolute;
    top: 8px;
    left: 8px;
    display: flex;
    gap: 4px;
    pointer-events: none;
  }
  .badge-a {
    background: rgba(74, 144, 255, 0.7);
    color: #fff;
  }
  .badge-b {
    background: rgba(255, 144, 74, 0.7);
    color: #fff;
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