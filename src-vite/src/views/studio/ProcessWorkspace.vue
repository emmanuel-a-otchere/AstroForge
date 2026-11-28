<template>
  <WorkspaceScreen
    title="Process"
    icon="auto_fix_high"
    description="Run the processing pipeline for the active session. Generate an auto-plan, review stages, and start the run."
  >
    <div v-if="historyError" class="error-line font-body" role="alert">
      {{ historyError }}
    </div>

    <section class="panel" aria-labelledby="sessions-title">
      <h2 id="sessions-title" class="font-display panel-title">Sessions</h2>
      <p v-if="historyLoading" class="hint font-body">Loading sessions…</p>
      <ul v-else-if="sessionHistory.length" class="session-list">
        <li v-for="entry in sessionHistory" :key="entry.sessionId" class="session-row">
          <div>
            <span class="font-display">{{ entry.name }}</span>
            <span class="hint font-body"> · {{ entry.processingStatus }}</span>
          </div>
          <button
            v-if="!entry.isActive"
            type="button"
            class="ghost-link font-body"
            @click="continueSession(entry)"
          >
            Continue
          </button>
          <span v-else class="status-pill" data-status="active">active</span>
        </li>
      </ul>
      <p v-else class="hint font-body">No sessions yet — import data first.</p>
    </section>

    <section class="panel" aria-labelledby="plan-title">
      <div class="panel-header">
        <h2 id="plan-title" class="font-display panel-title">Pipeline plan</h2>
        <div class="row-actions">
          <button
            type="button"
            class="secondary-cta font-display"
            :disabled="!canGenerateAutoPlan || planBusy"
            @click="generateAutoPlan"
          >
            Generate auto-plan
          </button>
          <button
            v-if="plan && runStatus !== 'running'"
            type="button"
            class="primary-cta font-display"
            :disabled="planBusy"
            @click="startRun"
          >
            Start run
          </button>
          <button
            v-if="runStatus === 'running'"
            type="button"
            class="secondary-cta font-display"
            :disabled="planBusy"
            @click="pauseRun"
          >
            Pause
          </button>
          <button
            v-if="runStatus === 'running'"
            type="button"
            class="danger-cta font-display"
            :disabled="planBusy"
            @click="cancelRun"
          >
            Cancel
          </button>
        </div>
      </div>
      <p v-if="planError" class="error-line font-body" role="alert">{{ planError }}</p>
      <p v-else-if="!plan" class="hint font-body">
        No active plan. Generate one from the active session.
      </p>
      <template v-else>
        <p class="hint font-body">
          Plan {{ plan.plan_id }} · status {{ runStatus }}
        </p>
        <VueDraggable
          v-model="stageOrder"
          class="stage-list"
          handle=".drag-handle"
          :animation="120"
          @end="onStageReorder"
        >
          <li v-for="stage in stageOrder" :key="stage.stage_id" class="stage-row">
            <span class="material-symbols-outlined drag-handle" aria-hidden="true">drag_indicator</span>
            <span
              class="material-symbols-outlined"
              :class="statusFor(stage.stage_id)"
              aria-hidden="true"
            >
              {{ statusFor(stage.stage_id) === "completed" ? "check_circle" : statusFor(stage.stage_id) === "failed" ? "error" : statusFor(stage.stage_id) === "running" ? "sync" : "schedule" }}
            </span>
            <span class="font-body">{{ stage.label || stage.stage_type }}</span>
            <span class="hint font-body">{{ statusFor(stage.stage_id) }}</span>
          </li>
        </VueDraggable>
        <details class="palette">
          <summary class="font-body">Insert stage</summary>
          <div class="palette-grid">
            <button
              v-for="entry in catalog"
              :key="entry.stage_type"
              type="button"
              class="palette-item font-body"
              :disabled="insertInFlight"
              @click="insertStageAt(entry)"
            >
              {{ entry.label }}
            </button>
          </div>
          <p v-if="insertError" class="error-line font-body" role="alert">{{ insertError }}</p>
        </details>
      </template>
    </section>
  </WorkspaceScreen>
