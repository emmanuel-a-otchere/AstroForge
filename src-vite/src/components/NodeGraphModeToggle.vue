<template>
  <div class="mode-toggle" role="group" aria-label="Process view mode">
    <button
      v-for="opt in options"
      :key="opt.id"
      type="button"
      class="mode-btn"
      :class="{ active: mode === opt.id }"
      :aria-pressed="mode === opt.id"
      @click="handleClick(opt.id)"
    >
      <span class="material-symbols-outlined" aria-hidden="true">{{ opt.icon }}</span>
      {{ opt.label }}
    </button>
  </div>
</template>

<script setup>
import { useStore } from "@/composables/useStore";
import {
  nodeGraphMode,
  setNodeGraphMode,
  writePersistedNodeGraphMode,
} from "@/lib/node-graph-store";

const mode = useStore(nodeGraphMode);

const options = [
  { id: "wizard", label: "Wizard", icon: "auto_awesome" },
  { id: "node", label: "Node", icon: "account_tree" },
];

function handleClick(id) {
  setNodeGraphMode(id);
  writePersistedNodeGraphMode(
    typeof window !== "undefined" ? window.localStorage : null,
    id,
  );
}
</script>

<style scoped>
.mode-toggle {
  display: inline-flex;
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.mode-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: transparent;
  border: none;
  color: var(--on-surface-variant);
  padding: var(--sp-xs) var(--sp-md);
  cursor: pointer;
  font-size: 0.85rem;
  font-family: var(--font-body);
}

.mode-btn .material-symbols-outlined {
  font-size: 18px;
}

.mode-btn.active {
  background: var(--surface-container-high);
  color: var(--on-surface);
}
</style>
