<script lang="ts">
  import {
    activeStepIndex,
    pipelineGraph,
    stageDefinitions,
    updateNodeParams,
  } from "../lib/pipeline-store";
  import type { PreviewParams } from "../lib/gl-renderer";

  export let previewParams: PreviewParams;
  export let onParamsChange: (params: Partial<PreviewParams>) => void;

  $: stepIdx = $activeStepIndex;
  $: node = $pipelineGraph.nodes[stepIdx];
  $: stage = stageDefinitions[stepIdx];

  function handleParam(key: string, value: number) {
    if (node) {
      updateNodeParams(node.id, { [key]: value });
    }
    onParamsChange({ [key]: value } as Partial<PreviewParams>);
  }
</script>

<div class="param-sidebar">
  <div class="sidebar-header">
    <span class="text-label-caps">Parameters</span>
    <h3 class="text-headline-mobile stage-name">{stage?.label ?? ""}</h3>
    <p class="text-metadata stage-desc">{stage?.description ?? ""}</p>
  </div>

  <div class="param-list">
    {#if stage?.type === "stretch"}
      <div class="param-group">
        <div class="param-row">
          <label class="text-label-caps" for="p-strength">Stretch Strength</label>
          <span class="text-data param-val">{(previewParams.strength * 100).toFixed(0)}%</span>
        </div>
        <input id="p-strength" type="range" min="0" max="1" step="0.01"
          value={previewParams.strength}
          on:input={(e) => handleParam("strength", parseFloat((e.target as HTMLInputElement).value))}
          class="slider" />
      </div>
      <div class="param-group">
        <div class="param-row">
          <label class="text-label-caps" for="p-mid">Midtones Balance</label>
          <span class="text-data param-val">{(previewParams.midtones * 100).toFixed(0)}%</span>
        </div>
        <input id="p-mid" type="range" min="0" max="1" step="0.01"
          value={previewParams.midtones}
          on:input={(e) => handleParam("midtones", parseFloat((e.target as HTMLInputElement).value))}
          class="slider" />
      </div>
      <div class="param-group">
        <div class="param-row">
          <label class="text-label-caps" for="p-bp">Black Point</label>
          <span class="text-data param-val">{(previewParams.blackPoint * 1000).toFixed(1)}‰</span>
        </div>
        <input id="p-bp" type="range" min="0" max="0.5" step="0.001"
          value={previewParams.blackPoint}
          on:input={(e) => handleParam("blackPoint", parseFloat((e.target as HTMLInputElement).value))}
          class="slider" />
      </div>
      <div class="param-group">
        <div class="param-row">
          <label class="text-label-caps" for="p-hl">Highlights</label>
          <span class="text-data param-val">{(previewParams.highlights * 100).toFixed(0)}%</span>
        </div>
        <input id="p-hl" type="range" min="0.5" max="1" step="0.01"
          value={previewParams.highlights}
          on:input={(e) => handleParam("highlights", parseFloat((e.target as HTMLInputElement).value))}
          class="slider" />
      </div>
    {:else if stage?.type === "star_handling"}
      <div class="param-group">
        <div class="param-row">
          <label class="text-label-caps" for="p-replace">Replace Strength</label>
          <span class="text-data param-val">{(previewParams.strength * 100).toFixed(0)}%</span>
        </div>
        <input id="p-replace" type="range" min="0" max="1" step="0.01"
          value={previewParams.strength}
          on:input={(e) => handleParam("strength", parseFloat((e.target as HTMLInputElement).value))}
          class="slider" />
      </div>
      <div class="param-group">
        <div class="param-row">
          <label class="text-label-caps" for="p-boost">Colour Boost</label>
          <span class="text-data param-val">{(previewParams.scnrStrength * 100).toFixed(0)}%</span>
        </div>
        <input id="p-boost" type="range" min="0" max="2" step="0.01"
          value={previewParams.scnrStrength}
          on:input={(e) => handleParam("scnrStrength", parseFloat((e.target as HTMLInputElement).value))}
          class="slider" />
      </div>
    {:else}
      <div class="param-group">
        <div class="param-row">
          <label class="text-label-caps" for="p-gen">Strength</label>
          <span class="text-data param-val">{(previewParams.strength * 100).toFixed(0)}%</span>
        </div>
        <input id="p-gen" type="range" min="0" max="1" step="0.01"
          value={previewParams.strength}
          on:input={(e) => handleParam("strength", parseFloat((e.target as HTMLInputElement).value))}
          class="slider" />
      </div>
    {/if}

    <div class="param-divider"></div>

    <div class="param-info">
      <span class="text-label-caps info-label">Node Version</span>
      <span class="text-data info-val">v{node?.version ?? 0}</span>
    </div>
    <div class="param-info">
      <span class="text-label-caps info-label">Node Status</span>
      <span class="text-data info-val">{node?.status ?? "pending"}</span>
    </div>
    {#if node?.receipt}
      <div class="param-info">
        <span class="text-label-caps info-label">Engine</span>
        <span class="text-data info-val">{node.receipt.engine ?? "cpu"}</span>
      </div>
      <div class="param-info">
        <span class="text-label-caps info-label">Duration</span>
        <span class="text-data info-val">{(node.receipt.durationMs / 1000).toFixed(2)}s</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .param-sidebar {
    width: 280px;
    height: 100%;
    background: var(--surface-container);
    border-left: 1px solid var(--outline-variant);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }

  .sidebar-header {
    padding: var(--sp-md);
    border-bottom: 1px solid var(--outline-variant);
  }

  .stage-name {
    margin-top: 4px;
    color: var(--on-surface);
  }

  .stage-desc {
    margin-top: 4px;
    color: var(--on-surface-variant);
  }

  .param-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--sp-md);
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
  }

  .param-group {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .param-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .param-val {
    color: var(--cobalt-accent);
    font-variant-numeric: tabular-nums;
  }

  .slider {
    width: 100%;
    height: 4px;
    -webkit-appearance: none;
    appearance: none;
    background: var(--surface-container-highest);
    border-radius: 2px;
    outline: none;
    cursor: pointer;
  }

  .slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--cobalt-accent);
    cursor: pointer;
    box-shadow: 0 0 6px rgba(203, 78, 61, 0.3);
  }

  .slider::-moz-range-thumb {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--cobalt-accent);
    cursor: pointer;
    border: none;
  }

  .param-divider {
    height: 1px;
    background: var(--outline-variant);
    margin: var(--sp-sm) 0;
  }

  .param-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .info-label {
    color: var(--on-surface-variant);
  }

  .info-val {
    color: var(--on-surface);
    font-variant-numeric: tabular-nums;
  }
</style>
