// CR-07 §29.3b.2: vitest setup file.
//
// Placeholder for future global polyfills + matcher
// extensions. The slice does not need any globals
// right now; the file exists so the vitest config
// can reference it without errors.
//
// Future additions might include:
//
// - `expect.extend(...)` for custom matchers (e.g. a
//   `toMatchRGBA8` helper that compares RGBA8 buffers
//   with a per-channel tolerance).
// - jsdom extensions for WebGPU mocks (see
//   `src/lib/__tests__/mock-gpu.ts`).
// - Global `beforeEach` hooks for setting up mock
//   GPUDevice instances.

export {};