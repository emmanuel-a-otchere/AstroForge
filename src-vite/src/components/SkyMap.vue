<template>
  <div class="sky-map-wrap">
    <div ref="mapEl" class="sky-map" role="img" :aria-label="`Sky map of ${targetName ?? 'target'} at RA ${raDeg?.toFixed(3)}°, Dec ${decDeg?.toFixed(3)}°`"></div>
    <p class="hint font-body">
      {{ targetName ?? "Target" }} · RA {{ raDeg?.toFixed(3) }}° · Dec
      {{ decDeg?.toFixed(3) }}° · FOV {{ fovArcmin }}′
      <span v-if="source">· {{ source }}</span>
    </p>
  </div>
</template>

<script setup>
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import L from "leaflet";
import { parseDecDeg, parseRaDeg } from "@/common/utils";

const props = defineProps({
  raCenter: { type: String, default: null },
  decCenter: { type: String, default: null },
  fovArcmin: { type: Number, default: 0 },
  targetName: { type: String, default: null },
  source: { type: String, default: null },
});

const mapEl = ref(null);
let map = null;
let marker = null;
let fovCircle = null;

const raDeg = ref(parseRaDeg(props.raCenter));
const decDeg = ref(parseDecDeg(props.decCenter));

function render() {
  if (!map || raDeg.value === null || decDeg.value === null) return;
  const latlng = [decDeg.value, raDeg.value];
  if (marker) marker.setLatLng(latlng);
  else marker = L.circleMarker(latlng, { radius: 7, color: "#ffb4a8", fillOpacity: 0.9 }).addTo(map);
  const fovDeg = props.fovArcmin / 60;
  if (fovCircle) fovCircle.setLatLng(latlng).setRadius((fovDeg / 2) * 111320);
  else if (fovDeg > 0)
    fovCircle = L.circle(latlng, { radius: (fovDeg / 2) * 111320, color: "#7787f9", weight: 1, fillOpacity: 0.08 }).addTo(map);
  map.setView(latlng, 10);
}

watch(
  () => [props.raCenter, props.decCenter, props.fovArcmin],
  () => {
    raDeg.value = parseRaDeg(props.raCenter);
    decDeg.value = parseDecDeg(props.decCenter);
    render();
  },
);

onMounted(() => {
  map = L.map(mapEl.value, {
    zoomControl: true,
    attributionControl: false,
    minZoom: 2,
    maxZoom: 14,
    worldCopyJump: true,
  });
  render();
});

onBeforeUnmount(() => {
  map?.remove();
  map = null;
});
</script>

<style scoped>
.sky-map-wrap {
  display: flex;
  flex-direction: column;
  gap: var(--sp-xs);
}

.sky-map {
  height: 260px;
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-md);
  z-index: 0;
}

.hint {
  margin: 0;
  font-size: 0.78rem;
  color: var(--on-surface-variant);
}
</style>
