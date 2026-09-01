---
name: AstroForge Precision
colors:
  surface: '#121414'
  surface-dim: '#121414'
  surface-bright: '#37393a'
  surface-container-lowest: '#0c0f0f'
  surface-container-low: '#1a1c1c'
  surface-container: '#1e2020'
  surface-container-high: '#282a2b'
  surface-container-highest: '#333535'
  on-surface: '#e2e2e2'
  on-surface-variant: '#dfbfba'
  inverse-surface: '#e2e2e2'
  inverse-on-surface: '#2f3131'
  outline: '#a78a86'
  outline-variant: '#58413e'
  surface-tint: '#ffb4a8'
  primary: '#ffb4a8'
  on-primary: '#680301'
  primary-container: '#ec6653'
  on-primary-container: '#5c0100'
  inverse-primary: '#a93627'
  secondary: '#ffb4a8'
  on-secondary: '#53211a'
  secondary-container: '#6f362e'
  on-secondary-container: '#efa196'
  tertiary: '#bcc3ff'
  on-tertiary: '#051c93'
  tertiary-container: '#7787f9'
  on-tertiary-container: '#001586'
  error: '#ffb4ab'
  on-error: '#690005'
  error-container: '#93000a'
  on-error-container: '#ffdad6'
  primary-fixed: '#ffdad4'
  primary-fixed-dim: '#ffb4a8'
  on-primary-fixed: '#410000'
  on-primary-fixed-variant: '#881e13'
  secondary-fixed: '#ffdad5'
  secondary-fixed-dim: '#ffb4a8'
  on-secondary-fixed: '#380c07'
  on-secondary-fixed-variant: '#6f362e'
  tertiary-fixed: '#dfe0ff'
  tertiary-fixed-dim: '#bcc3ff'
  on-tertiary-fixed: '#000c60'
  on-tertiary-fixed-variant: '#2839a9'
  background: '#121414'
  on-background: '#e2e2e2'
  surface-variant: '#333535'
typography:
  display-xl:
    fontFamily: Geist
    fontSize: 48px
    fontWeight: '700'
    lineHeight: '1.1'
    letterSpacing: -0.04em
  headline-lg:
    fontFamily: Geist
    fontSize: 32px
    fontWeight: '600'
    lineHeight: '1.2'
    letterSpacing: -0.02em
  headline-lg-mobile:
    fontFamily: Geist
    fontSize: 24px
    fontWeight: '600'
    lineHeight: '1.2'
  data-viz:
    fontFamily: Space Grotesk
    fontSize: 14px
    fontWeight: '500'
    lineHeight: '1.4'
    letterSpacing: 0.02em
  body-base:
    fontFamily: Inter
    fontSize: 15px
    fontWeight: '400'
    lineHeight: '1.6'
    letterSpacing: 0.01em
  label-caps:
    fontFamily: Space Grotesk
    fontSize: 11px
    fontWeight: '700'
    lineHeight: '1'
    letterSpacing: 0.1em
  metadata-sm:
    fontFamily: Space Grotesk
    fontSize: 12px
    fontWeight: '400'
    lineHeight: '1.4'
rounded:
  sm: 0.125rem
  DEFAULT: 0.25rem
  md: 0.375rem
  lg: 0.5rem
  xl: 0.75rem
  full: 9999px
spacing:
  unit: 4px
  xs: 4px
  sm: 8px
  md: 16px
  lg: 24px
  xl: 40px
  panel-gutter: 1px
  container-safe: 24px
---

## Brand & Style

The design system for this AI-augmented astrophotography suite is rooted in **Scientific Modernism**. It aims to evoke the feeling of a professional observatory workstation—precise, technical, and high-performance. The interface is optimized for low-light environments to preserve the user's dark-adapted vision while reviewing sensitive celestial data.

The style leverages **Minimalism** with subtle **Glassmorphism**. High-density information is organized through modular panels, hairline borders, and a strict 4px grid. Visual depth is achieved through translucent overlays and faint glows, mimicking the atmospheric distortion of light against the deep cosmos.

- **Tone**: Analytical, reliable, futuristic, and expert-grade.
- **Visual Strategy**: Use razor-thin 1px borders to delineate workspace modules. Avoid heavy shadows; instead, use tonal elevation and backdrop blurs (12px–20px) to establish hierarchy.
- **Interactions**: Sharp transitions (150ms) for data updates and micro-glows on active technical parameters using the warmer accent palette.

