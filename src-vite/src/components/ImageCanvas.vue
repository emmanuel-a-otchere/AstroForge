<template>
  <div class="image-canvas-wrap">
    <div class="canvas-toolbar">
      <button
        type="button"
        class="tool-btn font-body"
        :data-active="effectiveView.zoomMode === 'fit'"
        title="Fit to view"
        @click="setFit"
      >
        Fit
      </button>
      <button
        type="button"
        class="tool-btn font-body"
        :data-active="effectiveView.zoomMode === '1:1'"
        title="1:1 pixels"
        @click="set11"
      >
        1:1
      </button>
      <button type="button" class="tool-btn" title="Zoom out" @click="zoomBy(1 / 1.25)">−</button>
      <span class="font-body zoom-label">{{ zoomPercent }}%</span>
      <button type="button" class="tool-btn" title="Zoom in" @click="zoomBy(1.25)">+</button>
      <button type="button" class="tool-btn font-body" title="Reset pan" @click="resetPan">
        Reset pan
      </button>
      <label class="font-body slider-label">
        BP
        <input
          v-model.number="clipLow"
          type="range"
          min="0"
          max="0.95"
          step="0.01"
          @input="scheduleDraw"
        />
      </label>
      <label class="font-body slider-label">
        WP
        <input
          v-model.number="clipHigh"
          type="range"
          min="0.05"
          max="1"
          step="0.01"
          @input="scheduleDraw"
        />
      </label>
      <span class="font-body backend-label">{{ activeBackend ?? "…" }}</span>
    </div>
    <div class="canvas-host">
      <canvas
        ref="canvasEl"
        class="canvas"
        @wheel.prevent="onWheel"
        @pointerdown="onPointerDown"
        @pointermove="onPointerMove"
        @pointerup="onPointerUp"
        @pointerleave="onPointerUp"
      ></canvas>
      <p v-if="loading" class="overlay-msg font-body">Loading image…</p>
      <p v-if="loadError" class="overlay-msg error font-body" role="alert">{{ loadError }}</p>
    </div>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { readImageArtifact } from "@/lib/astroforge-api";
import { decodeTiff } from "@/lib/tiff-decode";
import {
  initImageCanvasGl,
  drawImageCanvasGl,
  destroyImageCanvasGl,
} from "@/lib/image-canvas-webgl";

const props = defineProps({
  versionId: { type: String, default: null },
  // When provided (sync mode), the parent owns the view state.
  view: { type: Object, default: null },
});
const emit = defineEmits(["view-change"]);

const canvasEl = ref(null);
const bytes = ref(null);
const dims = ref(null);
const loading = ref(false);
const loadError = ref(null);

const localView = ref({ zoomMode: "fit", zoomLevel: 1, panX: 0, panY: 0 });
const clipLow = ref(0);
const clipHigh = ref(1);

let glContext = null;
let glState = null;
let activeBackend = ref(null);
let webglFailed = false;
let dragStart = null;
let rafId = 0;
let resizeObserver = null;

const effectiveView = computed(() => props.view ?? localView.value);
const zoomPercent = computed(() => {
  const v = effectiveView.value;
  if (v.zoomMode === "fit") return "fit";
  return Math.round(v.zoomLevel * 100);
});

function emitView(next) {
  localView.value = next;
  emit("view-change", next);
}

function setFit() {
  emitView({ ...effectiveView.value, zoomMode: "fit", zoomLevel: 1, panX: 0, panY: 0 });
  scheduleDraw();
}

function set11() {
  emitView({ ...effectiveView.value, zoomMode: "1:1", zoomLevel: 1 });
  scheduleDraw();
}

function zoomBy(factor) {
  const v = effectiveView.value;
  const next = Math.min(4, Math.max(0.25, (v.zoomMode === "1:1" ? v.zoomLevel : 1) * factor));
  emitView({ ...v, zoomMode: "1:1", zoomLevel: next });
  scheduleDraw();
}

function resetPan() {
  emitView({ ...effectiveView.value, panX: 0, panY: 0 });
  scheduleDraw();
}

function onWheel(e) {
  zoomBy(e.deltaY < 0 ? 1.25 : 1 / 1.25);
}

function onPointerDown(e) {
  if (!canvasEl.value) return;
  canvasEl.value.setPointerCapture(e.pointerId);
  dragStart = { x: e.clientX, y: e.clientY, panX: effectiveView.value.panX, panY: effectiveView.value.panY };
}

