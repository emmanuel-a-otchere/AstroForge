<!--
  CR-04 P9 — Step 4 of the §12 Import wizard: Confirm.

  Renders the user's final confirmation controls:
  - capture kind override (radio buttons)
  - narrowband composition override (radio buttons)
  - "Confirm" + "Override + Confirm" + "Materialise" buttons

  The Materialise button marks the session as
  `materialised` (the wizard is complete); the host shell
  routes the user to the imported session.
-->
<script lang="ts">
  import {
    applyOverrides,
    captureKindLabel,
    confirmUnderstanding,
    fetchProvenance,
    goToStep,
    importWizard,
    materialiseSession,
    narrowbandCompositionLabel,
    provenanceLabel,
    setOverrideCaptureKind,
    setOverrideNarrowbandComposition,
    type CaptureKind,
    type NarrowbandComposition,
    type TargetProvenanceReportJson,
  } from "../../state/import-wizard";
  import { studioViewport } from "../../state/application";
  import { importRegisterAssets } from "../../lib/astroforge-api";

  let wizardState = $derived($importWizard);
  let classification = $derived(wizardState.classification);

  let provenance: TargetProvenanceReportJson | null = null;
  let provenanceError: string | null = null;
  let actionError: string | null = $state(null);
  let actionBusy = $state(false);

  async function viewProvenance(): Promise<void> {
    provenanceError = null;
    try {
      provenance = await fetchProvenance();
      if (!provenance) {
        provenanceError = "No provenance recorded for this session.";
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      provenanceError = msg;
    }
  }

  // #461 S2: materialisation used to only flip an enum — no
  // SourceAsset rows were ever written, so the pipeline's frame
  // loader found nothing and both workflows died before stacking.
  // Persist the scanned frames now, then land in the Process
  // workspace (the "host shell routes the user" comment finally
  // true).
  async function materialiseAndLand(): Promise<void> {
    const current = $importWizard;
    if (!current.sessionId || !current.projectId) {
      throw new Error("session context missing; cannot persist frames");
    }
    const frames = current.frames.map((f) => ({
      path: f.path,
      frameType: f.frameType,
      exptime: f.exptime,
      filter: f.filter,
      width: f.width,
      height: f.height,
    }));
    await importRegisterAssets(current.sessionId, current.projectId, frames);
    await materialiseSession();
    studioViewport.setView("process");
    goToStep("add-data");
  }

  async function onConfirm(): Promise<void> {
    actionBusy = true;
    actionError = null;
    try {
      await confirmUnderstanding();
      await materialiseAndLand();
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      actionBusy = false;
    }
  }

  async function onOverride(): Promise<void> {
    actionBusy = true;
    actionError = null;
    try {
      await applyOverrides();
      await materialiseAndLand();
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      actionBusy = false;
    }
  }
</script>

<section class="confirm" aria-labelledby="step-4-heading">
  <h2 id="step-4-heading" class="font-display">Confirm</h2>

  {#if !classification}
    <p class="font-body">No classification to confirm.</p>
  {:else}
    <fieldset>
      <legend class="font-body">Capture kind</legend>
      {#each ["deep_sky", "planetary_lunar", "ambiguous"] as kind (kind)}
        <label class="radio">
          <input
            type="radio"
            name="capture-kind"
            value={kind}
            checked={wizardState.overrideCaptureKind === kind ||
              (wizardState.overrideCaptureKind === null && classification.capture_kind === kind)}
            on:change={() => setOverrideCaptureKind(kind as CaptureKind)}
          />
          <span>{captureKindLabel(kind as CaptureKind)}</span>
        </label>
      {/each}
    </fieldset>

    <fieldset>
      <legend class="font-body">Narrowband composition</legend>
      {#each ["none", "mono", "hoo", "sho", "lrgb", "hoo_or_sho"] as composition (composition)}
        <label class="radio">
          <input
            type="radio"
            name="narrowband-composition"
            value={composition}
            checked={wizardState.overrideNarrowbandComposition === composition ||
              (wizardState.overrideNarrowbandComposition === null &&
                classification.narrowband_composition === composition)}
            on:change={() =>
              setOverrideNarrowbandComposition(composition as NarrowbandComposition)}
          />
          <span>{narrowbandCompositionLabel(composition as NarrowbandComposition)}</span>
        </label>
      {/each}
    </fieldset>

    <div class="actions">
      <button class="btn-secondary" on:click={() => goToStep("understanding")} type="button">
        Back
      </button>
      {#if actionError}
        <p class="action-error" role="alert">{actionError}</p>
      {/if}
      <button class="btn-secondary" on:click={onOverride} type="button" disabled={actionBusy}>
        Override + Confirm
      </button>
      <button class="btn-primary" on:click={onConfirm} type="button" disabled={actionBusy}>
        Confirm
      </button>
    </div>
  {/if}
</section>

<style>
  .confirm {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1.5rem;
  }
  h2 {
    margin: 0;
  }
  fieldset {
    border: 1px solid var(--color-border, #444);
    border-radius: 6px;
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  legend {
    padding: 0 0.5rem;
    color: var(--color-text-secondary, #999);
  }
  .radio {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
  }
  .btn-primary,
  .btn-secondary {
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
    border: 1px solid transparent;
  }
  .btn-primary {
    background: var(--color-accent, #4f9eff);
    color: white;
  }
  .btn-secondary {
    background: var(--color-surface, #222);
    color: var(--color-text-primary, #eee);
    border-color: var(--color-border, #444);
  }
  .action-error {
    color: var(--color-error, #b3261e);
    font-size: 0.85rem;
    margin: 0.25rem 0;
  }
</style>
