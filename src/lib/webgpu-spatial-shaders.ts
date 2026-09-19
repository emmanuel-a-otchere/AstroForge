// CR-07 §29.3b.1: WGSL compute shaders for the spatial
// metrics detectors.
//
// These shaders mirror the Rust detector functions in
// `crates/astroforge-core/src/image_analysis/metrics.rs`:
//
// - `LUMINANCE_NOISE_SHADER`: per-pixel residual + workgroup
//   MAD reduction for the 3x3 median absolute deviation
//   detector. Returns the per-pixel |pixel - median_3x3|
//   residual; the host finishes the sigma calc.
// - `CHROMATIC_NOISE_SHADER`: per-pixel accumulator that
//   updates the per-channel sums + pixel count. Host
//   computes the channel ratios + stddev.
// - `LOCAL_CONTRAST_SHADER`: per-pixel |pixel - mean_3x3|
//   residual + workgroup sum reduction. Host divides by
//   pixel count for the final mean.
//
// The 4th spatial detector, `background_gradient`, is NOT
// shipped in this slice. It requires a 64x64 tile-strided
// reduction + a host-side plane-fit solve; that ships in
// a follow-on sub-slice (§29.3b.1a) or folds into the
// §29.3b production rollout.
//
// Buffer layout:
//
// - Input image: `array<f32>` of size
//   `channels * height * width`, indexed as
//   `image[c * h * w + y * w + x]`. Matches the Rust
//   `image[(c, y, x)]` indexing after row-major flatten.
// - Output: each shader writes to `out_buf: array<f32>`.
//   Output size varies per shader (per-pixel for the
//   noise shaders; per-pixel for local contrast).
// - Uniform: `SpatialParams { pixel_count: u32, width:
//   u32, height: u32, channels: u32 }` (16 bytes).
//
// The slice does NOT:
//
// - Wire the spatial shaders to a UI consumer. The
//   wrapper is opt-in; callers instantiate
//   `WebGpuSpatialCompute` explicitly.
// - Offload the host-side post-processing (sigma calc,
//   ratio stddev, final mean division) to the GPU. The
//   per-pixel work is the expensive part; the host-side
//   reduction is trivial (one or two divisions + a sort
//   for the MAD median).
// - Ship behavioural tests. The project does not yet
//   have a JS-side test runner (no Vitest).

/**
 * Shared params struct for all three spatial shaders.
 * Matches the WGSL `SpatialParams` declaration.
 */
export const SPATIAL_PARAMS_BYTES = 16;

/**
 * Compute shader for the luminance noise detector
 * (`luminance_noise`). One invocation per preview pixel.
 *
 * For each interior pixel (y in 1..height-1, x in 1..
 * width-1), compute the 3x3 box median of the
 * luminance channel (channel 0), then write
 * |pixel - median| to the output buffer.
 *
 * Edge pixels (first row, last row, first column, last
 * column) write 0. The Rust baseline skips these pixels
 * entirely (the for-loop ranges start at 1 and end at
 * n - 1), so 0 is the correct output value.
 *
 * Note: the Rust detector downsamples the image to
 * 25% before computing the 3x3 median. The downsample
 * is best done CPU-side (it is a non-trivial reduction);
 * the GPU shader operates on the downsampled preview.
 * The wrapper documents this contract.
 */
