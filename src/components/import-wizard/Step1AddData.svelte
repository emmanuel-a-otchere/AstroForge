<!--
  CR-04 P9 — Step 1 of the §12 Import wizard: Add Data.

  Drop zone + directory picker. The "Scan" button calls
  `ingest_scan_directory` via the Tauri IPC, populates
  the wizard store with the scanned frames, and advances
  to Step 2.
-->
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import {
    goToStep,
    setFrames,
    setScanDir,
    setScanError,
    setScanning,
    type ScannedFrame,
  } from "../../state/import-wizard";

  let dir = "";
  let scanning = false;
  let error: string | null = null;

  async function scan(): Promise<void> {
    if (!dir) {
      error = "Enter a directory path first.";
      return;
    }
    scanning = true;
    error = null;
    setScanDir(dir);
    setScanning(true);
    try {
      const raw = await invoke<unknown[]>("ingest_scan_directory", {
        dirPath: dir,
      });
      const frames = (raw as Array<Record<string, unknown>>).map(
        (f): ScannedFrame => ({
          path: String(f.path ?? ""),
          frameType: String(f.frame_type ?? "unknown"),
          exptime: typeof f.exptime === "number" ? f.exptime : null,
          filter: typeof f.filter === "string" ? f.filter : null,
          width: typeof f.width === "number" ? f.width : null,
          height: typeof f.height === "number" ? f.height : null,
          binning: typeof f.binning === "number" ? f.binning : null,
          anomalies: Array.isArray(f.anomalies)
            ? f.anomalies.map((a) => String(a))
            : [],
        }),
      );
      setFrames(frames);
      goToStep("scanning");
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      error = msg;
      setScanError(msg);
    } finally {
      scanning = false;
      setScanning(false);
    }
  }

  function pickFile(): void {
    // The Tauri dialog plugin is the canonical picker; this
    // placeholder is replaced by the P9 host shell when it
    // lands. For now the user types the path directly.
    const input = document.createElement("input");
    input.type = "text";
    input.value = dir;
    input.placeholder = "/path/to/captures";
    const ok = confirm(
      "Enter the captures directory path in the next prompt.\n\nA native picker lands with the P9 host shell.",
    );
    if (ok) {
      const next = prompt("Captures directory:", dir);
      if (next !== null) {
        dir = next;
      }
    }
  }
</script>

<section class="add-data" aria-labelledby="step-1-heading">
  <h2 id="step-1-heading" class="font-display">Add Astro Data</h2>
  <p class="font-body">
    Drop a folder of captures, calibration frames, or stacked results and
    AstroForge handles the rest.
  </p>
  <div class="drop-zone" role="region" aria-label="Captures folder drop zone">
    <span class="material-symbols-outlined" aria-hidden="true">cloud_upload</span>
    <p class="font-body">FITS &middot; DNG &middot; TIFF &middot; PNG &middot; JPEG</p>
  </div>
  <div class="controls">
    <button class="btn-secondary" on:click={pickFile} type="button">
      Choose Folder
    </button>
    <label for="dir-input" class="sr-only">Captures directory</label>
    <input
      id="dir-input"
      type="text"
      bind:value={dir}
      placeholder="/path/to/captures"
      class="dir-input"
    />
    <button
      class="btn-primary"
      on:click={scan}
      type="button"
      disabled={scanning || !dir}
    >
      {scanning ? "Scanning…" : "Scan"}
    </button>
  </div>
  {#if error}
    <p class="error" role="alert">Scan failed: {error}</p>
  {/if}
</section>

<style>
  .add-data {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1.5rem;
  }
  h2 {
    margin: 0;
  }
  .drop-zone {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    padding: 2.5rem 1.5rem;
    border: 2px dashed var(--color-border, #444);
    border-radius: 8px;
    background: var(--color-surface-elevated, #1a1a1a);
  }
  .drop-zone .material-symbols-outlined {
    font-size: 48px;
    color: var(--color-text-secondary, #999);
  }
  .controls {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }
  .dir-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    background: var(--color-input-bg, #0f0f0f);
    color: var(--color-text-primary, #eee);
    border: 1px solid var(--color-border, #444);
    border-radius: 4px;
    font-family: monospace;
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
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .btn-secondary {
    background: var(--color-surface, #222);
    color: var(--color-text-primary, #eee);
    border-color: var(--color-border, #444);
  }
  .error {
    color: var(--color-error, #ff6b6b);
    margin: 0;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
