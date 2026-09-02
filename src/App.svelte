<!--
  App.svelte — workflow orchestrator.

  Delegates shell rendering to <AppShell> + a per-step mode wrapper.
  Each existing screen-card (file-select-panel, InitialDialog,
  ClassificationDialog, PreviewCanvas + WizardBottomSheet, etc.) renders
  inside its mode's snippet. The mode is chosen by `currentStep` via
  layout-mode.ts (context-driven) plus any manual override the user
  picks in the header's mode switcher.

  Uses Svelte 5 snippets for passing content to mode components.
-->
<script lang="ts">
  import { probeGpu, type GpuCapability } from "./lib/gpu";
  import { analyzeFiles, type AnalysisResult } from "./lib/image-analysis";
  import InitialDialog from "./components/InitialDialog.svelte";
  import ClassificationDialog from "./components/ClassificationDialog.svelte";
  import PreviewCanvas from "./components/PreviewCanvas.svelte";
  import WizardBottomSheet from "./components/WizardBottomSheet.svelte";
  import NodeSidebar from "./components/NodeSidebar.svelte";
  import ParameterSidebar from "./components/ParameterSidebar.svelte";
  import ScreenCard from "./components/ScreenCard.svelte";
  import AppShell from "./components/AppShell.svelte";
  import ModeA from "./components/ModeA.svelte";
  import ModeB from "./components/ModeB.svelte";
  import ModeC from "./components/ModeC.svelte";
  import ModeD from "./components/ModeD.svelte";
  import {
    initSession,
    sessionStore,
    activeStepIndex,
  } from "./lib/pipeline-store";
  import {
    setStage,
    currentLayoutMode,
    type AppStage,
  } from "./lib/layout-mode";
  import type { PreviewParams } from "./lib/gl-renderer";

  let showForgeMode = $state(false);
  let isTransitioning = $state(false);

  function toggleForgeMode() {
    isTransitioning = true;
    setTimeout(() => {
      showForgeMode = !showForgeMode;
      isTransitioning = false;
    }, 300);
  }

  type WorkflowStep = AppStage;

  let currentStep: WorkflowStep = $state("landing");
  let selectedFiles: File[] = $state([]);
  let classificationFrames: Array<{
    path: string;
    frame_type: string;
    exptime: number | null;
    filter: string | null;
    anomalies: string[];
  }> = $state([]);
  let sessionData: {
    targetName: string;
    cameraType: string;
    focalLength: number | null;
    lightsOnly: boolean;
    includeDithering: boolean;
    objectType: string;
  } | null = $state(null);
  let analysisResult: AnalysisResult | null = $state(null);
  let analyzing = $state(false);

  let previewParams: PreviewParams = $state({
    blackPoint: 0,
    midtones: 0.25,
    highlights: 1,
    strength: 0,
    scnrStrength: 0,
    scnrMethod: 0,
  });

  let renderMode: "identity" | "mtf" | "scnr" | "difference" | "composite" =
    $state("mtf");

  // Keep the layout-mode store in sync with the workflow step.
  $effect(() => {
    setStage(currentStep);
  });

  let currentMode = $derived($currentLayoutMode);

  function goToSelectFiles() {
    currentStep = "select-files";
  }

  function handleFileSelect(event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.files) {
      selectedFiles = Array.from(input.files).filter((f) =>
        /\.(fits|fit|tif|tiff|png|jpg|jpeg|dng|cr2|nef|arw)$/i.test(f.name)
      );
    }
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    if (event.dataTransfer?.files) {
      selectedFiles = Array.from(event.dataTransfer.files).filter((f) =>
        /\.(fits|fit|tif|tiff|png|jpg|jpeg|dng|cr2|nef|arw)$/i.test(f.name)
      );
    }
  }

  function handleDragOver(event: DragEvent) {
    event.preventDefault();
  }

  function clearFiles() {
    selectedFiles = [];
  }

  async function proceedToSessionSetup() {
    analyzing = true;
    try {
      analysisResult = await analyzeFiles(selectedFiles);
    } catch {
      analysisResult = null;
    }
    analyzing = false;
    currentStep = "session-setup";
  }

  function handleInitialConfirm(data: {
    targetName: string;
    cameraType: string;
    focalLength: number | null;
    lightsOnly: boolean;
    includeDithering: boolean;
    objectType: string;
  }) {
    sessionData = data;
    if (analysisResult) {
      classificationFrames = analysisResult.frames.map((f) => ({
        path: f.fileName,
        frame_type: f.frameType,
        exptime: f.exposureTime,
        filter: f.filter,
        anomalies: [],
      }));
    } else {
      classificationFrames = selectedFiles.map((f) => ({
        path: f.name,
        frame_type: guessFrameType(f.name),
        exptime: null,
        filter: null,
        anomalies: [],
      }));
    }
    currentStep = "review-frames";
  }

  function guessFrameType(filename: string): string {
    const lower = filename.toLowerCase();
    if (lower.includes("dark")) return "DARK";
    if (lower.includes("flat")) return "FLAT";
    if (lower.includes("bias") || lower.includes("darkflat")) return "BIAS";
    return "LIGHT";
  }

  function handleReclassify(index: number, newType: string) {
    if (classificationFrames[index]) {
      classificationFrames[index].frame_type = newType;
    }
  }

  function handleClassificationConfirm() {
    initSession(undefined, "automagic");
    currentStep = "processing";
  }

  function backToLanding() {
    currentStep = "landing";
    selectedFiles = [];
    sessionData = null;
    classificationFrames = [];
    analysisResult = null;
  }

  function backToSelectFiles() {
    currentStep = "select-files";
  }

  function handleParamsChange(params: Partial<PreviewParams>) {
    previewParams = { ...previewParams, ...params };
  }

  let stepIdx = $derived($activeStepIndex);
  let isStretchStage = $derived(
    $sessionStore?.pipelineGraph.nodes[stepIdx]?.type === "stretch"
  );
  let derivedRenderMode = $derived(isStretchStage ? "mtf" as const : "identity" as const);
  $effect(() => {
    renderMode = derivedRenderMode;
  });

  let fileCount = $derived(selectedFiles.length);
  let lightCount = $derived(
    classificationFrames.filter((f) => f.frame_type === "LIGHT").length
  );
  let darkCount = $derived(
    classificationFrames.filter((f) => f.frame_type === "DARK").length
  );
  let flatCount = $derived(
    classificationFrames.filter((f) => f.frame_type === "FLAT").length
  );
  let biasCount = $derived(
    classificationFrames.filter((f) => f.frame_type === "BIAS").length
  );
