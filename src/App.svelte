<script lang="ts">
  import { probeGpu, type GpuCapability } from "./lib/gpu";
  import InitialDialog from "./components/InitialDialog.svelte";
  import ClassificationDialog from "./components/ClassificationDialog.svelte";

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
  let sessionData: { targetName: string; cameraType: string; focalLength: number | null; lightsOnly: boolean; includeDithering: boolean } | null = null;

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

  function proceedToSessionSetup() {
    currentStep = "session-setup";
  }

  function handleInitialConfirm(data: {
    targetName: string;
    cameraType: string;
    focalLength: number | null;
    lightsOnly: boolean;
    includeDithering: boolean;
  }) {
    sessionData = data;
    classificationFrames = selectedFiles.map(f => ({
      path: f.name,
      frame_type: guessFrameType(f.name),
      exptime: null,
      filter: null,
      anomalies: [],
    }));
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
    currentStep = "processing";
  }

  function backToLanding() {
    currentStep = "landing";
    selectedFiles = [];
    sessionData = null;
    classificationFrames = [];
  }

  function backToSelectFiles() {
    currentStep = "select-files";
  }

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
        GPU: {gpuCapability === "webgpu" ? "WebGPU" : "Canvas2D"}
      {:else}
        Detecting GPU...
      {/if}
    </div>
  </header>

  {#if currentStep !== "landing"}
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
        <h1>AstroForge</h1>
        <p class="tagline">Raw telescope data to publication-ready images</p>
        <div class="version">v0.1.0 — Full Pipeline</div>
        <button class="btn-start" on:click={goToSelectFiles}>
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
            <polyline points="17 8 12 3 7 8"/>
            <line x1="12" y1="3" x2="12" y2="15"/>
          </svg>
          Load Your Data
        </button>
        <p class="hint">Select FITS, TIFF, PNG, JPEG, or DNG files from your telescope</p>
      </div>
    {:else if currentStep === "select-files"}
      <div class="file-select-panel">
        <h2>Select your image files</h2>
        <p class="subtitle">Choose the raw frames from your imaging session. You can drag and drop or browse.</p>

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
              <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                <polyline points="17 8 12 3 7 8"/>
                <line x1="12" y1="3" x2="12" y2="15"/>
              </svg>
              <p>Drag and drop your files here</p>
              <span class="drop-hint">FITS, TIFF, PNG, JPEG, DNG</span>
            </div>
          {:else}
            <div class="file-list-header">
              <span>{fileCount} file{fileCount !== 1 ? "s" : ""} selected</span>
              <button class="btn-clear" on:click={clearFiles}>Clear all</button>
            </div>
            <div class="file-list">
              {#each selectedFiles as file}
                <div class="file-item">
                  <span class="file-icon">📄</span>
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
      {#if sessionData}
        <InitialDialog onConfirm={handleInitialConfirm} onCancel={backToSelectFiles} />
      {:else}
        <InitialDialog onConfirm={handleInitialConfirm} onCancel={backToSelectFiles} />
      {/if}
    {:else if currentStep === "review-frames"}
      <ClassificationDialog
        frames={classificationFrames}
        onConfirm={handleClassificationConfirm}
        onReclassify={handleReclassify}
      />
    {:else if currentStep === "processing"}
      <div class="processing-panel">
        <h2>Ready to process</h2>
        <div class="processing-summary">
          <div class="summary-item">
            <span class="summary-label">Total files</span>
            <span class="summary-value">{fileCount}</span>
          </div>
          <div class="summary-item">
            <span class="summary-label">Lights</span>
            <span class="summary-value">{lightCount}</span>
          </div>
          <div class="summary-item">
            <span class="summary-label">Darks</span>
            <span class="summary-value">{darkCount}</span>
          </div>
          <div class="summary-item">
            <span class="summary-label">Flats</span>
            <span class="summary-value">{flatCount}</span>
          </div>
          <div class="summary-item">
            <span class="summary-label">Biases</span>
            <span class="summary-value">{biasCount}</span>
          </div>
        </div>
        {#if sessionData}
          <div class="session-info">
            <p><strong>Target:</strong> {sessionData.targetName || "Unspecified"}</p>
            <p><strong>Camera:</strong> {sessionData.cameraType === "smart_telescope" ? "Smart telescope" : "OSC / DSLR"}</p>
            <p><strong>Focal length:</strong> {sessionData.focalLength ? sessionData.focalLength + " mm" : "Not specified"}</p>
          </div>
        {/if}
        <p class="processing-note">The processing pipeline will run automatically through each stage. You can pause to review results at any step.</p>
        <div class="actions">
          <button class="btn-secondary" on:click={backToLanding}>Start Over</button>
        </div>
      </div>
    {/if}
  </section>

  <footer class="app-footer">
    <span>AstroForge v0.1.0</span>
    <span>Full Pipeline — Deep Sky, Planetary & Lunar</span>
  </footer>
</main>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary);
  }

  .app-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1.5rem;
    height: 3.5rem;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    color: var(--accent);
    cursor: pointer;
  }

  .app-name {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: 0.02em;
  }

  .gpu-badge {
    font-size: 0.8125rem;
    color: var(--text-muted);
    padding: 0.25rem 0.75rem;
    border-radius: 0.375rem;
    background: var(--bg-tertiary);
  }

  .gpu-badge.gpu-checked {
    color: var(--success);
  }

  .step-bar {
    display: flex;
    align-items: center;
    padding: 0.75rem 2rem;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .step {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--text-muted);
    font-size: 0.8125rem;
    white-space: nowrap;
  }

  .step.active {
    color: var(--accent);
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
    background: var(--bg-tertiary);
    font-size: 0.75rem;
    font-weight: 600;
    border: 1px solid var(--border);
  }

  .step.active .step-num {
    background: var(--accent);
    color: var(--bg-primary);
    border-color: var(--accent);
  }

  .step.done .step-num {
    background: var(--success);
    color: var(--bg-primary);
    border-color: var(--success);
  }

  .step-connector {
    width: 2rem;
    height: 1px;
    background: var(--border);
    margin: 0 0.5rem;
  }

  .step-connector.done {
    background: var(--success);
  }

  .workspace {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-primary);
    position: relative;
    overflow: auto;
  }

  .workspace-empty {
    text-align: center;
  }

  .workspace-empty h1 {
    font-size: 3rem;
    font-weight: 700;
    color: var(--accent);
    margin-bottom: 0.5rem;
    letter-spacing: -0.02em;
  }

  .tagline {
    font-size: 1.125rem;
    color: var(--text-secondary);
    margin-bottom: 1rem;
  }

  .version {
    font-size: 0.875rem;
    color: var(--text-muted);
    padding: 0.25rem 0.75rem;
    border-radius: 0.375rem;
    background: var(--bg-secondary);
    display: inline-block;
    margin-bottom: 1.5rem;
  }

  .btn-start {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 2rem;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: 0.5rem;
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .btn-start:hover {
    background: var(--accent-dim);
  }

  .hint {
    margin-top: 0.75rem;
    font-size: 0.8125rem;
    color: var(--text-muted);
  }

  .file-select-panel {
    width: 560px;
    max-width: 90vw;
    padding: 1.5rem;
  }

  .file-select-panel h2 {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 0.25rem;
  }

  .subtitle {
    font-size: 0.875rem;
    color: var(--text-secondary);
    margin-bottom: 1.25rem;
  }

  .drop-zone {
    border: 2px dashed var(--border);
    border-radius: 0.75rem;
    padding: 2rem 1.5rem;
    text-align: center;
    transition: border-color 0.15s ease, background 0.15s ease;
    min-height: 200px;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .drop-zone.has-files {
    border-style: solid;
    padding: 1rem;
    text-align: left;
  }

  .drop-empty {
    color: var(--text-muted);
  }

  .drop-empty svg {
    margin-bottom: 0.75rem;
    opacity: 0.5;
  }

  .drop-empty p {
    font-size: 0.9375rem;
    margin-bottom: 0.25rem;
  }

  .drop-hint {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .file-list-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .btn-clear {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 0.75rem;
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
    gap: 0.25rem;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.375rem 0.5rem;
    background: var(--bg-tertiary);
    border-radius: 0.25rem;
    font-size: 0.8125rem;
  }

  .file-icon {
    font-size: 0.875rem;
  }

  .file-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-primary);
  }

  .file-size {
    color: var(--text-muted);
    font-size: 0.75rem;
    white-space: nowrap;
  }

  .btn-browse {
    display: inline-block;
    margin-top: 1rem;
    padding: 0.5rem 1.25rem;
    background: var(--bg-tertiary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
    font-size: 0.9375rem;
    font-weight: 500;
    cursor: pointer;
    text-align: center;
  }

  .btn-browse:hover {
    border-color: var(--accent);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 1.5rem;
  }

  .btn-primary,
  .btn-secondary {
    padding: 0.5rem 1.25rem;
    border-radius: 0.375rem;
    font-size: 0.9375rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
  }

  .btn-primary {
    background: var(--accent);
    color: var(--bg-primary);
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--accent-dim);
  }

  .btn-primary:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
  }

  .btn-secondary:hover {
    background: var(--border);
  }

  .processing-panel {
    width: 480px;
    max-width: 90vw;
    text-align: center;
  }

  .processing-panel h2 {
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 1.5rem;
  }

  .processing-summary {
    display: flex;
    justify-content: center;
    gap: 1.5rem;
    margin-bottom: 1.5rem;
    flex-wrap: wrap;
  }

  .summary-item {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .summary-label {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin-bottom: 0.25rem;
  }

  .summary-value {
    font-size: 1.75rem;
    font-weight: 700;
    color: var(--accent);
  }

  .session-info {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    padding: 1rem;
    text-align: left;
    margin-bottom: 1.5rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .session-info p {
    margin-bottom: 0.25rem;
  }

  .session-info strong {
    color: var(--text-primary);
  }

  .processing-note {
    font-size: 0.8125rem;
    color: var(--text-muted);
    margin-bottom: 1.5rem;
  }

  .app-footer {
    display: flex;
    justify-content: space-between;
    padding: 0 1.5rem;
    height: 2.5rem;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
    font-size: 0.75rem;
    color: var(--text-muted);
    align-items: center;
    flex-shrink: 0;
  }
</style>
