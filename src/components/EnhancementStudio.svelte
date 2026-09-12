<!--
  CR-06 P4 — Enhancement Studio (Zone A/B/C shell).

  The Studio is the user-facing AI Enhancement surface
  per CR-06 §13 — three zones laid out left-to-right
  on the CR-03 Studio shell:

    Zone A — Image / Version Context
      Current Image Version, source version,
      enhancement history, masks, operation stack.
      P4 ships the version timeline + the operation
      stack list.

    Zone B — Image Canvas
      The primary workspace. P4 shows the active
      Image Version's label + sequence + the safety
      classification of the next operation (a
      preview-before-apply gate per CR-06 §15). Real
      pixel rendering is CR-07's territory; P4 ships
      the metadata-only canvas placeholder.

    Zone C — AI Intelligence Panel
      The analysis observations + the recommendation
      list (P2/P3) + the operations picker +
      per-operation cards (P4). The "Apply" button on
      each card creates a new Image Version per
      CR-06 §4 / §22 (non-destructive).

  The Studio panel reads everything from
  `aiEnhancementStore` and dispatches mutations +
  apply rounds through the helpers in
  `state/ai-enhancement.ts`. Apply + reorder +
  enable/disable + branch all work end-to-end against
  the P4 passthrough dispatcher; P5 swaps the
  dispatcher body for real ONNX inference.
-->
<script lang="ts">
  import {
    aiEnhancementStore,
    applyOperation,
    applyStackMutation,
    branchEnhancementStack,
    createEnhancementStack,
    generateRecommendationsFor,
    listImageVersions,
  } from "../state/ai-enhancement";
  import EnhancementOperationCard from "./EnhancementOperationCard.svelte";
  import ImageCanvas from "./ImageCanvas.svelte";
  import MaskEditor from "./MaskEditor.svelte";
  import QualityGatePanel from "./QualityGatePanel.svelte";
  import type {
    EnhancementStackJson,
    OperationRegistryEntryJson,
    StackMutation,
  } from "../lib/astroforge-api";

  interface ImageVersionJson {
    version_id: string;
    project_id: string;
    label: string;
    sequence: number;
    primary_artifact_id: string;
    source_version_id: string | null;
    created_at: string;
    hidden: boolean;
  }

  let versions: ImageVersionJson[] = [];
  let newBranchName = "";
  let branchFeedback: string | null = null;

  $: stack = $aiEnhancementStore.activeStack as EnhancementStackJson | null;
  $: opsRegistry = ($aiEnhancementStore.operationsRegistry ??
    []) as OperationRegistryEntryJson[];
  $: report = $aiEnhancementStore.recommendationsReport as
    | {
        recommendations?: Array<{ operation: string }>;
        sequencing_notes?: Array<{ moved: string }>;
      }
    | null;
  $: analysis = $aiEnhancementStore.analysis as
    | {
        image_version_id?: string;
        project_id?: string;
        engine_version?: string;
      }
    | null;
  $: imageVersionId =
    (analysis?.image_version_id as string | undefined) ??
    (stack?.source_image_version_id as string | undefined) ??
    "";
  $: projectId =
    (analysis?.project_id as string | undefined) ??
    (stack?.project_id as string | undefined) ??
    "";

  // Load the project's image versions on mount so Zone A's
  // timeline can render. The list refreshes whenever the
  // Studio mounts (the parent re-mounts on workspace
  // change).
  $: if (projectId) {
    void refreshVersions();
  }

  async function refreshVersions(): Promise<void> {
    if (!projectId) return;
    const list = (await listImageVersions(projectId)) as ImageVersionJson[];
    versions = list;
  }

  async function onAcceptRecommendations(): Promise<void> {
    if (!projectId || !imageVersionId) return;
    try {
      await generateRecommendationsFor({
        project_id: projectId,
        image_version_id: imageVersionId,
      });
    } catch (err) {
      console.error("generateRecommendationsFor failed", err);
    }
  }

  async function onCreateStackFromRecommendations(): Promise<void> {
    if (!projectId || !imageVersionId) return;
    const recs = $aiEnhancementStore.recommendations as Array<{
      operation: string;
      confidence: number;
      recommended_model: string | null;
      payload_json: string;
    }>;
    if (!Array.isArray(recs) || recs.length === 0) {
      branchFeedback =
        "No recommendations available. Generate them first.";
      return;
    }
    const operations = recs.map((r) => {
      let params = "{}";
      try {
        const parsed = JSON.parse(r.payload_json);
        params = JSON.stringify({
          recommendation_confidence: r.confidence,
          recommended_model: r.recommended_model,
        });
        void parsed;
      } catch {
        params = JSON.stringify({
          recommendation_confidence: r.confidence,
        });
      }
      return {
        operation_id: r.operation,
        recommendation_id: null,
        parameters_json: params,
        enabled: true,
      };
    });
    try {
      await createEnhancementStack({
        project_id: projectId,
        source_image_version_id: imageVersionId,
        operations,
      });
      branchFeedback = `Stack created with ${operations.length} operations.`;
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      branchFeedback = `Stack create failed: ${msg}`;
    }
  }

  async function onMutation(mutation: StackMutation): Promise<void> {
    if (!stack) return;
    try {
      await applyStackMutation(stack.stack_id, mutation);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      branchFeedback = `Stack mutation failed: ${msg}`;
    }
  }

  async function onApplyOperation(
    operationId: string,
    parametersJson: string,
  ): Promise<void> {
    if (!projectId || !imageVersionId) return;
    try {
      const result = await applyOperation({
        project_id: projectId,
        source_image_version_id: imageVersionId,
        operation_id: operationId,
        parameters_json: parametersJson,
        preview_id: null,
      });
      branchFeedback = `Applied ${operationId} → new version ${result.versionId}`;
      await refreshVersions();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      branchFeedback = `Apply failed: ${msg}`;
    }
  }

  async function onBranch(cutoff: number): Promise<void> {
    if (!stack) return;
    const trimmed = newBranchName.trim() || `branch-${cutoff}`;
    const newImageVersionId = `ver_${trimmed}_${Date.now()}`;
    try {
      await branchEnhancementStack({
        source_stack_id: stack.stack_id,
        cutoff,
        new_image_version_id: newImageVersionId,
      });
      branchFeedback = `Branched at ${cutoff} as ${trimmed}.`;
      newBranchName = "";
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      branchFeedback = `Branch failed: ${msg}`;
    }
  }

  function registryEntry(id: string): OperationRegistryEntryJson | null {
    return opsRegistry.find((op) => op.operation_id === id) ?? null;
  }

  function safetyLabel(c: string): string {
    return c.charAt(0).toUpperCase() + c.slice(1);
  }
