<!--
  CanvasBackdrop — ambient backdrop layer behind all four modes.

  Visibility scales with the current layout mode (option 2c from the
  user's spec):
    A — Load:           10% intensity (very faint, focus on overlay cards)
    B — Library:        30% intensity (subtle, gallery/canvas primary)
    C — Automagic Pro:  20% intensity (light, so they don't fight tabs)
    D — Refine:        100% intensity (immersive starfield)

  Always renders, intensity is just opacity on the radial gradients
  and stars layer. The header/footer get a stronger glass effect to
  remain legible when intensity is high.
-->
<script lang="ts">
  import { currentLayoutMode, type LayoutMode } from "../lib/layout-mode";

  /** Optional override — useful for previews and tests. */
  export let overrideMode: LayoutMode | null = null;

  $: effectiveMode = overrideMode ?? $currentLayoutMode;

  // Map mode -> opacity multipliers for the two gradient layers and stars
  $: gradientOpacity =
    effectiveMode === "d" ? 1.0 :
    effectiveMode === "b" ? 0.30 :
    effectiveMode === "c" ? 0.20 :
    /* a */                  0.10;

  $: starsOpacity =
    effectiveMode === "d" ? 0.6 :
    effectiveMode === "b" ? 0.18 :
    effectiveMode === "c" ? 0.12 :
    /* a */                  0.06;
</script>

<div class="canvas-backdrop" data-mode={effectiveMode} aria-hidden="true">
  <div
    class="canvas-gradient"
    style="opacity: {gradientOpacity}"
  ></div>
  <div
    class="canvas-stars"
    style="opacity: {starsOpacity}"
  ></div>
</div>

<style>
  .canvas-backdrop {
    position: fixed;
    inset: 0;
    pointer-events: none;
    z-index: 0;
  }

  .canvas-gradient {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(
        ellipse at 30% 20%,
        var(--glow-cobalt-soft),
        transparent 60%
      ),
      radial-gradient(
        ellipse at 70% 80%,
        var(--tile-glow-cool-strong),
        transparent 60%
      ),
      radial-gradient(
        ellipse at 50% 50%,
        var(--canvas-glow-mid),
        transparent 70%
      );
    transition: opacity var(--transition-slow);
  }

  .canvas-stars {
    position: absolute;
    inset: 0;
    background-image:
      radial-gradient(circle at 20% 30%, var(--star-color-100) 0.5px, transparent 1px),
      radial-gradient(circle at 70% 60%, var(--star-color-83) 0.5px, transparent 1px),
      radial-gradient(circle at 40% 80%, var(--star-color-67) 0.5px, transparent 1px),
      radial-gradient(circle at 85% 25%, var(--star-color-83) 0.5px, transparent 1px),
      radial-gradient(circle at 15% 70%, var(--star-color-67) 0.5px, transparent 1px),
      radial-gradient(circle at 60% 15%, var(--star-color-58) 0.5px, transparent 1px);
    transition: opacity var(--transition-slow);
  }

  /* Mode D gets a touch more saturation in the gradient for the
     "immersive refinement" feel */
  .canvas-backdrop[data-mode="d"] .canvas-gradient {
    background:
      radial-gradient(
        ellipse at 30% 20%,
        var(--canvas-mode-d-warm),
        transparent 60%
      ),
      radial-gradient(
        ellipse at 70% 80%,
        var(--tile-glow-cool-strong),
        transparent 60%
      );
  }
</style>