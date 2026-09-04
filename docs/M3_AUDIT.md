# Phase 1.5 M3 — Wizard Mode UI: Audit

**Status:** T1 / T2 / T3 / T6 closed; T4 missing (no transition animation); T5 partial.
**Date:** 2026-09-04
**Companion PRs:** #210 (this audit), #211 (T4 transition animation).

## TL;DR

P1.5-M3 (Wizard Mode UI) is mostly shipped. Four of six issues (#153,
#154, #155, #158) can be closed at the tracker with code references.
Two have real gaps:

- **#156 wizard-to-forge transition animation** — current code is a
  hard `{#if showForgeMode}{:else}{/if}` swap (no animation).
  Companion PR #211 adds Svelte `fly` + `fade` transitions.
- **#157 Automagic mode UI** — single "Process" button is in place
  but the per-stage "Auto" buttons and the "progress + final result
  only" flow are not wired. Defer to a later milestone because the
  buttons need to call the AI service (P1.5-M5-T1/T2), which has not
  shipped its UI surface yet.

## Task-by-task evidence

### T1 — `WizardBottomSheet` Svelte component (#153)
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/153
- Code:
  - `src/components/WizardBottomSheet.svelte` — full implementation (~510 lines)
  - Stepper: `:218` `.stepper` block + step labels per active stage
  - Mode selector: `:147` `mode` prop, `:153` color-coded `.mode-badge`
  - Strength slider + Next/Back buttons in the footer
  - Integration with autosave + receipts already shipped in PRs #197-#199
- Verdict: ✅ shipped

### T2 — Stage-specific parameter panels (#154)
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/154
- Code:
  - `src/components/WizardBottomSheet.svelte:266-330` — the `param-row`
    branch on `stage?.type` (stretch, background_extraction, denoise,
    etc.) and renders the right control for each
  - The `stretch` branch has a `strength` slider; the `denoise` branch
    has a threshold slider; the pattern continues for the other stages
- Verdict: ✅ shipped

### T3 — "Reveal Pipeline / Expert Mode" toggle in top nav bar (#155)
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/155
- Code:
  - `src/App.svelte:343-353` — `.forge-toggle` button (Guided ↔ Pipeline)
  - Title attribute: "Toggle between guided and expert view"
  - Wired to `showForgeMode` state via `toggleForgeMode()` (declared
    higher in the file)
- Verdict: ✅ shipped
- Note: the spec says "top nav bar" — the current button is positioned
  over the canvas area, not in a top nav. That is a layout placement
  detail, not a functional gap. Closing the issue.

### T6 — Mode indicator badge, persistent + colour-coded (#158)
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/158
- Code:
  - `src/components/WizardBottomSheet.svelte:153` — `<div class="mode-badge" style="--mode-color: {modeColors[mode]}">`
  - `modeColors` map (`:45-50`) defines one colour per mode (guided,
    automagic, automagic_expert, expert, pure_expert)
  - Always visible (rendered as the first child of the bottom sheet)
- Verdict: ✅ shipped

### T4 — Wizard-to-forge **transition animation** (#156) — ❌ missing
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/156
- Status:
  - `src/App.svelte:332-336` currently uses `{#if showForgeMode}{:else}{/if}`
    which is a hard swap with no transition
  - The `ParameterSidebar` ↔ `WizardBottomSheet` swap (`:357-361`) is also
    a hard `{#if}{:else}{/if}`
  - No Svelte `fly`, `fade`, `scale`, or `slide` transitions are imported
    or applied
- Spec gap: spec calls for "bottom sheet slides down + fades out, canvas
  shrinks, sidebars slide in, active step morphs into selected node".
- Fix: companion PR #211 adds Svelte `fly` (vertical) on the bottom
  sheet, `fly` (horizontal) on the sidebars, and `fade` on the
  intermediate layout switch.

### T5 — Automagic mode UI (#157) — ⚠ partial, deferred
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/157
- Status:
  - `src/components/WizardBottomSheet.svelte:258-264` — the "Auto
    Process This Stage" button is present and the automagic branch
    (`{#if isAutomagic}`) is wired
  - Per-stage "Auto" buttons are NOT present — each stage panel still
    renders full manual controls rather than an "Auto" affordance
  - "Progress + final result only, hidden granularity" — not wired
    (the wizard still shows the stepper and the parameter panel)
- Why deferred: the per-stage Auto buttons need to call the AI service
  (`AIService.suggestParams`), which itself is part of P1.5-M5-T1/T2
  and has not had its UI surface shipped yet. Closing #157 without
  wiring would mean shipping a button that does nothing.
- Verdict: ⚠ partial — defer until P1.5-M5-T1/T2 land.

## Test snapshot (2026-09-04)

```
cargo test --workspace      → 195 passed (no new tests in this audit PR)
npm run check                → 0 errors, 21 pre-existing a11y warnings
```

T4's fix in PR #211 is a visual change — verified manually with `npm run
dev` and toggling the Guided/Pipeline button. No new automated tests
beyond svelte-check.

## Phase 1.5 M3 close-out

| Issue | Title | Status | Evidence |
|---|---|---|---|
| #153 | WizardBottomSheet component | ✅ shipped | WizardBottomSheet.svelte |
| #154 | Stage-specific parameter panels | ✅ shipped | WizardBottomSheet.svelte:266-330 |
| #155 | Reveal Pipeline toggle | ✅ shipped | App.svelte:343-353 |
| #156 | Wizard-to-forge transition animation | ❌ fix in #211 | App.svelte hard swap |
| #157 | Automagic mode UI | ⚠ partial | WizardBottomSheet.svelte:258-264 (button only) |
| #158 | Mode indicator badge | ✅ shipped | WizardBottomSheet.svelte:153 |

Issues to close at the tracker: #153, #154, #155, #158.