<script lang="ts">
  import { probeGpu, type GpuCapability } from "./lib/gpu";
  import { analyzeFiles, type AnalysisResult } from "./lib/image-analysis";
  import InitialDialog from "./components/InitialDialog.svelte";
  import ClassificationDialog from "./components/ClassificationDialog.svelte";
  import PreviewCanvas from "./components/PreviewCanvas.svelte";
  import WizardBottomSheet from "./components/WizardBottomSheet.svelte";
  import NodeSidebar from "./components/NodeSidebar.svelte";
  import ParameterSidebar from "./components/ParameterSidebar.svelte";
  import {
    initSession,
    sessionStore,
    activeStepIndex,
  } from "./lib/pipeline-store";
  import type { PreviewParams } from "./lib/gl-renderer";

  let showForgeMode = false;
  let isTransitioning = false;

  function toggleForgeMode() {
    isTransitioning = true;
    setTimeout(() => {
      showForgeMode = !showForgeMode;
      isTransitioning = false;
    }, 300);
  }

  type WorkflowStep = "landing" | "select-files" | "session-setup" | "review-frames" | "processing";

  let gpuCapability: GpuCapability = "canvas2d";
  let gpuChecked = false;
  let currentStep: WorkflowStep = "landing";
  let selectedFiles: File[] = [];
  let classificationFrames: Array<{
    path: string;
    frame_type: string;
    exptime: number | null;
    filter: string | null;
    anomalies: string[];
  }> = [];
  let sessionData: { targetName: string; cameraType: string; focalLength: number | null; lightsOnly: boolean; includeDithering: boolean; objectType: string } | null = null;
  let analysisResult: AnalysisResult | null = null;
  let analyzing = false;

  let previewParams: PreviewParams = {
    blackPoint: 0,
    midtones: 0.25,
    highlights: 1,
    strength: 0,
    scnrStrength: 0,
    scnrMethod: 0,
  };

  let renderMode: "identity" | "mtf" | "scnr" | "difference" | "composite" = "mtf";

  function init() {
    gpuCapability = probeGpu();
    gpuChecked = true;
  }

  if (document.readyState !== "loading") {
    init();
  } else {
    document.addEventListener("DOMContentLoaded", init, { once: true });
  }

  function goToSelectFiles() {
    currentStep = "select-files";
  }

  function handleFileSelect(event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.files) {
      selectedFiles = Array.from(input.files).filter(f =>
        /\.(fits|fit|tif|tiff|png|jpg|jpeg|dng|cr2|nef|arw)$/i.test(f.name)
      );
    }
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    if (event.dataTransfer?.files) {
      selectedFiles = Array.from(event.dataTransfer.files).filter(f =>
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
      classificationFrames = analysisResult.frames.map(f => ({
        path: f.fileName,
        frame_type: f.frameType,
        exptime: f.exposureTime,
        filter: f.filter,
        anomalies: [],
      }));
    } else {
      classificationFrames = selectedFiles.map(f => ({
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

  $: stepIdx = $activeStepIndex;
  $: isStretchStage = $sessionStore?.pipelineGraph.nodes[stepIdx]?.type === "stretch";
  $: renderMode = isStretchStage ? "mtf" : "identity";

  const steps: { id: WorkflowStep; label: string; icon: string }[] = [
    { id: "select-files", label: "Load Files", icon: "1" },
    { id: "session-setup", label: "Session Info", icon: "2" },
    { id: "review-frames", label: "Review", icon: "3" },
    { id: "processing", label: "Process", icon: "4" },
  ];

  function stepIndex(): number {
    return steps.findIndex(s => s.id === currentStep);
  }

  $: fileCount = selectedFiles.length;
  $: lightCount = classificationFrames.filter(f => f.frame_type === "LIGHT").length;
  $: darkCount = classificationFrames.filter(f => f.frame_type === "DARK").length;
  $: flatCount = classificationFrames.filter(f => f.frame_type === "FLAT").length;
  $: biasCount = classificationFrames.filter(f => f.frame_type === "BIAS").length;
</script>

<main class="app">
  <header class="app-header">
    <button class="logo" on:click={backToLanding} type="button">
      <svg width="32" height="32" viewBox="0 0 64 64" fill="none">
        <circle cx="32" cy="32" r="20" stroke="currentColor" stroke-width="2.5" fill="none"/>
        <circle cx="32" cy="32" r="6" fill="currentColor"/>
        <path d="M32 6 L32 14 M32 50 L32 58 M6 32 L14 32 M50 32 L58 32" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      </svg>
      <span class="app-name">AstroForge</span>
    </button>
    <div class="gpu-badge" class:gpu-checked={gpuChecked}>
      {#if gpuChecked}
        GPU: {gpuCapability === "webgpu" ? "WebGPU" : "WebGL"}
      {:else}
        Detecting GPU...
      {/if}
    </div>
  </header>

  {#if currentStep !== "landing" && currentStep !== "processing"}
    <nav class="step-bar">
      {#each steps as step, i}
        <div class="step" class:active={currentStep === step.id} class:done={i < stepIndex()}>
          <span class="step-num">{i < stepIndex() ? "✓" : step.icon}</span>
          <span class="step-label">{step.label}</span>
        </div>
        {#if i < steps.length - 1}
          <div class="step-connector" class:done={i < stepIndex()}></div>
        {/if}
      {/each}
    </nav>
  {/if}

  <section class="workspace">
    {#if currentStep === "landing"}
      <div class="workspace-empty">
        <h1 class="text-display-xl">AstroForge</h1>
        <p class="tagline text-body">Raw telescope data to publication-ready images</p>
        <div class="version text-metadata">v0.1.0 — Full Pipeline</div>
        <button class="btn-start" on:click={goToSelectFiles}>
          <span class="material-symbols-outlined">upload</span>
          Load Your Data
        </button>
        <p class="hint text-metadata">Select FITS, TIFF, PNG, JPEG, or DNG files from your telescope</p>
      </div>
    {:else if currentStep === "select-files"}
      <div class="file-select-panel">
        <h2 class="text-headline-mobile">Select your image files</h2>
        <p class="subtitle text-body">Choose the raw frames from your imaging session. You can drag and drop or browse.</p>

        <div
          class="drop-zone"
          role="region"
          aria-label="File drop zone"
          on:drop={handleDrop}
          on:dragover={handleDragOver}
          class:has-files={fileCount > 0}
        >
          {#if fileCount === 0}
            <div class="drop-empty">
              <span class="material-symbols-outlined drop-icon">cloud_upload</span>
              <p class="text-body">Drag and drop your files here</p>
              <span class="drop-hint text-metadata">FITS, TIFF, PNG, JPEG, DNG</span>
            </div>
          {:else}
            <div class="file-list-header">
              <span class="text-body">{fileCount} file{fileCount !== 1 ? "s" : ""} selected</span>
              <button class="btn-clear" on:click={clearFiles}>Clear all</button>
            </div>
            <div class="file-list">
              {#each selectedFiles as file}
                <div class="file-item">
                  <span class="material-symbols-outlined file-icon">description</span>
                  <span class="file-name text-body" title={file.name}>{file.name}</span>
                  <span class="file-size text-metadata">{(file.size / 1024 / 1024).toFixed(1)} MB</span>
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
            on:change={handleFileSelect}
            hidden
          />
        </label>

        <div class="actions">
          <button class="btn-secondary" on:click={backToLanding}>Cancel</button>
          <button class="btn-primary" disabled={fileCount === 0} on:click={proceedToSessionSetup}>
            Continue
          </button>
        </div>
      </div>
    {:else if currentStep === "session-setup"}
      {#if analyzing}
        <div class="analyzing-panel">
          <div class="analyzing-spinner"></div>
          <p class="text-body">Analyzing your files...</p>
          <span class="text-metadata analyzing-hint">Reading FITS headers and EXIF data</span>
        </div>
      {:else}
        <InitialDialog analysis={analysisResult} onConfirm={handleInitialConfirm} onCancel={backToSelectFiles} />
      {/if}
    {:else if currentStep === "review-frames"}
      <ClassificationDialog
        frames={classificationFrames}
        onConfirm={handleClassificationConfirm}
        onReclassify={handleReclassify}
      />
    {:else if currentStep === "processing"}
      <div class="processing-workspace" class:forge={showForgeMode} class:transitioning={isTransitioning}>
        {#if showForgeMode}
          <div class="forge-layout">
            <NodeSidebar />
            <div class="forge-canvas-area">
              <PreviewCanvas
                params={previewParams}
                {renderMode}
              />
            </div>
            <ParameterSidebar
              {previewParams}
              onParamsChange={handleParamsChange}
            />
          </div>
        {:else}
          <PreviewCanvas
            params={previewParams}
            {renderMode}
          />
          <WizardBottomSheet
            {previewParams}
            onParamsChange={handleParamsChange}
          />
        {/if}

        <button class="forge-toggle" on:click={toggleForgeMode} type="button" title="Toggle between guided and expert view">
          <span class="material-symbols-outlined">{showForgeMode ? "view_agenda" : "account_tree"}</span>
          <span class="toggle-label">{showForgeMode ? "Guided" : "Pipeline"}</span>
        </button>
      </div>
    {/if}
  </section>

  {#if currentStep !== "processing"}
    <footer class="app-footer">
      <span class="text-metadata">AstroForge v0.1.0</span>
      <span class="text-metadata">Full Pipeline — Deep Sky, Planetary & Lunar</span>
    </footer>
  {/if}
</main>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--background);
  }

  .app-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--sp-lg);
    height: 3.5rem;
    background: var(--surface-container);
    border-bottom: 1px solid var(--outline-variant);
    flex-shrink: 0;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    color: var(--cobalt-accent);
    cursor: pointer;
    background: none;
    border: none;
  }

  .app-name {
    font-family: var(--font-display);
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--on-surface);
    letter-spacing: var(--ls-headline);
  }

  .gpu-badge {
    font-family: var(--font-data);
    font-size: var(--text-metadata);
    color: var(--on-surface-variant);
    padding: 4px 12px;
    border-radius: var(--radius-md);
    background: var(--surface-container-high);
  }

  .gpu-badge.gpu-checked {
    color: var(--success);
  }

  .step-bar {
    display: flex;
    align-items: center;
    padding: var(--sp-sm) var(--sp-lg);
    background: var(--surface-container);
    border-bottom: 1px solid var(--outline-variant);
    flex-shrink: 0;
  }

  .step {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    color: var(--on-surface-variant);
    font-size: var(--text-metadata);
    white-space: nowrap;
  }

  .step.active {
    color: var(--cobalt-accent);
  }

  .step.done {
    color: var(--success);
  }

  .step-num {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.5rem;
    height: 1.5rem;
    border-radius: 50%;
    background: var(--surface-container-high);
    font-size: 0.75rem;
    font-weight: 600;
    border: 1px solid var(--outline-variant);
  }

  .step.active .step-num {
    background: var(--cobalt-accent);
    color: var(--surface);
    border-color: var(--cobalt-accent);
  }

  .step.done .step-num {
    background: var(--success);
    color: var(--surface);
    border-color: var(--success);
  }

  .step-connector {
    width: 2rem;
    height: 1px;
    background: var(--outline-variant);
    margin: 0 var(--sp-sm);
  }

  .step-connector.done {
    background: var(--success);
  }

  .workspace {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--background);
    position: relative;
    overflow: auto;
  }

  .workspace-empty {
    text-align: center;
  }

  .workspace-empty h1 {
    color: var(--cobalt-accent);
    margin-bottom: var(--sp-xs);
  }

  .tagline {
    color: var(--on-surface-variant);
    margin-bottom: var(--sp-sm);
  }

  .version {
    color: var(--on-surface-variant);
    padding: 4px 12px;
    border-radius: var(--radius-md);
    background: var(--surface-container);
    display: inline-block;
    margin-bottom: var(--sp-lg);
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
    box-shadow: 0 0 16px rgba(203, 78, 61, 0.3);
  }

  .btn-start .material-symbols-outlined {
    font-size: 20px;
  }

  .hint {
    margin-top: var(--sp-sm);
    color: var(--on-surface-variant);
  }

  .file-select-panel {
    width: 560px;
    max-width: 90vw;
    padding: var(--sp-lg);
  }

  .file-select-panel h2 {
    margin-bottom: var(--sp-xs);
  }

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

  .analyzing-panel {
    text-align: center;
    color: var(--on-surface-variant);
  }

  .analyzing-spinner {
    width: 2.5rem;
    height: 2.5rem;
    border: 3px solid var(--outline-variant);
    border-top-color: var(--cobalt-accent);
    border-radius: 50%;
    margin: 0 auto var(--sp-sm);
    animation: spin 0.8s linear infinite;
  }

  .analyzing-panel p {
    color: var(--on-surface);
    margin-bottom: 4px;
  }

  .analyzing-hint {
    color: var(--on-surface-variant);
  }

  .processing-workspace {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
  }

  .processing-workspace.transitioning > * {
    opacity: 0.3;
    transition: opacity var(--transition-slow);
  }

  .forge-layout {
    display: flex;
    flex: 1;
    height: 100%;
    overflow: hidden;
  }

  .forge-canvas-area {
    flex: 1;
    position: relative;
    overflow: hidden;
    background: var(--surface-container-lowest);
  }

  .forge-toggle {
    position: absolute;
    top: var(--sp-md);
    left: var(--sp-md);
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    background: rgba(18, 20, 20, 0.85);
    color: var(--on-surface);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-full);
    font-family: var(--font-data);
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
    cursor: pointer;
    backdrop-filter: blur(8px);
    z-index: 30;
    transition: all var(--transition-fast);
  }

  .forge-toggle:hover {
    border-color: var(--cobalt-accent);
    color: var(--cobalt-accent);
    box-shadow: 0 0 8px rgba(203, 78, 61, 0.2);
  }

  .forge-toggle .material-symbols-outlined {
    font-size: 18px;
  }

  .app-footer {
    display: flex;
    justify-content: space-between;
    padding: 0 var(--sp-lg);
    height: 2.5rem;
    background: var(--surface-container);
    border-top: 1px solid var(--outline-variant);
    color: var(--on-surface-variant);
    align-items: center;
    flex-shrink: 0;
  }
</style>
