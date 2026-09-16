<!--
  CR-07 §22: Quality Profile picker.

  Dropdown that surfaces the 4 quality profiles
  (Natural / Detail / Clean / Publication)
  fetched from the new `quality_profile_list` IPC.

  Props:
    value: current selection (QualityProfile id)
    onChange: callback fired when the user picks
      a different profile
    disabled: optional disable flag
    label: optional override of the visible label
      (default "Quality Profile")

  The component owns its own loaded-catalog
  state (via $effect) so the parent doesn't
  have to fetch the IPC. If `value` is not in
  the catalog (legacy / unknown), the picker
  renders the catalog default (natural) and
  fires `onChange("natural")` once the catalog
  loads so the parent re-syncs.
-->
<script lang="ts">
  import {
    qualityProfileList,
    type QualityProfile,
    type QualityProfileInfoJson,
    DEFAULT_QUALITY_PROFILE,
    isQualityProfile,
  } from "../lib/astroforge-api";

  interface Props {
    value: QualityProfile;
    onChange: (next: QualityProfile) => void;
    disabled?: boolean;
    label?: string;
  }

  let {
    value,
    onChange,
    disabled = false,
    label = "Quality Profile",
  }: Props = $props();

  let profiles = $state<QualityProfileInfoJson[]>([]);
  let loaded = $state(false);
  let loadError = $state<string | null>(null);

  $effect(() => {
    let cancelled = false;
    (async () => {
      try {
        const list = await qualityProfileList();
        if (cancelled) return;
        profiles = list;
        loaded = true;
      } catch (e) {
        if (cancelled) return;
        loadError = e instanceof Error ? e.message : String(e);
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  // If the parent's `value` is unknown (legacy
  // data, or first render with a placeholder),
  // sync to the default once the catalog is
  // loaded. This is the only place we mutate
  // upward state.
  $effect(() => {
    if (!loaded) return;
    if (!isQualityProfile(value)) {
      onChange(DEFAULT_QUALITY_PROFILE);
    }
  });

  function handleSelect(event: Event) {
    const target = event.currentTarget as HTMLSelectElement;
    const next = target.value;
    if (isQualityProfile(next)) {
      onChange(next);
    }
  }

  // Find the matching profile for the description.
  // Falls back to the catalog's first entry if
  // the value is unknown.
  let description = $derived.by(() => {
    const found = profiles.find((p) => p.id === value);
    if (found) return found.description;
    if (profiles.length > 0) return profiles[0].description;
    return "";
  });
</script>

<div class="profile-picker" role="group" aria-label={label}>
  <label class="picker-label font-label" for="quality-profile-select">
    {label}
  </label>
  <select
    id="quality-profile-select"
    class="picker-select font-body"
    {value}
    onchange={handleSelect}
    {disabled}
    aria-describedby="quality-profile-description"
  >
    {#if !loaded && !loadError}
      <option value={value}>Loading…</option>
    {:else if loadError}
      <option value={value}>Unavailable</option>
    {:else}
      {#each profiles as p (p.id)}
        <option value={p.id}>{p.label}</option>
      {/each}
    {/if}
  </select>
  <p
    id="quality-profile-description"
    class="picker-description font-body"
    aria-live="polite"
  >
    {description}
  </p>
</div>

<style>
  .profile-picker {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
    padding: var(--sp-sm);
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
  }

  .picker-label {
    color: var(--on-surface);
    font-size: 0.85rem;
    font-weight: 600;
  }

  .picker-select {
    padding: var(--sp-xs) var(--sp-sm);
    background: var(--surface-container-high);
    color: var(--on-surface);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-sm);
    font-size: 0.9rem;
  }

  .picker-select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .picker-description {
    margin: 0;
    padding: 0;
    color: var(--on-surface-variant);
    font-size: 0.8rem;
    line-height: 1.4;
  }
</style>