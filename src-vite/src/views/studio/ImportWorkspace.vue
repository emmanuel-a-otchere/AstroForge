<template>
  <WorkspaceScreen
    title="Import"
    icon="upload_file"
    description="Bring new data into the project. Import FITS, TIFF, PNG or JPEG captures. Stacked results skip straight to post-processing."
  >
    <ol class="step-rail" aria-label="Import steps">
      <li
        v-for="(s, i) in steps"
        :key="s.id"
        class="step"
        :data-active="wizard.step === s.id"
        :data-done="stepIndex(wizard.step) > i"
      >
        <span class="step-num font-label">{{ i + 1 }}</span>
        <span class="font-body">{{ s.label }}</span>
      </li>
    </ol>

    <section class="step-body">
      <template v-if="wizard.step === 'add-data'">
        <p class="font-body hint">
          Choose a folder of raw captures. AstroForge scans it, classifies
          frames, and proposes an import plan.
        </p>
        <div class="row-actions">
          <button type="button" class="primary-cta font-display" @click="pickFolder">
            Choose folder…
          </button>
          <span v-if="wizard.scanDir" class="font-body hint">{{ wizard.scanDir }}</span>
        </div>
        <p v-if="wizard.scanError" class="error-line font-body" role="alert">
          {{ wizard.scanError }}
        </p>
        <div class="row-actions">
          <button
            type="button"
            class="primary-cta font-display"
            :disabled="!wizard.scanDir || wizard.scanning"
            @click="startScan"
          >
            {{ wizard.scanning ? "Scanning…" : "Scan folder" }}
          </button>
        </div>
      </template>

      <template v-else-if="wizard.step === 'scanning'">
        <p class="font-body">
          {{ wizard.scanning ? "Scanning " + (wizard.scanDir ?? "") + "…" : "Scan complete." }}
        </p>
        <p v-if="wizard.frames.length" class="font-body hint">
          {{ wizard.frames.length }} frames classified.
        </p>
        <p v-if="wizard.scanError" class="error-line font-body" role="alert">
          {{ wizard.scanError }}
        </p>
        <div class="row-actions">
          <button
            type="button"
            class="primary-cta font-display"
            :disabled="wizard.scanning || wizard.frames.length === 0"
            @click="analyse"
          >
            Continue
          </button>
        </div>
      </template>

      <template v-else-if="wizard.step === 'understanding'">
        <p class="font-body">
          Session understanding: {{ understanding?.target_name ?? "analyzing…" }}
        </p>
        <div class="row-actions">
          <button type="button" class="primary-cta font-display" @click="confirm">
            Confirm import
          </button>
        </div>
      </template>

      <template v-else>
        <p class="font-body">Import confirmed. {{ confirmResult ?? "" }}</p>
        <div class="row-actions">
          <button
            type="button"
            class="secondary-cta font-display"
            @click="resetImportWizard()"
          >
            Import another session
          </button>
        </div>
      </template>
    </section>
  </WorkspaceScreen>
</template>

<script setup>
import { ref } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import WorkspaceScreen from "@/components/WorkspaceScreen.vue";
import { useStore } from "@/composables/useStore";
import { useProjectContextStore } from "@/stores/projectContext";
import {
  importWizard,
  setSessionContext,
  setScanDir,
  setScanning,
  setScanError,
  setFrames,
  goToStep,
  resetImportWizard,
} from "@/state/import-wizard";
import {
  sessionCreate,
  importRegisterAssets,
  importAnalyseSession,
  importGetUnderstanding,
  importConfirm,
} from "@/lib/astroforge-api";
import { invokeCommand } from "@/common/tauri";

const wizard = useStore(importWizard);
const context = useProjectContextStore();

const steps = [
  { id: "add-data", label: "Add data" },
  { id: "scanning", label: "Scanning" },
  { id: "understanding", label: "Understanding" },
  { id: "confirm", label: "Confirm" },
];

const understanding = ref(null);
const confirmResult = ref(null);

function stepIndex(id) {
  return steps.findIndex((s) => s.id === id);
}

async function pickFolder() {
  try {
    const dir = await openDialog({ directory: true, multiple: false });
    if (typeof dir === "string") setScanDir(dir);
  } catch (e) {
    setScanError(e instanceof Error ? e.message : String(e));
  }
}

async function startScan() {
  const project = context.project;
  const dir = wizard.value.scanDir;
  if (!project || !dir) return;
  setScanning(true);
  setScanError(null);
  try {
    const raw = await invokeCommand("ingest_scan_directory", { dirPath: dir });
    const frames = (raw ?? []).map((f) => ({
      path: String(f.path ?? ""),
      frameType: String(f.frame_type ?? "unknown")
        .toLowerCase()
        .replace(/^\w/, (c) => c.toUpperCase()),
      exptime: typeof f.exptime === "number" ? f.exptime : null,
      filter: typeof f.filter === "string" ? f.filter : null,
      width: typeof f.width === "number" ? f.width : null,
      height: typeof f.height === "number" ? f.height : null,
      binning: typeof f.binning === "number" ? f.binning : null,
      anomalies: Array.isArray(f.anomalies) ? f.anomalies.map((a) => String(a)) : [],
    }));
    setFrames(frames);
    const sessionId = await sessionCreate(project.project_id, dir);
    setSessionContext(project.project_id, sessionId);
    goToStep("scanning");
  } catch (e) {
    setScanError(e instanceof Error ? e.message : String(e));
  } finally {
    setScanning(false);
  }
}

async function analyse() {
  const project = context.project;
  const sessionId = wizard.value.sessionId;
  if (!project || !sessionId) return;
  try {
    const frames = wizard.value.frames.map((f) => ({ path: f.path, frameType: f.frameType }));
    await importRegisterAssets(sessionId, project.project_id, frames);
    await importAnalyseSession(project.project_id, sessionId, [], null);
    understanding.value = await importGetUnderstanding(sessionId);
    goToStep("understanding");
  } catch (e) {
    setScanError(e instanceof Error ? e.message : String(e));
  }
}

async function confirm() {
  const sessionId = wizard.value.sessionId;
  if (!sessionId) return;
  try {
    await importConfirm(sessionId);
    confirmResult.value = "Session imported and materialised.";
    goToStep("confirm");
  } catch (e) {
    setScanError(e instanceof Error ? e.message : String(e));
  }
}
</script>

<style scoped>
.step-rail {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  gap: var(--sp-lg);
  flex-wrap: wrap;
}

.step {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-xs);
  color: var(--on-surface-variant);
  font-size: 0.85rem;
}

.step[data-active="true"] {
  color: var(--on-surface);
  font-weight: 600;
}

.step[data-done="true"] .step-num {
  background: var(--primary-container);
  color: var(--on-primary-container);
}

.step-num {
  width: 22px;
  height: 22px;
  border-radius: var(--radius-full);
  background: var(--surface-container);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 0.75rem;
}

.step-body {
  display: flex;
  flex-direction: column;
  gap: var(--sp-md);
  padding: var(--sp-lg);
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-lg);
}

.row-actions {
  display: flex;
  align-items: center;
  gap: var(--sp-md);
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
  font-weight: 600;
}

.hint {
  color: var(--on-surface-variant);
  font-size: 0.85rem;
  margin: 0;
}

.error-line {
  color: #ff8a80;
  margin: 0;
}
</style>
