<template>
  <div
    v-if="open && project"
    class="dialog-backdrop"
    role="presentation"
    @click="onCancel"
    @keydown="handleKeydown"
  >
    <div
      class="dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="delete-title"
      aria-describedby="delete-desc"
      tabindex="-1"
      @click.stop
      @keydown="handleKeydown"
    >
      <header class="dialog-header">
        <h2 id="delete-title" class="font-display">{{ $t("dialog.deleteTitle") }}</h2>
      </header>
      <div class="dialog-body">
        <p id="delete-desc" class="font-body">{{ $t("dialog.deleteBody") }}</p>
        <p class="font-body">
          {{ $t("dialog.deleteType", { name: project.name }) }}
        </p>
        <input
          v-model="typed"
          type="text"
          class="confirm-input"
          :placeholder="project.name"
          :disabled="submitting"
          autocomplete="off"
          autofocus
          aria-label="Type the project name to confirm"
        />
      </div>
      <footer class="dialog-footer">
        <button
          type="button"
          class="secondary-cta font-display"
          :disabled="submitting"
          @click="onCancel"
        >
          {{ $t("dialog.cancel") }}
        </button>
        <button
          type="button"
          class="danger-cta font-display"
          :disabled="!confirmEnabled"
          @click="handleConfirm"
        >
          {{ $t("dialog.deleteConfirm") }}
        </button>
      </footer>
    </div>
  </div>
</template>

<script setup>
import { computed, ref, watch } from "vue";

const props = defineProps({
  project: { type: Object, default: null },
  open: { type: Boolean, default: false },
});
const emit = defineEmits(["confirm", "cancel"]);

const typed = ref("");
const submitting = ref(false);

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      typed.value = "";
      submitting.value = false;
    }
  },
);

const confirmEnabled = computed(
  () =>
    !!props.project &&
    typed.value.trim() === props.project.name.trim() &&
    !submitting.value,
);

function handleConfirm() {
  if (!props.project) return;
  if (typed.value.trim() !== props.project.name.trim()) return;
  submitting.value = true;
  emit("confirm");
}

function handleKeydown(e) {
  if (e.key === "Escape") emit("cancel");
}

function onCancel() {
  emit("cancel");
}
</script>

<style scoped>
.dialog-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.dialog {
  background: var(--surface-container);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-lg);
  min-width: 380px;
  max-width: 520px;
  width: 90%;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
  color: var(--on-surface);
}

.dialog-header {
  padding: var(--sp-lg) var(--sp-lg) var(--sp-sm);
  border-bottom: 1px solid var(--outline-variant);
}

.dialog-header h2 {
  margin: 0;
  font-size: 1.25rem;
}

.dialog-body {
  padding: var(--sp-lg);
  display: flex;
  flex-direction: column;
  gap: var(--sp-sm);
}

.dialog-body p {
  margin: 0;
  font-size: 0.9rem;
  color: var(--on-surface-variant);
}

.project-name {
  color: var(--on-surface);
  font-weight: 600;
}

.confirm-input {
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-md);
  color: var(--on-surface);
  padding: var(--sp-sm) var(--sp-md);
  font-size: 0.95rem;
  font-family: var(--font-body);
}

.confirm-input:focus {
  outline: 2px solid #e53935;
  outline-offset: -1px;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-sm);
  padding: var(--sp-md) var(--sp-lg) var(--sp-lg);
}

.secondary-cta {
  background: transparent;
  color: var(--on-surface-variant);
  border: 1px solid var(--outline-variant);
  padding: var(--sp-sm) var(--sp-lg);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 0.95rem;
  font-weight: 600;
}

.danger-cta {
  background: #e53935;
  color: #fff;
  border: none;
  padding: var(--sp-sm) var(--sp-lg);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 0.95rem;
  font-weight: 600;
}

.danger-cta:disabled {
  opacity: 0.5;
  cursor: default;
}
</style>
