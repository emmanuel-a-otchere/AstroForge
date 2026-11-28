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

      <label class="sync-toggle font-body">
        <input v-model="syncNav" type="checkbox" />
        Sync zoom / pan
      </label>

      <div class="previews">
        <figure class="preview">
          <ImageCanvas
            :version-id="aId"
            :view="syncNav ? sharedView : null"
            @view-change="onViewChange"
          />
          <figcaption class="font-body">A · {{ versionA?.label ?? "—" }}</figcaption>
        </figure>
        <figure class="preview">
          <ImageCanvas
            :version-id="bId"
            :view="syncNav ? sharedView : null"
            @view-change="onViewChange"
          />
          <figcaption class="font-body">B · {{ versionB?.label ?? "—" }}</figcaption>
        </figure>
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
        <label class="picker font-body">
          Quality profile
          <select v-model="qualityProfile" class="select">
            <option v-for="q in qualityProfiles" :key="q" :value="q">{{ q }}</option>
          </select>
        </label>
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

      <section class="panel" aria-labelledby="sets-title">
        <div class="panel-header">
          <h2 id="sets-title" class="font-display panel-title">Comparison sets</h2>
          <div class="row-actions">
            <input
              v-model="setName"
              type="text"
              class="set-name font-body"
              placeholder="Set name"
              aria-label="Comparison set name"
            />
            <button
              type="button"
              class="secondary-cta font-display"
              :disabled="!aId || !bId || setsBusy"
              @click="saveSet"
            >
              Save A/B as set
            </button>
          </div>
        </div>
        <p v-if="setsError" class="error-line font-body" role="alert">{{ setsError }}</p>
        <ul v-if="sets.length" class="sets-list">
          <li v-for="s in sets" :key="s.id" class="set-row">
            <button type="button" class="ghost-link font-body" @click="loadSet(s)">
              {{ s.name }}
            </button>
            <span class="hint font-body">{{ s.version_ids.length }} versions</span>
            <button type="button" class="ghost-link danger font-body" @click="removeSet(s)">
              Delete
            </button>
          </li>
        </ul>
        <p v-else class="hint font-body">No saved comparison sets for this project.</p>
      </section>
    </template>
  </WorkspaceScreen>
</template>

<script setup>
import { computed, ref, watch } from "vue";
import WorkspaceScreen from "@/components/WorkspaceScreen.vue";
import ImageCanvas from "@/components/ImageCanvas.vue";
import { useStore } from "@/composables/useStore";
import { useProjectContextStore } from "@/stores/projectContext";
import { versionStore } from "@/state/versions";
import {
  compareVersionMetrics,
  applyImageDecision,
  listComparisonSetsForProject,
  saveComparisonSet,
  deleteComparisonSet,
  QUALITY_PROFILES,
  DEFAULT_QUALITY_PROFILE,
} from "@/lib/astroforge-api";

const versions = useStore(versionStore);
const context = useProjectContextStore();

const aId = ref(null);
const bId = ref(null);
const metrics = ref(null);
const metricsError = ref(null);
const decisionBusy = ref(false);
const decisionError = ref(null);

const syncNav = ref(true);
const sharedView = ref({ zoomMode: "fit", zoomLevel: 1, panX: 0, panY: 0 });

const qualityProfiles = QUALITY_PROFILES;
const qualityProfile = ref(DEFAULT_QUALITY_PROFILE);
const sets = ref([]);
const setName = ref("");
const setsBusy = ref(false);
const setsError = ref(null);

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

function onViewChange(view) {
  if (syncNav.value) sharedView.value = view;
}

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
    await applyImageDecision(
      version.version_id,
      state,
      "user decision in Compare",
      qualityProfile.value,
    );
  } catch (e) {
    decisionError.value = e instanceof Error ? e.message : String(e);
  } finally {
    decisionBusy.value = false;
  }
}

async function loadSets() {
  const project = context.project;
  if (!project) return;
  try {
    sets.value = await listComparisonSetsForProject(project.project_id);
  } catch (e) {
    setsError.value = e instanceof Error ? e.message : String(e);
  }
}

async function saveSet() {
  const project = context.project;
  if (!project || !aId.value || !bId.value) return;
  setsBusy.value = true;
  setsError.value = null;
  try {
    await saveComparisonSet({
      id: crypto.randomUUID(),
      project_id: project.project_id,
      name: setName.value.trim() || `A/B ${new Date().toISOString()}`,
      version_ids: [aId.value, bId.value],
      created_at: new Date().toISOString(),
      slot_labels: ["A", "B"],
    });
    setName.value = "";
    await loadSets();
  } catch (e) {
    setsError.value = e instanceof Error ? e.message : String(e);
  } finally {
    setsBusy.value = false;
  }
}

function loadSet(s) {
  if (s.version_ids.length >= 2) {
    aId.value = s.version_ids[0];
    bId.value = s.version_ids[1];
    void loadMetrics();
  }
}

async function removeSet(s) {
  setsError.value = null;
  try {
    await deleteComparisonSet(s.id);
    await loadSets();
  } catch (e) {
    setsError.value = e instanceof Error ? e.message : String(e);
  }
}

watch(
  () => context.project?.project_id,
  (pid) => {
    if (pid) void loadSets();
  },
  { immediate: true },
);
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

.sync-toggle {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-xs);
  font-size: 0.85rem;
  color: var(--on-surface-variant);
  width: fit-content;
}

.previews {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: var(--sp-md);
}

.preview {
  margin: 0;
}

.preview figcaption {
  padding: var(--sp-xs) var(--sp-sm);
  font-size: 0.8rem;
  color: var(--on-surface-variant);
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
