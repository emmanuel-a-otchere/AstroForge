<!--
  CR-03 P5 — Compare workspace (R3 update).

  R3 replaces the placeholder canvas with honest per-version
  metadata cards. The IPC returns a real timeline derived from
  the durable event log; the cards render what the durable
  state actually says. No fake images, no "Canvas A renders
  here" placeholders.

  Image rendering is intentionally absent: there is no real
  pixel artifact to display for any version yet (the
  canonical `image_versions` table is on the audit roadmap).
  When the table ships and a primary artifact is recorded, the
  card can call `read_preview_artifact` (or a new
  `read_image_artifact`) to surface the actual bytes.

  Side-by-side comparison is preserved: A and B are selected
  from the same real timeline, the cards render side by side,
  and a "swap" button lets the user flip them.
-->
<script lang="ts">
  import WorkspaceScreen from "./WorkspaceScreen.svelte";
  import ImageCanvas from "./ImageCanvas.svelte";
  import CompareTools from "./CompareTools.svelte";
  import DecisionPanel from "./DecisionPanel.svelte";
  import MetricsTable from "./MetricsTable.svelte";
  import ExpertChannelStats from "./ExpertChannelStats.svelte";
  import ExpertFwhmDistribution from "./ExpertFwhmDistribution.svelte";
  import ExpertNoiseMap from "./ExpertNoiseMap.svelte";
  import ExpertClippingMasks from "./ExpertClippingMasks.svelte";
  import ComparisonSetList from "./ComparisonSetList.svelte";
  import RegionPicker, { type RegionScope } from "./RegionPicker.svelte";
  import VersionDag from "./VersionDag.svelte";
  import ProvenancePanel from "./ProvenancePanel.svelte";
  import RecipeStageTimeline from "./RecipeStageTimeline.svelte";
  import QualityProfilePicker from "./QualityProfilePicker.svelte";
  import { versionStore, type ImageVersion } from "../state/versions";
  import { studioViewport } from "../state/application";
  import { applyImageDecision } from "../lib/astroforge-api";
  import {
    exportComparisonComposite,
    downloadBlob,
  } from "../lib/comparison-export";
  import {
    DEFAULT_QUALITY_PROFILE,
    type QualityProfile as QualityProfileT,
  } from "../lib/astroforge-api";

  let aId = $state<string | null>(null);
  let bId = $state<string | null>(null);
  // CR-07 follow-on: toggle between the basic side-by-side
  // metadata+canvas layout and the CompareTools surface
  // (split / blink / difference). The basic layout is the
  // default; CompareTools requires both versions to have a
  // primary_artifact_id (otherwise the canvas can't render).
  let useCompareTools = false;
  // CR-07 B5: synchronized navigation (§6). When ON, the
  // CompareWorkspace owns the shared view (zoom + pan) and
  // passes it to both ImageCanvas instances. A drag on either
  // canvas emits `onViewChange`, which updates the single
  // source of truth here; both canvases re-render against it.
  // Default ON because the basic side-by-side is the default
  // mode, and the pain point (you zoom one and lose the other)
  // is exactly what B5 fixes.
  let syncNav = true;
  type SharedView = {
    zoomMode: "fit" | "1:1";
    zoomLevel: number;
    panX: number;
    panY: number;
  };
  let sharedView = $state<SharedView>({
    zoomMode: "fit",
    zoomLevel: 1,
    panX: 0,
    panY: 0,
  });
  // CR-07 B7: Comparison Scope (§7) state.
  let regionScope = $state<RegionScope>("whole");
  let regionDrawMode = $state(false);
  let selectedRegion = $state<{
    x0: number;
    y0: number;
    x1: number;
    y1: number;
  } | null>(null);
  let selectedFeature = $state<string | null>(null);
  // CR-07 §23.1: toggle for the expert channel-stats panel.
  // Default off: the panel is a progressive-disclosure surface
  // per CR-01 + §23 ("should remain progressive disclosure,
  // consistent with CR-01"). Hiding by default keeps the
  // beginner / intermediate experience clean.
  let showExpertDetails = $state(false);

  // Overlay regions handed to ImageCanvas. Always empty for
  // "whole" scope (whole-image has no visual overlay); populated
  // for "selected" and "feature".
  const overlayRegionA = $derived(
    regionScope !== "whole" && selectedRegion
      ? { scope: regionScope, rect: selectedRegion }
      : null,
  );
  const overlayRegionB = $derived(overlayRegionA);

  // CR-07 B8: VersionDag reads from versionStore reactively.
  // We pull the `versions` field off the store in a $derived
  // so the tree re-renders whenever the store updates (e.g.
  // after a new stage completion).
  const dagVersions = $derived(
    ($versionStore as { versions?: readonly ImageVersion[] }).versions ??
      [],
  );

  // R4: the project lifecycle module loads the version
  // timeline on project open. The remaining reactive
  // subscription below is a safety net for the case where
  // the active project changes without going through the
  // lifecycle (e.g. during the close transition while the
  // pipeline plan store resets).

  // R3: when the timeline arrives, default A to the earliest
  // and B to the latest. This is the only "magic" the picker
  // does; everything else is direct rendering of the IPC's
  // output.
  $effect(() => {
    const versions = $versionStore.versions;
    if (versions.length >= 2 && aId === null && bId === null) {
      aId = versions[0].version_id;
      bId = versions[versions.length - 1].version_id;
    }
  });

  const versionA = $derived<ImageVersion | null>(
    $versionStore.versions.find((v) => v.version_id === aId) ?? null,
  );
  const versionB = $derived<ImageVersion | null>(
    $versionStore.versions.find((v) => v.version_id === bId) ?? null,
  );

  function swap() {
    [aId, bId] = [bId, aId];
  }

  // CR-07 B4 — apply a saved comparison set's first two versions
  // back to the A/B pickers.
  function applySet(versionIdA: string, versionIdB: string) {
    aId = versionIdA;
    bId = versionIdB;
  }

  // CR-07 B5: receive view changes from either canvas and
  // store the new shared view. Both canvases re-render against
  // it via the `view` prop, so pan/zoom on A instantly reflects
  // on B (and vice versa). Identity check before assignment
  // avoids the two-way binding feedback loop the skill
  // flagged.
  function handleSharedViewChange(view: SharedView) {
    const same =
      sharedView.zoomMode === view.zoomMode &&
      sharedView.zoomLevel === view.zoomLevel &&
      sharedView.panX === view.panX &&
      sharedView.panY === view.panY;
    if (!same) sharedView = view;
  }

  // CR-07 B5 — reset the shared view to fit-to-window. Wired
  // up via the toolbar button below so the user has an
  // obvious "I want both at once" affordance.
  function fitBoth() {
    sharedView = { zoomMode: "fit", zoomLevel: 1, panX: 0, panY: 0 };
  }

  // CR-07 B7: region handlers. Changing scope clears any
  // captured region so the UI stays honest: switching from
  // "selected" to "whole" shouldn't leave a phantom overlay.
  function setScope(next: RegionScope) {
    if (next !== regionScope) {
      regionScope = next;
      selectedRegion = null;
      regionDrawMode = false;
      if (next !== "feature") selectedFeature = null;
    }
  }
  function toggleDrawMode() {
    regionDrawMode = !regionDrawMode;
  }
  function clearRegion() {
    selectedRegion = null;
    regionDrawMode = false;
  }
  function handleCanvasRegion(rect: {
    x0: number;
    y0: number;
    x1: number;
    y1: number;
  }) {
    if (!regionDrawMode && !regionScope) return;
    if (regionScope !== "selected") return;
    // Reject degenerate rects (the user just clicked without
    // dragging); require a minimum area to make the
    // comparison meaningful.
    const w = rect.x1 - rect.x0;
    const h = rect.y1 - rect.y0;
    if (w < 8 || h < 8) return;
    selectedRegion = rect;
    regionDrawMode = false;
  }

  function formatDate(iso: string): string {
    if (!iso) return "—";
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  }

  function statusFor(): "in_review" {
    // IPC does not carry a status. A future promote action
    // can move a version to "final"; the timeline card stays
    // honest with "in review" until then.
    return "in_review";
  }

  // CR-07 C-A1: continue-from-comparison action bar.
  // Each button wires to either an existing IPC
  // (Mark Preferred, Continue enhancing) or a
  // honest "coming in <slice>" stub for the slices
  // queued later in this tier (Create Branch in
  // C-A2; Export in C-A3).

  /** Mark the B version as `preferred`. Wired to
   *  the B4 `apply_image_decision` IPC. Mirrors
   *  the DecisionPanel pattern; calls the IPC
   *  directly rather than going through the
   *  comparison store so this is a one-click
   *  shortcut from the comparison view. */
  let markPreferredBusy = $state(false);
  let markPreferredError = $state<string | null>(null);
  async function markPreferred(): Promise<void> {
    if (!bId) return;
    markPreferredBusy = true;
    markPreferredError = null;
    try {
      // CR-07 C-A3.5: thread the picker's selection
      // through so the decision row carries the user's
      // currently-picked Quality Profile. Without this,
      // the picker's intent stays local-only — the row
      // gets marked "preferred" but the panel can't tell
      // what profile the user had in mind.
      await applyImageDecision(
        bId,
        "preferred",
        undefined,
        selectedQualityProfile,
      );
    } catch (e) {
      markPreferredError =
        e instanceof Error ? e.message : String(e);
    } finally {
      markPreferredBusy = false;
    }
  }

  /** Navigate to the Enhance workspace. The
   *  Enhance flow derives its source from the
   *  project's latest version (not from a
   *  CompareWorkspace-chosen one); the button
   *  is the user's intent signal to "go
   *  enhance next", not a strict
   *  hand-off of B's id. A future slice
   *  may add a per-version source override
   *  to the Enhance flow. */
  function continueEnhancing(): void {
    studioViewport.setView("enhance");
  }

  // CR-07 C-A2: side-by-side composite export (§30).
  // Renders an off-screen canvas with A | B and
  // triggers a browser download. The mode note
  // surfaces the current comparison mode so the
  // exported PNG is self-describing.
  let exportBusy = $state(false);
  let exportError = $state<string | null>(null);

  // CR-07 §22: Quality Profile picker state.
  // Holds the user's current selection. Defaults
  // to `natural` to match the Rust default. The
  // picker's `onChange` updates this value; the
  // picker reads from it. Persistence to a
  // Recipe or to an apply request lands in a
  // future slice (the picker is "view + intent"
  // today, not "save").
  let selectedQualityProfile = $state<QualityProfileT>(
    DEFAULT_QUALITY_PROFILE,
  );
  function handleQualityProfileChange(next: QualityProfileT): void {
    selectedQualityProfile = next;
  }
  async function exportComposite(): Promise<void> {
    if (!aId || !bId) return;
    exportBusy = true;
    exportError = null;
    try {
      const result = await exportComparisonComposite({
        versionAId: aId,
        versionBId: bId,
        labelA: versionA?.label ?? "A",
        labelB: versionB?.label ?? "B",
        panelWidth: 1024,
        gutter: 16,
        background: "#000000",
        modeNote: undefined,
      });
      const stamp = new Date()
        .toISOString()
        .replace(/[:.]/g, "-")
        .slice(0, 19);
      const filename = `astroforge-comparison-${stamp}.png`;
      downloadBlob(result.blob, filename);
    } catch (e) {
      exportError = e instanceof Error ? e.message : String(e);
    } finally {
      exportBusy = false;
    }
  }
