<template>
  <section class="placeholder-screen font-body" aria-live="polite">
    <p class="placeholder-title font-display">{{ $t(titleKey) }}</p>
    <p class="placeholder-hint">{{ $t("placeholder.body") }}</p>
  </section>
</template>

<script setup>
defineProps({
  titleKey: { type: String, required: true },
});
</script>

<style scoped>
.placeholder-screen {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: var(--sp-xl);
  gap: var(--sp-md);
  color: var(--on-surface-variant);
  height: 100%;
}

.placeholder-title {
  font-size: 1.5rem;
  margin: 0;
  color: var(--on-surface);
}

.placeholder-hint {
  max-width: 60ch;
  font-size: 0.85rem;
}
</style>
