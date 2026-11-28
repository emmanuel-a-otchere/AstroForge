<template>
  <section class="overview" aria-labelledby="overview-title">
    <header class="overview-header">
      <h1 id="overview-title" class="font-display">Project Overview</h1>
      <p class="overview-subtitle font-body">
        Track where the project is in the pipeline. Status is derived from
        the durable project store (event log + per-table fallbacks).
      </p>
    </header>

    <div v-if="project" class="project-summary">
      <span class="font-label">Project</span>
      <span class="project-name font-display">{{ project.project.name }}</span>
      <span class="status-pill" :data-status="project.project.status">
        {{ project.project.status }}
      </span>
    </div>

    <ol class="checklist" aria-label="Processing-stage checklist">
      <li
        v-for="(stage, idx) in stageDefs"
        :key="stage.id"
        class="checklist-item"
        :data-status="statuses[stage.id]"
      >
        <div class="step-number font-label">{{ idx + 1 }}</div>
        <div class="step-body">
          <h2 class="step-title font-display">{{ stage.label }}</h2>
          <p class="step-description font-body">{{ stage.description }}</p>
          <p class="step-detail font-body" :data-stage="stage.id">
            {{ detailFor(stage.id) }}
          </p>
        </div>
        <div class="step-state" :aria-label="`Status: ${statuses[stage.id]}`">
          <span
            class="material-symbols-outlined state-icon"
            :class="statuses[stage.id]"
            aria-hidden="true"
          >
            {{ statuses[stage.id] === "complete" ? "check_circle" : statuses[stage.id] === "blocked" ? "block" : "schedule" }}
          </span>
        </div>
      </li>
    </ol>

    <footer class="overview-footer">
      <p class="hint font-body">
        The Overview reflects the durable project state. A manual refresh
        re-queries the durable state.
      </p>
      <button
        type="button"
        class="refresh-cta font-body"
        :disabled="ws.overviewLoading"
        @click="ws.refreshOverview()"
      >
        <span class="material-symbols-outlined" aria-hidden="true">refresh</span>
        Refresh overview
      </button>
    </footer>

    <section class="timeline-section" aria-labelledby="timeline-title">
      <h2 id="timeline-title" class="font-display section-title">Version timeline</h2>
      <p class="section-subtitle font-body">
        Each version captures a snapshot of the project's working image and
        the action that produced it.
      </p>
      <VersionTimeline />
    </section>
  </section>
</template>

<script setup>
import { computed } from "vue";
import VersionTimeline from "./VersionTimeline.vue";
import { useStore } from "@/composables/useStore";
import { useProjectContextStore } from "@/stores/projectContext";
import { stageStatuses, workspaceState } from "@/state/workspace";

const context = useProjectContextStore();
const ws = useStore(workspaceState);
const statuses = useStore(stageStatuses);

const project = computed(() => (context.project ? { project: context.project } : null));

const stageDefs = [
  { id: "import", label: "Import", description: "Source files loaded into the project workspace." },
  { id: "analyze", label: "Analyze", description: "Plate-solve, calibration, and target classification." },
  { id: "process", label: "Process", description: "At least one completed pipeline run." },
  { id: "review", label: "Review", description: "Final image version is rendered and reviewed." },
  { id: "export", label: "Export", description: "At least one image version exported." },
];

function detailFor(id) {
  const overview = ws.value.overview;
  if (!overview) return "Awaiting project state…";
  if (ws.value.overviewLoading) return "Loading…";
  if (ws.value.overviewError) return "Overview unavailable: " + ws.value.overviewError;
  switch (id) {
    case "import":
      return overview.imported ? "Source assets registered on the project." : "No source assets yet. Import a folder to begin.";
    case "analyze":
      return overview.analyzed ? "Session analysis complete." : "No analysis event recorded yet.";
    case "review":
      return overview.versioned ? "A final image version is recorded." : "No image version recorded yet.";
    case "export":
      return overview.exported ? "An export event is recorded." : "No export recorded yet.";
    case "process":
      return "";
  }
}
</script>

<style scoped>
.overview {
  display: flex;
  flex-direction: column;
  gap: var(--sp-lg);
  padding: var(--sp-xl);
  max-width: 1100px;
  margin: 0 auto;
  width: 100%;
  box-sizing: border-box;
  overflow-y: auto;
  height: 100%;
  color: var(--on-surface);
}

.overview-header h1 {
  font-size: 1.75rem;
  margin: 0 0 var(--sp-xs);
}

.overview-subtitle {
  margin: 0;
  color: var(--on-surface-variant);
  font-size: 0.95rem;
}

.project-summary {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-sm) var(--sp-md);
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-md);
  width: fit-content;
}

.project-name {
  font-weight: 600;
}

.status-pill {
  font-size: 0.7rem;
  padding: 2px 8px;
  border-radius: var(--radius-full);
  background: var(--surface-container-high);
  color: var(--on-surface-variant);
}

.checklist {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--sp-sm);
}

.checklist-item {
  display: flex;
  align-items: flex-start;
  gap: var(--sp-md);
  padding: var(--sp-md) var(--sp-lg);
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-md);
}

.step-number {
  font-size: 0.8rem;
  color: var(--on-surface-variant);
  background: var(--surface-container);
  border-radius: var(--radius-full);
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 auto;
}

.step-body {
  flex: 1 1 auto;
}

.step-title {
  margin: 0;
  font-size: 1.05rem;
}

.step-description {
  margin: 2px 0;
  font-size: 0.85rem;
  color: var(--on-surface-variant);
}

.step-detail {
  margin: 2px 0 0;
  font-size: 0.8rem;
  color: var(--on-surface-variant);
}

.state-icon.complete {
  color: #10b981;
}

.state-icon.blocked {
  color: #e53935;
}

.state-icon.pending {
  color: var(--outline);
}

.overview-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-md);
}

.hint {
  margin: 0;
  font-size: 0.8rem;
  color: var(--on-surface-variant);
  max-width: 70ch;
}

.refresh-cta {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-xs);
  background: transparent;
  border: 1px solid var(--outline-variant);
  color: var(--on-surface-variant);
  border-radius: var(--radius-md);
  padding: var(--sp-xs) var(--sp-md);
  cursor: pointer;
  font-size: 0.85rem;
}

.refresh-cta:hover {
  background: var(--surface-container);
  color: var(--on-surface);
}

.section-title {
  font-size: 1.3rem;
  margin: 0 0 var(--sp-xs);
}

.section-subtitle {
  margin: 0 0 var(--sp-md);
  color: var(--on-surface-variant);
  font-size: 0.9rem;
}
</style>
