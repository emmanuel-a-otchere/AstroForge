// CR-07 §29.3b.2: mock GPUDevice for behavioural tests.
//
// The WebGPU wrappers (`webgpu-diff.ts`, `webgpu-spatial.ts`)
// interact with the WebGPU API to dispatch compute shaders.
// In a real browser, these calls land on a GPU. In the
// test environment (jsdom + Node), no GPU is available,
// so we provide a minimal mock that records every call
// and returns fake buffers.
//
// The mock is **NOT** a complete WebGPU implementation.
// It captures the structure of every call so tests can
// assert:
//
// - Which shader sources are loaded (per mode).
// - How many buffers are created and what their sizes
//   are.
// - How many workgroups are dispatched.
// - Whether the uniform buffer is written to with the
//   expected params.
// - Whether the readback sequence (mapAsync -> unmap ->
//   destroy) is invoked.
//
// The mock returns pre-defined output data so the
// wrappers can run their full code paths (including
// the post-dispatch readback + result unpacking). The
// test author sets the expected output via
// `setReadbackData()` before each call.

import type { SpatialMode } from "../webgpu-spatial-shaders";

// ─── Types ──────────────────────────────────────────────

export interface RecordedCall {
  method: string;
  args: unknown[];
}

export interface RecordedShader {
  code: string;
}

export interface RecordedBuffer {
  size: number;
  usage: number;
  mappedAtCreation?: boolean;
  destroyed: boolean;
  /** The last data written via `queue.writeBuffer`. */
  lastWritten?: ArrayBuffer;
}

// ─── Mock GPUBuffer ─────────────────────────────────────

class MockGPUBuffer implements Partial<GPUBuffer> {
  size: number;
  usage: number;
  destroyed = false;
  private _mappedRange: ArrayBuffer | null = null;
  /** Test-visible: the most recent data written to this buffer. */
  public lastWritten: ArrayBuffer | undefined;

  constructor(size: number, usage: number, mappedAtCreation?: boolean) {
    this.size = size;
    this.usage = usage;
    if (mappedAtCreation) {
      this._mappedRange = new ArrayBuffer(size);
    }
  }

  destroy(): void {
    this.destroyed = true;
    this._mappedRange = null;
  }

  getMappedRange(): ArrayBuffer {
    if (!this._mappedRange) {
      throw new Error("buffer not mapped");
    }
    return this._mappedRange;
  }

  unmap(): void {
    this._mappedRange = null;
  }

  async mapAsync(_mode: number): Promise<undefined> {
    this._mappedRange = new ArrayBuffer(this.size);
    return undefined;
  }
}

// ─── Mock GPUQueue ─────────────────────────────────────

class MockGPUQueue implements Partial<GPUQueue> {
  submitted: GPUCommandBuffer[] = [];

  submit(commandBuffers: GPUCommandBuffer[]): undefined {
    this.submitted.push(...commandBuffers);
  }

  writeBuffer(
    buffer: MockGPUBuffer,
    _bufferOffset: number,
    data: ArrayBuffer,
  ): undefined {
    buffer.lastWritten = data;
  }
}

// ─── Mock GPUComputePassEncoder ─────────────────────────

class MockComputePassEncoder {
  pipeline: unknown = null;
  bindGroups: Map<number, unknown> = new Map();
  dispatchedWorkgroups: Array<{ x: number; y?: number; z?: number }> = [];

  setPipeline(pipeline: unknown): undefined {
    this.pipeline = pipeline;
  }
  setBindGroup(groupIndex: number, group: unknown): undefined {
    this.bindGroups.set(groupIndex, group);
  }
  dispatchWorkgroups(x: number, y?: number, z?: number): undefined {
    this.dispatchedWorkgroups.push({ x, y, z });
  }
  end(): undefined {}
}

// ─── Mock GPUCommandEncoder ─────────────────────────────

class MockCommandEncoder {
  computePasses: MockComputePassEncoder[] = [];
  copies: Array<{
    source: unknown;
    destination: unknown;
    sourceOffset?: number;
    destinationOffset?: number;
    size?: number;
  }> = [];

  beginComputePass(): MockComputePassEncoder {
    const pass = new MockComputePassEncoder();
    this.computePasses.push(pass);
    return pass;
  }

  copyBufferToBuffer(
    source: unknown,
    destination: unknown,
    sourceOffset?: number,
    destinationOffset?: number,
    size?: number,
  ): undefined {
    this.copies.push({ source, destination, sourceOffset, destinationOffset, size });
  }

  finish(): GPUCommandBuffer {
    return {} as GPUCommandBuffer;
  }
}

// ─── Mock GPUDevice ────────────────────────────────────

export interface MockGpuDeviceOptions {
  /** Pre-defined data for readback. If unset, the readback returns zeros. */
  readbackData?: ArrayBuffer;
}

export class MockGpuDevice {
  queue = new MockGPUQueue();
  shaders: RecordedShader[] = [];
  buffers: RecordedBuffer[] = [];
  /** Parallel array of MockGPUBuffer instances matching `buffers`. */
  bufferInstances: MockGPUBuffer[] = [];
  pipelines: Array<{ entryPoint: string; moduleCode: string }> = [];
  bindGroups: Array<{ layout: unknown; entries: unknown[] }> = [];
  commandEncoders: MockCommandEncoder[] = [];

