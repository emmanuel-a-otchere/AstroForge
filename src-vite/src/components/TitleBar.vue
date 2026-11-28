<template>
  <div class="titlebar" :class="{ mac: isMac }" data-tauri-drag-region>
    <div class="titlebar-left" data-tauri-drag-region>
      <span class="titlebar-brand font-display" data-tauri-drag-region>
        {{ $t("app.name") }}
      </span>
    </div>
    <div class="titlebar-center" data-tauri-drag-region></div>
    <div
      v-if="inTauri && !isMac"
      class="titlebar-controls"
      @mousedown.stop
    >
      <button
        type="button"
        class="tb-btn material-symbols-outlined"
        aria-label="Minimize"
        @click="minimize"
      >minimize</button>
      <button
        type="button"
        class="tb-btn material-symbols-outlined"
        :aria-label="isMaximized ? 'Restore' : 'Maximize'"
        @click="toggleMaximize"
      >{{ isMaximized ? "filter_none" : "check_box_outline_blank" }}</button>
      <button
        type="button"
        class="tb-btn tb-close material-symbols-outlined"
        aria-label="Close"
        @click="closeWindow"
      >close</button>
    </div>
  </div>
</template>

<script setup>
import { onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { platform } from "@tauri-apps/plugin-os";
import { inTauri } from "@/common/tauri";

const isMac = ref(false);
const isMaximized = ref(false);

onMounted(() => {
  if (!inTauri) return;
  try {
    isMac.value = platform() === "macos";
  } catch {
    isMac.value = false;
  }
  getCurrentWindow()
    .isMaximized()
    .then((v) => (isMaximized.value = v))
    .catch(() => {});
});

async function minimize() {
  if (!inTauri) return;
  await getCurrentWindow().minimize();
}

async function toggleMaximize() {
  if (!inTauri) return;
  const appWindow = getCurrentWindow();
  if (await appWindow.isMaximized()) {
    await appWindow.unmaximize();
  } else {
    await appWindow.maximize();
  }
  isMaximized.value = await appWindow.isMaximized();
}

async function closeWindow() {
  if (!inTauri) return;
  await getCurrentWindow().close();
}
</script>

<style scoped>
.titlebar {
  display: flex;
  align-items: stretch;
  height: 40px;
  flex: 0 0 auto;
  background: var(--surface-container-lowest);
  border-bottom: 1px solid var(--outline-variant);
  user-select: none;
}

.titlebar.mac {
  padding-left: 80px;
}

.titlebar-left {
  display: flex;
  align-items: center;
  padding: 0 var(--sp-md);
}

.titlebar-center {
  flex: 1 1 auto;
}

.titlebar-brand {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--on-surface-variant);
}

.titlebar-controls {
  display: flex;
  align-items: stretch;
}

.tb-btn {
  width: 46px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--on-surface-variant);
  font-size: 18px;
  cursor: pointer;
}

.tb-btn:hover {
  background: var(--surface-container-high);
  color: var(--on-surface);
}

.tb-close:hover {
  background: #e53935;
  color: #fff;
}
</style>
