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
    <!-- #460 W3 — the step counter reads as a flat gray band; the
         accent hairline + progress dots give it identity and state. -->
    <div class="step-row">
      <span class="step-label font-body">{describeStep($importWizard.step)}</span>
      <ol class="step-dots" aria-hidden="true">
        {#each ["add-data", "scanning", "understanding", "confirm"] as dot, i (dot)}
          <li
            class="dot"
            class:dot-active={$importWizard.step === dot}
            class:dot-done={i <
              ["add-data", "scanning", "understanding", "confirm"].indexOf(
                $importWizard.step,
              )}
          ></li>
        {/each}
      </ol>
    </div>
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
    border-bottom: 1px solid var(--outline-variant);
    background: var(--surface-container-low);
    /* Accent hairline on the leading edge — the banner carries the
       theme's warm identity instead of anonymous gray. */
    border-left: 2px solid var(--cobalt-accent);
  }
  .step-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-md);
  }
  .step-label {
    color: var(--on-surface-variant);
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .step-dots {
    display: flex;
    gap: 6px;
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--surface-container-highest);
    border: 1px solid var(--outline-variant);
    transition: background var(--transition-fast), border-color var(--transition-fast),
      box-shadow var(--transition-fast);
  }
  .dot-active {
    background: var(--cobalt-accent);
    border-color: var(--cobalt-accent);
    box-shadow: 0 0 6px var(--glow-cobalt-mid);
  }
  .dot-done {
    background: var(--outline);
    border-color: var(--outline);
  }
  .step-content {
    flex: 1;
  }
</style>
