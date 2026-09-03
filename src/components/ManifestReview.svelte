<script lang="ts">
  /**
   * ManifestReview — Phase 9 vertical slice UI.
   *
   * Lets the user pick a folder of FITS frames, scan + classify them,
   * review the manifest, and trigger the MVP pipeline. Lives behind
   * a single collapseable panel so it doesn't fight the existing Mode
   * B load flow for screen real estate.
   *
   * Wires:
   *   pickFitsFolder() → src/lib/pipeline.ts
   *   scanDirectory() → Tauri ingest_scan_directory
   *   runPipeline() → Tauri pipeline_run_session
   */
  import {
    pickFitsFolder,
    scanDirectory,
    runPipeline,
    countByType,
    totalExposure,
    formatExposure,
    type IngestFrame,
    type PipelineResult,
  } from "../lib/pipeline";

  export let sessionId: string;

  let folder: string | null = null;
  let frames: IngestFrame[] = [];
  let scanning = false;
  let running = false;
  let lastResult: PipelineResult | null = null;
  let errorMessage: string | null = null;
  let open = false;

  $: counts = countByType(frames);
  $: exposure = totalExposure(frames);
  $: hasLights = counts.LIGHT > 0;

  async function handlePick() {
    errorMessage = null;
    lastResult = null;
    frames = [];
    folder = await pickFitsFolder();
    if (!folder) return;
    await handleScan();
  }

  async function handleScan() {
    if (!folder) return;
    scanning = true;
    errorMessage = null;
    try {
      frames = await scanDirectory(folder);
    } catch (err) {
      errorMessage = (err as Error).message ?? String(err);
    } finally {
      scanning = false;
    }
  }

  async function handleRun() {
    if (!folder || !hasLights) return;
    running = true;
    errorMessage = null;
    try {
      lastResult = await runPipeline(sessionId, folder, "beginner");
      if (!lastResult.success && lastResult.error) {
        errorMessage = lastResult.error;
      }
    } catch (err) {
      errorMessage = (err as Error).message ?? String(err);
    } finally {
      running = false;
    }
  }

  function shortPath(p: string): string {
    return p.split(/[\\/]/).pop() ?? p;
  }
</script>

