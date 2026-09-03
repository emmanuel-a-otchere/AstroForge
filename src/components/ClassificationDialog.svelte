<script lang="ts">
  export let frames: Array<{
    path: string;
    frame_type: string;
    exptime: number | null;
    filter: string | null;
    anomalies: string[];
  }>;
  export let onConfirm: () => void;
  export let onReclassify: (index: number, newType: string) => void;

  let sortKey: "path" | "frame_type" | "exptime" | "filter" = "frame_type";
  let sortAsc = true;

  $: sortedFrames = [...frames].sort((a, b) => {
    let cmp = 0;
    if (sortKey === "path") {
      cmp = a.path.localeCompare(b.path);
    } else if (sortKey === "frame_type") {
      cmp = a.frame_type.localeCompare(b.frame_type);
    } else if (sortKey === "exptime") {
      cmp = (a.exptime ?? -1) - (b.exptime ?? -1);
    } else if (sortKey === "filter") {
      cmp = (a.filter ?? "").localeCompare(b.filter ?? "");
    }
    return sortAsc ? cmp : -cmp;
  });

  function toggleSort(key: typeof sortKey) {
    if (sortKey === key) {
      sortAsc = !sortAsc;
    } else {
      sortKey = key;
      sortAsc = true;
    }
  }

  const frameTypes = ["LIGHT", "DARK", "FLAT", "BIAS"];
</script>

<div class="dialog-overlay">
  <div class="dialog">
    <h2>Classification Confirmation</h2>
    <p class="subtitle">
      {frames.length} files detected. Review and adjust classifications before processing.
    </p>

    <div class="table-container">
      <table>
        <thead>
          <tr>
            <th on:click={() => toggleSort("path")} class:active={sortKey === "path"}>
              File {sortKey === "path" ? (sortAsc ? "↑" : "↓") : ""}
            </th>
            <th on:click={() => toggleSort("frame_type")} class:active={sortKey === "frame_type"}>
              Type {sortKey === "frame_type" ? (sortAsc ? "↑" : "↓") : ""}
            </th>
            <th on:click={() => toggleSort("exptime")} class:active={sortKey === "exptime"}>
              Exp (s) {sortKey === "exptime" ? (sortAsc ? "↑" : "↓") : ""}
            </th>
            <th on:click={() => toggleSort("filter")} class:active={sortKey === "filter"}>
              Filter {sortKey === "filter" ? (sortAsc ? "↑" : "↓") : ""}
            </th>
            <th>Notes</th>
          </tr>
        </thead>
        <tbody>
          {#each sortedFrames as frame, i}
            <tr class:has-anomaly={frame.anomalies.length > 0}>
              <td class="path" title={frame.path}>
                {frame.path.split("/").pop() || frame.path}
              </td>
              <td>
                <select
                  value={frame.frame_type}
                  on:change={(e) => {
                    const originalIndex = frames.indexOf(frame);
                    if (originalIndex >= 0) {
                      onReclassify(originalIndex, e.currentTarget.value);
                    }
                  }}
                >
                  {#each frameTypes as t}
                    <option value={t} selected={frame.frame_type === t}>{t}</option>
                  {/each}
                </select>
              </td>
              <td>{frame.exptime ?? "—"}</td>
              <td>{frame.filter ?? "—"}</td>
              <td class="notes">
                {#if frame.anomalies.length > 0}
                  <span class="anomaly" title={frame.anomalies.join(", ")}>⚠</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <div class="actions">
      <button class="btn-primary" on:click={onConfirm}>Confirm &amp; Process</button>
    </div>
  </div>
</div>

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-scrim);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .dialog {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 0.75rem;
    padding: 1.5rem;
    width: 720px;
    max-width: 95vw;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
  }

  h2 {
    font-size: 1.25rem;
    font-weight: 600;
    margin-bottom: 0.25rem;
  }

  .subtitle {
    font-size: 0.8125rem;
    color: var(--text-secondary);
    margin-bottom: 1rem;
  }

  .table-container {
    flex: 1;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: 0.375rem;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8125rem;
  }

  thead {
    position: sticky;
    top: 0;
    background: var(--bg-tertiary);
  }

  th {
    padding: 0.5rem 0.75rem;
    text-align: left;
    color: var(--text-secondary);
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
    user-select: none;
  }

  th.active {
    color: var(--accent);
  }

  td {
    padding: 0.375rem 0.75rem;
    border-top: 1px solid var(--border);
    color: var(--text-primary);
  }

  td.path {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: monospace;
    font-size: 0.75rem;
  }

  select {
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 0.25rem;
    padding: 0.125rem 0.25rem;
    font-size: 0.75rem;
  }

  tr.has-anomaly {
    background: var(--warning-bg);
  }

  .anomaly {
    color: var(--warning);
    cursor: help;
  }

  .notes {
    text-align: center;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 1rem;
  }

  .btn-primary {
    padding: 0.5rem 1.25rem;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: 0.375rem;
    font-weight: 500;
    cursor: pointer;
  }

  .btn-primary:hover {
    background: var(--accent-dim);
  }
</style>
