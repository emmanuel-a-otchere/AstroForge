<template>
  <span
    class="save-indicator"
    :data-state="state"
    :title="stateTitle"
    aria-live="polite"
    aria-atomic="true"
  >
    <span class="dot" aria-hidden="true"></span>
    <span v-if="!compact" class="label font-body">{{ label }}</span>
  </span>
</template>

<script setup>
import { computed } from "vue";
import { storeToRefs } from "pinia";
import { useProjectContextStore } from "@/stores/projectContext";

const props = defineProps({
  compact: { type: Boolean, default: false },
});

const context = useProjectContextStore();
const { isProjectDirty } = storeToRefs(context);

const state = computed(() => (isProjectDirty.value ? "saving" : "saved"));
const label = computed(() =>
  state.value === "saved" ? "Saved" : state.value === "saving" ? "Saving…" : "Save failed",
);
const stateTitle = computed(() =>
  state.value === "saved"
    ? "All changes saved"
    : state.value === "saving"
      ? "Saving changes…"
      : "Save failed — retry from the project menu",
);
</script>

<style scoped>
.save-indicator {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-xs);
  padding: var(--sp-xs) var(--sp-sm);
  border-radius: var(--radius-md);
  background: var(--surface-container);
  color: var(--on-surface-variant);
  font-size: 0.8rem;
}

.save-indicator .dot {
  width: 8px;
  height: 8px;
  border-radius: var(--radius-full);
  background: #10b981;
}

.save-indicator[data-state="saving"] .dot {
  background: #7787f9;
  animation: pulse 1s ease-in-out infinite;
}

.save-indicator[data-state="failed"] .dot {
  background: #e53935;
}

@keyframes pulse {
  50% { opacity: 0.4; }
}
</style>