<section class="manifest-review">
  <header>
    <button
      type="button"
      class="toggle"
      on:click={() => (open = !open)}
      aria-expanded={open}
    >
      {open ? "▾ Hide" : "▸"} Pick folder & review manifest
    </button>
  </header>

  {#if open}
    <div class="panel">
      <div class="row">
        <button
          type="button"
          class="btn-primary"
          on:click={handlePick}
          disabled={scanning || running}
        >
          {scanning ? "Scanning…" : folder ? "Re-pick folder" : "Pick folder…"}
        </button>
        {#if folder}
          <code class="folder-path" title={folder}>{folder}</code>
        {/if}
      </div>

      {#if errorMessage}
        <p class="error">⚠ {errorMessage}</p>
      {/if}

      {#if frames.length > 0}
        <div class="counts">
          <span class="count count-light">{counts.LIGHT} lights</span>
          <span class="count count-dark">{counts.DARK} darks</span>
          <span class="count count-flat">{counts.FLAT} flats</span>
          <span class="count count-bias">{counts.BIAS} biases</span>
          <span class="count count-total">{formatExposure(exposure)}</span>
        </div>

        <table class="frame-table" aria-label="Frame manifest">
          <thead>
            <tr>
              <th>File</th>
              <th>Type</th>
              <th>Filter</th>
              <th>Exp (s)</th>
              <th>Size</th>
            </tr>
          </thead>
          <tbody>
            {#each frames as frame (frame.path)}
              <tr>
                <td class="path" title={frame.path}>{shortPath(frame.path)}</td>
                <td><span class="pill pill-{frame.frameType.toLowerCase()}">{frame.frameType}</span></td>
                <td>{frame.filter ?? "—"}</td>
                <td class="num">{frame.exptime?.toFixed(0) ?? "—"}</td>
                <td class="num">{frame.width ?? "?"}×{frame.height ?? "?"}</td>
              </tr>
            {/each}
          </tbody>
        </table>

        {#if !hasLights}
          <p class="warning">
            No light frames in this folder — pipeline needs at least one.
          </p>
        {/if}

        <div class="actions">
          <button
            type="button"
            class="btn-primary"
            on:click={handleRun}
            disabled={!hasLights || running || scanning}
          >
            {running ? "Running…" : "Run MVP pipeline"}
          </button>
        </div>

        {#if lastResult}
          <div class="result">
            <h4>Pipeline result</h4>
            <dl>
              <dt>Success</dt><dd>{lastResult.success ? "yes" : "no"}</dd>
              <dt>Lights processed</dt>
              <dd>{lastResult.report.frameStats.lights}</dd>
              <dt>Total exposure</dt>
              <dd>{formatExposure(lastResult.report.frameStats.totalExposure)}</dd>
              <dt>Stages recorded</dt>
              <dd>{lastResult.report.stageParameters.length}</dd>
            </dl>
            {#if lastResult.report.stageParameters.length > 0}
              <ul class="stages">
                {#each lastResult.report.stageParameters as s}
                  <li>
                    <code>{s.stageId}</code>
                    {#each Object.entries(s.params) as [k, v]}
                      <span class="stage-param">{k}={v}</span>
                    {/each}
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</section>

<style>
  .manifest-review {
    width: 100%;
    margin-top: var(--sp-md, 1rem);
  }

  .toggle {
    background: var(--overlay-glass);
    border: 1px solid var(--glow-primary-hairline);
    border-radius: 999px;
    color: var(--text-primary);
    font-size: 0.78rem;
    padding: 0.4rem 0.9rem;
    cursor: pointer;
  }
  .toggle:hover {
    background: var(--overlay-card);
  }

  .panel {
    margin-top: var(--sp-sm, 0.5rem);
    display: flex;
    flex-direction: column;
    gap: var(--sp-md, 1rem);
    padding: var(--sp-md, 1rem);
    background: var(--overlay-card);
    border: 1px solid var(--glow-primary-hairline);
    border-radius: var(--radius-lg, 0.75rem);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--sp-sm, 0.5rem);
    flex-wrap: wrap;
  }

  .btn-primary {
    background: var(--cobalt-accent);
    color: var(--text-on-accent);
    border: none;
    border-radius: 999px;
    padding: 0.45rem 1rem;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .folder-path {
    font-size: 0.7rem;
    color: var(--text-secondary);
    background: var(--overlay-glass);
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .error {
    color: var(--error-fg);
    font-size: 0.78rem;
    margin: 0;
  }
  .warning {
    color: var(--warning-fg);
    font-size: 0.78rem;
    margin: 0;
  }

  .counts {
    display: flex;
    gap: var(--sp-sm, 0.5rem);
    flex-wrap: wrap;
  }
  .count {
    background: var(--overlay-glass);
    border-radius: 999px;
    padding: 0.2rem 0.7rem;
    font-size: 0.7rem;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
  .count-light { color: var(--cobalt-accent); }
  .count-dark { color: var(--text-secondary); }

  .frame-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.75rem;
  }
  .frame-table th {
    text-align: left;
    padding: 0.3rem 0.5rem;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--glow-primary-hairline);
    font-weight: 500;
  }
  .frame-table td {
    padding: 0.3rem 0.5rem;
    border-bottom: 1px solid var(--overlay-glass);
  }
  .frame-table .path {
    color: var(--text-primary);
    max-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .frame-table .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .pill {
    padding: 0.05rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.65rem;
    text-transform: uppercase;
  }
  .pill-light {
    background: var(--glow-cobalt-dim);
    color: var(--cobalt-accent);
  }
  .pill-dark {
    background: var(--overlay-glass);
    color: var(--text-secondary);
  }
  .pill-flat {
    background: var(--glow-tertiary, var(--tertiary-container));
    color: var(--text-primary);
  }
  .pill-bias {
    background: var(--overlay-glass);
    color: var(--text-muted);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
  }

  .result {
    margin-top: var(--sp-sm, 0.5rem);
    padding: var(--sp-sm, 0.5rem);
    background: var(--overlay-glass);
    border-radius: var(--radius-md, 0.5rem);
    font-size: 0.78rem;
  }
  .result h4 {
    margin: 0 0 0.4rem;
    font-size: 0.78rem;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .result dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.2rem 0.6rem;
    margin: 0;
  }
  .result dt {
    color: var(--text-secondary);
  }
  .result dd {
    margin: 0;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }
  .stages {
    list-style: none;
    margin: 0.5rem 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .stages li {
    font-size: 0.7rem;
  }
  .stages code {
    color: var(--cobalt-accent);
    margin-right: 0.4rem;
  }
  .stage-param {
    background: var(--overlay-card);
    padding: 0.05rem 0.35rem;
    border-radius: 0.25rem;
    margin-right: 0.2rem;
    font-size: 0.65rem;
  }
</style>
