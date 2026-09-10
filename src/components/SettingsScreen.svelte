<!--
  SettingsScreen — application-level Settings destination.

  CR-05 R1: replaces the placeholder route for the "Settings" nav
  target. The current implementation is read-only and surfaces:

    - Application identity (version + build provenance)
    - Keyboard shortcuts (mirrors `useKeyboardShortcuts` truth)
    - A clear "Preferences persistence" status row that is
      honest about what's implemented today.

  Persistence (a real user preferences store) is intentionally
  deferred — the audit marked that as a separate R-item. This
  screen is real content, not a placeholder.
-->
<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { onMount } from "svelte";

  let appVersion: string | null = $state(null);
  let versionError: string | null = $state(null);
  let tauri = $state(true);

  onMount(async () => {
    try {
      appVersion = await getVersion();
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      versionError = message;
      tauri = false;
    }
  });

  const shortcuts: { keys: string; description: string }[] = [
    { keys: "⌘N / Ctrl+N", description: "New project" },
    { keys: "⌘O / Ctrl+O", description: "Open Projects" },
    { keys: "⌘S / Ctrl+S", description: "Mark project saved" },
    {
      keys: "⌘1–⌘6 / Ctrl+1–Ctrl+6",
      description: "Switch Studio workspace (when a project is open)",
    },
    { keys: "Esc", description: "Close project or current dialog" },
  ];
</script>

<section class="settings-screen" aria-label="Settings">
  <header class="screen-header">
    <h1 class="font-display">Settings</h1>
    <p class="font-body">
      Application identity and keyboard shortcuts. Persistent user
      preferences ship in a follow-up tranche; this screen is
      intentionally read-only.
    </p>
  </header>

  <article class="card" aria-labelledby="settings-identity">
    <h2 id="settings-identity" class="font-display">Application</h2>
    <dl class="kv">
      <dt>Version</dt>
      <dd>
        {#if tauri && appVersion}
          <code class="version-chip">{appVersion}</code>
        {:else if versionError && !tauri}
          <span class="muted">Not running inside Tauri (browser dev).</span>
        {:else}
          <span class="muted">Unknown</span>
        {/if}
      </dd>
      <dt>Build</dt>
      <dd><span class="muted">Tauri 2 · Svelte 5 · Rust workspace</span></dd>
    </dl>
  </article>

  <article class="card" aria-labelledby="settings-shortcuts">
    <h2 id="settings-shortcuts" class="font-display">Keyboard shortcuts</h2>
    <table class="shortcuts">
      <thead>
        <tr>
          <th scope="col">Keys</th>
          <th scope="col">Action</th>
        </tr>
      </thead>
      <tbody>
        {#each shortcuts as s (s.keys)}
          <tr>
            <td class="keys">
              {#each s.keys.split(" / ") as part, i (part + i)}
                {#if i > 0}<span class="alt-sep">/</span>{/if}
                <kbd>{part}</kbd>
              {/each}
            </td>
            <td class="desc font-body">{s.description}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </article>

  <article class="card" aria-labelledby="settings-prefs">
    <h2 id="settings-prefs" class="font-display">User preferences</h2>
    <p class="muted font-body">
      Persistent preferences (theme, default recipe, processing defaults) are
      not yet stored. The application reads from
      <code>astroforge_core</code> domain stores for project state and from
      Rust for resource detection. Preference persistence is on the audit
      roadmap.
    </p>
  </article>
</section>

<style>
  .settings-screen {
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

  .card {
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
    padding: var(--sp-lg);
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
  }

  .card h2 {
    margin: 0;
    font-size: 1.05rem;
  }

  .kv {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--sp-xs) var(--sp-md);
    margin: 0;
  }

  .kv dt {
    font-weight: 600;
    color: var(--on-surface-variant);
  }

  .kv dd {
    margin: 0;
  }

  .muted {
    color: var(--on-surface-variant);
  }

  .version-chip {
    display: inline-block;
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    background: var(--surface-container-highest);
    border: 1px solid var(--outline-variant);
    font-family: var(--font-data, monospace);
    font-size: 0.85rem;
  }

  .shortcuts {
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
  }

  .shortcuts th,
  .shortcuts td {
    padding: var(--sp-sm);
    text-align: left;
    border-bottom: 1px solid var(--outline-variant);
  }

  .shortcuts th {
    color: var(--on-surface-variant);
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .shortcuts tr:last-child td {
    border-bottom: none;
  }

  .keys {
    white-space: nowrap;
  }

  kbd {
    display: inline-block;
    padding: 2px 8px;
    background: var(--surface-container-highest);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-sm);
    font-family: var(--font-data, monospace);
    font-size: 0.8rem;
  }

  .alt-sep {
    color: var(--on-surface-variant);
    margin: 0 var(--sp-xs);
  }
</style>
