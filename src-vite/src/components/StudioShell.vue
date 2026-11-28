<template>
  <div v-if="appStore.studioProject" class="studio" data-studio-root="true">
    <header class="studio-header">
      <div class="header-left">
        <span class="material-symbols-outlined header-icon" aria-hidden="true">
          collections
        </span>
        <div class="project-identity">
          <span class="project-name font-display">{{ appStore.studioProject.name }}</span>
          <span class="project-context font-body">Studio workspace</span>
        </div>
      </div>
      <div class="header-actions">
        <SaveIndicator />
        <button
          type="button"
          class="close-cta font-display"
          aria-label="Close project"
          @click="closeProject"
        >
          <span class="material-symbols-outlined" aria-hidden="true">close</span>
          Close project
        </button>
      </div>
    </header>

    <nav class="tab-rail" aria-label="Studio workspaces">
      <ul role="tablist">
        <li v-for="item in navItems" :key="item.id" role="presentation">
          <button
            type="button"
            role="tab"
            class="tab"
            :aria-selected="appStore.studioView === item.id"
            :data-active="appStore.studioView === item.id"
            @click="appStore.setView(item.id)"
          >
            <span class="material-symbols-outlined tab-icon" aria-hidden="true">
              {{ item.icon }}
            </span>
            <span class="tab-label font-body">{{ item.label }}</span>
          </button>
        </li>
      </ul>
    </nav>

    <div class="studio-workspace" role="tabpanel" :aria-label="`${appStore.studioView} workspace`">
      <component :is="currentWorkspace" />
    </div>
  </div>
</template>

<script setup>
import { computed } from "vue";
import SaveIndicator from "./SaveIndicator.vue";
import OverviewWorkspace from "@/views/studio/OverviewWorkspace.vue";
import ImportWorkspace from "@/views/studio/ImportWorkspace.vue";
import ProcessWorkspace from "@/views/studio/ProcessWorkspace.vue";
import EnhanceWorkspace from "@/views/studio/EnhanceWorkspace.vue";
import CompareWorkspace from "@/views/studio/CompareWorkspace.vue";
import ExportWorkspace from "@/views/studio/ExportWorkspace.vue";
import { STUDIO_NAV_ITEMS, useApplicationStore } from "@/stores/application";
import { useLifecycle } from "@/stores/lifecycle";

const appStore = useApplicationStore();
const { closeProject } = useLifecycle();
const navItems = STUDIO_NAV_ITEMS;

const workspaces = {
  overview: OverviewWorkspace,
  import: ImportWorkspace,
  process: ProcessWorkspace,
  enhance: EnhanceWorkspace,
  compare: CompareWorkspace,
  export: ExportWorkspace,
};

const currentWorkspace = computed(() => workspaces[appStore.studioView] ?? OverviewWorkspace);
</script>

<style scoped>
.studio {
  display: grid;
  grid-template-rows: auto auto 1fr;
  height: 100%;
  width: 100%;
  background: var(--surface);
  color: var(--on-surface);
}

.studio-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--sp-md) var(--sp-lg);
  border-bottom: 1px solid var(--outline-variant);
  background: var(--surface-container-low);
  gap: var(--sp-md);
}

.header-left {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  min-width: 0;
}

.header-icon {
  font-size: 28px;
  color: var(--primary);
}

.project-identity {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.project-name {
  font-size: 1.1rem;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.project-context {
  font-size: 0.8rem;
  color: var(--on-surface-variant);
}

.header-actions {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
}

.close-cta {
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

.close-cta:hover {
  background: var(--surface-container-high);
  color: var(--on-surface);
}

.close-cta .material-symbols-outlined {
  font-size: 18px;
}

.tab-rail ul {
  list-style: none;
  margin: 0;
  padding: 0 var(--sp-lg);
  display: flex;
  gap: var(--sp-xs);
  border-bottom: 1px solid var(--outline-variant);
  background: var(--surface-container-lowest);
}

.tab {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-xs);
  background: transparent;
  border: none;
  border-bottom: 2px solid transparent;
  color: var(--on-surface-variant);
  padding: var(--sp-sm) var(--sp-md);
  cursor: pointer;
  font-size: 0.9rem;
}

.tab:hover {
  color: var(--on-surface);
}

.tab[data-active="true"] {
  color: var(--on-surface);
  border-bottom-color: var(--primary);
}

.tab-icon {
  font-size: 20px;
}

.studio-workspace {
  min-height: 0;
  overflow: hidden;
}
</style>
