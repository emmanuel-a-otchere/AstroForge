<template>
  <section
    class="workspace"
    :class="{ 'has-sidebar': $slots.sidebar }"
    aria-labelledby="workspace-title"
  >
    <header class="workspace-header">
      <span class="material-symbols-outlined workspace-icon" aria-hidden="true">
        {{ icon }}
      </span>
      <div>
        <h1 id="workspace-title" class="font-display">{{ title }}</h1>
        <p class="workspace-description font-body">{{ description }}</p>
      </div>
    </header>
    <div class="workspace-layout">
      <aside v-if="$slots.sidebar" class="workspace-sidebar" aria-label="Workspace sidebar">
        <slot name="sidebar"></slot>
      </aside>
      <div class="workspace-body">
        <slot></slot>
      </div>
    </div>
  </section>
</template>

<script setup>
defineProps({
  title: { type: String, required: true },
  description: { type: String, default: "" },
  icon: { type: String, default: "widgets" },
});
</script>

<style scoped>
.workspace {
  display: flex;
  flex-direction: column;
  gap: var(--sp-lg);
  padding: var(--sp-xl);
  max-width: 1100px;
  margin: 0 auto;
  width: 100%;
  box-sizing: border-box;
  color: var(--on-surface);
  overflow-y: auto;
  height: 100%;
}

.workspace.has-sidebar {
  max-width: 1440px;
}

.workspace-layout {
  display: flex;
  gap: var(--sp-lg);
  align-items: flex-start;
  width: 100%;
}

.workspace-sidebar {
  flex: 0 0 auto;
  align-self: stretch;
}

.workspace-body {
  flex: 1 1 auto;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--sp-md);
}

.workspace-header {
  display: flex;
  align-items: flex-start;
  gap: var(--sp-md);
}

.workspace-icon {
  font-size: 36px;
  color: var(--primary);
}

.workspace-header h1 {
  font-size: 1.75rem;
  margin: 0 0 var(--sp-xs) 0;
}

.workspace-description {
  margin: 0;
  color: var(--on-surface-variant);
  max-width: 70ch;
  font-size: 0.95rem;
}
</style>