</script>

<AppShell currentStage={currentStep}>
  {#if currentMode === "a"}
    <ModeA>
      <ScreenCard kicker="01 · Load Files" title="Select your image files">
        <p class="subtitle">
          Choose the raw frames from your imaging session. You can drag and drop or browse.
        </p>

        <div
          class="drop-zone"
          role="region"
          aria-label="File drop zone"
          ondrop={handleDrop}
          ondragover={handleDragOver}
          class:has-files={fileCount > 0}
        >
          {#if fileCount === 0}
            <div class="drop-empty">
              <span class="material-symbols-outlined drop-icon">cloud_upload</span>
              <p>Drag and drop your files here</p>
              <span class="drop-hint">FITS, TIFF, PNG, JPEG, DNG</span>
            </div>
          {:else}
            <div class="file-list-header">
              <span>{fileCount} file{fileCount !== 1 ? "s" : ""} selected</span>
              <button class="btn-clear" onclick={clearFiles}>Clear all</button>
            </div>
            <div class="file-list">
              {#each selectedFiles as file}
                <div class="file-item">
                  <span class="material-symbols-outlined file-icon">description</span>
                  <span class="file-name" title={file.name}>{file.name}</span>
                  <span class="file-size">{(file.size / 1024 / 1024).toFixed(1)} MB</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <label class="btn-browse">
          Browse Files
          <input
            type="file"
            multiple
            accept=".fits,.fit,.tif,.tiff,.png,.jpg,.jpeg,.dng,.cr2,.nef,.arw"
            onchange={handleFileSelect}
            hidden
          />
        </label>

        {#snippet footer()}
        <div class="actions">
          <button class="btn-secondary" onclick={backToLanding}>Cancel</button>
          <button
            class="btn-primary"
            disabled={fileCount === 0}
            onclick={proceedToSessionSetup}
          >
            Continue
          </button>
        </div>
        {/snippet}
      </ScreenCard>
    </ModeA>
  {:else if currentMode === "b" && currentStep === "landing"}
    <ModeB>
      {#snippet canvas()}
      <div class="landing-canvas">
        <div class="landing-hero">
          <h1 class="text-display-xl">AstroForge</h1>
          <p class="tagline">Raw telescope data to publication-ready images</p>
          <div class="version">v0.1.0 — Full Pipeline</div>
          <button class="btn-start" onclick={goToSelectFiles}>
            <span class="material-symbols-outlined">upload</span>
            Load Your Data
          </button>
          <p class="hint">Select FITS, TIFF, PNG, JPEG, or DNG files from your telescope</p>
        </div>
      </div>
      {/snippet}

      {#snippet workflow()}
      <ScreenCard kicker="Welcome" title="Start here">
        <p class="landing-card-hint">
          Your recent sessions live in the gallery on the left. Pick one to
          continue, or load new data to begin a fresh workflow.
        </p>
        <button class="btn-primary full-width" onclick={goToSelectFiles}>
          Load new files
        </button>
      </ScreenCard>
      {/snippet}
    </ModeB>
  {:else if currentMode === "b" && currentStep === "processing"}
    <ModeB>
      {#snippet canvas()}
      <div class="processing-canvas">
        {#if showForgeMode}
          <div class="forge-layout">
            <NodeSidebar />
            <div class="forge-canvas-area">
              <PreviewCanvas params={previewParams} {renderMode} />
            </div>
          </div>
        {:else}
          <PreviewCanvas params={previewParams} {renderMode} />
        {/if}
        <button
          class="forge-toggle"
          onclick={toggleForgeMode}
          type="button"
          title="Toggle between guided and expert view"
        >
          <span class="material-symbols-outlined">{showForgeMode ? "view_agenda" : "account_tree"}</span>
          <span class="toggle-label">{showForgeMode ? "Guided" : "Pipeline"}</span>
        </button>
      </div>
      {/snippet}

      {#snippet workflow()}
      {#if showForgeMode}
        <ParameterSidebar {previewParams} onParamsChange={handleParamsChange} />
      {:else}
        <WizardBottomSheet {previewParams} onParamsChange={handleParamsChange} />
      {/if}
      {/snippet}
    </ModeB>
  {:else if currentMode === "c"}
    <ModeC />
  {:else if currentMode === "d"}
    <ModeD />
  {/if}
</AppShell>

<style>
  .subtitle {
    color: var(--on-surface-variant);
    margin-bottom: var(--sp-md);
  }

  .drop-zone {
    border: 2px dashed var(--outline-variant);
    border-radius: var(--radius-xl);
    padding: var(--sp-xl) var(--sp-lg);
    text-align: center;
    transition: border-color var(--transition-base), background var(--transition-base);
    min-height: 200px;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .drop-zone.has-files {
    border-style: solid;
    padding: var(--sp-md);
    text-align: left;
  }

  .drop-empty {
    color: var(--on-surface-variant);
  }

  .drop-icon {
    font-size: 48px;
    margin-bottom: var(--sp-sm);
    opacity: 0.5;
  }

  .drop-hint {
    color: var(--on-surface-variant);
  }

  .file-list-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--sp-sm);
  }

  .btn-clear {
    background: none;
    border: none;
    color: var(--on-surface-variant);
    font-size: var(--text-metadata);
    cursor: pointer;
    text-decoration: underline;
  }

  .btn-clear:hover {
    color: var(--error);
  }

  .file-list {
    max-height: 240px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    padding: 6px var(--sp-sm);
    background: var(--surface-container-high);
    border-radius: var(--radius-default);
  }

  .file-icon {
    font-size: 1rem;
    color: var(--on-surface-variant);
  }

  .file-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--on-surface);
  }

  .file-size {
    color: var(--on-surface-variant);
    white-space: nowrap;
  }

  .btn-browse {
    display: inline-block;
    margin-top: var(--sp-sm);
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container-high);
    color: var(--on-surface);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    font-family: var(--font-body);
    font-size: var(--text-body);
    font-weight: 500;
    cursor: pointer;
    text-align: center;
  }

  .btn-browse:hover {
    border-color: var(--cobalt-accent);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-sm);
    margin-top: var(--sp-lg);
  }

  .btn-primary,
  .btn-secondary {
    padding: var(--sp-sm) var(--sp-md);
    border-radius: var(--radius-md);
    font-family: var(--font-body);
    font-size: var(--text-body);
    font-weight: 500;
    cursor: pointer;
    border: none;
  }

  .btn-primary {
    background: var(--cobalt-accent);
    color: var(--surface);
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--primary-container);
  }

  .btn-primary:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
  }

  .btn-secondary:hover {
    background: var(--surface-container-highest);
  }

  .btn-primary.full-width {
    width: 100%;
    margin-top: var(--sp-md);
  }

  /* ── Landing (Mode B center canvas) ───────────────────────────────────── */
  .landing-canvas {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--sp-xl);
  }

  .landing-hero {
    text-align: center;
    max-width: 480px;
  }

  .text-display-xl {
    font-family: var(--font-display);
    font-size: var(--text-display-xl);
    font-weight: 700;
    letter-spacing: var(--ls-display);
    color: var(--cobalt-accent);
    margin-bottom: var(--sp-xs);
    line-height: var(--lh-display);
  }

  .tagline {
    color: var(--on-surface-variant);
    margin-bottom: var(--sp-sm);
    font-family: var(--font-body);
  }

  .version {
    color: var(--on-surface-variant);
    padding: 4px 12px;
    border-radius: var(--radius-default);
    background: var(--surface-container);
    display: inline-block;
    margin-bottom: var(--sp-lg);
    font-family: var(--font-data);
    font-size: var(--text-metadata);
  }

  .btn-start {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-sm);
    padding: 12px 32px;
    background: var(--cobalt-accent);
    color: var(--surface);
    border: none;
    border-radius: var(--radius-lg);
    font-family: var(--font-body);
    font-size: var(--text-body);
    font-weight: 600;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .btn-start:hover {
    background: var(--primary-container);
    box-shadow: 0 0 16px var(--glow-cobalt-strong);
  }

  .btn-start .material-symbols-outlined {
    font-size: 20px;
  }

  .hint {
    margin-top: var(--sp-sm);
    color: var(--on-surface-variant);
  }

  .landing-card-hint {
    color: var(--on-surface-variant);
    margin-bottom: var(--sp-md);
    line-height: var(--lh-body);
  }

  /* ── Processing (Mode B center canvas) ─────────────────────────────────── */
  .processing-canvas {
    flex: 1;
    position: relative;
    display: flex;
    flex-direction: column;
  }

  .forge-layout {
    flex: 1;
    display: grid;
    grid-template-columns: 240px 1fr;
    min-height: 0;
  }

  .forge-canvas-area {
    overflow: hidden;
  }

  .forge-toggle {
    position: absolute;
    bottom: var(--sp-md);
    right: var(--sp-md);
    display: inline-flex;
    align-items: center;
    gap: var(--sp-sm);
    padding: 8px 12px;
    background: var(--surface-container-high);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-default);
    color: var(--on-surface);
    cursor: pointer;
    font-family: var(--font-data);
    font-size: var(--text-metadata);
    z-index: 5;
  }

  .forge-toggle:hover {
    border-color: var(--cobalt-accent);
  }
</style>