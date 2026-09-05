<!--
  DestructiveConfirmDialog — generic warning gate for destructive stages.

  Phase 1.5 PR-B3b (M7 T4): when the user commits or re-applies a stage
  in DESTRUCTIVE_STAGES (crop_rotate, sharpen_deconvolution, denoise,
  background_extraction, color_calibration), this modal asks them to
  confirm before the action proceeds.

  Why a separate component?
  - Used from multiple sites: commit button in WizardBottomSheet,
    "Re-run from here" context menu in NodeSidebar, profile-apply from
    InitialDialog (when applying a profile that includes destructive
    stages to a session that already has them).
  - Renders at the top level (modal-overlay pattern matching
    ProfileManager).
  - Emits onConfirm() only if user clicks the destructive primary
    action. Escape + click-outside cancel.
-->
<script lang="ts">
  import type { PipelineStageType } from "../lib/pipeline-store";

  export let stageType: PipelineStageType;
  export let stageLabel: string;
  export let action: "commit" | "reapply" = "commit";
  export let onConfirm: () => void = () => {};
  export let onCancel: () => void = () => {};

  // Per-stage warning copy. Each explains what makes this stage
  // destructive and what happens on undo.
  const WARNINGS: Partial<Record<PipelineStageType, string>> = {
    crop_rotate:
      "Cropping and rotation physically reposition pixels. The original is recoverable from history, but downstream stages will see the new layout immediately.",
    background_extraction:
      "Background subtraction modifies the image structure. Subtracting a wrong model can lose faint nebulosity permanently.",
    color_calibration:
      "Color balance decisions are applied multiplicatively. A wrong white balance can shift star colors in ways that are hard to undo perceptually.",
    sharpen_deconvolution:
      "Deconvolution is a non-inverse operation — information lost in PSF convolution cannot be recovered. Iterations are not reversible.",
    denoise:
      "Wavelet / SwinIR denoising discards noise plus signal in equal measure at the smallest scales. Per-pixel recovery is impossible.",
  };

  $: warning = WARNINGS[stageType] ?? "This operation modifies pixels irreversibly.";
  $: actionLabel = action === "reapply" ? "Re-run" : "Apply";
</script>

<div
  class="modal-overlay"
  role="alertdialog"
  aria-modal="true"
  aria-labelledby="destructive-title"
  aria-describedby="destructive-warning"
  tabindex="-1"
  on:click|self={onCancel}
  on:keydown={(e) => e.key === "Escape" && onCancel()}
>
  <div class="modal" role="document">
    <header class="modal-header">
      <div class="icon-wrap" aria-hidden="true">
        <span class="material-symbols-outlined">warning</span>
      </div>
      <div>
        <div class="kicker">Destructive operation</div>
        <h2 id="destructive-title">{actionLabel} {stageLabel}?</h2>
      </div>
      <button
        class="btn-close"
        on:click={onCancel}
        aria-label="Cancel and close"
      >
        <span class="material-symbols-outlined">close</span>
      </button>
    </header>

    <div class="modal-body">
      <p id="destructive-warning" class="warning-text">{warning}</p>

      <details class="more-info">
        <summary>What can I undo?</summary>
        <p>
          Undo restores the metadata (params, version, history entry) for this
          stage, but pixel data is not snapshotted in v1 — undo returns the
          stage to "active" so you can re-edit params without re-running.
        </p>
        <p>
          To discard the pixel-level change, run the stage again with the
          previous parameters after undo.
        </p>
      </details>
    </div>

    <footer class="modal-footer">
      <button class="btn-cancel" on:click={onCancel}>Cancel</button>
      <button class="btn-confirm" on:click={onConfirm}>
        {actionLabel} {stageLabel}
      </button>
    </footer>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-scrim);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 250;
    backdrop-filter: blur(4px);
  }

  .modal {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 0.75rem;
    width: 480px;
    max-width: 92vw;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
  }

  .modal-header {
    display: flex;
    align-items: flex-start;
    gap: 0.875rem;
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid var(--border);
  }

  .icon-wrap {
    background: rgba(255, 165, 0, 0.15);
    color: var(--warning, #ffa500);
    border-radius: 50%;
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .icon-wrap .material-symbols-outlined {
    font-size: 1.5rem;
  }

  .kicker {
    font-family: var(--font-data);
    font-size: 0.6875rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--warning, #ffa500);
    margin-bottom: 0.25rem;
  }

  .modal-header h2 {
    font-size: 1.0625rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
    line-height: 1.3;
  }

  .btn-close {
    margin-left: auto;
    background: none;
    border: 1px solid transparent;
    color: var(--text-secondary);
    border-radius: 0.375rem;
    padding: 0.375rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .btn-close:hover {
    color: var(--text-primary);
    border-color: var(--border);
  }

  .modal-body {
    padding: 1.25rem 1.5rem;
    overflow-y: auto;
  }

  .warning-text {
    font-size: 0.9375rem;
    line-height: 1.5;
    color: var(--text-primary);
    margin: 0 0 1rem;
  }

  .more-info {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    padding: 0.625rem 0.875rem;
    font-size: 0.8125rem;
    color: var(--text-secondary);
  }

  .more-info summary {
    cursor: pointer;
    color: var(--text-primary);
    font-weight: 600;
    list-style: none;
    padding: 0.25rem 0;
  }

  .more-info summary::-webkit-details-marker {
    display: none;
  }

  .more-info summary::before {
    content: "▸ ";
    color: var(--accent);
    font-size: 0.75rem;
  }

  .more-info[open] summary::before {
    content: "▾ ";
  }

  .more-info p {
    margin: 0.5rem 0 0;
    line-height: 1.45;
  }

  .more-info p + p {
    margin-top: 0.5rem;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    padding: 1rem 1.5rem;
    border-top: 1px solid var(--border);
    background: var(--bg-tertiary);
  }

  .btn-cancel,
  .btn-confirm {
    padding: 0.5rem 1.25rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
  }

  .btn-cancel {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }
  .btn-cancel:hover {
    color: var(--text-primary);
  }

  .btn-confirm {
    background: var(--warning, #ffa500);
    color: var(--bg-primary);
  }
  .btn-confirm:hover {
    background: var(--warning-dim, #cc8400);
  }
</style>