</template>

<script setup>
import { computed, onMounted, ref, watch } from "vue";
import { VueDraggable } from "vue-draggable-plus";
import WorkspaceScreen from "@/components/WorkspaceScreen.vue";
import { useStore } from "@/composables/useStore";
import { useProjectContextStore } from "@/stores/projectContext";
import {
  activePlan,
  runStatus as runStatusStore,
  stageExecutions,
} from "@/lib/pipeline-plan-store";
import { loadNodeCatalog } from "@/lib/node-catalog";
import {
  sessionListForProject,
  sessionContinue,
  projectGet,
  createPipelinePlan,
  startPipelinePlan,
  pausePipelinePlan,
  cancelPipelinePlan,
  pipelinePlanListStageExecutions,
  insertStage,
  reorderStage,
} from "@/lib/astroforge-api";

const context = useProjectContextStore();
const plan = useStore(activePlan);
const runStatus = useStore(runStatusStore);
const execsStore = useStore(stageExecutions);

const sessionHistory = ref([]);
const historyLoading = ref(false);
const historyError = ref(null);
const planError = ref(null);
const planBusy = ref(false);
const insertError = ref(null);
const insertInFlight = ref(false);

const project = computed(() => context.project);
const sessionId = computed(() => project.value?.active_session_id ?? null);
const canGenerateAutoPlan = computed(() => Boolean(project.value && sessionId.value));
const catalog = ref([]);
const stageOrder = ref([]);

const stageExecs = computed(() => {
  if (!plan.value) return [];
  return execsStore.value[plan.value.plan_id] ?? [];
});

function statusFor(stageId) {
  const exec = stageExecs.value.find((e) => e.stage_id === stageId);
  return exec?.status ?? "pending";
}

function syncStageOrder() {
  stageOrder.value = [...(plan.value?.stages ?? [])].sort((a, b) => a.sequence - b.sequence);
}

async function onStageReorder(evt) {
  if (!plan.value || evt.oldIndex === undefined || evt.newIndex === undefined) return;
  if (evt.oldIndex === evt.newIndex) return;
  const moved = stageOrder.value[evt.newIndex];
  if (!moved) return;
  try {
    const updated = await reorderStage(plan.value.plan_id, moved.stage_id, evt.newIndex);
    activePlan.set(updated);
  } catch (e) {
    planError.value = e instanceof Error ? e.message : String(e);
    syncStageOrder();
  }
}

async function loadHistory() {
  if (!project.value) return;
  historyLoading.value = true;
  historyError.value = null;
  try {
    sessionHistory.value = await sessionListForProject(project.value.project_id);
  } catch (e) {
    historyError.value = e instanceof Error ? e.message : String(e);
  } finally {
    historyLoading.value = false;
  }
}

async function continueSession(entry) {
  if (!project.value) return;
  historyError.value = null;
  try {
    await sessionContinue(project.value.project_id, entry.sessionId);
    context.open(await projectGet(project.value.project_id));
    await loadHistory();
  } catch (e) {
    historyError.value = e instanceof Error ? e.message : String(e);
  }
}

async function refreshExecs() {
  if (!plan.value) return;
  try {
    const execs = await pipelinePlanListStageExecutions(plan.value.plan_id);
    stageExecutions.update((map) => ({ ...map, [plan.value.plan_id]: execs }));
  } catch {
    // non-fatal: the stage list just stays stale
  }
}

async function generateAutoPlan() {
  if (!project.value || !sessionId.value) return;
  planBusy.value = true;
  planError.value = null;
  try {
    const created = await createPipelinePlan({
      project_id: project.value.project_id,
      session_id: sessionId.value,
    });
    activePlan.set(created);
    await refreshExecs();
  } catch (e) {
    planError.value = e instanceof Error ? e.message : String(e);
  } finally {
    planBusy.value = false;
  }
}

