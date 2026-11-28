<template>
  <div
    v-if="open"
    class="dialog-backdrop"
    role="presentation"
    @click="onCancel"
    @keydown="handleKeydown"
  >
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="project-dialog-title"
      tabindex="-1"
      @click.stop
      @keydown="handleKeydown"
    >
      <header class="dialog-header">
        <h2 id="project-dialog-title" class="font-display">{{ title }}</h2>
      </header>
      <div class="dialog-body">
        <label class="field font-body" for="project-name-input">
          {{ $t("dialog.nameLabel") }}
        </label>
        <input
          id="project-name-input"
          v-model="name"
          type="text"
          class="name-input"
          :placeholder="mode === 'create' ? $t('dialog.namePlaceholder') : ''"
          :disabled="submitting"
          autocomplete="off"
          autofocus
        />
        <p v-if="error" class="field-error font-body" role="alert">{{ error }}</p>
        <p class="hint font-body">{{ $t("dialog.nameHint") }}</p>
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
          class="primary-cta font-display"
          :disabled="submitting || !name.trim()"
          @click="handleSubmit"
        >
          {{ submitLabel }}
        </button>
      </footer>
    </div>
  </div>
</template>

<script setup>
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

const props = defineProps({
  mode: { type: String, default: "create" },
  project: { type: Object, default: undefined },
  open: { type: Boolean, default: false },
});
const emit = defineEmits(["submit", "cancel"]);

const { t } = useI18n();

const name = ref("");
const error = ref(null);
const submitting = ref(false);

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      name.value = props.mode === "rename" && props.project ? props.project.name : "";
      error.value = null;
      submitting.value = false;
    }
  },
);

const title = computed(() =>
  props.mode === "create" ? t("dialog.createTitle") : t("dialog.renameTitle"),
);
const submitLabel = computed(() =>
  props.mode === "create" ? t("dialog.create") : t("dialog.save"),
);

async function handleSubmit() {
  const trimmed = name.value.trim();
  if (!trimmed) {
    error.value = t("dialog.nameEmpty");
    return;
  }
  submitting.value = true;
  try {
    await emit("submit", { name: trimmed });
  } finally {
    submitting.value = false;
  }
}

function handleKeydown(e) {
  if (e.key === "Escape") emit("cancel");
  if (e.key === "Enter" && !submitting.value) void handleSubmit();
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

.field {
  font-size: 0.85rem;
  color: var(--on-surface-variant);
}

.name-input {
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-md);
  color: var(--on-surface);
  padding: var(--sp-sm) var(--sp-md);
  font-size: 0.95rem;
  font-family: var(--font-body);
}

.name-input:focus {
  outline: 2px solid var(--primary);
  outline-offset: -1px;
}

.field-error {
  color: #ff8a80;
  font-size: 0.85rem;
  margin: 0;
}

.hint {
  color: var(--on-surface-variant);
  font-size: 0.8rem;
  margin: 0;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-sm);
  padding: var(--sp-md) var(--sp-lg) var(--sp-lg);
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

.primary-cta:disabled {
  opacity: 0.5;
  cursor: default;
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

.secondary-cta:hover {
  background: var(--surface-container-high);
  color: var(--on-surface);
}
</style>
