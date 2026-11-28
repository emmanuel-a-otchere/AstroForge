<template>
  <section class="screen" aria-labelledby="ai-title">
    <header>
      <h1 id="ai-title" class="font-display">AI Models</h1>
      <p class="subtitle font-body">Manage installed AI capabilities. Downloads are on-device and local-first.</p>
    </header>

    <p v-if="loading" class="status-line font-body" role="status">Loading models…</p>
    <p v-else-if="error" class="error-line font-body" role="alert">
      {{ error }}
      <button type="button" class="ghost-link font-body" @click="load">Retry</button>
    </p>
    <ul v-else class="model-list" aria-label="AI models">
      <li v-for="model in models" :key="model.name" class="model-row">
        <div>
          <span class="font-display model-name">{{ model.name }}</span>
          <span class="hint font-body">
            · {{ model.size_mb ?? "?" }} MB · {{ model.installed ? "installed" : "not installed" }}
          </span>
        </div>
        <button
          v-if="!model.installed"
          type="button"
          class="secondary-cta font-display"
          :disabled="busy === model.name"
          @click="download(model.name)"
        >
          {{ busy === model.name ? "Downloading…" : "Download" }}
        </button>
      </li>
    </ul>
  </section>
</template>

<script setup>
import { onMounted, ref } from "vue";
import { aiModelList, aiModelDownload } from "@/lib/astroforge-api";

const models = ref([]);
const loading = ref(false);
const error = ref(null);
const busy = ref(null);

async function load() {
  loading.value = true;
  error.value = null;
  try {
    models.value = await aiModelList();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function download(name) {
  busy.value = name;
  error.value = null;
  try {
    await aiModelDownload(name);
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    busy.value = null;
  }
}

onMounted(load);
</script>

<style scoped>
.screen {
  display: flex;
  flex-direction: column;
  gap: var(--sp-lg);
  padding: var(--sp-xl);
  max-width: 1100px;
  margin: 0 auto;
  width: 100%;
  box-sizing: border-box;
  overflow-y: auto;
  color: var(--on-surface);
}

.screen h1 {
  font-size: 2rem;
  margin: 0 0 var(--sp-xs);
}

.subtitle {
  margin: 0;
  color: var(--on-surface-variant);
  font-size: 0.95rem;
}

.model-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--sp-xs);
}

.model-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-md);
  padding: var(--sp-sm) var(--sp-md);
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-md);
}

.hint {
  color: var(--on-surface-variant);
  font-size: 0.8rem;
}

.secondary-cta {
  background: transparent;
  color: var(--on-surface-variant);
  border: 1px solid var(--outline-variant);
  padding: var(--sp-xs) var(--sp-lg);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-weight: 600;
}

.ghost-link {
  background: transparent;
  border: none;
  color: var(--primary);
  cursor: pointer;
}

.error-line {
  color: #ff8a80;
}
</style>