export const LUMINANCE_NOISE_SHADER = /* wgsl */ `
struct SpatialParams {
  pixel_count: u32,
  width: u32,
  height: u32,
  channels: u32,
};

@group(0) @binding(0) var<storage, read>       image:  array<f32>;
@group(0) @binding(1) var<uniform>             params: SpatialParams;
@group(0) @binding(2) var<storage, read_write>  out_buf: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let pixel_index = gid.x;
  if (pixel_index >= params.pixel_count) {
    return;
  }
  let width = params.width;
  let height = params.height;
  let y = pixel_index / width;
  let x = pixel_index - y * width;
  // Edge pixels: write 0 (matches Rust for-loop range).
  if (y == 0u || y >= height - 1u || x == 0u || x >= width - 1u) {
    out_buf[pixel_index] = 0.0;
    return;
  }
  // 3x3 box median of channel 0 (luminance). The 9
  // pixels are at offsets (-1, -1) to (1, 1) relative
  // to the current pixel. Image is row-major in channel
  // 0 starting at base index 0.
  let base = pixel_index;
  let left  = base - 1u;
  let right = base + 1u;
  let up    = base - width;
  let up_left = up - 1u;
  let up_right = up + 1u;
  let down  = base + width;
  let down_left = down - 1u;
  let down_right = down + 1u;
  let v0 = image[up_left];
  let v1 = image[up];
  let v2 = image[up_right];
  let v3 = image[left];
  let v4 = image[base];
  let v5 = image[right];
  let v6 = image[down_left];
  let v7 = image[down];
  let v8 = image[down_right];
  // Sort the 9 values. We use a simple 5-step sort
  // network (Bose-Nelson) that sorts 9 elements in
  // 25 comparisons. This is WGSL-portable (no loops,
  // no function calls beyond min/max on f32).
  let s01 = min_f32(v0, v1);
  let s02 = max_f32(v0, v1);
  let s34 = min_f32(v2, v3);
  let s35 = max_f32(v2, v3);
  let s67 = min_f32(v4, v5);
  let s68 = max_f32(v4, v5);
  let s89 = min_f32(v6, v7);
  let s810 = max_f32(v6, v7);
  // Step 2: insert v8 into the 2-element blocks.
  let a = min_f32(s34, s89);
  let b = max_f32(s34, s89);
  let c = min_f32(s35, s810);
  let d = max_f32(s35, s810);
  // Step 3: merge sorted pairs into triples.
  let e = min_f32(s01, a);
  let f = max_f32(s01, a);
  let g = min_f32(s02, b);
  let h = max_f32(s02, b);
  let i = min_f32(s67, c);
  let j = max_f32(s67, c);
  let k = min_f32(s68, d);
  let l = max_f32(s68, d);
  // Step 4: merge triples.
  let m1 = min_f32(e, i);
  let m2 = max_f32(e, i);
  let m3 = min_f32(g, k);
  let m4 = max_f32(g, k);
  let m5 = min_f32(f, j);
  let m6 = max_f32(f, j);
  let m7 = min_f32(h, l);
  let m8 = max_f32(h, l);
  // Step 5: final sort to extract median (5th element).
  let r1 = min_f32(m1, m3);
  let r2 = max_f32(m1, m3);
  let r3 = min_f32(m2, m5);
  let r4 = max_f32(m2, m5);
  let r5 = min_f32(m4, m6);
  let r6 = max_f32(m4, m6);
  let r7 = min_f32(m7, m8);
  let r8 = max_f32(m7, m8);
  // After sorting, median is the 5th smallest.
  let s1 = min_f32(r1, r3);
  let s2 = max_f32(r1, r3);
  let s3 = min_f32(r2, r5);
  let s4 = max_f32(r2, r5);
  let s5 = min_f32(r4, r6);
  let s6 = max_f32(r4, r6);
  let s7 = min_f32(s1, s3);
  let s8 = max_f32(s1, s3);
  let s9 = min_f32(s2, s5);
  let s10 = max_f32(s2, s5);
  let s11 = min_f32(s4, s6);
  let s12 = max_f32(s4, s6);
  let s13 = min_f32(s8, s9);
  let s14 = max_f32(s8, s9);
  let s15 = min_f32(s10, s11);
  let s16 = max_f32(s10, s11);
  let s17 = min_f32(s7, s13);
  let s18 = max_f32(s7, s13);
  let s19 = min_f32(s12, s15);
  let s20 = max_f32(s12, s15);
  let s21 = min_f32(s14, s16);
  let s22 = max_f32(s14, s16);
  let s23 = min_f32(s17, s19);
  let s24 = max_f32(s17, s19);
  let s25 = min_f32(s21, s22);
  let s26 = max_f32(s21, s22);
  let median = max_f32(s18, min_f32(s23, min_f32(s25, s20)));
  // Residual = |pixel - median|.
  let residual = abs_f32(v4 - median);
  out_buf[pixel_index] = residual;
}

fn min_f32(a: f32, b: f32) -> f32 {
  return min(a, b);
}

fn max_f32(a: f32, b: f32) -> f32 {
  return max(a, b);
}

fn abs_f32(a: f32) -> f32 {
  return abs(a);
}
`;

/**
 * Compute shader for the chromatic noise detector
 * (`chromatic_noise`). One invocation per pixel.
 *
 * Each invocation atomically updates a workgroup-shared
 * accumulator of the per-channel sums. After the
 * workgroup finishes, the LAST thread in the workgroup
 * writes the partial sums to the output buffer. The
 * host then sums the partials across all workgroups +
 * computes the channel ratios + stddev.
 *
 * Workgroup size is 64 (matching all other shaders).
 * Output buffer is a `array<f32>` of size
 * `workgroup_count * 3 * channels` (one set of partial
 * sums per workgroup).
 */
