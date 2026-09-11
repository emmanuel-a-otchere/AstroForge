<!--
  CR-04 P9 — the §12 4-step Import wizard.

  Mounts the four Step components, advances through them
  per the `canTransition` rules in
  `state/import-wizard.ts`. The host shell wires
  project_id + session_id into the store before the
  user reaches Step 1.
-->
<script lang="ts">
  import {
    describeStep,
    importWizard,
  } from "../../state/import-wizard";
  import Step1AddData from "./Step1AddData.svelte";
  import Step2Scanning from "./Step2Scanning.svelte";
  import Step3Understanding from "./Step3Understanding.svelte";
  import Step4Confirm from "./Step4Confirm.svelte";
</script>

<section class="wizard" aria-live="polite">
  <header class="stepper">
    <span class="step-label font-body">{describeStep($importWizard.step)}</span>
  </header>

  <div class="step-content">
    {#if $importWizard.step === "add-data"}
      <Step1AddData />
    {:else if $importWizard.step === "scanning"}
      <Step2Scanning />
    {:else if $importWizard.step === "understanding"}
      <Step3Understanding />
    {:else if $importWizard.step === "confirm"}
      <Step4Confirm />
    {/if}
  </div>
</section>

<style>
  .wizard {
    display: flex;
    flex-direction: column;
    gap: 0;
  }
  .stepper {
    padding: 0.75rem 1.5rem;
    border-bottom: 1px solid var(--color-border, #444);
    background: var(--color-surface-elevated, #1a1a1a);
  }
  .step-label {
    color: var(--color-text-secondary, #999);
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .step-content {
    flex: 1;
  }
</style>
