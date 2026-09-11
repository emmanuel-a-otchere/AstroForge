<!--
  CR-06 P5 — Mask Editor.

  The Mask Editor lives in the Enhancement Studio's
  Zone C (per CR-06 §13). It lists the existing masks
  for the active Image Version, lets the user pick
  one as the "current" mask, and offers three actions:

  - **Build auto mask** — calls `build_auto_mask` on
    the backend with a target kind (stars,
    background, bright_core) and the latest analysis
    pixels. The result is a fresh `AiMask` row.
  - **Compose two masks** — calls `compose_mask`
    with the two selected masks + a boolean operator
    (union, intersect, difference). The result is a
    fresh `AiMask` row tagged as `composite`
    provenance.
  - **Refresh from store** — re-reads the per-Image
    Version mask list so the picker reflects any
    background changes.

  The actual brush / polygon painting is a separate
  surface (`MaskPainter.svelte` ships in a later
  slice); P5 ships the editor shell + the
  server-side auto + composite build flows so the
  editor is exercisable end-to-end on CI without a
  GPU.
-->
<script lang="ts">
  import {
    aiMaskListForVersion,
    buildAutoMask,
    composeMask,
    createAiMask,
    type AiMaskJson,
    type AutoMaskTarget,
    type CompositeMaskOp,
  } from "../lib/astroforge-api";
  import { aiEnhancementStore } from "../state/ai-enhancement";

  let masks: AiMaskJson[] = [];
  let feedback: string | null = null;
  let selectedA: string = "";
  let selectedB: string = "";
  let composeOp: CompositeMaskOp = "union";
  let autoTarget: AutoMaskTarget = "stars";

  $: imageVersionId =
    ($aiEnhancementStore.analysis as { image_version_id?: string } | null)
      ?.image_version_id ??
    "";
  $: projectId =
    ($aiEnhancementStore.analysis as { project_id?: string } | null)
      ?.project_id ??
    "";
  $: analysis =
    ($aiEnhancementStore.analysis as {
      width?: number;
      height?: number;
      channels?: number;
      pixels?: number[];
    } | null) ?? null;

  // Refresh on mount + whenever the active Image
  // Version changes.
  $: if (imageVersionId) {
    void refresh();
  }

  async function refresh(): Promise<void> {
    if (!imageVersionId) return;
    try {
      const response = await aiMaskListForVersion(imageVersionId);
      masks = Array.isArray(response?.items) ? response.items : [];
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      feedback = `Mask list failed: ${msg}`;
    }
  }

  async function buildAuto(): Promise<void> {
    if (!imageVersionId || !analysis) {
      feedback = "Run the analyzer first — no pixel buffer to segment.";
      return;
    }
    try {
      const response = await buildAutoMask({
        image_version_id: imageVersionId,
        width: analysis.width ?? 0,
        height: analysis.height ?? 0,
        channels: analysis.channels ?? 3,
        pixels: analysis.pixels ?? [],
        target: autoTarget,
      });
      // Persist the produced mask so it survives the
      // session.
      await createAiMask({
        project_id: projectId,
        image_version_id: imageVersionId,
        provenance: response.provenance,
        parents_json: null,
        mask_json: response.mask_json,
      });
      feedback = `Built auto mask (${autoTarget})`;
      await refresh();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      feedback = `Auto build failed: ${msg}`;
    }
  }

  async function compose(): Promise<void> {
    if (!selectedA || !selectedB || !imageVersionId) {
      feedback = "Pick two masks to compose.";
      return;
    }
    try {
      const response = await composeMask({
        project_id: projectId,
        image_version_id: imageVersionId,
        parent_a_id: selectedA,
        parent_b_id: selectedB,
        op: composeOp,
      });
      feedback = `Composed ${composeOp} → ${response.mask.mask_id}`;
      await refresh();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      feedback = `Compose failed: ${msg}`;
    }
  }

  function maskLabel(m: AiMaskJson): string {
    return `${m.provenance} · ${m.mask_id.slice(0, 12)}…`;
  }
</script>

