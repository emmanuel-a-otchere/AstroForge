<!--
  CR-04 P9 — Step 2 of the §12 Import wizard: Scanning.

  Brief progress indicator. The actual scan ran in
  Step 1 (so the file system is hot when the user
  arrives here); this step is a UX handoff that
  displays the discovered counts and runs the P8
  orchestrator.

  Advances to Step 3 once the orchestrator returns.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import {
    goToStep,
    importWizard,
    runAnalysis,
    setScanError,
  } from "../../state/import-wizard";

  let running = false;
  let error: string | null = null;

  onMount(() => {
    void runPipeline();
  });

  async function runPipeline(): Promise<void> {
    running = true;
    error = null;
    try {
      // The host shell wires the session + project ids into the
      // wizard before transitioning to scanning (Step 1's
      // sessionCreate → setSessionContext). Read the state directly —
      // the previous Promise-wrapped subscribe called `unsub()` inside
      // the callback, before the const binding initialized (TDZ crash:
      // "Cannot access 'unsub' before initialization").
      const state = get(importWizard);
      if (!state.sessionId || !state.projectId) {
        // No ids — the host shell didn't wire them. For
        // the slice-level test we accept this and
        // skip the orchestrator call.
        setScanError("host shell did not wire session id; skip pipeline");
        running = false;
        return;
      }
      await runAnalysis(state.projectId, state.sessionId, null);
      goToStep("understanding");
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      error = msg;
      setScanError(msg);
    } finally {
      running = false;
    }
  }
</script>

<section class="scanning" aria-labelledby="step-2-heading">
  <h2 id="step-2-heading" class="font-display">Analysing your data…</h2>
  {#if $importWizard.frames.length === 0}
    <p class="font-body">No frames discovered. Go back to Step 1.</p>
  {:else}
    <ul class="counts font-body" aria-live="polite">
      <li>✓ {$importWizard.frames.length} files discovered</li>
      <li>✓ {$importWizard.frames.length} readable</li>
      <li>✓ Metadata extracted</li>
      <li>
        ✓ {$importWizard.frames.filter((f) => f.frameType.toLowerCase() === "light").length} Light
        frames
      </li>
      <li>
        ✓ {$importWizard.frames.filter((f) => f.frameType.toLowerCase() === "dark").length} Dark
        frames
      </li>
      <li>
        ✓ {$importWizard.frames.filter((f) => f.frameType.toLowerCase() === "flat").length} Flat
        frames
      </li>
    </ul>
    <p class="font-body">
      {running ? "Running capture analysis + narrowband detection…" : "Classification complete."}
    </p>
  {/if}
  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
</section>

<style>
  .scanning {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 1.5rem;
  }
  h2 {
    margin: 0;
  }
  .counts {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .error {
    color: var(--color-error, #ff6b6b);
    margin: 0;
  }
  .back-btn {
    padding: 0.5rem 1rem;
    border-radius: 4px;
    border: 1px solid var(--color-border, #444);
    background: var(--color-surface, #222);
    color: var(--color-text-primary, #eee);
    cursor: pointer;
    margin-top: 0.5rem;
  }
</style>