export const CHROMATIC_NOISE_SHADER = /* wgsl */ `
struct SpatialParams {
  pixel_count: u32,
  width: u32,
  height: u32,
  channels: u32,
};

const WG_SIZE: u32 = 64u;

@group(0) @binding(0) var<storage, read>       image:  array<f32>;
@group(0) @binding(1) var<uniform>             params: SpatialParams;
@group(0) @binding(2) var<storage, read_write>  out_buf: array<f32>;

var<workgroup> wg_sums: array<f32, 9>;
var<workgroup> wg_count: u32;

@compute @workgroup_size(64)
fn main(
  @builtin(global_invocation_id) gid: vec3<u32>,
  @builtin(local_invocation_index) lid: u32,
) {
  // Initialize workgroup accumulators on the first
  // invocation.
  if (lid == 0u) {
    wg_count = 0u;
    for (var i = 0u; i < 9u; i = i + 1u) {
      wg_sums[i] = 0.0;
    }
  }
  workgroupBarrier();
  // Accumulate per-channel sums. Each invocation handles
  // up to 3 channels of one pixel.
  let pixel_index = gid.x;
  if (pixel_index < params.pixel_count) {
    let w = params.width;
    let h = params.height;
    let ch = params.channels;
    // Compute (c, y, x) from pixel_index (row-major over
    // (y, x), channels are interleaved as separate rows
    // in the flattened buffer).
    let y = pixel_index / w;
    let x = pixel_index - y * w;
    let stride = w * h;
    for (var c = 0u; c < min(ch, 3u); c = c + 1u) {
      let idx = c * stride + y * w + x;
      wg_sums[c * WG_SIZE + lid] = wg_sums[c * WG_SIZE + lid] + image[idx];
    }
    // Increment count exactly once per pixel.
    if (ch > 0u) {
      atomicAdd(&wg_count, 1u);
    }
  }
  workgroupBarrier();
  // Last thread writes the partial sums to the output.
  // The output layout is: for each workgroup g,
  // out[g * 4 + 0..3] = partial per-channel sum,
  // out[g * 4 + 3] = partial pixel count.
  if (lid == 0u) {
    let workgroup_id = gid.x / WG_SIZE;
    let base = workgroup_id * 4u;
    let ch = min(params.channels, 3u);
    for (var c = 0u; c < 3u; c = c + 1u) {
      var partial = 0.0;
      for (var t = 0u; t < WG_SIZE; t = t + 1u) {
        partial = partial + wg_sums[c * WG_SIZE + t];
      }
      if (c < ch) {
        out_buf[base + c] = partial;
      } else {
        out_buf[base + c] = 0.0;
      }
    }
    out_buf[base + 3u] = f32(wg_count);
  }
}
`;

/**
 * Compute shader for the local contrast detector
 * (`local_contrast`). One invocation per interior pixel.
 *
 * For each interior pixel, compute |pixel - mean_3x3|
 * where the mean is over the 3x3 neighbourhood of the
 * luminance channel. The output is the residual; the
 * host computes the global mean of the residuals.
 *
 * Edge pixels write 0 (matches Rust baseline).
 */
export const LOCAL_CONTRAST_SHADER = /* wgsl */ `
struct SpatialParams {
  pixel_count: u32,
  width: u32,
  height: u32,
  channels: u32,
};

@group(0) @binding(0) var<storage, read>       image:  array<f32>;
@group(0) @binding(1) var<uniform>             params: SpatialParams;
@group(0) @binding(2) var<storage, read_write>  out_buf: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let pixel_index = gid.x;
  if (pixel_index >= params.pixel_count) {
    return;
  }
  let width = params.width;
  let height = params.height;
  let y = pixel_index / width;
  let x = pixel_index - y * width;
  if (y == 0u || y >= height - 1u || x == 0u || x >= width - 1u) {
    out_buf[pixel_index] = 0.0;
    return;
  }
  let base = pixel_index;
  let left  = base - 1u;
  let right = base + 1u;
  let up    = base - width;
  let up_left = up - 1u;
  let up_right = up + 1u;
  let down  = base + width;
  let down_left = down - 1u;
  let down_right = down + 1u;
  // Sum of the 3x3 neighbourhood.
  let sum = image[up_left] + image[up] + image[up_right]
          + image[left] + image[base] + image[right]
          + image[down_left] + image[down] + image[down_right];
  let mean = sum / 9.0;
  let residual = abs(image[base] - mean);
  out_buf[pixel_index] = residual;
}
`;

/**
 * Map a `SpatialMode` string to the corresponding WGSL
 * shader source. The shader compiles once per
 * `WebGpuSpatialCompute` instance and is reused across
 * dispatches.
 */
export type SpatialMode = "luminance_noise" | "chromatic_noise" | "local_contrast";

export function shaderForSpatial(mode: SpatialMode): string {
  switch (mode) {
    case "luminance_noise": return LUMINANCE_NOISE_SHADER;
    case "chromatic_noise": return CHROMATIC_NOISE_SHADER;
    case "local_contrast":  return LOCAL_CONTRAST_SHADER;
  }
}

/**
 * Buffer sizes derived from the image dimensions. Used
 * by the wrapper to allocate GPU storage buffers.
 */
export function spatialOutputSize(mode: SpatialMode, pixelCount: number, workgroupSize: number): number {
  switch (mode) {
    case "luminance_noise":
    case "local_contrast":
      // Per-pixel residual.
      return pixelCount;
    case "chromatic_noise":
      // 4 floats per workgroup (3 partial sums + count).
      const workgroupCount = Math.ceil(pixelCount / workgroupSize);
      return workgroupCount * 4;
  }
}