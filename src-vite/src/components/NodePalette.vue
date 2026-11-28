<template>
  <aside class="node-palette" aria-label="Stage palette">
    <h2 class="font-display palette-title">Stages</h2>
    <p class="font-body palette-hint">Click a stage to append it to the plan.</p>
    <div v-for="group in groups" :key="group.name" class="palette-group">
      <h3 class="font-label group-label">{{ group.name }}</h3>
      <button
        v-for="entry in group.entries"
        :key="entry.stage_type"
        type="button"
        class="palette-item"
        :title="entry.description"
        @click="emit('select', entry)"
      >
        <span class="font-body item-label">{{ entry.label }}</span>
        <span v-if="entry.ai_uses_ai" class="material-symbols-outlined ai-flag" aria-label="AI">auto_awesome</span>
      </button>
    </div>
  </aside>
</template>

<script setup>
import { computed, onMounted, ref } from "vue";
import { loadNodeCatalog } from "@/lib/node-catalog";
import { groupEntriesByPalette } from "@/lib/node-palette-logic";

const emit = defineEmits(["select"]);

const catalog = ref({ version: 0, entries: [] });

const groups = computed(() => groupEntriesByPalette(catalog.value));

onMounted(() => {
  loadNodeCatalog()
    .then((c) => (catalog.value = c))
    .catch(() => (catalog.value = { version: 0, entries: [] }));
});
</script>

<style scoped>
.node-palette {
  width: 240px;
  flex: 0 0 auto;
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-lg);
  padding: var(--sp-md);
  display: flex;
  flex-direction: column;
  gap: var(--sp-sm);
  max-height: 100%;
  overflow-y: auto;
}

.palette-title {
  font-size: 1rem;
  margin: 0;
}

.palette-hint {
  font-size: 0.75rem;
  color: var(--on-surface-variant);
  margin: 0;
}

.palette-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.group-label {
  font-size: 0.7rem;
  text-transform: uppercase;
  color: var(--on-surface-variant);
  margin: var(--sp-xs) 0 0;
}

.palette-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-xs);
  background: var(--surface-container);
  border: 1px solid var(--outline-variant);
  color: var(--on-surface);
  border-radius: var(--radius-md);
  padding: var(--sp-xs) var(--sp-sm);
  cursor: pointer;
  text-align: left;
  font-size: 0.85rem;
}

.palette-item:hover {
  border-color: var(--primary);
}

.ai-flag {
  font-size: 16px;
  color: var(--tertiary);
}
</style>
