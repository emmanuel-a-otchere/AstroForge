<template>
  <section class="screen" aria-labelledby="settings-title">
    <header>
      <h1 id="settings-title" class="font-display">Settings</h1>
      <p class="subtitle font-body">Application preferences. Persisted locally on this device.</p>
    </header>

    <section class="panel" aria-labelledby="appearance-title">
      <h2 id="appearance-title" class="font-display panel-title">Appearance</h2>
      <p class="font-body hint">
        AstroForge ships the "AstroForge Precision" dark theme (docs/DESIGN.md).
        In the Vue tree the theme is generated from those tokens into
        Tailwind/daisyUI (`src/assets/tokens.css`).
      </p>
    </section>

    <section class="panel" aria-labelledby="data-title">
      <h2 id="data-title" class="font-display panel-title">Data</h2>
      <p class="font-body hint">
        Projects live in the app data directory
        (<code>com.astroforge.app</code>). The window position and size are
        persisted via the window-state plugin.
      </p>
    </section>

    <section class="panel" aria-labelledby="updates-title">
      <h2 id="updates-title" class="font-display panel-title">Updates</h2>
      <div class="row">
        <button
          type="button"
          class="secondary-cta font-display"
          :disabled="updateBusy"
          @click="checkUpdates"
        >
          {{ updateBusy ? "Checking…" : "Check for updates" }}
        </button>
        <p class="font-body hint" role="status">
          <template v-if="updateStatus">{{ updateStatus }}</template>
          <template v-else>Current version {{ appVersion }}.</template>
        </p>
      </div>
      <p v-if="updateError" class="error-line font-body" role="alert">{{ updateError }}</p>
    </section>

    <section class="panel" aria-labelledby="system-title">
      <h2 id="system-title" class="font-display panel-title">System</h2>
      <p class="font-body hint">
        <template v-if="sysInfo">
          {{ sysInfo.platform }} · {{ sysInfo.arch }} · OS {{ sysInfo.version }} ·
          AstroForge {{ appVersion }}
        </template>
        <template v-else>Detecting platform…</template>
      </p>
    </section>
  </section>
</template>

<script setup>
import { onMounted, ref } from "vue";
import { check } from "@tauri-apps/plugin-updater";
import { platform, arch, version } from "@tauri-apps/plugin-os";
import { getVersion } from "@tauri-apps/api/app";
import { inTauri } from "@/common/tauri";

const appVersion = ref("");
const updateBusy = ref(false);
const updateStatus = ref(null);
const updateError = ref(null);
const sysInfo = ref(null);

async function checkUpdates() {
  if (!inTauri) {
    updateStatus.value = "Updates require the desktop app.";
    return;
  }
  updateBusy.value = true;
  updateError.value = null;
  try {
    const update = await check();
    updateStatus.value = update
      ? `Update available: v${update.version} — install it from the banner.`
      : "You're up to date.";
  } catch (e) {
    updateError.value = e instanceof Error ? e.message : String(e);
  } finally {
    updateBusy.value = false;
  }
}

onMounted(async () => {
  try {
    appVersion.value = await getVersion();
  } catch {
    appVersion.value = "0.1.0";
  }
  if (inTauri) {
    try {
      sysInfo.value = { platform: platform(), arch: arch(), version: version() };
    } catch {
      sysInfo.value = null;
    }
  }
});
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

.panel {
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-lg);
  padding: var(--sp-lg);
}

.panel-title {
  font-size: 1.05rem;
  margin: 0 0 var(--sp-sm);
}

.hint {
  color: var(--on-surface-variant);
  font-size: 0.88rem;
  margin: 0;
}

code {
  font-family: var(--font-mono);
  font-size: 0.85em;
}

.row {
  display: flex;
  align-items: center;
  gap: var(--sp-md);
  flex-wrap: wrap;
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

.secondary-cta:disabled {
  opacity: 0.5;
  cursor: default;
}

.error-line {
  color: #ff8a80;
  margin: var(--sp-xs) 0 0;
}
</style>
