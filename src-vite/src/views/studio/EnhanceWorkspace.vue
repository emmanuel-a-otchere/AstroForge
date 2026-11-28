<template>
  <WorkspaceScreen
    title="Enhance"
    icon="tune"
    description="AI-assisted recommendations that propose tweaks to the latest image version. Accept or reject each suggestion; every accepted change is recorded on the version timeline."
  >
    <div v-if="versions.recommendations.length === 0" class="empty-state">
      <span class="material-symbols-outlined empty-icon" aria-hidden="true">auto_awesome</span>
      <p class="empty-title font-display">No recommendations yet</p>
      <p class="empty-body font-body">
        Once a pipeline run completes, AstroForge analyses the result and
        proposes a small set of targeted enhancements.
      </p>
    </div>
    <ul v-else class="rec-list" aria-label="AI recommendations">
      <li v-for="rec in versions.recommendations" :key="rec.id">
        <RecommendationCard :recommendation="rec" />
      </li>
    </ul>
  </WorkspaceScreen>
</template>

<script setup>
import WorkspaceScreen from "@/components/WorkspaceScreen.vue";
import RecommendationCard from "./RecommendationCard.vue";
import { useStore } from "@/composables/useStore";
import { versionStore } from "@/state/versions";

const versions = useStore(versionStore);
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

.rec-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: var(--sp-md);
}
</style>
