<!--
  SecurityValidationPanel — CR-08 §20 Recipe security UI.

  Renders the §20 SecurityValidationReport with:
  - Total resource units vs ceiling (visual bar)
  - Violation list grouped by class (range /
    dependency / filesystem / executable /
    resource)
  - Per-row tooltip with the violation's full
    message
  - Apply button enable state mirrors
    `report.is_safe()`

  Backed by the `recipe_security_validate` IPC.
  Parent passes the (profileId, version) pair;
  the panel resolves the report via the IPC.

  Honest affordances:
  - The "Apply anyway" button is rendered as a
    disabled stub. The §20 apply-flow integration
    (threading the report through `recipe_apply`)
    is a follow-on slice. For this slice the
    panel surfaces the report and lets the user
    see what's wrong.
-->
<script lang="ts">
  import {
    recipeSecurityValidate,
    type SecurityValidationReportFromRust,
    type SecurityViolationFromRust,
  } from "../lib/astroforge-api";

  interface Props {
    profileId: string;
    version: number;
  }

  let { profileId, version }: Props = $props();

  let report = $state<SecurityValidationReportFromRust | null>(null);
  let loading = $state(true);
  let loadError = $state<string | null>(null);

  // Display order for violation kinds; matches
  // the §20 audit doc's bullet list so the
  // panel reads top-to-bottom in the same order
  // the user sees them in the spec.
  const KIND_ORDER: ReadonlyArray<{
    kind: string;
    label: string;
    icon: string;
  }> = [
    { kind: "range", label: "Parameter range", icon: "tune" },
    { kind: "dependency", label: "Dependency", icon: "account_tree" },
    { kind: "filesystem", label: "Filesystem reference", icon: "folder_off" },
    { kind: "executable", label: "Executable payload", icon: "block" },
    { kind: "resource", label: "Resource budget", icon: "memory" },
  ];

  $effect(() => {
    let cancelled = false;
    (async () => {
      loading = true;
      loadError = null;
      try {
        const r = await recipeSecurityValidate(profileId, version);
        if (cancelled) return;
        report = r;
      } catch (e) {
        if (cancelled) return;
        loadError = e instanceof Error ? e.message : String(e);
      } finally {
        if (!cancelled) loading = false;
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  function violationsOfKind(
    list: SecurityViolationFromRust[],
    kind: string,
  ): SecurityViolationFromRust[] {
    return list.filter((v) => v.kind === kind);
  }

  function isSafe(r: SecurityValidationReportFromRust | null): boolean {
    if (!r) return false;
    return r.violations.length === 0;
  }

  // Cap for the visual resource bar.
  const RESOURCE_CEILING = 300;
</script>

<section
  class="security-panel"
  aria-label="Recipe security validation"
  data-testid="security-validation-panel"
>
  <header class="panel-header">
    <span class="material-symbols-outlined" aria-hidden="true">verified_user</span>
    <h3 class="font-display">Security validation</h3>
    {#if loading}
      <span class="status-pill status-loading font-label">Loading…</span>
    {:else if loadError}
      <span
        class="status-pill status-error font-label"
        data-testid="security-error"
        title={loadError}
      >
        Error
      </span>
    {:else if report && isSafe(report)}
      <span
        class="status-pill status-safe font-label"
        data-testid="security-safe-pill"
      >
        Safe
      </span>
    {:else if report}
      <span
        class="status-pill status-unsafe font-label"
        data-testid="security-unsafe-pill"
      >
        {report?.violations.length ?? 0} violation{(report?.violations.length ?? 0) === 1 ? "" : "s"}
      </span>
    {/if}
  </header>

  {#if report}
    <!-- Resource budget visual bar -->
    <div class="resource-row">
      <span class="resource-label font-label">Resource budget</span>
      <div class="resource-bar" role="progressbar" aria-valuemin="0" aria-valuemax={RESOURCE_CEILING} aria-valuenow={report.total_resource_units}>
        <div
          class="resource-bar-fill"
          class:over={report.total_resource_units > RESOURCE_CEILING}
          style:width="{Math.min(100, (report.total_resource_units / RESOURCE_CEILING) * 100)}%"
        ></div>
      </div>
      <span class="resource-value font-body" data-testid="resource-value">
        {report.total_resource_units} / {RESOURCE_CEILING}
      </span>
    </div>

    <!-- Violation lists grouped by class -->
    {#if isSafe(report)}
      <p class="safe-message font-body">
        All §20 checks pass. The Recipe is safe to apply.
      </p>
    {:else}
      <ul class="violation-list" role="list">
        {#each KIND_ORDER as { kind, label, icon } (kind)}
          {@const items = violationsOfKind(report.violations, kind)}
          {#if items.length > 0}
            <li class="violation-group" data-testid="violation-group-{kind}">
              <header class="group-header">
                <span class="material-symbols-outlined" aria-hidden="true">{icon}</span>
                <span class="font-label group-label">{label}</span>
                <span class="group-count font-body">{items.length}</span>
              </header>
              <ul class="violation-items" role="list">
                {#each items as v, i (kind + "-" + i)}
                  <li
                    class="violation-item"
                    data-testid="violation-item"
                    title={v.message}
                  >
                    <span class="violation-locus font-body">
                      {v.stage_id ?? "?"}
                      {#if v.param_key}/ {v.param_key}{/if}
                    </span>
                    <span class="violation-message font-body">{v.message}</span>
                  </li>
                {/each}
              </ul>
            </li>
          {/if}
        {/each}
      </ul>
    {/if}

    <!--
      Honest affordance: Apply button is disabled
      when the Recipe has any violation. The
      §20 apply-flow integration that threads
      this report through `recipe_apply` lands
      in a follow-on slice. Until then the
      panel is read-only diagnostic surface.
    -->
    <footer class="panel-footer">
      <button
        type="button"
        class="apply-cta font-label"
        disabled={!isSafe(report)}
        data-testid="security-apply-btn"
        title={isSafe(report)
          ? "Apply round-trip wires to recipe_apply IPC in a follow-on slice"
          : "Apply is blocked until all violations are resolved"}
      >
        <span class="material-symbols-outlined" aria-hidden="true">play_arrow</span>
        Apply (preview)
      </button>
    </footer>
  {/if}
</section>

<style>
  .security-panel {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
    padding: var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
  }

  .panel-header {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    flex-wrap: wrap;
  }

  .panel-header h3 {
    margin: 0;
    font-size: 1rem;
  }

  .status-pill {
    padding: 4px 10px;
    border-radius: 999px;
    font-size: 0.8rem;
    margin-left: auto;
  }

  .status-safe {
    background: var(--tertiary-container);
    color: var(--on-tertiary-container);
  }

  .status-unsafe {
    background: var(--error-container);
    color: var(--on-error-container);
  }

  .status-loading,
  .status-error {
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
  }

  .resource-row {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    flex-wrap: wrap;
  }

  .resource-label {
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .resource-bar {
    flex: 1 1 200px;
    height: 8px;
    background: var(--surface-container-high);
    border-radius: 4px;
    overflow: hidden;
    border: 1px solid var(--outline-variant);
  }

  .resource-bar-fill {
    height: 100%;
    background: var(--primary);
    transition: width 0.2s ease;
  }

  .resource-bar-fill.over {
    background: var(--error);
  }

  .resource-value {
    font-variant-numeric: tabular-nums;
    font-size: 0.9rem;
    color: var(--on-surface-variant);
  }

  .safe-message {
    margin: 0;
    padding: var(--sp-sm) var(--md);
    background: var(--tertiary-container);
    color: var(--on-tertiary-container);
    border-radius: var(--radius-md);
  }

  .violation-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
  }

  .violation-group {
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
    padding: var(--sp-xs) var(--sp-sm);
    background: var(--surface-container-high);
    font-size: 0.85rem;
  }

  .group-label {
    flex: 1;
  }

  .group-count {
    color: var(--on-surface-variant);
    font-variant-numeric: tabular-nums;
  }

  .violation-items {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .violation-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--sp-sm);
    border-top: 1px solid var(--outline-variant);
    cursor: help;
  }

  .violation-item:first-child {
    border-top: 0;
  }

  .violation-locus {
    font-size: 0.8rem;
    color: var(--on-surface-variant);
    font-family: var(--font-mono, monospace);
  }

  .violation-message {
    font-size: 0.85rem;
    color: var(--on-surface);
  }

  .panel-footer {
    display: flex;
    justify-content: flex-end;
  }

  .apply-cta {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    background: var(--primary);
    color: var(--on-primary);
    border: none;
    border-radius: var(--radius-md);
    padding: var(--sp-sm) var(--sp-md);
    cursor: pointer;
    font-size: 0.9rem;
  }

  .apply-cta:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>