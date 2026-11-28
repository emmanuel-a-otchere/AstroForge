// CR-07 §29.3a: WGSL compute shaders for `compute_diff`.
//
// Each shader mirrors one variant of the Rust
// `compute_diff` function in
// `crates/astroforge-core/src/difference.rs`. The shader
// output is byte-equivalent to the Rust output on the
// per-pixel modes (Absolute / Signed / Amplified) and
// on the Structural mode for non-edge pixels.
//
// Pixel format: each input/output buffer holds RGBA8
// data, with 4 bytes per pixel stored in interleaved
// order (R, G, B, A). The alpha channel is always
// preserved as 255 in the output (matching the Rust
// baseline, which writes 255 to every alpha byte).
//
// The compute kernels operate on one pixel per
// invocation. The dispatch is
// `@workgroup_count(ceil(pixel_count / 64), 1, 1)`
// with `@workgroup_size(64)`.

/**
 * Compute shader for `DiffKind::Absolute`.
 * Output channel c = `|a[c] - b[c]|` for c in {R, G, B}.
 * Output alpha = 255.
 */
export const ABSOLUTE_SHADER = /* wgsl */ `
struct DiffParams {
  pixel_count: u32,
  // gain is unused for Absolute; included to keep all
  // shaders using the same uniform buffer layout.
  gain: f32,
  width: u32,
  pad0: u32,
};

@group(0) @binding(0) var<storage, read>       a_in:  array<u32>;
@group(0) @binding(1) var<storage, read>       b_in:  array<u32>;
@group(0) @binding(2) var<uniform>             params: DiffParams;
@group(0) @binding(3) var<storage, read_write>  out_buf: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let pixel_index = gid.x;
  if (pixel_index >= params.pixel_count) {
    return;
  }
  let byte_base = pixel_index * 4u;
  let a_packed = a_in[pixel_index];
  let b_packed = b_in[pixel_index];
  let ar = (a_packed >> 0u)  & 0xffu;
  let ag = (a_packed >> 8u)  & 0xffu;
  let ab = (a_packed >> 16u) & 0xffu;
  let aa = (a_packed >> 24u) & 0xffu;
  let br = (b_packed >> 0u)  & 0xffu;
  let bg = (b_packed >> 8u)  & 0xffu;
  let bb_ = (b_packed >> 16u) & 0xffu;
  let dr = abs_i32(i32(ar) - i32(br));
  let dg = abs_i32(i32(ag) - i32(bg));
  let db = abs_i32(i32(ab) - i32(bb_));
  let out_packed = dr | (dg << 8u) | (db << 16u) | (255u << 24u);
  out_buf[pixel_index] = out_packed;
}

fn abs_i32(x: i32) -> u32 {
  // |x| expressed as u32. For x >= 0, return u32(x).
  // For x < 0, return u32(-x). Equivalent to the
  // Rust '(a as i16 - b as i16).unsigned_abs()'.
  if (x >= 0) {
    return u32(x);
  } else {
    return u32(-x);
  }
}
`;

/**
 * Compute shader for `DiffKind::Signed`.
 * Output channel c = `clamp((a[c] - b[c]) + 128, 0, 255)`
 * for c in {R, G, B}. Output alpha = 255.
 *
 * The +128 offset maps a difference of 0 to mid-grey,
 * a-b > 0 to brighter, a-b < 0 to darker.
 */
