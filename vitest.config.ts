// CR-07 §29.3b.2: Vitest configuration.
//
// Sets up the JS-side test runner for the AstroForge
// project. The project previously had no JS-side
// testing infrastructure; the §29.3 GPU/WebGPU
// acceleration slices shipped WGSL shaders + TS
// wrappers without behavioural tests.
//
// Test environment: `jsdom` (NOT `happy-dom`). jsdom
// provides a more complete DOM + Window + Navigator
// polyfill, which is required by the WebGPU module
// (`navigator.gpu.requestAdapter`). The mock GPUDevice
// tests do not actually use the navigator; jsdom is
// chosen for the more complete API surface so future
// Svelte-component tests can be added later without
// changing the environment.
//
// Globals: enabled. Tests can use `describe`, `it`,
// `expect`, `vi`, etc. without explicit imports.
// Reduces boilerplate for the slice's 20+ tests.
//
// Setup file: `vitest.setup.ts` (does nothing right
// now; placeholder for future global polyfills like
// `expect.extend` matchers or jsdom extensions).
//
// Coverage: enabled (v8 provider) but NOT enforced
// by the gate. `npm run test:coverage` is a manual
// command; the default `npm run test` skips
// coverage for fast feedback.
//
// The configuration does NOT:
//
// - Wire vitest to the CI gate. The CI gate is
//   `npm run check` (svelte-check) which is
//   unchanged by this slice. Adding vitest to CI is
//   §29.3b.2a (a follow-on if the project decides
//   to make JS-side tests a gate).
// - Enable `coverage` thresholds. Coverage is a
//   manual `npm run test:coverage` command for
//   ad-hoc inspection; the slice does not commit to
//   any threshold.

import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./vitest.setup.ts"],
    include: ["src/**/__tests__/**/*.test.ts", "src/**/*.test.ts"],
    exclude: ["node_modules", "dist", "src-tauri", "crates"],
    coverage: {
      provider: "v8",
      reporter: ["text", "html"],
      include: ["src/lib/**/*.ts"],
      exclude: ["src/lib/__tests__/**", "src/lib/webgpu-types.ts"],
    },
  },
});