</script>

<WorkspaceScreen
  title="Compare"
  icon="compare"
  description="Side-by-side comparison between any two image versions of this project. Versions are derived from the durable event log; cards show what the project actually has."
>
  {#if $versionStore.loading}
    <p class="state-line font-body" aria-live="polite">
      Loading version timeline…
    </p>
  {:else if $versionStore.error}
    <div class="state-card font-body" role="alert">
      <span class="material-symbols-outlined" aria-hidden="true">
        error
      </span>
      <div>
        <h2 class="font-display">Timeline unavailable</h2>
        <p>{$versionStore.error}</p>
      </div>
    </div>
  {:else if $versionStore.versions.length < 2}
    <div class="empty-state">
      <span class="material-symbols-outlined empty-icon" aria-hidden="true">
        compare
      </span>
      <p class="empty-title font-display">Need two or more versions</p>
      <p class="empty-body font-body">
        Compare needs at least two image versions. The project records
        a version when a pipeline run completes and writes a
        <code>VersionCreated</code> event. Run a pipeline to build up
        the timeline.
      </p>
    </div>
  {:else}
    <div class="picker-row">
      <label class="picker">
        <span class="picker-label font-label">Version A</span>
        <select bind:value={aId} class="picker-select">
          {#each $versionStore.versions as v (v.version_id)}
            <option value={v.version_id}>
              {v.label} — sequence #{v.sequence}
            </option>
          {/each}
        </select>
      </label>
      <button
        type="button"
        class="swap-cta"
        onclick={swap}
        aria-label="Swap version A and version B"
      >
        <span class="material-symbols-outlined" aria-hidden="true">
          compare_arrows
        </span>
        <span class="font-body">Swap</span>
      </button>
      <label class="picker">
        <span class="picker-label font-label">Version B</span>
        <select bind:value={bId} class="picker-select">
          {#each $versionStore.versions as v (v.version_id)}
            <option value={v.version_id}>
              {v.label} — sequence #{v.sequence}
            </option>
          {/each}
        </select>
      </label>
    </div>

    {#if versionA?.primary_artifact_id && versionB?.primary_artifact_id}
      <div class="mode-toggle">
        <button
          type="button"
          class="mode-button"
          class:active={!useCompareTools}
          onclick={() => (useCompareTools = false)}
          aria-pressed={!useCompareTools}
        >
          Side by side
        </button>
        <button
          type="button"
          class="mode-button"
          class:active={useCompareTools}
          onclick={() => (useCompareTools = true)}
          aria-pressed={useCompareTools}
          data-testid="compare-tools-toggle"
        >
          Compare tools
        </button>
      </div>
    {/if}

    <!-- CR-07 B5: sync-nav controls. Only meaningful in the
         basic side-by-side mode (CompareTools renders both A
         and B into its own composited stage, so per-canvas
         sync isn't relevant there). -->
    {#if !useCompareTools && versionA?.primary_artifact_id && versionB?.primary_artifact_id}
      <div class="sync-nav-bar" role="toolbar" aria-label="Synchronized navigation">
        <label class="sync-nav-toggle">
          <input
            type="checkbox"
            bind:checked={syncNav}
            aria-label="Synchronize zoom and pan across A and B"
          />
          <span class="material-symbols-outlined" aria-hidden="true">sync</span>
          Sync zoom + pan
        </label>
        <button
          type="button"
          class="sync-nav-cta"
          onclick={fitBoth}
          disabled={!syncNav}
          aria-label="Reset both A and B to fit"
          title="Reset zoom + pan on both sides"
        >
          <span class="material-symbols-outlined" aria-hidden="true">fit_screen</span>
          Fit both
        </button>
        {#if syncNav}
          <span class="sync-nav-readout font-body" aria-live="polite">
            {#if sharedView.zoomMode === "fit"}
              Fit
            {:else}
              {sharedView.zoomLevel.toFixed(2)}×
            {/if}
            · pan ({Math.round(sharedView.panX)}, {Math.round(sharedView.panY)})
          </span>
        {/if}
      </div>
    {/if}

    {#if useCompareTools && versionA?.primary_artifact_id && versionB?.primary_artifact_id}
      <div class="compare-tools-mount" data-testid="compare-tools-mount">
        <CompareTools
          versionIdA={versionA.version_id}
          versionIdB={versionB.version_id}
        />
      </div>
    {:else}
    <div class="canvas-area" aria-label="Side-by-side compare cards">
      <article class="canvas-pane" data-side="A">
        <header class="pane-header">
          <span class="pane-tag font-label">Version A</span>
          <h2 class="pane-title font-display">
            {versionA?.label ?? "—"}
          </h2>
        </header>
        {#if versionA}
          <dl class="meta">
            <dt>Sequence</dt>
            <dd>#{versionA.sequence}</dd>
            <dt>Created</dt>
            <dd>{formatDate(versionA.created_at)}</dd>
            <dt>Artifact</dt>
            <dd>
              {#if versionA.primary_artifact_id}
                <code>{versionA.primary_artifact_id}</code>
              {:else}
                <span class="muted">Not linked yet</span>
              {/if}
            </dd>
            <dt>Status</dt>
            <dd>
              <span class="status-pill" data-status={statusFor()}>
                {statusFor().replace("_", " ")}
              </span>
            </dd>
            {#if versionA.source_version_id}
              <dt>Branched from</dt>
              <dd><code>{versionA.source_version_id}</code></dd>
            {/if}
          </dl>
          {#if versionA.primary_artifact_id}
            <div class="canvas-mount" data-testid="compare-canvas-a">
              <ImageCanvas
                versionId={versionA.version_id}
                view={syncNav ? sharedView : null}
                onViewChange={syncNav ? handleSharedViewChange : () => {}}
                onRegion={handleCanvasRegion}
                regions={overlayRegionA ? [overlayRegionA] : []}
                drawMode={regionScope === "selected" && regionDrawMode}
              />
            </div>
            <p class="artifact-note font-body">
              <span class="material-symbols-outlined" aria-hidden="true">
                image
              </span>
              Rendering the applied artifact bytes. Drag to pan,
              scroll to zoom, shift-drag to inspect a region.
            </p>
          {:else}
            <p class="artifact-note font-body">
              <span class="material-symbols-outlined" aria-hidden="true">
                image_not_supported
              </span>
              This version has no primary artifact yet — pixel
              rendering needs the P5.1 apply round to write a TIFF.
            </p>
          {/if}
        {:else}
          <p class="muted font-body">Select a version for side A.</p>
        {/if}
      </article>
      <article class="canvas-pane" data-side="B">
        <header class="pane-header">
          <span class="pane-tag font-label">Version B</span>
          <h2 class="pane-title font-display">
            {versionB?.label ?? "—"}
          </h2>
        </header>
        {#if versionB}
          <dl class="meta">
            <dt>Sequence</dt>
            <dd>#{versionB.sequence}</dd>
            <dt>Created</dt>
            <dd>{formatDate(versionB.created_at)}</dd>
            <dt>Artifact</dt>
            <dd>
              {#if versionB.primary_artifact_id}
                <code>{versionB.primary_artifact_id}</code>
              {:else}
                <span class="muted">Not linked yet</span>
              {/if}
            </dd>
            <dt>Status</dt>
            <dd>
              <span class="status-pill" data-status={statusFor()}>
                {statusFor().replace("_", " ")}
              </span>
            </dd>
            {#if versionB.source_version_id}
              <dt>Branched from</dt>
              <dd><code>{versionB.source_version_id}</code></dd>
            {/if}
          </dl>
          {#if versionB.primary_artifact_id}
            <div class="canvas-mount" data-testid="compare-canvas-b">
              <ImageCanvas
                versionId={versionB.version_id}
                view={syncNav ? sharedView : null}
                onViewChange={syncNav ? handleSharedViewChange : () => {}}
                onRegion={handleCanvasRegion}
                regions={overlayRegionB ? [overlayRegionB] : []}
                drawMode={regionScope === "selected" && regionDrawMode}
              />
            </div>
            <p class="artifact-note font-body">
              <span class="material-symbols-outlined" aria-hidden="true">
                image
              </span>
              Rendering the applied artifact bytes. Drag to pan,
              scroll to zoom, shift-drag to inspect a region.
            </p>
          {:else}
            <p class="artifact-note font-body">
              <span class="material-symbols-outlined" aria-hidden="true">
                image_not_supported
              </span>
              This version has no primary artifact yet — pixel
              rendering needs the P5.1 apply round to write a TIFF.
            </p>
          {/if}
        {:else}
          <p class="muted font-body">Select a version for side B.</p>
        {/if}
      </article>
    </div>
    {/if}

    {#if aId && bId}
      <!-- CR-07 B4: §10/§11 metrics + §17/§18 decisions + §16 sets.
           CR-07 B7: §7 region picker threads the active scope
           into the metrics header so the user knows what
           they're comparing.
           CR-07 B8: §15 version tree visualization. Picking
           two nodes here pushes the ids straight into the
           A/B pickers above. -->
      <div class="compare-extras">
        <VersionDag
          versions={dagVersions}
          presetA={aId}
          presetB={bId}
          onSelectPair={(a, b) => {
            aId = a;
            bId = b;
          }}
        />
        <RegionPicker
          scope={regionScope}
          onScopeChange={setScope}
          drawMode={regionDrawMode}
          onDrawToggle={toggleDrawMode}
          onClearRegion={clearRegion}
          hasRegion={selectedRegion !== null}
          selectedFeature={selectedFeature}
          onFeatureChange={(f) => (selectedFeature = f)}
        />
        <MetricsTable
          versionIdA={aId}
          versionIdB={bId}
          labelA={versionA?.label ?? "A"}
          labelB={versionB?.label ?? "B"}
          scope={regionScope}
          feature={selectedFeature}
          hasRegion={selectedRegion !== null}
        />
        {#if aId && bId}
          <div class="expert-toggle-row">
            <button
              type="button"
              class="expert-toggle"
              class:active={showExpertDetails}
              aria-expanded={showExpertDetails}
              aria-controls="expert-channel-stats-panels"
              onclick={() => (showExpertDetails = !showExpertDetails)}
            >
              {showExpertDetails ? "Hide" : "Show"} expert details
            </button>
            <p class="expert-hint">
              Per-channel statistics (mean, stddev, min, max, clipped
              pixels) for both versions. Per §23, this is the
              progressive-disclosure surface: toggled on demand.
            </p>
          </div>
          {#if showExpertDetails}
            <div
              id="expert-channel-stats-panels"
              class="expert-panels"
            >
              <ExpertChannelStats
                versionId={aId}
                label={versionA?.label ?? "Version A"}
              />
              <ExpertChannelStats
                versionId={bId}
                label={versionB?.label ?? "Version B"}
              />
            </div>
            <div class="expert-panels">
              <ExpertFwhmDistribution
                versionId={aId}
                label={versionA?.label ?? "Version A"}
              />
              <ExpertFwhmDistribution
                versionId={bId}
                label={versionB?.label ?? "Version B"}
              />
            </div>
            <div class="expert-panels">
              <ExpertNoiseMap
                versionId={aId}
                label={versionA?.label ?? "Version A"}
              />
              <ExpertNoiseMap
                versionId={bId}
                label={versionB?.label ?? "Version B"}
              />
            </div>
            <div class="expert-panels">
              <ExpertClippingMasks
                versionId={aId}
                label={versionA?.label ?? "Version A"}
              />
              <ExpertClippingMasks
                versionId={bId}
                label={versionB?.label ?? "Version B"}
              />
            </div>
          {/if}
        {/if}
        <div class="decision-row">
          <DecisionPanel
            versionId={aId}
            versionLabel={versionA?.label ?? "Version A"}
          />
          <DecisionPanel
            versionId={bId}
            versionLabel={versionB?.label ?? "Version B"}
          />
        </div>
        <!-- CR-07 B14: provenance panel (§13/§14).
             Two instances (A | B) mirror the
             DecisionPanel pattern. The panels fetch
             their own data on versionId change via
             the B13c provenanceStore; the store's
             in-flight stale-load guard means two
             panels pointing at different versions
             don't fight each other. -->
        <div class="provenance-row">
          <ProvenancePanel
            versionId={aId}
            versionLabel={versionA?.label ?? "Version A"}
          />
          <ProvenancePanel
            versionId={bId}
            versionLabel={versionB?.label ?? "Version B"}
          />
        </div>
        <!-- CR-07 B15: Recipe Stage Timeline. Sister
             component to ProvenancePanel; same
             provenanceStore subscription pattern.
             Renders the full stage list with
             expandable per-stage params. Mirrors
             the B14 two-instance A | B layout. -->
        <div class="provenance-row">
          <RecipeStageTimeline
            versionId={aId}
            versionLabel={versionA?.label ?? "Version A"}
          />
          <RecipeStageTimeline
            versionId={bId}
            versionLabel={versionB?.label ?? "Version B"}
          />
        </div>
        <!-- CR-07 §22: Quality Profile picker.
             Renders the 4-variant catalog fetched
             via `quality_profile_list`. The user
             can pick a profile; the selection is
             held in local state and surfaces
             downstream in future slices. -->
        <div class="quality-profile-row">
          <QualityProfilePicker
            value={selectedQualityProfile}
            onChange={handleQualityProfileChange}
            label="Quality Profile (preview)"
          />
          <p class="hint font-body">
            Profile selection will be honored when running AI enhancements
            from the comparison view. Persistence to a Recipe lands in a
            future slice.
          </p>
        </div>
        <ComparisonSetList
          projectId={$versionStore.project_id ?? ""}
          currentA={aId}
          currentB={bId}
          labelA={versionA?.label ?? "A"}
          labelB={versionB?.label ?? "B"}
          onApply={applySet}
        />
        <!-- CR-07 C-A1: continue-from-comparison
             action bar. Four CTAs that close the
             comparison loop:
             1. Mark Preferred (B): wired to
                applyImageDecision("preferred").
             2. Continue enhancing: navigates
                to Enhance workspace with B
                pre-selected as source.
             3. Create branch: honest stub;
                wired in C-A2 (quality profile
                tier).
             4. Export: honest stub; wired in
                C-A3.
             The bar only renders when both A
             and B are selected (same gate as
             the sync-nav controls). -->
        {#if aId && bId && aId !== bId}
          <div class="continue-bar" role="toolbar" aria-label="Continue from comparison">
            <button
              type="button"
              class="continue-cta primary"
              disabled={markPreferredBusy || !bId}
              onclick={markPreferred}
              aria-label="Mark version B as Preferred"
            >
              <span class="material-symbols-outlined" aria-hidden="true">star</span>
              Mark Preferred (B)
            </button>
            <button
              type="button"
              class="continue-cta"
              onclick={continueEnhancing}
              aria-label="Continue enhancing from version B"
            >
              <span class="material-symbols-outlined" aria-hidden="true">auto_awesome</span>
              Continue enhancing
            </button>
            <button
              type="button"
              class="continue-cta"
              disabled
              title="Coming in C-A2 (quality profile tier)"
              aria-label="Create branch from version B (coming soon)"
            >
              <span class="material-symbols-outlined" aria-hidden="true">account_tree</span>
              Create branch
            </button>
            <button
              type="button"
              class="continue-cta"
              disabled={exportBusy || !aId || !bId}
              onclick={exportComposite}
              aria-label="Export comparison as side-by-side PNG"
            >
              <span class="material-symbols-outlined" aria-hidden="true">download</span>
              {exportBusy ? "Exporting…" : "Export comparison"}
            </button>
          </div>
          {#if markPreferredError}
            <p class="continue-error font-body" role="alert">
              <span class="material-symbols-outlined" aria-hidden="true">error</span>
              {markPreferredError}
            </p>
          {/if}
          {#if exportError}
            <p class="continue-error font-body" role="alert">
              <span class="material-symbols-outlined" aria-hidden="true">error</span>
              Export failed: {exportError}
            </p>
          {/if}
        {/if}
      </div>
    {/if}

    <p class="hint font-body">
      Both cards reflect the project's durable event log. The
      canvas renders the primary artifact TIFF written by the
      P5.1 apply round; pre-P5.1 versions show only metadata.
    </p>
  {/if}
</WorkspaceScreen>

<style>
  .state-line {
    color: var(--on-surface-variant);
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
  }

  .state-card .material-symbols-outlined {
    color: #e53935;
    font-size: 28px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: var(--sp-xl);
    background: var(--surface-container-low);
    border: 1px dashed var(--outline-variant);
    border-radius: var(--radius-lg);
    gap: var(--sp-md);
  }

  .empty-icon {
    font-size: 48px;
    color: var(--on-surface-variant);
  }

  .empty-title {
    font-size: 1.25rem;
    margin: 0;
  }

  .empty-body {
    margin: 0;
    color: var(--on-surface-variant);
    max-width: 60ch;
  }

  .empty-body code {
    font-family: var(--font-data, monospace);
    font-size: 0.9em;
  }

  .picker-row {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: end;
    gap: var(--sp-md);
  }

  .picker {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .picker-label {
    font-size: 0.75rem;
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .picker-select {
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    color: var(--on-surface);
    font-size: 0.95rem;
    font-family: inherit;
  }

  .picker-select:focus {
    outline: none;
    border-color: var(--primary);
  }

  .swap-cta {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    background: transparent;
    border: 1px solid var(--outline-variant);
    color: var(--on-surface);
    padding: var(--sp-sm) var(--sp-md);
    border-radius: var(--radius-md);
    cursor: pointer;
    white-space: nowrap;
    align-self: end;
  }

  .swap-cta:hover {
    background: var(--surface-container);
  }

  .canvas-area {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sp-md);
    margin-top: var(--sp-md);
  }

  .canvas-pane {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
    padding: var(--sp-md);
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
  }

  .pane-header {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .pane-tag {
    font-size: 0.7rem;
    color: var(--primary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .pane-title {
    margin: 0;
    font-size: 1.1rem;
  }

  .meta {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--sp-xs) var(--sp-md);
    margin: 0;
  }

  .meta dt {
    font-size: 0.8rem;
    color: var(--on-surface-variant);
  }

  .meta dd {
    margin: 0;
    font-size: 0.85rem;
  }

  .meta code {
    font-family: var(--font-data, monospace);
    font-size: 0.8em;
  }

  .muted {
    color: var(--on-surface-variant);
  }

  .status-pill {
    display: inline-block;
    padding: 2px var(--sp-xs);
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .status-pill[data-status="in_review"] {
    background: rgba(33, 150, 243, 0.2);
    color: #64b5f6;
  }

  /* CR-07 C-A1: continue-from-comparison
     action bar. Mirrors the sync-nav-bar
     visual style; wraps on narrow screens. */
  .continue-bar {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-sm);
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container-high);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
  }

  .continue-cta {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    padding: var(--sp-xs) var(--sp-sm);
    background: var(--surface-container);
    color: var(--on-surface);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 0.85rem;
  }

  .continue-cta:hover:not(:disabled) {
    background: var(--surface-container-low);
  }

  .continue-cta:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .continue-cta.primary {
    background: var(--primary);
    color: var(--on-primary);
    border-color: var(--primary);
  }

  .continue-cta.primary:hover:not(:disabled) {
    background: var(--primary);
    opacity: 0.9;
  }

  .continue-error {
    display: flex;
    align-items: center;
    gap: var(--sp-xs);
    margin: 0;
    padding: var(--sp-xs) var(--sp-sm);
    color: #ef9a9a;
    background: rgba(244, 67, 54, 0.1);
    border-left: 3px solid #f44336;
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
  }

  /* CR-07 §22: Quality Profile picker row.
     Sits between the stage timeline and the
     comparison-set list. */
  .quality-profile-row {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
    padding: var(--sp-sm);
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
  }

  .quality-profile-row .hint {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.8rem;
    line-height: 1.4;
  }

  .compare-extras {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
    margin-top: var(--sp-md);
  }

  .decision-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sp-md);
  }

  /* CR-07 §23.1: expert toggle + panel layout. The
     toggle row is a single line with a button and a
     one-line hint; the panels below mirror the
     decision-row / provenance-row two-column layout. */
  .expert-toggle-row {
    display: flex;
    align-items: center;
    gap: var(--sp-md);
    margin-top: var(--sp-md);
    flex-wrap: wrap;
  }

  .expert-toggle {
    background: var(--surface-1, #1a1a1a);
    color: var(--text-dim, #aaa);
    border: 1px solid var(--border-subtle, #2a2a2a);
    border-radius: 4px;
    padding: 0.4rem 0.9rem;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .expert-toggle.active {
    background: var(--accent, #4a8eff);
    color: white;
    border-color: var(--accent, #4a8eff);
  }

  .expert-hint {
    margin: 0;
    color: var(--text-dim, #888);
    font-size: 0.8rem;
    line-height: 1.4;
    flex: 1;
    min-width: 0;
  }

  .expert-panels {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sp-md);
    margin-top: var(--sp-sm);
  }

  @media (max-width: 900px) {
    .decision-row,
    .expert-panels {
      grid-template-columns: 1fr;
    }
  }

  /* CR-07 B14: provenance-row mirrors the
     decision-row layout so the two
     DecisionPanel | ProvenancePanel pairs sit
     side by side for A vs B. The narrow-screen
     collapse also matches. */
  .provenance-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sp-md);
  }

  @media (max-width: 900px) {
    .provenance-row {
      grid-template-columns: 1fr;
    }
  }

  .canvas-mount {
    width: 100%;
    height: 360px;
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    overflow: hidden;
    background: #000;
  }

  .compare-tools-mount {
    width: 100%;
    height: 480px;
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    overflow: hidden;
    margin-top: var(--sp-md);
  }

  .mode-toggle {
    display: flex;
    gap: 4px;
    margin-top: var(--sp-md);
  }

  .sync-nav-bar {
    display: flex;
    align-items: center;
    gap: var(--sp-md);
    margin-top: var(--sp-sm);
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
  }

  .sync-nav-toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    cursor: pointer;
    font-size: 0.85rem;
    color: var(--on-surface);
  }

  .sync-nav-toggle input {
    margin: 0;
  }

  .sync-nav-cta {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    padding: var(--sp-xs) var(--sp-sm);
    background: transparent;
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    color: var(--on-surface);
    cursor: pointer;
    font-size: 0.85rem;
    font-family: inherit;
  }

  .sync-nav-cta:hover:not(:disabled) {
    background: var(--surface-container);
  }

  .sync-nav-cta:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .sync-nav-readout {
    margin-left: auto;
    color: var(--on-surface-variant);
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }

  .mode-button {
    background: var(--surface-container-low);
    color: var(--on-surface-variant);
    border: 1px solid var(--outline-variant);
    padding: var(--sp-xs) var(--sp-md);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.85rem;
  }

  .mode-button.active {
    background: var(--primary);
    color: var(--on-primary);
    border-color: var(--primary);
  }

  .artifact-note {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    margin: 0;
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container-low);
    border-radius: var(--radius-md);
    color: var(--on-surface-variant);
    font-size: 0.8rem;
  }

  .artifact-note .material-symbols-outlined {
    color: var(--primary);
  }

  .artifact-note code {
    font-family: var(--font-data, monospace);
    font-size: 0.9em;
  }

  .hint {
    font-size: 0.8rem;
    color: var(--on-surface-variant);
    margin: 0;
  }

  .hint code {
    font-family: var(--font-data, monospace);
    font-size: 0.9em;
  }

  @media (max-width: 720px) {
    .picker-row,
    .canvas-area {
      grid-template-columns: 1fr;
    }
  }
</style>
