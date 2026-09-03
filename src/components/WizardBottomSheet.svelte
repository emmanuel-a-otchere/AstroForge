<script lang="ts">
  import {
    sessionStore,
    activeStepIndex,
    currentMode,
    pipelineGraph,
    stageDefinitions,
    commitStage,
    nextStep,
    prevStep,
    goToStep,
    undo,
    redo,
    canUndo,
    canRedo,
    setMode,
    type ProcessingMode,
  } from "../lib/pipeline-store";
  import type { PreviewParams } from "../lib/gl-renderer";
  import { recordStage } from "../lib/session";

  export let previewParams: PreviewParams;
  export let onParamsChange: (params: Partial<PreviewParams>) => void;

  let sliderValue = 0.5;
  let showModeSelector = false;
  let showModeConfirm = false;
  let pendingMode: ProcessingMode | null = null;
  let keepPixelState = true;

  $: stepIdx = $activeStepIndex;
  $: stage = stageDefinitions[stepIdx];
  $: node = $pipelineGraph.nodes[stepIdx];
  $: mode = $currentMode;
  $: totalSteps = stageDefinitions.length;

  const modeLabels: Record<ProcessingMode, string> = {
    automagic: "Automagic",
    automagic_expert: "Automagic Expert",
    pure_expert: "Pure Expert",
  };

  const modeColors: Record<ProcessingMode, string> = {
    automagic: "var(--cobalt-accent)",
    automagic_expert: "var(--tertiary-container)",
    pure_expert: "var(--primary-container)",
  };

  function handleSliderChange(e: Event) {
    const input = e.target as HTMLInputElement;
    sliderValue = parseFloat(input.value);
    onParamsChange({ strength: sliderValue } as Partial<PreviewParams>);
  }

  /**
   * Commit the current stage + advance. Fires the autosave hook so a
   * stage_run row is written to the local rusqlite store. The hook is
   * fire-and-forget: failures are best-effort, never blocking the UI.
   *
   * Spec §5 NFR + issue #144.
   */
  async function handleNext() {
    const params = { ...node.params, strength: sliderValue };
    const stageId = node.type;
    const sessionId = $sessionStore.sessionId;
    commitStage(params);
    nextStep();
    sliderValue = 0.5;
    try {
      await recordStage({
        sessionId,
        stageId,
        status: "completed",
        params,
      });
    } catch (err) {
      console.warn("session autosave failed (non-blocking):", err);
    }
  }

  function handleBack() {
    prevStep();
  }

  function handleStepClick(index: number) {
    goToStep(index);
  }

  function handleUndo() {
    undo();
  }

  function handleRedo() {
    redo();
  }

  function handleModeSelect(m: ProcessingMode) {
    if (m === mode) {
      showModeSelector = false;
      return;
    }
    pendingMode = m;
    showModeSelector = false;
    showModeConfirm = true;
    keepPixelState = true;
  }

  function confirmModeSwitch() {
    if (pendingMode) {
      setMode(pendingMode, keepPixelState);
    }
    showModeConfirm = false;
    pendingMode = null;
  }

  function cancelModeSwitch() {
    showModeConfirm = false;
    pendingMode = null;
  }

  function isStageAccessible(index: number): boolean {
    if (index <= stepIdx) return true;
    return $pipelineGraph.nodes[index - 1]?.status === "completed";
  }

  $: isAutomagic = mode === "automagic";
</script>

<svelte:window on:keydown={(e) => e.key === 'Escape' && showModeConfirm && cancelModeSwitch()} />

<!-- Mode Indicator Badge -->
<div class="mode-badge" style="--mode-color: {modeColors[mode]}">
  <span class="status-led" style="background: {modeColors[mode]}; box-shadow: 0 0 4px 1px {modeColors[mode]}40"></span>
  <span class="mode-label">{modeLabels[mode]}</span>
  <button class="mode-change-btn" on:click={() => (showModeSelector = !showModeSelector)} type="button">
    <span class="material-symbols-outlined mode-icon">swap_horiz</span>
  </button>
