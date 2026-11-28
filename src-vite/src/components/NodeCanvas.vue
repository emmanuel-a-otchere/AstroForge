<template>
  <section class="node-canvas" aria-label="Pipeline node canvas (free-form)">
    <div class="canvas-toolbar">
      <span class="hint font-body">drag cards · wheel zooms · background pans</span>
      <button type="button" class="tool-btn font-body" @click="resetView">
        Reset layout
      </button>
    </div>
    <p v-if="!plan || stages.length === 0" class="empty font-body">
      No stages yet — switch to the list to insert from the palette.
    </p>
    <svg
      v-else
      class="canvas"
      viewBox="0 0 1000 700"
      role="img"
      aria-label="Pipeline graph"
      @pointerdown="onBackgroundPointerDown"
      @wheel.prevent="onWheel"
    >
      <g :transform="viewTransform(pan, zoom)">
        <path v-for="w in wires" :key="w.key" class="wire" :d="w.d" />
        <g
          v-for="node in nodes"
          :key="node.stage_id"
          class="node"
          :class="{ 'node-disabled': !node.enabled }"
          :transform="`translate(${node.x} ${node.y})`"
          @pointerdown="(e) => onNodePointerDown(e, node)"
        >
          <rect
            :width="NODE_WIDTH"
            :height="NODE_HEIGHT"
            rx="8"
            :fill="node.enabled ? 'var(--surface-container-low)' : 'var(--surface-container)'"
            :stroke="
              draggingId === node.stage_id
                ? 'var(--cobalt-accent)'
                : 'var(--outline-variant)'
            "
          />
          <text class="node-label" x="12" y="21">{{ node.label }}</text>
          <text class="node-type" x="12" y="38">{{ node.stage_type }}</text>
          <text
            v-if="!node.enabled"
            class="node-off"
            :x="NODE_WIDTH - 12"
            y="21"
            text-anchor="end"
            >off</text
          >
        </g>
      </g>
    </svg>
  </section>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import {
  bezierPath,
  clampNodePos,
  clampZoom,
  defaultLayout,
  NODE_HEIGHT,
  NODE_WIDTH,
  screenToWorld,
  viewTransform,
} from "@/lib/node-canvas-logic";
import { activePlan } from "@/lib/pipeline-plan-store";
import { useStore } from "@/composables/useStore";

const plan = useStore(activePlan);

const pan = ref({ x: 0, y: 0 });
const zoom = ref(1);
const arranged = ref({});
const arrangedPlanId = ref(null);
const draggingId = ref(null);
const dragOffset = ref({ x: 0, y: 0 });
const panning = ref(false);
const panStart = ref({ x: 0, y: 0 });

const stages = computed(() =>
  plan.value ? [...plan.value.stages].sort((a, b) => a.sequence - b.sequence) : [],
);

const nodes = computed(() => defaultLayout(stages.value, arranged.value));
const wires = computed(() =>
  nodes.value.slice(0, -1).map((n, i) => ({
    d: bezierPath(n, nodes.value[i + 1]),
    key: `${n.stage_id}->${nodes.value[i + 1].stage_id}`,
  })),
);

function loadArrangement(planId) {
  try {
    const raw = localStorage.getItem(`node-canvas-${planId}`);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

function saveArrangement(planId) {
  try {
    localStorage.setItem(`node-canvas-${planId}`, JSON.stringify(arranged.value));
  } catch {
    // storage full or unavailable — arrangement is best-effort
  }
}

watchPlan();
function watchPlan() {
  const pid = computed(() => plan.value?.plan_id ?? null);
  // reset arrangement when the plan changes
  let last = null;
  const check = () => {
    const p = pid.value;
    if (p === last) return;
    last = p;
    arranged.value = p ? loadArrangement(p) : {};
    pan.value = { x: 0, y: 0 };
    zoom.value = 1;
  };
  check();
  const unsub = activePlan.subscribe(() => check());
  onBeforeUnmount(unsub);
}

function onNodePointerDown(event, node) {
  event.stopPropagation();
  event.target.setPointerCapture?.(event.pointerId);
  const world = screenToWorld(event.clientX, event.clientY, pan.value, zoom.value);
  draggingId.value = node.stage_id;
  dragOffset.value = { x: world.x - node.x, y: world.y - node.y };
}

function onBackgroundPointerDown(event) {
  panning.value = true;
  panStart.value = { x: event.clientX - pan.value.x, y: event.clientY - pan.value.y };
}

function onPointerMove(event) {
  if (draggingId.value && plan.value) {
    const world = screenToWorld(event.clientX, event.clientY, pan.value, zoom.value);
    const next = clampNodePos(
      world.x - dragOffset.value.x,
      world.y - dragOffset.value.y,
      2000,
      4000,
    );
    arranged.value = { ...arranged.value, [draggingId.value]: next };
  } else if (panning.value) {
    pan.value = {
      x: event.clientX - panStart.value.x,
      y: event.clientY - panStart.value.y,
    };
  }
}

function onPointerUp() {
  if (draggingId.value && plan.value) saveArrangement(plan.value.plan_id);
  draggingId.value = null;
  panning.value = false;
}

function onWheel(event) {
  const factor = event.deltaY < 0 ? 1.1 : 1 / 1.1;
  zoom.value = clampZoom(zoom.value * factor);
}

function resetView() {
  arranged.value = {};
  if (plan.value) saveArrangement(plan.value.plan_id);
  pan.value = { x: 0, y: 0 };
  zoom.value = 1;
}

onMounted(() => {
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp);
});

onBeforeUnmount(() => {
  window.removeEventListener("pointermove", onPointerMove);
  window.removeEventListener("pointerup", onPointerUp);
});
</script>

<style scoped>
.node-canvas {
  display: flex;
  flex-direction: column;
  gap: var(--sp-sm);
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-lg);
  padding: var(--sp-md);
}

.canvas-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-sm);
}

.hint {
  font-size: 0.78rem;
  color: var(--on-surface-variant);
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

.empty {
  color: var(--on-surface-variant);
  padding: var(--sp-lg);
  text-align: center;
}

.canvas {
  width: 100%;
  height: 480px;
  background: var(--surface-container-lowest);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-md);
  touch-action: none;
}

.wire {
  fill: none;
  stroke: var(--outline);
  stroke-width: 2;
}

.node {
  cursor: grab;
}

.node-label {
  fill: var(--on-surface);
  font-size: 13px;
  font-family: var(--font-display);
  font-weight: 600;
}

.node-type {
  fill: var(--on-surface-variant);
  font-size: 10px;
  font-family: var(--font-data);
}

.node-off {
  fill: var(--outline);
  font-size: 10px;
  font-family: var(--font-data);
}

.node-disabled rect {
  opacity: 0.6;
}
</style>
