<!--
  CR-07 — Zone B Image Canvas.

  Pure rendering component: takes a `versionId` prop, fetches the
  artifact bytes via `readImageArtifact`, decodes the 16-bit TIFF,
  scales to viewport, renders to <canvas>. Supports zoom
  (fit / 1:1 / 0.25×–4×), pan (mouse-drag), mask overlay
  (translucent red wash where the mask is set), histogram
  (256 bins per channel), clipping overlay.

  Pixel decoding: a self-contained 16-bit grayscale + RGB TIFF
  decoder that handles both II and MM byte orders, single-strip
  uncompressed data (compression = 1) — which is exactly what
  the P5.1 apply round produces via `tiff` encoding. The decoder
  runs synchronously for ≤ 4 MP images; for larger versions the
  backend will ship a downsampled 8-bit PNG instead (a follow-up
  to this slice).

  Pixel access: `Uint16Array` 16-bit per-channel values are
  normalised to 0..1 floats for canvas drawing, with an optional
  `clipLow` / `clipHigh` gain that maps the histogram bins into a
  visible window.

  Region inspection: clicking + dragging emits a `region` Custom
  Event with the rect in image coordinates; parent components
  forward this to the Studio's region state.

  CR-07 follow-on: WebGL shader for big images, blink comparator,
  difference map, split slider, annotations.