<section class="mask-editor" aria-label="Mask editor">
  <header>
    <h4>Region Masks</h4>
    <button type="button" class="ghost" on:click={refresh}>Refresh</button>
  </header>

  <div class="auto-block">
    <label>
      Auto target
      <select bind:value={autoTarget}>
        <option value="stars">stars</option>
        <option value="background">background</option>
        <option value="bright_core">bright_core</option>
      </select>
    </label>
    <button type="button" class="primary" on:click={buildAuto}>
      Build auto mask
    </button>
  </div>

  <div class="compose-block">
    <label>
      Mask A
      <select bind:value={selectedA}>
        <option value="">—</option>
        {#each masks as m}
          <option value={m.mask_id}>{maskLabel(m)}</option>
        {/each}
      </select>
    </label>
    <label>
      Op
      <select bind:value={composeOp}>
        <option value="union">∪ union</option>
        <option value="intersect">∩ intersect</option>
        <option value="difference">− difference</option>
      </select>
    </label>
    <label>
      Mask B
      <select bind:value={selectedB}>
        <option value="">—</option>
        {#each masks as m}
          <option value={m.mask_id}>{maskLabel(m)}</option>
        {/each}
      </select>
    </label>
    <button
      type="button"
      class="primary"
      on:click={compose}
      disabled={!selectedA || !selectedB}
    >
      Compose
    </button>
  </div>

  {#if masks.length === 0}
    <p class="empty">No masks yet. Build an auto mask above.</p>
  {:else}
    <ol class="mask-list">
      {#each masks as m}
        <li class="mask-row" class:composite={m.provenance.includes(" ")}>
          <span class="mask-prov">{m.provenance}</span>
          <span class="mask-id">{m.mask_id.slice(0, 16)}</span>
        </li>
      {/each}
    </ol>
  {/if}

  {#if feedback}
    <p class="feedback">{feedback}</p>
  {/if}
</section>

<style>
  .mask-editor {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm, 0.5rem);
    padding: var(--sp-sm, 0.75rem);
    border: 1px solid var(--color-border, #2a2f3a);
    border-radius: 6px;
    background: var(--color-surface-muted, #1f242d);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  header h4 {
    margin: 0;
    font-size: 0.95rem;
  }
  .auto-block,
  .compose-block {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-xs, 0.5rem);
    align-items: center;
  }
  label {
    display: flex;
    flex-direction: column;
    font-size: 0.8rem;
    color: var(--color-text-muted, #9aa3b2);
  }
  select {
    background: var(--color-surface, #15181f);
    color: var(--color-text, #e6e9ef);
    border: 1px solid var(--color-border, #2a2f3a);
    border-radius: 4px;
    padding: 0.2rem 0.4rem;
    font-size: 0.85rem;
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
    background: var(--color-surface, #15181f);
    color: var(--color-text-muted, #9aa3b2);
    cursor: not-allowed;
  }
  .ghost {
    background: transparent;
    color: var(--color-text-muted, #9aa3b2);
    border: 1px solid var(--color-border, #2a2f3a);
    border-radius: 4px;
    padding: 0.2rem 0.5rem;
    font-size: 0.75rem;
    cursor: pointer;
  }
  .mask-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .mask-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.25rem 0.5rem;
    background: var(--color-surface, #15181f);
    border-radius: 3px;
    font-size: 0.8rem;
  }
  .mask-row.composite {
    border-left: 2px solid var(--color-accent, #4f9cff);
  }
  .mask-prov {
    font-weight: 500;
  }
  .mask-id {
    font-family: ui-monospace, monospace;
    color: var(--color-text-muted, #9aa3b2);
    font-size: 0.7rem;
  }
  .empty {
    margin: 0;
    padding: 0.4rem;
    border: 1px dashed var(--color-border, #2a2f3a);
    border-radius: 3px;
    color: var(--color-text-muted, #9aa3b2);
    font-size: 0.8rem;
  }
  .feedback {
    margin: 0;
    padding: 0.3rem 0.5rem;
    border-left: 3px solid var(--color-accent, #4f9cff);
    background: var(--color-surface, #15181f);
    border-radius: 3px;
    font-size: 0.8rem;
  }
</style>