</script>

<section class="enhancement-studio" aria-label="Enhancement Studio">
  <!-- Zone A — Image / Version Context -->
  <aside class="zone zone-a" aria-label="Image context">
    <header>
      <h3>Context</h3>
    </header>
    {#if versions.length === 0}
      <p class="empty">No Image Versions yet. Apply an operation to create v1.</p>
    {:else}
      <ol class="version-timeline" aria-label="Version timeline">
        {#each versions as v}
          <li class="version-row" class:active={v.version_id === imageVersionId}>
            <span class="version-seq">v{v.sequence}</span>
            <span class="version-label">{v.label || "—"}</span>
            <span class="version-id">{v.version_id}</span>
          </li>
        {/each}
      </ol>
    {/if}

    <h4>Active Stack</h4>
    {#if !stack}
      <p class="empty">No stack loaded. Generate recommendations, then build a stack.</p>
    {:else}
      <ol class="stack-list" aria-label="Operation stack">
        {#each stack.operations as op, index}
          {@const entry = registryEntry(op.operation_id)}
          <li class="stack-row" class:disabled={!op.enabled}>
            <span class="stack-step">{index + 1}</span>
            <span class="stack-op">
              {entry?.display_name ?? op.operation_id}
            </span>
            <span class="stack-safety safety-{entry?.safety_classification ?? "deterministic"}">
              {entry ? safetyLabel(entry.safety_classification) : "Deterministic"}
            </span>
            <button
              type="button"
              class="stack-btn"
              title={op.enabled ? "Disable" : "Enable"}
              on:click={() =>
                onMutation({
                  SetEnabled: {
                    operation_id: op.operation_id,
                    enabled: !op.enabled,
                  },
                })}
            >
              {op.enabled ? "On" : "Off"}
            </button>
            <button
              type="button"
              class="stack-btn danger"
              title="Remove"
              on:click={() =>
                onMutation({ Remove: { operation_id: op.operation_id } })}
            >
              ✕
            </button>
          </li>
        {/each}
      </ol>
      <div class="branch-controls">
        <input
          type="text"
          placeholder="Branch name"
          bind:value={newBranchName}
        />
        <button
          type="button"
          class="primary"
          on:click={() => onBranch(stack.operations.length)}
          disabled={stack.operations.length === 0}
        >
          Branch full
        </button>
        <button
          type="button"
          on:click={() => onBranch(stack.operations.length - 1)}
          disabled={stack.operations.length === 0}
        >
          Branch before last
        </button>
      </div>
    {/if}
    {#if branchFeedback}
      <p class="feedback">{branchFeedback}</p>
    {/if}
  </aside>

  <!-- Zone B — Image Canvas (CR-07 pixel render) -->
  <section class="zone zone-b" aria-label="Image canvas">
    {#if analysis}
      <header>
        <h3>Active Image Version</h3>
        <p class="meta">
          {imageVersionId} · engine
          <code>{(analysis as { engine_version?: string }).engine_version}</code>
        </p>
      </header>
      <div class="canvas-mount" data-testid="studio-canvas">
        {#if imageVersionId}
          <ImageCanvas {imageVersionId} />
        {:else}
          <p class="empty">No active Image Version.</p>
        {/if}
      </div>
    {:else}
      <p class="empty">
        No analysis yet. Run the analyzer (P2) on the active
        Image Version to populate the Studio.
      </p>
    {/if}
  </section>

  <!-- Zone C — AI Intelligence Panel -->
  <aside class="zone zone-c" aria-label="AI intelligence">
    <header>
      <h3>Intelligence</h3>
    </header>
    <button
      type="button"
      class="primary"
      on:click={onAcceptRecommendations}
      disabled={!imageVersionId || $aiEnhancementStore.loading}
    >
      Regenerate recommendations
    </button>
    {#if !$aiEnhancementStore.recommendationsReport}
      <p class="empty">
        Generate the recommendations above to see the engine
        output.
      </p>
    {:else if opsRegistry.length === 0}
      <p class="empty">Loading operations registry…</p>
    {:else}
      <button
        type="button"
        class="primary"
        on:click={onCreateStackFromRecommendations}
        disabled={!imageVersionId}
      >
        Build stack from recommendations
      </button>
      <h4>Operations</h4>
      <ul class="op-list" aria-label="Operations registry">
        {#each opsRegistry as op}
          <li>
            <EnhancementOperationCard
              operation={op}
              onApply={(params) => onApplyOperation(op.operation_id, params)}
            />
          </li>
        {/each}
      </ul>
      <!-- CR-06 P5 — region-aware mask editor. Lives
        below the operations list in Zone C; the
        editor shell exercises auto + composite
        build flows. Painting (brush / polygon) ships
        in a future slice. -->
      <MaskEditor />
      <!-- CR-06 P6 — quality gate panel. Lives below
        the Mask Editor in Zone C; the panel surfaces
        the §37 verdict + per-gate findings. The
        user can run the gate manually against the
        current image; the apply round will populate
        this automatically once real ONNX inference
        produces a distinct result. -->
      <QualityGatePanel />
    {/if}
    {#if $aiEnhancementStore.error}
      <p class="error">{$aiEnhancementStore.error}</p>
    {/if}
  </aside>
</section>

<style>
  .enhancement-studio {
    display: grid;
    grid-template-columns: 22rem 1fr 24rem;
    gap: var(--sp-md, 1rem);
    padding: var(--sp-sm, 0.5rem);
    min-height: 100%;
  }
  .zone {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm, 0.5rem);
    padding: var(--sp-sm, 0.75rem);
    border: 1px solid var(--color-border, #2a2f3a);
    border-radius: 6px;
    background: var(--color-surface, #15181f);
    overflow-y: auto;
  }
  .zone h3 {
    margin: 0;
    font-size: 1rem;
  }
  .zone h4 {
    margin: var(--sp-md, 0.75rem) 0 var(--sp-xs, 0.25rem) 0;
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-text-muted, #9aa3b2);
  }
  .empty {
    margin: 0;
    padding: var(--sp-sm, 0.5rem);
    border: 1px dashed var(--color-border, #2a2f3a);
    border-radius: 4px;
    color: var(--color-text-muted, #9aa3b2);
    font-size: 0.85rem;
  }
  .feedback {
    margin: 0;
    padding: var(--sp-xs, 0.25rem) var(--sp-sm, 0.5rem);
    border-left: 3px solid var(--color-accent, #4f9cff);
    background: var(--color-surface-muted, rgba(79, 156, 255, 0.08));
    border-radius: 3px;
    font-size: 0.8rem;
  }
  .error {
    margin: 0;
    padding: var(--sp-xs, 0.25rem) var(--sp-sm, 0.5rem);
    border-left: 3px solid #ff6464;
    background: rgba(255, 100, 100, 0.08);
    border-radius: 3px;
    font-size: 0.8rem;
    color: #ff6464;
  }
  .version-timeline,
  .stack-list,
  .op-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs, 0.25rem);
  }
  .version-row,
  .stack-row {
    display: flex;
    align-items: center;
    gap: var(--sp-xs, 0.5rem);
    padding: var(--sp-xs, 0.25rem) var(--sp-sm, 0.5rem);
    border-radius: 4px;
    background: var(--color-surface-muted, #1f242d);
    font-size: 0.85rem;
  }
  .version-row.active {
    border-left: 3px solid var(--color-accent, #4f9cff);
  }
  .stack-row.disabled {
    opacity: 0.55;
  }
  .stack-step {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.4rem;
    height: 1.4rem;
    border-radius: 50%;
    background: var(--color-surface, #15181f);
    font-weight: 600;
    font-size: 0.8rem;
  }
  .stack-safety {
    margin-left: auto;
    padding: 0.1rem 0.4rem;
    border-radius: 3px;
    font-size: 0.7rem;
  }
  .stack-safety.safety-deterministic {
    background: rgba(80, 200, 120, 0.15);
    color: #50c878;
  }
  .stack-safety.safety-perceptual {
    background: rgba(255, 195, 0, 0.15);
    color: #ffc300;
  }
  .stack-safety.safety-generative {
    background: rgba(255, 138, 79, 0.2);
    color: #ff8a4f;
  }
  .stack-btn {
    background: var(--color-surface, #15181f);
    color: var(--color-text, #e6e9ef);
    border: 1px solid var(--color-border, #2a2f3a);
    border-radius: 3px;
    padding: 0.1rem 0.4rem;
    font-size: 0.75rem;
    cursor: pointer;
  }
  .stack-btn.danger:hover {
    background: rgba(255, 100, 100, 0.15);
  }
  .branch-controls {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-xs, 0.25rem);
    margin-top: var(--sp-xs, 0.5rem);
  }
  .branch-controls input {
    flex: 1 1 8rem;
    background: var(--color-surface-muted, #1f242d);
    color: var(--color-text, #e6e9ef);
    border: 1px solid var(--color-border, #2a2f3a);
    border-radius: 3px;
    padding: 0.2rem 0.4rem;
    font-size: 0.8rem;
  }
  .primary {
    background: var(--color-accent, #4f9cff);
    color: var(--color-on-accent, #fff);
    border: none;
    border-radius: 4px;
    padding: 0.3rem 0.6rem;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .primary:disabled {
    background: var(--color-surface-muted, #1f242d);
    color: var(--color-text-muted, #9aa3b2);
    cursor: not-allowed;
  }
  .canvas-mount {
      width: 100%;
      flex: 1 1 auto;
      min-height: 360px;
      border: 1px solid var(--outline-variant);
      border-radius: var(--radius-md);
      overflow: hidden;
      background: #000;
    }

    .canvas-placeholder {
      padding: 16px;
      border: 1px dashed var(--outline-variant);
      border-radius: var(--radius-md);
      color: var(--on-surface-variant);
      background: var(--surface-container-low);
    }
</style>