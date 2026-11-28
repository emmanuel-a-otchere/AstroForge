<template>
  <p v-if="versions.versions.length === 0" class="empty font-body">
    No image versions yet. A version appears in the timeline when the
    project records a `VersionCreated` event.
  </p>
  <ol v-else class="timeline" aria-label="Image version timeline">
    <li
      v-for="version in versions.versions"
      :key="version.version_id"
      class="version-card"
      data-status="in_review"
    >
      <button
        type="button"
        class="version-button"
        :data-active="selected === version.version_id"
        :aria-label="`Select ${version.label}`"
        @click="selectVersion(version)"
      >
        <div class="version-header">
          <span class="version-label font-label">{{ version.label }}</span>
          <span class="status-pill" data-status="in_review">in review</span>
        </div>
        <p class="version-meta font-body">{{ formatDate(version.created_at) }}</p>
        <p class="version-source font-body">
          Sequence #{{ version.sequence }}
          {{ version.primary_artifact_id ? `· artifact ${version.primary_artifact_id}` : "· no artifact yet" }}
        </p>
      </button>
    </li>
  </ol>
</template>

<script setup>
import { ref } from "vue";
import { versionStore } from "@/state/versions";
import { useStore } from "@/composables/useStore";
import { formatDate } from "@/common/utils";

const emit = defineEmits(["select"]);
const versions = useStore(versionStore);
const selected = ref(null);

function selectVersion(v) {
  selected.value = v.version_id;
  emit("select", v);
}
</script>

<style scoped>
.empty {
  color: var(--on-surface-variant);
}

.timeline {
  list-style: none;
  padding: 0 0 var(--sp-xs);
  margin: 0;
  display: flex;
  gap: var(--sp-sm);
  overflow-x: auto;
}

.version-card {
  flex: 0 0 240px;
}

.version-button {
  width: 100%;
  text-align: left;
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-md);
  padding: var(--sp-sm) var(--sp-md);
  color: var(--on-surface);
  cursor: pointer;
}

.version-button[data-active="true"] {
  border-color: var(--primary);
}

.version-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.version-label {
  font-size: 0.75rem;
  color: var(--on-surface-variant);
}

.status-pill {
  font-size: 0.7rem;
  padding: 2px 8px;
  border-radius: var(--radius-full);
  background: var(--surface-container-high);
  color: var(--on-surface-variant);
}

.version-meta,
.version-source {
  margin: 2px 0 0;
  font-size: 0.78rem;
  color: var(--on-surface-variant);
}
</style>
