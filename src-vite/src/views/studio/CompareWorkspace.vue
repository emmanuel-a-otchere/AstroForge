<template>
  <WorkspaceScreen
    title="Compare"
    icon="compare"
    description="Compare two image versions side by side, inspect metrics, and record which one you keep."
  >
    <div v-if="versions.versions.length < 2" class="empty-state">
      <span class="material-symbols-outlined empty-icon" aria-hidden="true">compare</span>
      <p class="empty-title font-display">Need at least two versions to compare</p>
      <p class="empty-body font-body">
        Run the pipeline to produce image versions, then come back here to
        compare them.
      </p>
    </div>

    <template v-else>
      <div class="pickers">
        <label class="picker font-body">
          Version A
          <select v-model="aId" class="select">
            <option v-for="v in versions.versions" :key="v.version_id" :value="v.version_id">
              {{ v.label }} (seq {{ v.sequence }})
            </option>
          </select>
        </label>
        <label class="picker font-body">
          Version B
          <select v-model="bId" class="select">
            <option v-for="v in versions.versions" :key="v.version_id" :value="v.version_id">
              {{ v.label }} (seq {{ v.sequence }})
            </option>
          </select>
        </label>
        <button type="button" class="secondary-cta font-display" @click="loadMetrics">
          Compare metrics
        </button>
      </div>

      <p v-if="metricsError" class="error-line font-body" role="alert">{{ metricsError }}</p>

      <table v-if="metrics" class="metrics font-body" aria-label="Version metrics comparison">
        <thead>
          <tr>
            <th scope="col">Metric</th>
            <th scope="col">A ({{ versionA?.label }})</th>
            <th scope="col">B ({{ versionB?.label }})</th>
            <th scope="col">Δ</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in metricRows" :key="row.key">
            <th scope="row">{{ row.key }}</th>
            <td>{{ row.a }}</td>
            <td>{{ row.b }}</td>
            <td>{{ row.delta }}</td>
          </tr>
        </tbody>
      </table>

      <section class="decision" aria-label="Decision">
        <button
          type="button"
          class="accept-cta font-display"
          :disabled="!versionA || decisionBusy"
          @click="decide('promoted')"
        >
          Keep A
        </button>
        <button
          type="button"
          class="accept-cta font-display"
          :disabled="!versionB || decisionBusy"
          @click="decide('promoted', versionB)"
        >
          Keep B
        </button>
        <p v-if="decisionError" class="error-line font-body" role="alert">{{ decisionError }}</p>
      </section>
    </template>
  </WorkspaceScreen>
</template>

<script setup>
import { computed, ref, watch } from "vue";
import WorkspaceScreen from "@/components/WorkspaceScreen.vue";
import { useStore } from "@/composables/useStore";
import { versionStore } from "@/state/versions";
import { compareVersionMetrics, applyImageDecision } from "@/lib/astroforge-api";

const versions = useStore(versionStore);

const aId = ref(null);
const bId = ref(null);
const metrics = ref(null);
const metricsError = ref(null);
const decisionBusy = ref(false);
const decisionError = ref(null);

const versionA = computed(() => versions.value.versions.find((v) => v.version_id === aId.value) ?? null);
const versionB = computed(() => versions.value.versions.find((v) => v.version_id === bId.value) ?? null);

const metricRows = computed(() => {
  if (!metrics.value) return [];
  const m = metrics.value;
  const keys = new Set([...Object.keys(m.a ?? {}), ...Object.keys(m.b ?? {})]);
  return [...keys].map((key) => {
    const a = m.a?.[key];
    const b = m.b?.[key];
    const delta =
      typeof a === "number" && typeof b === "number" ? (b - a).toPrecision(4) : "—";
    return { key, a: fmt(a), b: fmt(b), delta };
  });
});

function fmt(v) {
  if (v === undefined || v === null) return "—";
  return typeof v === "number" ? v.toPrecision(4) : String(v);
}

watch(
  () => versions.value.versions,
  (list) => {
    if (list.length >= 2 && aId.value === null && bId.value === null) {
      aId.value = list[0].version_id;
      bId.value = list[list.length - 1].version_id;
    }
  },
  { immediate: true },
);

async function loadMetrics() {
  if (!aId.value || !bId.value) return;
  metricsError.value = null;
  try {
    metrics.value = await compareVersionMetrics(aId.value, bId.value);
  } catch (e) {
    metricsError.value = e instanceof Error ? e.message : String(e);
  }
}

async function decide(state, version = versionA.value) {
  if (!version) return;
  decisionBusy.value = true;
  decisionError.value = null;
  try {
    await applyImageDecision(version.version_id, state, "user decision in Compare");
  } catch (e) {
    decisionError.value = e instanceof Error ? e.message : String(e);
  } finally {
    decisionBusy.value = false;
  }
}
</script>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: var(--sp-xl);
  background: var(--surface-container-low);
  border: 1px dashed var(--outline-variant);
  border-radius: var(--radius-lg);
  gap: var(--sp-md);
}

.empty-icon {
  font-size: 48px;
  color: var(--on-surface-variant);
}

.empty-title {
  font-size: 1.25rem;
  margin: 0;
}

.empty-body {
  margin: 0;
  color: var(--on-surface-variant);
  max-width: 60ch;
}

.pickers {
  display: flex;
  align-items: flex-end;
  gap: var(--sp-md);
  flex-wrap: wrap;
}

.picker {
  display: flex;
  flex-direction: column;
  gap: var(--sp-xs);
  font-size: 0.85rem;
  color: var(--on-surface-variant);
}

.select {
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-md);
  color: var(--on-surface);
  padding: var(--sp-xs) var(--sp-sm);
  font-family: var(--font-body);
}

.metrics {
  border-collapse: collapse;
  width: 100%;
  font-size: 0.85rem;
}

.metrics th,
.metrics td {
  text-align: left;
  padding: var(--sp-xs) var(--sp-sm);
  border-bottom: 1px solid var(--outline-variant);
}

.metrics thead th {
  color: var(--on-surface-variant);
  font-weight: 600;
}

.decision {
  display: flex;
  gap: var(--sp-sm);
  align-items: center;
}

.accept-cta {
  background: var(--primary);
  color: var(--on-primary);
  border: none;
  padding: var(--sp-xs) var(--sp-lg);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-weight: 600;
}

.secondary-cta {
  background: transparent;
  color: var(--on-surface-variant);
  border: 1px solid var(--outline-variant);
  padding: var(--sp-xs) var(--sp-lg);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-weight: 600;
}

button:disabled {
  opacity: 0.5;
  cursor: default;
}

.error-line {
  color: #ff8a80;
  margin: 0;
}
</style>
