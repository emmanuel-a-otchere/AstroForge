<template>
  <WorkspaceScreen
    title="Export"
    icon="download"
    description="Export a final image version to the format of your choice. AstroForge keeps a full export history so you can re-export any past version at any time."
  >
    <div v-if="versions.versions.length === 0" class="empty-state">
      <span class="material-symbols-outlined empty-icon" aria-hidden="true">download</span>
      <p class="empty-title font-display">Nothing to export yet</p>
      <p class="empty-body font-body">
        Run a pipeline, accept any AI recommendations, and the final image
        version will be available to export here.
      </p>
    </div>
    <template v-else>
      <div class="export-grid">
        <section class="version-picker" aria-labelledby="version-pick-title">
          <h2 id="version-pick-title" class="font-display picker-title">Version</h2>
          <ul class="version-list">
            <li v-for="v in versions.versions" :key="v.version_id">
              <label class="version-row" :data-active="selectedVersionId === v.version_id">
                <input
                  v-model="selectedVersionId"
                  type="radio"
                  name="export-version"
                  :value="v.version_id"
                  class="version-radio"
                />
                <div class="version-info">
                  <span class="version-label font-label">{{ v.label }}</span>
                  <span class="version-title font-display">Sequence #{{ v.sequence }}</span>
                  <span class="version-meta font-body">
                    in review · {{ formatDate(v.created_at) }}
                    {{ v.primary_artifact_id ? `· ${v.primary_artifact_id}` : "· no artifact yet" }}
                  </span>
                </div>
              </label>
            </li>
          </ul>
        </section>

        <section class="format-picker" aria-labelledby="format-pick-title">
          <h2 id="format-pick-title" class="font-display picker-title">Format</h2>
          <ul class="format-list">
            <li v-for="fmt in FORMATS" :key="fmt.id">
              <label class="format-row" :data-active="selectedFormat === fmt.id">
                <input
                  v-model="selectedFormat"
                  type="radio"
                  name="export-format"
                  :value="fmt.id"
                  class="format-radio"
                />
                <div class="format-info">
                  <span class="format-label font-display">{{ fmt.label }}</span>
                  <span class="format-desc font-body">{{ fmt.description }}</span>
                </div>
              </label>
            </li>
          </ul>
        </section>
      </div>

      <footer class="export-footer">
        <button type="button" class="primary-cta font-display" disabled>
          Export…
        </button>
        <p class="hint font-body">
          Export IPC lands in P5a once the Rust side ships image-version
          export. Selected: {{ selectedFormat }}.
        </p>
      </footer>
    </template>
  </WorkspaceScreen>
</template>

<script setup>
import { ref, watch } from "vue";
import WorkspaceScreen from "@/components/WorkspaceScreen.vue";
import { useStore } from "@/composables/useStore";
import { latestVersion, versionStore } from "@/state/versions";
import { formatDate } from "@/common/utils";

const versions = useStore(versionStore);
const latest = useStore(latestVersion);

const FORMATS = [
  { id: "png16", label: "PNG (8-bit)", description: "Lossless, widely supported. Best for web sharing." },
  { id: "tiff32", label: "TIFF (16-bit)", description: "Lossless, archival. Preserves linear data." },
  { id: "fits", label: "FITS", description: "Astronomy standard. Preserves metadata + WCS." },
  { id: "jpeg", label: "JPEG (PNG container)", description: "Small, lossy. For previews and social media." },
];

const selectedFormat = ref("png16");
const selectedVersionId = ref(null);

watch(
  latest,
  (v) => {
    if (selectedVersionId.value === null && v) selectedVersionId.value = v.version_id;
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

.export-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: var(--sp-lg);
}

.picker-title {
  font-size: 1.05rem;
  margin: 0 0 var(--sp-sm);
}

.version-list,
.format-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--sp-xs);
}

.version-row,
.format-row {
  display: flex;
  gap: var(--sp-sm);
  align-items: flex-start;
  padding: var(--sp-sm) var(--sp-md);
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-md);
  cursor: pointer;
}

.version-row[data-active="true"],
.format-row[data-active="true"] {
  border-color: var(--primary);
}

.version-info,
.format-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.version-label,
.format-desc {
  font-size: 0.8rem;
  color: var(--on-surface-variant);
}

.version-title,
.format-label {
  font-weight: 600;
  font-size: 0.95rem;
}

.version-meta {
  font-size: 0.75rem;
  color: var(--on-surface-variant);
}

.export-footer {
  display: flex;
  align-items: center;
  gap: var(--sp-md);
}

.primary-cta {
  background: var(--primary);
  color: var(--on-primary);
  border: none;
  padding: var(--sp-sm) var(--sp-lg);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 0.95rem;
  font-weight: 600;
}

.error-line {
  color: #ff8a80;
  margin: 0;
}

.hint {
  margin: 0;
  font-size: 0.8rem;
  color: var(--on-surface-variant);
}
</style>