  private _readbackData: ArrayBuffer;

  constructor(options: MockGpuDeviceOptions = {}) {
    this._readbackData = options.readbackData ?? new ArrayBuffer(0);
  }

  setReadbackData(data: ArrayBuffer): void {
    this._readbackData = data;
  }

  createBuffer(descriptor: GPUBufferDescriptor): GPUBuffer {
    const buf = new MockGPUBuffer(
      descriptor.size,
      descriptor.usage,
      descriptor.mappedAtCreation,
    );
    const recorded: RecordedBuffer = {
      size: descriptor.size,
      usage: descriptor.usage,
      mappedAtCreation: descriptor.mappedAtCreation,
      destroyed: false,
    };
    this.buffers.push(recorded);
    this.bufferInstances.push(buf);
    // Hook destroy() to update the recorded entry.
    const originalDestroy = buf.destroy.bind(buf);
    buf.destroy = () => {
      recorded.destroyed = true;
      originalDestroy();
    };
    // The MockGPUBuffer's `lastWritten` field is the
    // single source of truth. Tests can read it via
    // `mock.bufferInstances[i].lastWritten`. The
    // RecordedBuffer's lastWritten is NOT automatically
    // synced (would require a Proxy on each buffer); tests
    // can call `mock.syncRecordedWrites()` if needed.
    if (descriptor.mappedAtCreation) {
      // Pre-fill the mapped range with the readback data
      // (so the wrapper can copy its input into it).
      const view = new Uint8Array(buf.getMappedRange());
      const src = new Uint8Array(this._readbackData);
      view.set(src.subarray(0, Math.min(src.length, view.length)));
    }
    return buf as unknown as GPUBuffer;
  }

  createShaderModule(descriptor: GPUShaderModuleDescriptor): GPUShaderModule {
    this.shaders.push({ code: descriptor.code });
    return {} as GPUShaderModule;
  }

  createBindGroupLayout(_descriptor: GPUBindGroupLayoutDescriptor): GPUBindGroupLayout {
    return {} as GPUBindGroupLayout;
  }

  createPipelineLayout(descriptor: GPUPipelineLayoutDescriptor): GPUPipelineLayout {
    return { bindGroupLayouts: descriptor.bindGroupLayouts } as GPUPipelineLayout;
  }

  createComputePipeline(descriptor: GPUComputePipelineDescriptor): GPUComputePipeline {
    const moduleCode = this.shaders[this.shaders.length - 1]?.code ?? "";
    this.pipelines.push({
      entryPoint: descriptor.compute.entryPoint,
      moduleCode,
    });
    return {} as unknown as GPUComputePipeline;
  }

  createBindGroup(descriptor: GPUBindGroupDescriptor): GPUBindGroup {
    this.bindGroups.push({
      layout: descriptor.layout,
      entries: descriptor.entries,
    });
    return {} as GPUBindGroup;
  }

  createCommandEncoder(_descriptor?: object): GPUCommandEncoder {
    const encoder = new MockCommandEncoder();
    this.commandEncoders.push(encoder);
    return encoder as unknown as GPUCommandEncoder;
  }

  destroy(): void {}

  /** Helper: get the last compute pass (for assertions). */
  lastComputePass(): MockComputePassEncoder | undefined {
    for (let i = this.commandEncoders.length - 1; i >= 0; i--) {
      const encoder = this.commandEncoders[i]!;
      if (encoder.computePasses.length > 0) {
        return encoder.computePasses[encoder.computePasses.length - 1];
      }
    }
    return undefined;
  }

  /** Sync recorded buffers' lastWritten from the
   * MockGPUBuffer instances. The queue's writeBuffer
   * already updates each MockGPUBuffer's `lastWritten`
   * field. We copy that to the matching RecordedBuffer
   * entry so tests that read `mock.buffers[i].lastWritten`
   * see the up-to-date value. */
  syncRecordedWrites(): void {
    for (let i = 0; i < this.buffers.length; i++) {
      const recorded = this.buffers[i]!;
      const instance = this.bufferInstances[i]!;
      recorded.lastWritten = instance.lastWritten;
    }
  }
}

// ─── Helpers ───────────────────────────────────────────

/**
 * Build a `Uint8Array` of `width * height * 4` bytes
 * representing an RGBA8 image. Each pixel is
 * `(x * 7 + y * 13) & 0xff` for R/G/B, alpha 255.
 */
export function makeRgbaGrid(width: number, height: number): Uint8Array {
  const out = new Uint8Array(width * height * 4);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const base = (y * width + x) * 4;
      out[base + 0] = (x * 7 + y * 13) & 0xff;
      out[base + 1] = (x * 11 + y * 5) & 0xff;
      out[base + 2] = (x * 17 + y * 3) & 0xff;
      out[base + 3] = 255;
    }
  }
  return out;
}

/**
 * Build a `Float32Array` of `width * height * channels`
 * floats representing an f32 image (e.g. luminance in
 * [0, 1]).
 */
export function makeF32Image(width: number, height: number, channels: number): Float32Array {
  const out = new Float32Array(width * height * channels);
  for (let c = 0; c < channels; c++) {
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const i = c * width * height + y * width + x;
        out[i] = ((x + y + c) % 16) / 16.0;
      }
    }
  }
  return out;
}