export const SIGNED_SHADER = /* wgsl */ `
struct DiffParams {
  pixel_count: u32,
  gain: f32,
  width: u32,
  pad0: u32,
};

@group(0) @binding(0) var<storage, read>       a_in:  array<u32>;
@group(0) @binding(1) var<storage, read>       b_in:  array<u32>;
@group(0) @binding(2) var<uniform>             params: DiffParams;
@group(0) @binding(3) var<storage, read_write>  out_buf: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let pixel_index = gid.x;
  if (pixel_index >= params.pixel_count) {
    return;
  }
  let a_packed = a_in[pixel_index];
  let b_packed = b_in[pixel_index];
  let ar = (a_packed >> 0u)  & 0xffu;
  let ag = (a_packed >> 8u)  & 0xffu;
  let ab = (a_packed >> 16u) & 0xffu;
  let br = (b_packed >> 0u)  & 0xffu;
  let bg = (b_packed >> 8u)  & 0xffu;
  let bb_ = (b_packed >> 16u) & 0xffu;
  let dr = clamp_i32(i32(ar) - i32(br) + 128);
  let dg = clamp_i32(i32(ag) - i32(bg) + 128);
  let db = clamp_i32(i32(ab) - i32(bb_) + 128);
  let out_packed = dr | (dg << 8u) | (db << 16u) | (255u << 24u);
  out_buf[pixel_index] = out_packed;
}

fn clamp_i32(x: i32) -> u32 {
  // 'clamp(x, 0, 255)' expressed as u32.
  if (x < 0) {
    return 0u;
  } else if (x > 255) {
    return 255u;
  } else {
    return u32(x);
  }
}
`;

/**
 * Compute shader for `DiffKind::Amplified`.
 * Output channel c = `clamp(|a[c] - b[c]| * gain, 0, 255)`
 * for c in {R, G, B}. Output alpha = 255.
 *
 * `gain` is passed via uniform. The Rust baseline
 * treats negative / non-finite gain as 1.0; we mirror
 * that with a CPU-side clamp before the uniform write.
 */
export const AMPLIFIED_SHADER = /* wgsl */ `
struct DiffParams {
  pixel_count: u32,
  gain: f32,
  width: u32,
  pad0: u32,
};

@group(0) @binding(0) var<storage, read>       a_in:  array<u32>;
@group(0) @binding(1) var<storage, read>       b_in:  array<u32>;
@group(0) @binding(2) var<uniform>             params: DiffParams;
@group(0) @binding(3) var<storage, read_write>  out_buf: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let pixel_index = gid.x;
  if (pixel_index >= params.pixel_count) {
    return;
  }
  let a_packed = a_in[pixel_index];
  let b_packed = b_in[pixel_index];
  let ar = (a_packed >> 0u)  & 0xffu;
  let ag = (a_packed >> 8u)  & 0xffu;
  let ab = (a_packed >> 16u) & 0xffu;
  let br = (b_packed >> 0u)  & 0xffu;
  let bg = (b_packed >> 8u)  & 0xffu;
  let bb_ = (b_packed >> 16u) & 0xffu;
  let dr = clamp_f32(f32(abs_i32(i32(ar) - i32(br))) * params.gain);
  let dg = clamp_f32(f32(abs_i32(i32(ag) - i32(bg))) * params.gain);
  let db = clamp_f32(f32(abs_i32(i32(ab) - i32(bb_))) * params.gain);
  let out_packed = dr | (dg << 8u) | (db << 16u) | (255u << 24u);
  out_buf[pixel_index] = out_packed;
}

fn abs_i32(x: i32) -> u32 {
  if (x >= 0) {
    return u32(x);
  } else {
    return u32(-x);
  }
}

fn clamp_f32(x: f32) -> u32 {
  let r = floor(clamp(x, 0.0, 255.0));
  return u32(r);
}
`;

/**
 * Compute shader for `DiffKind::Structural`.
 *
 * The Rust baseline computes a per-pixel edge magnitude
 * for both a and b (Sobel-style: max of |pixel - left|
 * and |pixel - up| per RGB channel), then writes
 * `max(|ea - eb|)` summed across RGB into the output
 * pixel.
 *
 * The WGSL version operates on 2D pixel coordinates
 * derived from the pixel_index. For non-edge pixels
 * (first row / first / last column), the shader writes
 * 0 (matching the Rust baseline, which skips interior-
 * only writes and relies on the `vec![0u8; n]`
 * allocation).
 *
 * `width` is the image width in pixels. Pixels with
 * `x == 0`, `x == width - 1`, or `y == 0` are edge
 * pixels (write 0). Otherwise, compute the structural
 * diff for the interior pixel.
 */