</div>

{#if showModeSelector}
  <div class="mode-selector glass-panel">
    <div class="mode-selector-header">
      <span class="text-label-caps">Select Mode</span>
      <button class="mode-close" on:click={() => (showModeSelector = false)} type="button">
        <span class="material-symbols-outlined">close</span>
      </button>
    </div>
    {#each Object.entries(modeLabels) as [key, label]}
      <button
        class="mode-option"
        class:active={mode === key}
        on:click={() => handleModeSelect(key as ProcessingMode)}
        type="button"
      >
        <span class="status-led" style="background: {modeColors[key as ProcessingMode]}"></span>
        <div class="mode-option-text">
          <span class="mode-option-label">{label}</span>
          {#if key === "automagic"}
            <span class="mode-option-desc">AI-driven, one-click processing</span>
          {:else if key === "automagic_expert"}
            <span class="mode-option-desc">AI proposes, you refine with live controls</span>
          {:else}
            <span class="mode-option-desc">AI disabled, full manual control</span>
          {/if}
        </div>
      </button>
    {/each}
  </div>
{/if}

{#if showModeConfirm}
  <div class="modal-overlay" role="button" tabindex="-1" aria-label="Close dialog" on:click={cancelModeSwitch}>
    <div class="mode-confirm glass-panel" role="document">
      <h3 class="text-headline-mobile">Switch to {pendingMode ? modeLabels[pendingMode] : ""}?</h3>
      <p class="text-body mode-confirm-desc">
        Switching modes mid-session. Choose how to handle your current work:
      </p>
      <div class="mode-confirm-options">
        <label class="radio-option">
          <input type="radio" name="modeSwitch" bind:group={keepPixelState} value={true} checked />
          <span class="text-body">Keep current pixel state</span>
        </label>
        <label class="radio-option">
          <input type="radio" name="modeSwitch" bind:group={keepPixelState} value={false} />
          <span class="text-body">Re-process from current stage under new mode</span>
        </label>
      </div>
      <div class="mode-confirm-actions">
        <button class="btn-secondary" on:click={cancelModeSwitch} type="button">Cancel</button>
        <button class="btn-primary" on:click={confirmModeSwitch} type="button">Switch Mode</button>
      </div>
    </div>
  </div>
{/if}

<!-- Stepper -->
<div class="stepper">
  {#each stageDefinitions as s, i}
    <button
      class="step-dot"
      class:active={i === stepIdx}
      class:done={$pipelineGraph.nodes[i]?.status === "completed"}
      class:accessible={isStageAccessible(i)}
      class:locked={!isStageAccessible(i)}
      on:click={() => isStageAccessible(i) && handleStepClick(i)}
      disabled={!isStageAccessible(i)}
      type="button"
      title={s.label}
    >
      <span class="step-num">{i + 1}</span>
      {#if i < totalSteps - 1}
        <span class="step-line" class:done={$pipelineGraph.nodes[i]?.status === "completed"}></span>
      {/if}
    </button>
  {/each}
</div>

<!-- Bottom Sheet -->
<div class="bottom-sheet glass-panel" class:automagic={isAutomagic}>
  <div class="bottom-sheet-header">
    <div class="stage-info">
      <span class="text-label-caps stage-counter">Step {stepIdx + 1} of {totalSteps}</span>
      <h2 class="text-headline-mobile stage-title">{stage?.label ?? ""}</h2>
      <p class="text-body stage-desc">{stage?.description ?? ""}</p>
    </div>
    <div class="header-actions">
      <button class="icon-btn" on:click={handleUndo} disabled={!$canUndo} type="button" title="Undo">
        <span class="material-symbols-outlined">undo</span>
      </button>
      <button class="icon-btn" on:click={handleRedo} disabled={!$canRedo} type="button" title="Redo">
        <span class="material-symbols-outlined">redo</span>
      </button>
    </div>
  </div>

  {#if isAutomagic}
    <!-- Automagic mode: minimal controls -->
    <div class="automagic-controls">
      <button class="btn-automagic-process" type="button">
        <span class="material-symbols-outlined">auto_awesome</span>
        Auto Process This Stage
      </button>
      <p class="text-metadata automagic-hint">AI will select optimal parameters for your data type</p>
    </div>
  {:else}
    <!-- Expert modes: full parameter controls -->
    <div class="param-section">
      <div class="param-row">
        <label class="text-label-caps" for="strength-slider">
          {#if stage?.type === "stretch"}
            Stretch Strength
          {:else if stage?.type === "background_extraction"}
            Gradient Removal
          {:else if stage?.type === "denoise"}
            Noise Reduction
          {:else if stage?.type === "star_handling"}
            Star Replace Strength
          {:else if stage?.type === "sharpen_deconvolution"}
            Sharpening Amount
          {:else if stage?.type === "color_calibration"}
            Colour Balance Strength
          {:else}
            Strength
          {/if}
        </label>
        <span class="text-data param-value">{(sliderValue * 100).toFixed(0)}%</span>
      </div>
      <input
        id="strength-slider"
        type="range"
        min="0"
        max="1"
        step="0.01"
        bind:value={sliderValue}
        on:input={handleSliderChange}
        class="param-slider"
      />
    </div>

    {#if stage?.type === "stretch"}
      <div class="param-section">
        <div class="param-row">
          <label class="text-label-caps" for="midtones-slider">Midtones Balance</label>
          <span class="text-data param-value">{(previewParams.midtones * 100).toFixed(0)}%</span>
        </div>
        <input
          id="midtones-slider"
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={previewParams.midtones}
          on:input={(e) => onParamsChange({ midtones: parseFloat((e.target as HTMLInputElement).value) })}
          class="param-slider"
        />
      </div>
      <div class="param-section">
        <div class="param-row">
          <label class="text-label-caps" for="bp-slider">Black Point</label>
          <span class="text-data param-value">{(previewParams.blackPoint * 100).toFixed(0)}%</span>
        </div>
        <input
          id="bp-slider"
          type="range"
          min="0"
          max="0.5"
          step="0.001"
          value={previewParams.blackPoint}
          on:input={(e) => onParamsChange({ blackPoint: parseFloat((e.target as HTMLInputElement).value) })}
          class="param-slider"
        />
      </div>
    {:else if stage?.type === "star_handling"}
      <div class="param-section">
        <div class="param-row">
          <label class="text-label-caps" for="color-boost-slider">Star Colour Boost</label>
          <span class="text-data param-value">{(previewParams.scnrStrength * 100).toFixed(0)}%</span>
        </div>
        <input
          id="color-boost-slider"
          type="range"
          min="0"
          max="2"
          step="0.01"
          value={previewParams.scnrStrength}
          on:input={(e) => onParamsChange({ scnrStrength: parseFloat((e.target as HTMLInputElement).value) })}
          class="param-slider"
        />
      </div>
    {/if}

    {#if mode === "automagic_expert"}
      <div class="ai-suggestion glass-panel">
        <div class="ai-suggestion-header">
          <span class="material-symbols-outlined ai-icon">auto_awesome</span>
          <span class="text-label-caps">AI Suggestion</span>
        </div>
        <p class="text-body ai-text">
          Based on your {stage?.type === "stretch" ? "linear data" : "image analysis"}, the AI recommends
          a strength of 65%{#if stage?.type === "stretch"} with midtones at 25%{/if}.
        </p>
        <div class="ai-suggestion-actions">
          <button class="btn-accept" type="button">Accept</button>
          <button class="btn-refine" type="button">Refine</button>
        </div>
      </div>
    {/if}
  {/if}

  <div class="bottom-sheet-actions">
    <button class="btn-secondary" on:click={handleBack} disabled={stepIdx === 0} type="button">
      <span class="material-symbols-outlined">arrow_back</span>
      Back
    </button>
    <button class="btn-primary" on:click={handleNext} disabled={stepIdx === totalSteps - 1} type="button">
      {stepIdx === totalSteps - 1 ? "Finish" : "Next"}
      <span class="material-symbols-outlined">arrow_forward</span>
    </button>
  </div>
</div>

<style>
  .mode-badge {
    position: absolute;
    top: var(--sp-md);
    right: var(--sp-md);
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    padding: 6px 12px;
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-full);
    z-index: 20;
  }

  .mode-label {
    font-family: var(--font-data);
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
    color: var(--mode-color, var(--on-surface));
  }

  .mode-change-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    display: flex;
    align-items: center;
    color: var(--on-surface-variant);
  }

  .mode-icon {
    font-size: 16px;
  }

  .mode-selector {
    position: absolute;
    top: 52px;
    right: var(--sp-md);
    width: 280px;
    padding: var(--sp-md);
    border-radius: var(--radius-lg);
    z-index: 20;
  }

  .mode-selector-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--sp-sm);
  }

  .mode-close {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--on-surface-variant);
    padding: 0;
  }

  .mode-option {
    display: flex;
    align-items: flex-start;
    gap: var(--sp-sm);
    width: 100%;
    padding: var(--sp-sm);
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    cursor: pointer;
    text-align: left;
    transition: background var(--transition-fast);
  }

  .mode-option:hover {
    background: var(--surface-container-high);
  }

  .mode-option.active {
    border-color: var(--cobalt-accent);
    background: var(--surface-container-high);
  }

  .mode-option-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .mode-option-label {
    font-family: var(--font-body);
    font-size: var(--text-body);
    font-weight: 600;
    color: var(--on-surface);
  }

  .mode-option-desc {
    font-family: var(--font-body);
    font-size: var(--text-metadata);
    color: var(--on-surface-variant);
  }

  .modal-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-scrim);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .mode-confirm {
    width: 400px;
    max-width: 90vw;
    padding: var(--sp-lg);
    border-radius: var(--radius-xl);
  }

  .mode-confirm-desc {
    margin-top: var(--sp-sm);
    color: var(--on-surface-variant);
  }

  .mode-confirm-options {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
    margin: var(--sp-md) 0;
  }

  .radio-option {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    cursor: pointer;
  }

  .radio-option input {
    accent-color: var(--cobalt-accent);
  }

  .mode-confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-sm);
  }

  .stepper {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--sp-sm) var(--sp-lg);
    gap: 0;
  }

  .step-dot {
    display: flex;
    align-items: center;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
  }

  .step-dot.locked {
    cursor: not-allowed;
    opacity: 0.3;
  }

  .step-num {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--surface-container-high);
    border: 1px solid var(--outline-variant);
    font-family: var(--font-data);
    font-size: 11px;
    font-weight: 700;
    color: var(--on-surface-variant);
    transition: all var(--transition-base);
  }

  .step-dot.active .step-num {
    background: var(--cobalt-accent);
    color: var(--surface);
    border-color: var(--cobalt-accent);
    box-shadow: 0 0 8px var(--glow-cobalt-bright);
  }

  .step-dot.done .step-num {
    background: var(--tertiary-container);
    color: var(--surface);
    border-color: var(--tertiary-container);
  }

  .step-line {
    width: 24px;
    height: 1px;
    background: var(--outline-variant);
    margin: 0 4px;
    transition: background var(--transition-base);
  }

  .step-line.done {
    background: var(--tertiary-container);
  }

  .bottom-sheet {
    position: absolute;
    bottom: var(--sp-md);
    left: var(--sp-md);
    right: var(--sp-md);
    padding: var(--sp-lg);
    border-radius: var(--radius-xl);
    z-index: 10;
    max-height: 50vh;
    overflow-y: auto;
    transition: transform var(--transition-slow), opacity var(--transition-slow);
  }

  .bottom-sheet-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: var(--sp-md);
  }

  .stage-counter {
    color: var(--cobalt-accent);
    display: block;
    margin-bottom: 4px;
  }

  .stage-title {
    color: var(--on-surface);
    margin-bottom: 4px;
  }

  .stage-desc {
    color: var(--on-surface-variant);
    font-size: var(--text-metadata);
  }

  .header-actions {
    display: flex;
    gap: var(--sp-xs);
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: var(--surface-container-high);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    cursor: pointer;
    color: var(--on-surface-variant);
    transition: all var(--transition-fast);
  }

  .icon-btn:hover:not(:disabled) {
    border-color: var(--cobalt-accent);
    color: var(--cobalt-accent);
  }

  .icon-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .icon-btn .material-symbols-outlined {
    font-size: 18px;
  }

  .automagic-controls {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sp-sm);
    padding: var(--sp-md) 0;
  }

  .btn-automagic-process {
    display: flex;
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

  .btn-automagic-process:hover {
    background: var(--primary-container);
    box-shadow: 0 0 16px var(--glow-cobalt-strong);
  }

  .btn-automagic-process .material-symbols-outlined {
    font-size: 20px;
  }

  .automagic-hint {
    color: var(--on-surface-variant);
  }

  .param-section {
    margin-bottom: var(--sp-md);
  }

  .param-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--sp-xs);
  }

  .param-value {
    color: var(--cobalt-accent);
    font-variant-numeric: tabular-nums;
  }

  .param-slider {
    width: 100%;
    height: 4px;
    -webkit-appearance: none;
    appearance: none;
    background: var(--surface-container-highest);
    border-radius: 2px;
    outline: none;
    cursor: pointer;
  }

  .param-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--cobalt-accent);
    cursor: pointer;
    box-shadow: 0 0 6px var(--glow-cobalt-strong);
    transition: transform var(--transition-fast);
  }

  .param-slider::-webkit-slider-thumb:hover {
    transform: scale(1.2);
  }

  .param-slider::-moz-range-thumb {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--cobalt-accent);
    cursor: pointer;
    border: none;
    box-shadow: 0 0 6px var(--glow-cobalt-strong);
  }

  .ai-suggestion {
    padding: var(--sp-md);
    border-radius: var(--radius-lg);
    margin-bottom: var(--sp-md);
    border: 1px solid var(--tertiary-container);
  }

  .ai-suggestion-header {
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
    margin-bottom: var(--sp-xs);
  }

  .ai-icon {
    font-size: 18px;
    color: var(--tertiary-container);
  }

  .ai-text {
    color: var(--on-surface-variant);
    margin-bottom: var(--sp-sm);
  }

  .ai-suggestion-actions {
    display: flex;
    gap: var(--sp-sm);
  }

  .btn-accept {
    padding: 6px 16px;
    background: var(--tertiary-container);
    color: var(--surface);
    border: none;
    border-radius: var(--radius-md);
    font-family: var(--font-body);
    font-size: var(--text-metadata);
    font-weight: 600;
    cursor: pointer;
  }

  .btn-refine {
    padding: 6px 16px;
    background: transparent;
    color: var(--on-surface-variant);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    font-family: var(--font-body);
    font-size: var(--text-metadata);
    font-weight: 500;
    cursor: pointer;
  }

  .bottom-sheet-actions {
    display: flex;
    justify-content: space-between;
    gap: var(--sp-sm);
    margin-top: var(--sp-md);
    padding-top: var(--sp-md);
    border-top: 1px solid var(--outline-variant);
  }

  .btn-primary,
  .btn-secondary {
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
    padding: 10px 24px;
    border-radius: var(--radius-md);
    font-family: var(--font-body);
    font-size: var(--text-body);
    font-weight: 600;
    cursor: pointer;
    border: none;
    transition: all var(--transition-fast);
  }

  .btn-primary {
    background: var(--cobalt-accent);
    color: var(--surface);
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--primary-container);
    box-shadow: 0 0 12px var(--glow-cobalt-mid);
  }

  .btn-primary:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
  }

  .btn-secondary:hover:not(:disabled) {
    color: var(--on-surface);
  }

  .btn-secondary:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .btn-primary .material-symbols-outlined,
  .btn-secondary .material-symbols-outlined {
    font-size: 18px;
  }
</style>
