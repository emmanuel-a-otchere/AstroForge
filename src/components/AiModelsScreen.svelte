<!--
  AiModelsScreen — application-level AI Models destination.

  CR-05 R1: replaces the placeholder route for the "AI Models" nav
  target. Renders the canonical AI model catalog exposed by the
  `ai_model_list` Tauri command as a table of stage + license +
  size + input shape.

  Honest failure when the IPC layer is not available: the screen
  shows a clear "catalog unavailable" message instead of
  pretending an empty list is the truth.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { aiModelList, type AiModelInfo } from "../lib/astroforge-api";

  let models: AiModelInfo[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);
  let tauri = $state(true);

  onMount(async () => {
    try {
      models = await aiModelList();
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      // Browser-mode fallback path: invoke throws because the Tauri
      // shell is absent. Mark `tauri = false` so the screen can
      // render the honest "unavailable in browser dev" notice.
      error = message;
      tauri = !/invoke/i.test(message) || /Tauri|tauri/i.test(message)
        ? false
        : true;
      if (!/Tauri|tauri/i.test(message)) tauri = true;
    } finally {
      loading = false;
    }
  });

  function formatBytes(n: number): string {
    if (n < 1024) return `${n} B`;
    const units = ["KB", "MB", "GB"];
    let val = n / 1024;
    let unit = 0;
    while (val >= 1024 && unit < units.length - 1) {
      val /= 1024;
      unit += 1;
    }
    return `${val.toFixed(1)} ${units[unit]}`;
  }

  function stageLabel(stage: string): string {
    return stage
      .split("_")
      .map((s) => s.charAt(0).toUpperCase() + s.slice(1))
      .join(" ");
  }
</script>

<section class="ai-screen" aria-label="AI Models">
  <header class="screen-header">
    <div>
      <h1 class="font-display">AI Models</h1>
      <p class="font-body">
        Inference capabilities AstroForge can route processing through. The
        catalog is bundled with the application; model install / download is
        a follow-up tranche.
      </p>
    </div>
  </header>

  {#if loading}
    <p class="state-line font-body" aria-live="polite">Loading catalog…</p>
  {:else if !tauri}
    <div class="state-card font-body" role="status">
      <span class="material-symbols-outlined" aria-hidden="true">
        info
      </span>
      <div>
        <h2 class="font-display">Catalog available in the Tauri build</h2>
        <p>
          The AI model catalog is exposed by the Rust backend. Run the
          application via Tauri (not Vite browser dev) to inspect the
          models AstroForge ships with.
        </p>
      </div>
    </div>
  {:else if error}
    <p class="state-line error font-body" role="alert">{error}</p>
  {:else if models.length === 0}
    <p class="state-line font-body">Catalog returned no models.</p>
  {:else}
    <table class="catalog">
      <thead>
        <tr>
          <th scope="col" class="col-name">Model</th>
          <th scope="col" class="col-stage">Stage</th>
          <th scope="col" class="col-version">Version</th>
          <th scope="col" class="col-license">License</th>
          <th scope="col" class="col-size">Size</th>
          <th scope="col" class="col-shape">I/O</th>
          <th scope="col" class="col-state">Status</th>
        </tr>
      </thead>
      <tbody>
        {#each models as model (model.name)}
          <tr>
            <td class="col-name">
              <span class="model-name font-display">{model.name}</span>
            </td>
            <td class="col-stage">{stageLabel(model.stage)}</td>
            <td class="col-version">v{model.version}</td>
            <td class="col-license">{model.license}</td>
            <td class="col-size">{formatBytes(model.size_bytes)}</td>
            <td class="col-shape">
              {model.input_channels}×{model.input_tile_size}px →
              {model.output_channels}×{model.input_tile_size}px
              {#if model.scale_factor > 1}
                · {model.scale_factor}× scale
              {/if}
            </td>
            <td class="col-state">
              <span
                class="status-badge"
                data-installed={model.installed}
                aria-label={model.installed ? "Installed" : "Not installed"}
              >
                {model.installed ? "Installed" : "Not installed"}
              </span>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  .ai-screen {
    display: flex;
    flex-direction: column;
    gap: var(--sp-lg);
    padding: var(--sp-xl);
    overflow-y: auto;
    height: 100%;
  }

  .screen-header h1 {
    margin: 0 0 var(--sp-xs) 0;
    font-size: 1.5rem;
  }

  .screen-header p {
    margin: 0;
    color: var(--on-surface-variant);
    max-width: 60ch;
  }

  .state-line {
    color: var(--on-surface-variant);
  }

  .state-line.error {
    color: #e53935;
  }

  .state-card {
    display: flex;
    gap: var(--sp-md);
    padding: var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
    color: var(--on-surface);
  }

  .state-card h2 {
    margin: 0 0 var(--sp-xs) 0;
    font-size: 1rem;
  }

  .state-card p {
    margin: 0;
    color: var(--on-surface-variant);
    max-width: 60ch;
  }

  .state-card .material-symbols-outlined {
    color: var(--primary);
    font-size: 28px;
  }

  .catalog {
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  .catalog th,
  .catalog td {
    padding: var(--sp-sm) var(--sp-md);
    text-align: left;
    border-bottom: 1px solid var(--outline-variant);
    font-size: 0.9rem;
  }

  .catalog th {
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
    font-weight: 600;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .catalog tr:last-child td {
    border-bottom: none;
  }

  .model-name {
    font-weight: 600;
  }

  .col-name {
    min-width: 220px;
  }

  .col-version,
  .col-license,
  .col-size {
    white-space: nowrap;
  }

  .status-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: var(--radius-full);
    background: var(--surface-container-highest);
    border: 1px solid var(--outline-variant);
    color: var(--on-surface-variant);
    font-size: 0.75rem;
  }

  .status-badge[data-installed="true"] {
    background: var(--primary-container);
    color: var(--on-primary-container);
    border-color: var(--primary);
  }
</style>