export const STRUCTURAL_SHADER = /* wgsl */ `
struct DiffParams {
  pixel_count: u32,
  gain: f32,
  width: u32,
  pad0: u32,
};

@group(0) @binding(0) var<storage, read>       a_in:  array<u32>;
@group(0) @binding(1) var<storage, read>       b_in:  array<u32>;
@group(0) @binding(2) var<uniform>             params: DiffParams;
@group(0) @binding(3) var<storage, read_write>  out_buf: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let pixel_index = gid.x;
  if (pixel_index >= params.pixel_count) {
    return;
  }
  let width = params.width;
  let y = pixel_index / width;
  let x = pixel_index - y * width;
  // Edge pixels (first row, first column, last column)
  // get zero output. Matches Rust baseline.
  if (y == 0u || x == 0u || x == width - 1u) {
    out_buf[pixel_index] = 0xff000000u;
    return;
  }
  let cur  = pixel_index;
  let left = pixel_index - 1u;
  let up   = pixel_index - width;
  let a_cur = unpack_rgba(a_in[cur]);
  let a_left = unpack_rgba(a_in[left]);
  let a_up   = unpack_rgba(a_in[up]);
  let b_cur = unpack_rgba(b_in[cur]);
  let b_left = unpack_rgba(b_in[left]);
  let b_up   = unpack_rgba(b_in[up]);
  // Edge magnitude per channel = max(|cur - left|, |cur - up|).
  let ea_r = max(abs_u8(a_cur.r, a_left.r), abs_u8(a_cur.r, a_up.r));
  let ea_g = max(abs_u8(a_cur.g, a_left.g), abs_u8(a_cur.g, a_up.g));
  let ea_b = max(abs_u8(a_cur.b, a_left.b), abs_u8(a_cur.b, a_up.b));
  let eb_r = max(abs_u8(b_cur.r, b_left.r), abs_u8(b_cur.r, b_up.r));
  let eb_g = max(abs_u8(b_cur.g, b_left.g), abs_u8(b_cur.g, b_up.g));
  let eb_b = max(abs_u8(b_cur.b, b_left.b), abs_u8(b_cur.b, b_up.b));
  // Per-channel structural diff = |ea - eb|.
  let dr = abs_u8(ea_r, eb_r);
  let dg = abs_u8(ea_g, eb_g);
  let db = abs_u8(ea_b, eb_b);
  // Rust baseline sums dr + dg + db into R, then writes
  // the same sum into G and B (greyscale edge map).
  let sum = dr + dg + db;
  let s = clamp_u8(sum);
  let out_packed = s | (s << 8u) | (s << 16u) | (255u << 24u);
  out_buf[pixel_index] = out_packed;
}

struct Rgba {
  r: u32,
  g: u32,
  b: u32,
  a: u32,
}

fn unpack_rgba(packed: u32) -> Rgba {
  return Rgba(
    (packed >> 0u)  & 0xffu,
    (packed >> 8u)  & 0xffu,
    (packed >> 16u) & 0xffu,
    (packed >> 24u) & 0xffu,
  );
}

fn abs_u8(x: u32, y: u32) -> u32 {
  if (x > y) { return x - y; } else { return y - x; }
}

fn clamp_u8(x: u32) -> u32 {
  if (x > 255u) { return 255u; } else { return x; }
}
`;

/**
 * Map `DiffKind` (a string mirror of the Rust enum) to
 * the corresponding WGSL shader source. The shader
 * compiles once per `WebGpuDiffCompute` instance and is
 * reused across dispatches.
 */
export function shaderFor(mode: "absolute" | "signed" | "amplified" | "structural"): string {
  switch (mode) {
    case "absolute":   return ABSOLUTE_SHADER;
    case "signed":     return SIGNED_SHADER;
    case "amplified":  return AMPLIFIED_SHADER;
    case "structural": return STRUCTURAL_SHADER;
  }
}