## Colors

The palette is engineered for a **Deep Space Dark Mode** experience, shifted toward warmer, spectral reds to better preserve night vision in field conditions. By shifting the neutral seed to pure white, the system generates high-clarity technical surfaces.

- **Primary (Solar Flare Red - #CB4E3D)**: Used for primary brand moments, complex processing nodes, and high-energy states. This hue is specifically selected for its low impact on rod-cell desensitization.
- **Secondary (Dust Muted Rose - #A8655B)**: The functional workhorse. Used for interactive data points, active sliders, and precise metrics.
- **Surface (Technical Grey)**: Derived from the neutral white seed to separate modular panels and containers from the midnight background with maximum legibility.
- **Border (Deep Orbit Blue - #001891)**: Used as a subtle tertiary differentiator for grouping data or providing structure without the aggression of red.
- **Status Tints**: Success (Emerald #10B981), Processing (Rose #A8655B), and Error (Crimson #640000).

## Typography

The typography system differentiates between **Narrative/Directional** text and **Technical/Metadata** content.

- **Geist** is used for headlines and primary UI labels to provide a clean, modern, and slightly technical aesthetic.
- **Inter** handles all body copy and instructions, ensuring maximum legibility during long processing sessions.
- **Space Grotesk** is the technical engine of the system. It is reserved for all variable data: FITS headers, sensor parameters, coordinates (RA/Dec), and histogram values. Its quirky, geometric structure provides high scan-rate legibility for tabular data.

**Scaling**: On mobile devices, display type scales down aggressively to maintain the "instrument panel" feel without horizontal scrolling.

## Layout & Spacing

This design system utilizes a **Fluid Grid** for the main workspace, paired with **Fixed-Width Sidebar Panels** (typically 320px) for technical controls. 

- **Rhythm**: A 4px baseline grid ensures all components align perfectly. Use `16px (md)` for standard padding and `24px (lg)` for major section separation.
- **Density**: The layout is "High Density." UI elements are packed tightly to maximize the visibility of the image canvas.
- **Breakpoints**: 
  - **Mobile (<768px)**: Single column, bottom-sheet technical controls.
  - **Tablet (768px - 1280px)**: Collapsible sidebars, 2-column data grids.
  - **Desktop (>1280px)**: Permanent dual-sidebars (Left: Pipeline, Right: Histogram/Controls).

## Elevation & Depth

Depth is conveyed through **Tonal Layering** and **Glassmorphism** rather than traditional drop shadows.

1.  **Level 0 (Base)**: Darkest neutral foundation. No effects.
2.  **Level 1 (Panels)**: Surface tiers with a 1px border. 
3.  **Level 2 (Overlays/Modals)**: Semi-transparent surface (80% opacity) with a `backdrop-filter: blur(16px)`.
4.  **Active Focus**: Instead of elevation, use a **Solar Flare Red** 1px outer glow (4px spread, 20% opacity) to indicate the active processing node or selected slider.

## Shapes

The shape language is **Soft** but disciplined. 

- UI elements like buttons and input fields use a **4px (0.25rem)** radius to maintain a professional, "machined" look.
- Larger containers and cards use **8px (0.5rem)** to slightly soften the technical edge.
- Interactive nodes in the pipeline view are circular (pill-shaped) to distinguish them from data inputs.
- Use 0px (sharp) corners for the histogram bars and technical graph line-segments to emphasize mathematical accuracy.

## Components

- **Technical Sliders**: Track should be a 2px muted line. The handle is a 12px circular Solar Flare Red disc. Display live Space Grotesk numerical values above the handle during interaction.
- **Node-Based Pipeline**: Connectors are 1px solid lines using the tertiary Deep Orbit Blue (#001891). Active nodes have a Solar Flare Red pulse.
- **Histogram Visualizations**: Use the darkest generated neutral for the chart area. Data should be rendered in Solar Flare Red with a subtle 10% opacity fill below the curve.
- **Buttons**:
  - *Primary*: Solar Flare Red background, high-contrast text, 4px radius.
  - *Ghost*: 1px border using the neutral-derived outline, transparent background, neutral text.
- **Status Indicators**: Small 8px "LED" style circles. Use a CSS `box-shadow` to create a "lit" effect when the status is active.
- **Cards**: Use the "Level 1" elevation style. Headers within cards should have a 1px bottom border and use the `label-caps` typography style.