async function startRun() {
  if (!plan.value) return;
  planBusy.value = true;
  planError.value = null;
  try {
    await startPipelinePlan(plan.value.plan_id);
    await refreshExecs();
  } catch (e) {
    planError.value = e instanceof Error ? e.message : String(e);
  } finally {
    planBusy.value = false;
  }
}

async function pauseRun() {
  if (!plan.value) return;
  planBusy.value = true;
  try {
    await pausePipelinePlan(plan.value.plan_id);
  } catch (e) {
    planError.value = e instanceof Error ? e.message : String(e);
  } finally {
    planBusy.value = false;
  }
}

async function cancelRun() {
  if (!plan.value) return;
  planBusy.value = true;
  try {
    await cancelPipelinePlan(plan.value.plan_id);
  } catch (e) {
    planError.value = e instanceof Error ? e.message : String(e);
  } finally {
    planBusy.value = false;
  }
}

async function insertStageAt(entry) {
  if (!plan.value) {
    insertError.value = "Create or activate a plan before inserting stages.";
    return;
  }
  insertInFlight.value = true;
  insertError.value = null;
  try {
    const updated = await insertStage({
      plan_id: plan.value.plan_id,
      stage_type: entry.stage_type,
    });
    activePlan.set(updated);
    await refreshExecs();
  } catch (e) {
    insertError.value = e instanceof Error ? e.message : String(e);
  } finally {
    insertInFlight.value = false;
  }
}

watch(plan, (p) => {
  syncStageOrder();
  if (p) void refreshExecs();
});

onMounted(() => {
  void loadHistory();
  loadNodeCatalog()
    .then((c) => (catalog.value = c.entries))
    .catch(() => (catalog.value = []));
});
</script>

<style scoped>
.panel {
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-lg);
  padding: var(--sp-lg);
  display: flex;
  flex-direction: column;
  gap: var(--sp-sm);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-md);
  flex-wrap: wrap;
}

.panel-title {
  font-size: 1.05rem;
  margin: 0;
}

.session-list,
.stage-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--sp-xs);
}

.session-row,
.stage-row {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-xs) var(--sp-sm);
  border-radius: var(--radius-sm);
}

.drag-handle {
  cursor: grab;
  color: var(--outline);
  font-size: 18px;
}

.stage-row .completed {
  color: #10b981;
}

.stage-row .failed {
  color: #e53935;
}

.stage-row .running {
  color: #7787f9;
}

.row-actions {
  display: flex;
  gap: var(--sp-sm);
}

.primary-cta {
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

.danger-cta {
  background: #e53935;
  color: #fff;
  border: none;
  padding: var(--sp-xs) var(--sp-lg);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-weight: 600;
}

button:disabled {
  opacity: 0.5;
  cursor: default;
}

.ghost-link {
  background: transparent;
  border: none;
  color: var(--primary);
  cursor: pointer;
  font-size: 0.85rem;
}

.status-pill {
  font-size: 0.7rem;
  padding: 2px 8px;
  border-radius: var(--radius-full);
  background: var(--primary-container);
  color: var(--on-primary-container);
}

.palette summary {
  cursor: pointer;
  color: var(--on-surface-variant);
}

.palette-grid {
  display: flex;
  flex-wrap: wrap;
  gap: var(--sp-xs);
  margin-top: var(--sp-sm);
}

.palette-item {
  background: var(--surface-container);
  border: 1px solid var(--outline-variant);
  color: var(--on-surface);
  border-radius: var(--radius-md);
  padding: var(--sp-xs) var(--sp-md);
  cursor: pointer;
  font-size: 0.85rem;
}

.palette-item:hover {
  border-color: var(--primary);
}

.hint {
  color: var(--on-surface-variant);
  font-size: 0.85rem;
  margin: 0;
}

.error-line {
  color: #ff8a80;
  margin: 0;
}
</style>
