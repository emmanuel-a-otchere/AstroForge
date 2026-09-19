// CR-07 §29.3a: minimal WebGPU type declarations.
//
// The project does not depend on `@webgpu/types` yet,
// so the WebGPU types used by `webgpu-diff.ts` are
// declared inline here. The declarations cover only
// the subset of the WebGPU API that `webgpu-diff.ts`
// actually uses:
//
// - `GPUAdapter` + `navigator.gpu.requestAdapter`
// - `GPUDevice` + `adapter.requestDevice`
// - `GPUBuffer` + `GPUBufferUsage`
// - `GPUComputePipeline` + `GPUPipelineLayout`
// - `GPUBindGroupLayout` + `GPUBindGroup` + bind
//   group entry shapes
// - `GPUCommandEncoder` + `GPUComputePassEncoder`
// - `GPUShaderModule` + `GPUShaderStage`
// - `GPUMapMode` (READ for readback)
//
// All declarations are deliberately minimal:
// no deprecated / experimental entry points, no
// surface that the slice does not exercise. Future
// slices that need more WebGPU API surface should
// extend this file or migrate to the
// `@webgpu/types` package.

interface GPURequestAdapterOptions {
  powerPreference?: "low-power" | "high-performance";
}

interface GPUDeviceDescriptor {
  label?: string;
}

interface GPUShaderModuleDescriptor {
  code: string;
}

interface GPUPipelineLayoutDescriptor {
  bindGroupLayouts: GPUBindGroupLayout[];
}

interface GPUBindGroupLayoutEntry {
  binding: number;
  visibility: number;
  buffer?:
    | { type: "uniform" }
    | { type: "storage" }
    | { type: "read-only-storage" };
}

interface GPUBindGroupLayoutDescriptor {
  entries: GPUBindGroupLayoutEntry[];
}

interface GPUBindGroupEntry {
  binding: number;
  resource: { buffer: GPUBuffer };
}

interface GPUBindGroupDescriptor {
  layout: GPUBindGroupLayout;
  entries: GPUBindGroupEntry[];
}

interface GPUComputePipelineDescriptor {
  layout: GPUPipelineLayout;
  compute: {
    module: GPUShaderModule;
    entryPoint: string;
  };
}

interface GPUBufferDescriptor {
  size: number;
  usage: number;
  label?: string;
  mappedAtCreation?: boolean;
}

interface GPUCommandBuffer {
}

interface GPUComputePassDescriptor {
}

interface GPUBufferCopyView {
  buffer: GPUBuffer;
}

interface GPULoadStoreOp {
}

interface GPUProgrammableStage {
  module: GPUShaderModule;
  entryPoint: string;
}

interface GPUBufferBinding {
  buffer: GPUBuffer;
  offset?: number;
  size?: number;
}

interface GPUShaderModule {}

interface GPUBindGroupLayout {
}

interface GPUBuffer {
  size: number;
  destroy(): void;
  getMappedRange(): ArrayBuffer;
  unmap(): void;
  mapAsync(mode: number): Promise<undefined>;
}

declare const GPUShaderStage_: never;
interface GPUShaderStage {
  readonly COMPUTE: number;
  readonly VERTEX: number;
  readonly FRAGMENT: number;
}
const GPUShaderStage: GPUShaderStage = {
  COMPUTE: 1,
  VERTEX: 2,
  FRAGMENT: 4,
};

declare const GPUBufferUsage_: never;
interface GPUBufferUsage {
  readonly MAP_READ: number;
  readonly MAP_WRITE: number;
  readonly COPY_SRC: number;
  readonly COPY_DST: number;
  readonly INDEX: number;
  readonly VERTEX: number;
  readonly UNIFORM: number;
  readonly STORAGE: number;
  readonly INDIRECT: number;
  readonly QUERY_RESOLVE: number;
}
const GPUBufferUsage: GPUBufferUsage = {
  MAP_READ: 1,
  MAP_WRITE: 2,
  COPY_SRC: 4,
  COPY_DST: 8,
  INDEX: 16,
  VERTEX: 32,
  UNIFORM: 64,
  STORAGE: 128,
  INDIRECT: 256,
  QUERY_RESOLVE: 512,
};

declare const GPUMapMode_: never;
interface GPUMapMode {
  readonly READ: number;
  readonly WRITE: number;
}
const GPUMapMode: GPUMapMode = {
  READ: 1,
  WRITE: 2,
};

interface GPUComputePipeline {}

interface GPUPipelineLayout {}

interface GPUCommandEncoder {
  beginComputePass(descriptor?: GPUComputePassDescriptor): GPUComputePassEncoder;
  copyBufferToBuffer(
    source: GPUBuffer,
    destination: GPUBuffer,
    sourceOffset?: number,
    destinationOffset?: number,
    size?: number,
  ): undefined;
  finish(descriptor?: object): GPUCommandBuffer;
}

interface GPUComputePassEncoder {
  setPipeline(pipeline: GPUComputePipeline): undefined;
  setBindGroup(groupIndex: number, group: GPUBindGroup): undefined;
  dispatchWorkgroups(x: number, y?: number, z?: number): undefined;
  end(): undefined;
}

interface GPUBindGroup {}

interface GPUQueue {
  submit(commandBuffers: GPUCommandBuffer[]): undefined;
  writeBuffer(
    buffer: GPUBuffer,
    bufferOffset: number,
    data: ArrayBuffer | SharedArrayBuffer,
    dataOffset?: number,
    size?: number,
  ): undefined;
}

interface GPUAdapter {
  requestDevice(descriptor?: GPUDeviceDescriptor): Promise<GPUDevice>;
}

interface GPUDevice {
  label?: string;
  queue: GPUQueue;
  createBuffer(descriptor: GPUBufferDescriptor): GPUBuffer;
  createShaderModule(descriptor: GPUShaderModuleDescriptor): GPUShaderModule;
  createBindGroupLayout(
    descriptor: GPUBindGroupLayoutDescriptor,
  ): GPUBindGroupLayout;
  createPipelineLayout(
    descriptor: GPUPipelineLayoutDescriptor,
  ): GPUPipelineLayout;
  createComputePipeline(
    descriptor: GPUComputePipelineDescriptor,
  ): GPUComputePipeline;
  createBindGroup(descriptor: GPUBindGroupDescriptor): GPUBindGroup;
  createCommandEncoder(descriptor?: object): GPUCommandEncoder;
  destroy(): void;
}

interface NavigatorGPU {
  requestAdapter(
    options?: GPURequestAdapterOptions,
  ): Promise<GPUAdapter | null>;
}

interface Navigator {
  gpu: NavigatorGPU;
}