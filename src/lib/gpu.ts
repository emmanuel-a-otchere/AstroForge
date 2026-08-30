export type GpuCapability = "webgpu" | "canvas2d";

export function probeGpu(): GpuCapability {
  if (typeof navigator !== "undefined" && "gpu" in navigator) {
    return "webgpu";
  }
  const canvas = document.createElement("canvas");
  if (canvas.getContext("webgpu")) {
    return "webgpu";
  }
  if (canvas.getContext("2d")) {
    return "canvas2d";
  }
  return "canvas2d";
}
