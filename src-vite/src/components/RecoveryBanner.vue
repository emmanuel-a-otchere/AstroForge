<template>
  <div v-if="resumables.length" class="recovery-banner" role="status">
    <span class="material-symbols-outlined" aria-hidden="true">pause_circle</span>
    <div class="banner-body">
      <p class="font-body">
        {{ resumables.length }} paused pipeline
        {{ resumables.length === 1 ? "run" : "runs" }} can resume.
      </p>
      <ul class="resumable-list">
        <li v-for="p in resumables" :key="p.plan_id">
          <span class="font-body">{{ p.plan_id.slice(0, 8) }}…</span>
          <button
            type="button"
            class="ghost-link font-body"
            :disabled="resumingId === p.plan_id"
            @click="onResume(p.plan_id)"
          >
            {{ resumingId === p.plan_id ? "Resuming…" : "Resume" }}
          </button>
          <button type="button" class="ghost-link font-body" @click="onDismiss(p.plan_id)">
            Dismiss
          </button>
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup>
import { onMounted, ref } from "vue";
import {
  loadPlan,
  refreshResumablePlans,
  refreshStageExecutions,
  resumablePlans,
  resumePipelineRun,
} from "@/lib/pipeline-plan-store";
import { useStore } from "@/composables/useStore";

const props = defineProps({
  projectId: { type: String, required: true },
});

const resumables = useStore(resumablePlans);
const resumingId = ref(null);

onMounted(() => {
  void refreshResumablePlans(props.projectId);
});

async function onResume(planId) {
  resumingId.value = planId;
  try {
    await resumePipelineRun(planId);
    await loadPlan(planId);
    await refreshStageExecutions(planId);
    await refreshResumablePlans(props.projectId);
  } finally {
    resumingId.value = null;
  }
}

function onDismiss(planId) {
  resumablePlans.update((list) => list.filter((p) => p.plan_id !== planId));
}
</script>

<style scoped>
.recovery-banner {
  display: flex;
  gap: var(--sp-sm);
  padding: var(--sp-md) var(--sp-lg);
  background: var(--tertiary-container);
  border: 1px solid var(--tertiary);
  border-radius: var(--radius-md);
  color: var(--on-tertiary-container);
}

.recovery-banner > .material-symbols-outlined {
  color: var(--on-tertiary-container);
}

.banner-body p {
  margin: 0;
  font-size: 0.88rem;
}

.resumable-list {
  list-style: none;
  margin: var(--sp-xs) 0 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.resumable-list li {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  font-size: 0.8rem;
}

.ghost-link {
  background: transparent;
  border: none;
  color: var(--on-tertiary-container);
  cursor: pointer;
  text-decoration: underline;
  font-size: 0.8rem;
  padding: 0;
}
</style>