function onPointerMove(e) {
  if (!dragStart) return;
  const dx = e.clientX - dragStart.x;
  const dy = e.clientY - dragStart.y;
  emitView({ ...effectiveView.value, panX: dragStart.panX + dx, panY: dragStart.panY + dy });
  scheduleDraw();
}

function onPointerUp() {
  dragStart = null;
}

async function loadArtifact(vid) {
  loading.value = true;
  loadError.value = null;
  bytes.value = null;
  dims.value = null;
  try {
    const res = await readImageArtifact(vid);
    const bin = atob(res.base64_data);
    const arr = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
    bytes.value = arr;
    dims.value = { width: res.width, height: res.height, channels: res.channels };
    emitView({ zoomMode: "fit", zoomLevel: 1, panX: 0, panY: 0 });
    scheduleDraw();
  } catch (e) {
    loadError.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function scheduleDraw() {
  if (rafId) cancelAnimationFrame(rafId);
  rafId = requestAnimationFrame(draw);
}

function draw() {
  const el = canvasEl.value;
  if (!el || !bytes.value || !dims.value) return;
  if (!activeBackend.value) {
    activeBackend.value = webglFailed ? "canvas2d" : "webgl";
  }
  if (activeBackend.value === "webgl") {
    try {
      drawWebGL(el);
      return;
    } catch (e) {
      console.warn("ImageCanvas WebGL render failed:", e);
      webglFailed = true;
      activeBackend.value = "canvas2d";
      if (glContext && glState) destroyImageCanvasGl(glContext, glState);
      glContext = null;
      glState = null;
    }
  }
  drawCanvas2D(el);
}

function drawWebGL(el) {
  const decoded = decodeTiff(bytes.value);
  if (!decoded) {
    drawCanvas2D(el);
    return;
  }
  if (!glContext || !glState) {
    const gl = el.getContext("webgl", {
      premultipliedAlpha: false,
      preserveDrawingBuffer: false,
      antialias: false,
    });
    if (!gl) {
      webglFailed = true;
      activeBackend.value = "canvas2d";
      drawCanvas2D(el);
      return;
    }
    const init = initImageCanvasGl(gl);
    if (!init.ok) {
      webglFailed = true;
      activeBackend.value = "canvas2d";
      drawCanvas2D(el);
      return;
    }
    glContext = gl;
    glState = init;
  }
  const cw = el.clientWidth || 600;
  const ch = el.clientHeight || 400;
  const v = effectiveView.value;
  let scale;
  if (v.zoomMode === "fit") scale = Math.min(cw / decoded.width, ch / decoded.height);
  else scale = v.zoomMode === "1:1" ? v.zoomLevel : v.zoomLevel;
  const dispW = Math.max(1, Math.round(decoded.width * scale));
  const dispH = Math.max(1, Math.round(decoded.height * scale));
  const baseX = (cw - dispW) / 2;
  const baseY = (ch - dispH) / 2;
  const uvPanX = decoded.width > 0 ? (baseX + v.panX) / decoded.width : 0;
  const uvPanY = decoded.height > 0 ? (baseY + v.panY) / decoded.height : 0;
  const uvZoom = dispW > 0 ? cw / dispW : 1;
  drawImageCanvasGl({
    gl: glContext,
    state: glState,
    width: decoded.width,
    height: decoded.height,
    channels: decoded.channels === 3 ? 3 : 1,
    data: decoded.data,
    params: { blackPoint: clipLow.value, whitePoint: clipHigh.value },
    viewport: { zoom: uvZoom, panX: uvPanX, panY: uvPanY },
  });
}

function drawCanvas2D(el) {
  const decoded = decodeTiff(bytes.value);
  const cw = el.clientWidth || 600;
  const ch = el.clientHeight || 400;
  el.width = cw;
  el.height = ch;
  const ctx = el.getContext("2d");
  if (!ctx) return;
  ctx.fillStyle = "#101216";
  ctx.fillRect(0, 0, cw, ch);
  if (!decoded) {
    ctx.fillStyle = "#f55";
    ctx.font = "14px monospace";
    ctx.fillText("Failed to decode TIFF bytes.", 12, 28);
    return;
  }
  const { width, height, channels, data } = decoded;
  const v = effectiveView.value;
  let scale;
  if (v.zoomMode === "fit") scale = Math.min(cw / width, ch / height);
  else if (v.zoomMode === "1:1") scale = 1;
  else scale = v.zoomLevel;
  const dispW = Math.max(1, Math.round(width * scale));
  const dispH = Math.max(1, Math.round(height * scale));
  const baseX = Math.floor((cw - dispW) / 2);
  const baseY = Math.floor((ch - dispH) / 2);
  const offX = baseX + Math.round(v.panX);
  const offY = baseY + Math.round(v.panY);
  ctx.imageSmoothingEnabled = scale < 4;
  const range = Math.max(1e-6, clipHigh.value - clipLow.value);
  const imgData = ctx.createImageData(dispW, dispH);
  const sample = (xi, yi, c) => {
    const fx = (xi / dispW) * (width - 1);
    const fy = (yi / dispH) * (height - 1);
    const x0 = Math.floor(fx);
    const y0 = Math.floor(fy);
    const x1 = Math.min(width - 1, x0 + 1);
    const y1 = Math.min(height - 1, y0 + 1);
    const tx = fx - x0;
    const ty = fy - y0;
    const idx = (x, y) => c + (y * width + x) * channels;
    const v00 = data[idx(x0, y0)];
    const v01 = data[idx(x1, y0)];
    const v10 = data[idx(x0, y1)];
    const v11 = data[idx(x1, y1)];
    return v00 * (1 - tx) * (1 - ty) + v01 * tx * (1 - ty) + v10 * (1 - tx) * ty + v11 * tx * ty;
  };
  for (let y = 0; y < dispH; y++) {
    for (let x = 0; x < dispW; x++) {
      const i = (y * dispW + x) * 4;
      if (channels === 1) {
        const val = sample(x, y, 0);
        const c = Math.max(0, Math.min(1, (val - clipLow.value) / range)) * 255;
        imgData.data[i] = c;
        imgData.data[i + 1] = c;
        imgData.data[i + 2] = c;
        imgData.data[i + 3] = 255;
      } else {
        const r = sample(x, y, 0);
        const g = sample(x, y, 1);
        const b = sample(x, y, 2);
        imgData.data[i] = Math.max(0, Math.min(255, ((r - clipLow.value) / range) * 255));
        imgData.data[i + 1] = Math.max(0, Math.min(255, ((g - clipLow.value) / range) * 255));
        imgData.data[i + 2] = Math.max(0, Math.min(255, ((b - clipLow.value) / range) * 255));
        imgData.data[i + 3] = 255;
      }
    }
  }
  ctx.putImageData(imgData, offX, offY);
}

watch(
  () => props.versionId,
  (vid) => {
    if (vid) void loadArtifact(vid);
  },
  { immediate: true },
);

watch(
  () => props.view,
  () => scheduleDraw(),
  { deep: true },
);

onMounted(() => {
  if (canvasEl.value && typeof ResizeObserver !== "undefined") {
    resizeObserver = new ResizeObserver(() => scheduleDraw());
    resizeObserver.observe(canvasEl.value);
  }
});

onBeforeUnmount(() => {
  if (rafId) cancelAnimationFrame(rafId);
  resizeObserver?.disconnect();
  if (glContext && glState) destroyImageCanvasGl(glContext, glState);
  glContext = null;
  glState = null;
});
</script>

<style scoped>
.image-canvas-wrap {
  display: flex;
  flex-direction: column;
  gap: var(--sp-xs);
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.canvas-toolbar {
  display: flex;
  align-items: center;
  gap: var(--sp-xs);
  padding: var(--sp-xs) var(--sp-sm);
  border-bottom: 1px solid var(--outline-variant);
  flex-wrap: wrap;
}

.tool-btn {
  background: var(--surface-container);
  border: 1px solid var(--outline-variant);
  color: var(--on-surface);
  border-radius: var(--radius-sm);
  padding: 2px 10px;
  cursor: pointer;
  font-size: 0.8rem;
}

.tool-btn[data-active="true"] {
  border-color: var(--primary);
  color: var(--primary);
}

.zoom-label {
  font-size: 0.75rem;
  color: var(--on-surface-variant);
  min-width: 44px;
  text-align: center;
}

.slider-label {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 0.7rem;
  color: var(--on-surface-variant);
}

.backend-label {
  margin-left: auto;
  font-size: 0.7rem;
  color: var(--outline);
}

.canvas-host {
  position: relative;
  height: 420px;
}

.canvas {
  width: 100%;
  height: 100%;
  display: block;
  touch-action: none;
  cursor: grab;
}

.canvas:active {
  cursor: grabbing;
}

.overlay-msg {
  position: absolute;
  top: var(--sp-sm);
  left: var(--sp-sm);
  margin: 0;
  font-size: 0.8rem;
  color: var(--on-surface-variant);
  background: rgba(0, 0, 0, 0.55);
  padding: 2px 8px;
  border-radius: var(--radius-sm);
}

.overlay-msg.error {
  color: #ff8a80;
}
</style>