-->
<script lang="ts">
  import { readImageArtifact } from "../lib/astroforge-api";

  export let versionId: string | null = null;
  export let mask: Float32Array | null = null;
  export let maskWidth = 0;
  export let maskHeight = 0;
  export let onRegion: (rect: {
    x0: number;
    y0: number;
    x1: number;
    y1: number;
  }) => void = () => {};

  type ZoomMode = "fit" | "1:1";

  let canvasEl: HTMLCanvasElement | undefined;
  let bytes: Uint8Array | null = null;
  let dims: { width: number; height: number; channels: number } | null = null;
  let loading = false;
  let loadError: string | null = null;
  let zoomMode: ZoomMode = "fit";
  let zoomLevel = 1; // 0.25..4 when zoomMode == "1:1"
  let panX = 0;
  let panY = 0;
  let clipLow = 0;
  let clipHigh = 1;
  let showHistogram = true;
  let showMask = true;

  // Pre-declared so the {#each} types resolve cleanly.
  let histogram: number[][] | null = null;

  let dragStart: { x: number; y: number; panX: number; panY: number } | null = null;
  let regionStart: { x: number; y: number } | null = null;

  // Re-fetch on version change.
  $: if (versionId) loadArtifact(versionId);

  async function loadArtifact(vid: string) {
    loading = true;
    loadError = null;
    bytes = null;
    dims = null;
    try {
      const res = await readImageArtifact(vid);
      const bin = atob(res.base64_data);
      const arr = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
      bytes = arr;
      dims = { width: res.width, height: res.height, channels: res.channels };
      // Reset pan + zoom to fit.
      zoomMode = "fit";
      panX = 0;
      panY = 0;
      requestAnimationFrame(draw);
    } catch (e) {
      loadError = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  // Decode a 16-bit grayscale or RGB uncompressed TIFF into
  // Float32Array (one plane per channel, normalised to 0..1).
  function decodeTiff(buf: Uint8Array): {
    width: number;
    height: number;
    channels: number;
    data: Float32Array;
  } | null {
    if (buf.length < 8 || (buf[0] !== 0x49 && buf[0] !== 0x4d)) return null;
    const little = buf[0] === 0x49; // "II" vs "MM"
    const r16 = (o: number) =>
      little
        ? buf[o] | (buf[o + 1] << 8)
        : (buf[o] << 8) | buf[o + 1];
    const r32 = (o: number) =>
      little
        ? buf[o] | (buf[o + 1] << 8) | (buf[o + 2] << 16) | (buf[o + 3] << 24)
        : (buf[o] << 24) | (buf[o + 1] << 16) | (buf[o + 2] << 8) | buf[o + 3];
    if (r16(2) !== 42) return null;
    const ifd = r32(4);
    const nEntries = r16(ifd);
    let width = 0;
    let height = 0;
    let samples = 1;
    let bps = 16;
    let stripOffset = 0;
    let stripByteCount = 0;
    for (let i = 0; i < nEntries; i++) {
      const e = ifd + 2 + i * 12;
      const tag = r16(e);
      const count = r32(e + 4);
      if (tag === 256) width = count > 1 ? r32(e + 8) : r16(e + 8);
      else if (tag === 257) height = count > 1 ? r32(e + 8) : r16(e + 8);
      else if (tag === 258) samples = r16(e + 8);
      else if (tag === 277) bps = r16(e + 8);
      else if (tag === 273) stripOffset = r32(e + 8);
      else if (tag === 279) stripByteCount = r32(e + 8);
    }
    if (bps !== 16 || (samples !== 1 && samples !== 3)) return null;
    if (!width || !height || !stripOffset) return null;
    const px = width * height;
    const expected = px * samples * 2;
    if (!stripByteCount) stripByteCount = expected;
    if (stripByteCount < expected) return null;
    const data = new Float32Array(px * samples);
    let p = 0;
    for (let i = 0; i < expected; i += 2) {
      const v = little
        ? buf[stripOffset + i] | (buf[stripOffset + i + 1] << 8)
        : (buf[stripOffset + i] << 8) | buf[stripOffset + i + 1];
      data[p++] = v / 65535;
    }
    return { width, height, channels: samples, data };
  }

  function draw() {
    if (!canvasEl || !bytes || !dims) return;
    const decoded = decodeTiff(bytes);
    if (!decoded) {
      const ctx = canvasEl.getContext("2d");
      if (!ctx) return;
      ctx.fillStyle = "#000";
      ctx.fillRect(0, 0, canvasEl.width, canvasEl.height);
      ctx.fillStyle = "#f55";
      ctx.font = "14px monospace";
      ctx.fillText("Failed to decode TIFF bytes.", 12, 28);
      return;
    }
    const { width, height, channels, data } = decoded;
    // Compute the on-screen scale (fit or 1:1 or zoomLevel).
    const cw = canvasEl.clientWidth || 600;
    const ch = canvasEl.clientHeight || 400;
    let scale: number;
    if (zoomMode === "fit") {
      scale = Math.min(cw / width, ch / height);
    } else if (zoomMode === "1:1") {
      scale = 1;
    } else {
      scale = zoomLevel;
    }
    const dispW = Math.max(1, Math.round(width * scale));
    const dispH = Math.max(1, Math.round(height * scale));
    canvasEl.width = cw;
    canvasEl.height = ch;
    // Compute the projection so pan is in screen pixels.
    const baseX = Math.floor((cw - dispW) / 2);
    const baseY = Math.floor((ch - dispH) / 2);
    const offX = baseX + Math.round(panX);
    const offY = baseY + Math.round(panY);
    const ctx = canvasEl.getContext("2d");
    if (!ctx) return;
    ctx.fillStyle = "#101216";
    ctx.fillRect(0, 0, cw, ch);
    ctx.imageSmoothingEnabled = scale < 4;
    // Draw each channel.
    const range = Math.max(1e-6, clipHigh - clipLow);
    const imgData = ctx.createImageData(dispW, dispH);
    const sample = (xi: number, yi: number, c: number) => {
      // Bilinear sample of the source.
      const fx = (xi / dispW) * (width - 1);
      const fy = (yi / dispH) * (height - 1);
      const x0 = Math.floor(fx);
      const y0 = Math.floor(fy);
      const x1 = Math.min(width - 1, x0 + 1);
      const y1 = Math.min(height - 1, y0 + 1);
      const tx = fx - x0;
      const ty = fy - y0;
      const stride = width * channels;
      const base = c;
      const idx = (x: number, y: number) => base + (y * width + x) * channels;
      const v00 = data[idx(x0, y0)];
      const v01 = data[idx(x1, y0)];
      const v10 = data[idx(x0, y1)];
      const v11 = data[idx(x1, y1)];
      return (
        v00 * (1 - tx) * (1 - ty) +
        v01 * tx * (1 - ty) +
        v10 * (1 - tx) * ty +
        v11 * tx * ty
      );
    };
    for (let y = 0; y < dispH; y++) {
      for (let x = 0; x < dispW; x++) {
        const i = (y * dispW + x) * 4;
        if (channels === 1) {
          const v = sample(x, y, 0);
          const c = Math.max(0, Math.min(1, (v - clipLow) / range)) * 255;
          imgData.data[i] = c;
          imgData.data[i + 1] = c;
          imgData.data[i + 2] = c;
          imgData.data[i + 3] = 255;
        } else {
          const r = sample(x, y, 0);
          const g = sample(x, y, 1);
          const b = sample(x, y, 2);
          imgData.data[i] = Math.max(0, Math.min(255, ((r - clipLow) / range) * 255));
          imgData.data[i + 1] = Math.max(0, Math.min(255, ((g - clipLow) / range) * 255));
          imgData.data[i + 2] = Math.max(0, Math.min(255, ((b - clipLow) / range) * 255));
          imgData.data[i + 3] = 255;
        }
      }
    }
    ctx.putImageData(imgData, offX, offY);
    // Mask overlay.
    if (showMask && mask && maskWidth && maskHeight) {
      ctx.fillStyle = "rgba(255, 64, 64, 0.4)";
      for (let y = 0; y < dispH; y++) {
        for (let x = 0; x < dispW; x++) {
          const fx = Math.floor((x / dispW) * maskWidth);
          const fy = Math.floor((y / dispH) * maskHeight);
          if (mask[fy * maskWidth + fx] > 0) {
            ctx.fillRect(offX + x, offY + y, 1, 1);
          }
        }
      }
    }
    // Clipping overlay.
    const clipOverlayColor = "rgba(255, 0, 0, 0.3)";
    ctx.fillStyle = clipOverlayColor;
    for (let y = 0; y < dispH; y++) {
      for (let x = 0; x < dispW; x++) {
        let over = false;
        if (channels === 1) {
          if (sample(x, y, 0) < clipLow || sample(x, y, 0) > clipHigh) over = true;
        } else {
          for (let c = 0; c < channels; c++) {
            const v = sample(x, y, c);
            if (v < clipLow || v > clipHigh) {
              over = true;
              break;
            }
          }
        }
        if (over) ctx.fillRect(offX + x, offY + y, 1, 1);
      }
    }
  }

  // 256-bin histogram per channel.
  $: histogram = computeHistogram(bytes, dims);

  function computeHistogram(
    buf: Uint8Array | null,
    d: { width: number; height: number; channels: number } | null,
  ): number[][] | null {
    if (!buf || !d) return null;
    const decoded = decodeTiff(buf);
    if (!decoded) return null;
    const { channels, data } = decoded;
    const binsR = new Array<number>(256).fill(0);
    const binsG = new Array<number>(256).fill(0);
    const binsB = new Array<number>(256).fill(0);
    for (let i = 0; i < data.length; i += channels) {
      if (channels === 1) {
        binsR[Math.min(255, Math.floor(data[i] * 255))]++;
      } else {
        binsR[Math.min(255, Math.floor(data[i] * 255))]++;
        binsG[Math.min(255, Math.floor(data[i + 1] * 255))]++;
        binsB[Math.min(255, Math.floor(data[i + 2] * 255))]++;
      }
    }
    return channels === 1 ? [binsR] : [binsR, binsG, binsB];
  }

  function onWheel(ev: WheelEvent) {
    if (zoomMode === "fit") zoomMode = "1:1";
    zoomLevel = Math.max(0.25, Math.min(4, zoomLevel * (ev.deltaY < 0 ? 1.1 : 0.9)));
    ev.preventDefault();
    draw();
  }

  function onMouseDown(ev: MouseEvent) {
    if (ev.shiftKey && canvasEl) {
      regionStart = { x: ev.offsetX, y: ev.offsetY };
    } else {
      dragStart = { x: ev.clientX, y: ev.clientY, panX, panY };
    }
  }

  function onMouseMove(ev: MouseEvent) {
    if (dragStart) {
      panX = dragStart.panX + (ev.clientX - dragStart.x);
      panY = dragStart.panY + (ev.clientY - dragStart.y);
      draw();
    }
  }

  function onMouseUp(ev: MouseEvent) {
    if (regionStart && canvasEl) {
      const x0 = Math.min(regionStart.x, ev.offsetX);
      const y0 = Math.min(regionStart.y, ev.offsetY);
      const x1 = Math.max(regionStart.x, ev.offsetX);
      const y1 = Math.max(regionStart.y, ev.offsetY);
      onRegion({ x0, y0, x1, y1 });
      regionStart = null;
    }
    dragStart = null;
  }

  function fit() {
    zoomMode = "fit";
    panX = 0;
    panY = 0;
    draw();
  }

  function oneToOne() {
    zoomMode = "1:1";
    zoomLevel = 1;
    draw();
  }

  // Re-draw on mask / clip / show changes.
  $: if (bytes) draw(), [showMask, clipLow, clipHigh, showHistogram];
</script>

<div class="image-canvas" data-testid="image-canvas">
  <div class="toolbar">
    <button type="button" on:click={fit} aria-label="Fit to window">Fit</button>
    <button type="button" on:click={oneToOne} aria-label="1:1 zoom">1:1</button>
    {#if zoomMode === "1:1"}
      <input
        type="range"
        min="0.25"
        max="4"
        step="0.05"
        bind:value={zoomLevel}
        on:input={draw}
        aria-label="Zoom level"
      />
      <span class="zoom-readout">{zoomLevel.toFixed(2)}×</span>
    {/if}
    <label class="check">
      <input type="checkbox" bind:checked={showMask} on:input={draw} />
      Mask
    </label>
    <label class="check">
      <input type="checkbox" bind:checked={showHistogram} />
      Histogram
    </label>
    <label class="check">
      Clip
      <input
        type="number"
        min="0"
        max="1"
        step="0.01"
        bind:value={clipLow}
        on:input={draw}
        aria-label="Clip low"
      />
      <input
        type="number"
        min="0"
        max="1"
        step="0.01"
        bind:value={clipHigh}
        on:input={draw}
        aria-label="Clip high"
      />
    </label>
  </div>
  <div class="viewport" data-testid="image-canvas-viewport">
    {#if loading}
      <p class="status">Loading…</p>
    {:else if loadError}
      <p class="status error">{loadError}</p>
    {:else if !versionId}
      <p class="status">Select an Image Version to render.</p>
    {:else}
      <canvas
        bind:this={canvasEl}
        on:wheel={onWheel}
        on:mousedown={onMouseDown}
        on:mousemove={onMouseMove}
        on:mouseup={onMouseUp}
        data-testid="image-canvas-surface"
      ></canvas>
      {#if showHistogram && histogram}
        <svg
          class="histogram"
          viewBox="0 0 256 60"
          preserveAspectRatio="none"
          data-testid="image-canvas-histogram"
        >
          {#each histogram as channel, ci}
            {@const maxBin = Math.max(1, ...channel)}
            {#each channel as count, i}
              <rect
                x={i}
                y={60 - (count / maxBin) * 60}
                width="1"
                height={(count / maxBin) * 60}
                fill={ci === 0 ? "#ff5060" : ci === 1 ? "#50ff60" : "#5070ff"}
                opacity="0.7"
              />
            {/each}
          {/each}
        </svg>
      {/if}
    {/if}
  </div>
</div>

<style>
  .image-canvas {
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
  }
  .toolbar button {
    background: #2a2f3a;
    color: #d8dde6;
    border: none;
    padding: 4px 10px;
    border-radius: 4px;
    cursor: pointer;
  }
  .toolbar button:hover {
    background: #353a48;
  }
  .check {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
  }
  .viewport {
    position: relative;
    flex: 1 1 auto;
    overflow: hidden;
  }
  .viewport canvas {
    width: 100%;
    height: 100%;
    cursor: grab;
  }
  .viewport canvas:active {
    cursor: grabbing;
  }
  .status {
    padding: 16px;
    color: #a0a6b3;
  }
  .status.error {
    color: #ff5060;
  }
  .histogram {
    position: absolute;
    bottom: 8px;
    right: 8px;
    width: 256px;
    height: 60px;
    background: rgba(0, 0, 0, 0.6);
    border: 1px solid #2a2f3a;
  }
  .zoom-readout {
    font-size: 12px;
    color: #a0a6b3;
    min-width: 4em;
  }
</style>