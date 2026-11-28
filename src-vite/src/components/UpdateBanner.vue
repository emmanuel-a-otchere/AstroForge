<template>
  <div v-if="visible" class="update-banner" role="status">
    <span class="material-symbols-outlined" aria-hidden="true">system_update</span>
    <div class="banner-body">
      <p class="font-body">
        <template v-if="installing">Installing update v{{ update?.version }}…</template>
        <template v-else-if="ready">Update v{{ update?.version }} is ready to install. Relaunch to apply.</template>
        <template v-else>Update available: v{{ update?.version }} (current {{ currentVersion }}).</template>
      </p>
      <p v-if="error" class="error-line font-body" role="alert">{{ error }}</p>
    </div>
    <div class="banner-actions">
      <button
        v-if="!ready && !installing"
        type="button"
        class="primary-cta font-display"
        :disabled="busy"
        @click="install"
      >
        {{ busy ? "Downloading…" : "Download & install" }}
      </button>
      <button
        v-if="ready"
        type="button"
        class="primary-cta font-display"
        @click="applyAndRelaunch"
      >
        Relaunch now
      </button>
      <button type="button" class="ghost-link font-body" @click="dismiss">Dismiss</button>
    </div>
  </div>
</template>

<script setup>
import { onMounted, ref } from "vue";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getVersion } from "@tauri-apps/api/app";
import { inTauri } from "@/common/tauri";

const DISMISS_KEY = "astroforge.update.dismissed";

const visible = ref(false);
const update = ref(null);
const currentVersion = ref("");
const busy = ref(false);
const installing = ref(false);
const ready = ref(false);
const error = ref(null);

async function checkForUpdates(showStatus = false) {
  if (!inTauri) return;
  busy.value = true;
  error.value = null;
  try {
    currentVersion.value = await getVersion();
    update.value = await check();
    if (update.value && !dismissedFor(update.value.version)) {
      visible.value = true;
    } else if (showStatus) {
      visible.value = false;
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
    if (showStatus) visible.value = true;
  } finally {
    busy.value = false;
  }
}

function dismissedFor(version) {
  try {
    return localStorage.getItem(DISMISS_KEY) === version;
  } catch {
    return false;
  }
}

async function install() {
  if (!update.value) return;
  busy.value = true;
  installing.value = true;
  error.value = null;
  try {
    await update.value.downloadAndInstall();
    ready.value = true;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    busy.value = false;
    installing.value = false;
  }
}

async function applyAndRelaunch() {
  try {
    await relaunch();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

function dismiss() {
  visible.value = false;
  try {
    if (update.value) localStorage.setItem(DISMISS_KEY, update.value.version);
  } catch {
    // storage unavailable — just hide for this session
  }
}

defineExpose({ checkForUpdates });

onMounted(() => {
  void checkForUpdates(false);
});
</script>

<style scoped>
.update-banner {
  position: fixed;
  top: calc(40px + var(--sp-md));
  right: var(--sp-lg);
  z-index: 90;
  display: flex;
  align-items: flex-start;
  gap: var(--sp-sm);
  max-width: 420px;
  padding: var(--sp-md) var(--sp-lg);
  background: var(--surface-container-high);
  border: 1px solid var(--primary);
  border-radius: var(--radius-lg);
  color: var(--on-surface);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
}

.update-banner > .material-symbols-outlined {
  color: var(--primary);
}

.banner-body {
  flex: 1 1 auto;
}

.banner-body p {
  margin: 0;
  font-size: 0.85rem;
}

.banner-actions {
  display: flex;
  flex-direction: column;
  gap: var(--sp-xs);
  align-items: flex-end;
}

.primary-cta {
  background: var(--primary);
  color: var(--on-primary);
  border: none;
  padding: var(--sp-xs) var(--sp-md);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 0.85rem;
  font-weight: 600;
}

.primary-cta:disabled {
  opacity: 0.5;
  cursor: default;
}

.ghost-link {
  background: transparent;
  border: none;
  color: var(--on-surface-variant);
  cursor: pointer;
  font-size: 0.78rem;
}

.error-line {
  color: #ff8a80;
}
</style>
