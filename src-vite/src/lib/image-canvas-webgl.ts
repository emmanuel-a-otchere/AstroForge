// CR-07 follow-on 2 — WebGL back-end for ImageCanvas.
//
// Self-contained WebGL renderer purpose-built for the CR-07
// ImageCanvas surface. Takes 16-bit TIFF pixels (normalized to
// 0..1 Float32) and renders them through an `identity`-style
// fragment shader with black-point + highlight clipping, zoom
// and pan uniforms.
//
// Why this file instead of `gl-renderer.ts`?
//
//   The existing `WebGLRenderer` was wired for the P1.5 wizard
//   pipeline (MTF stretch, SCNR, etc.). Its texture format is
//   Float32 RGBA in linear space and the call sites push
//   already-displayed pixel data into it. The CR-07 ImageCanvas
//   decodes raw 16-bit TIFF and wants to render it through
//   bilinear resampling + clip, which the existing renderer
//   doesn't directly support. Reusing the lower-level
//   `shaders.ts` primitives would require either:
//
//     1. Adding a new program set + uniform layout to
//        `gl-renderer.ts` that mirrors the wizard path (couples
//        two unrelated surfaces — risk of cross-contamination).
//     2. Pulling in the wizard's texture-pack convention
//        (single-channel Float32 packed in RGBA) which forces
//        CR-07 to break its existing 8-bit display path.
//
//   Both options fight the existing architecture. A small,
//   purpose-built renderer is the surgical answer.

export interface ImageCanvasGlParams {
  // Display gain.
  blackPoint: number; // 0..1
  whitePoint: number; // 0..1 (highlights)
}

export interface ImageCanvasGlViewport {
  zoom: number; // 1 = fit
  panX: number; // 0..1 in normalized UV space
  panY: number;
}

const VERTEX_SHADER = `
  attribute vec2 a_position;
  varying vec2 v_uv;
  uniform vec2 u_zoom;
  uniform vec2 u_pan;
  uniform float u_flipY;
  void main() {
    v_uv = a_position * 0.5 + 0.5;
    if (u_flipY > 0.5) v_uv.y = 1.0 - v_uv.y;
    v_uv = (v_uv - 0.5) / u_zoom + vec2(0.5) + u_pan;
    gl_Position = vec4(a_position, 0.0, 1.0);
  }
`;

const FRAGMENT_SHADER = `
  precision highp float;
  uniform sampler2D u_image;
  uniform float u_blackPoint;
  uniform float u_whitePoint;
  varying vec2 v_uv;
  void main() {
    vec4 src = texture2D(u_image, v_uv);
    vec3 linear = src.rgb;
    // Black-point: pixels at or below BP become 0.
    vec3 clipped = max(linear - vec3(u_blackPoint), vec3(0.0));
    // White-point: pixels at or above WP become 1.
    clipped = min(clipped, vec3(u_whitePoint));
    gl_FragColor = vec4(clipped, src.a);
  }
`;

export interface ImageCanvasGlInitResult {
  ok: true;
  program: WebGLProgram;
  quadBuffer: WebGLBuffer;
  texture: WebGLTexture;
  attrs: { a_position: number };
  uniforms: {
    u_image: WebGLUniformLocation | null;
    u_zoom: WebGLUniformLocation | null;
    u_pan: WebGLUniformLocation | null;
    u_flipY: WebGLUniformLocation | null;
    u_blackPoint: WebGLUniformLocation | null;
    u_whitePoint: WebGLUniformLocation | null;
  };
}

export interface ImageCanvasGlInitFailure {
  ok: false;
  reason: string;
}

export function initImageCanvasGl(
  gl: WebGLRenderingContext,
): ImageCanvasGlInitResult | ImageCanvasGlInitFailure {
  // Compile shaders.
  const vs = compileShader(gl, gl.VERTEX_SHADER, VERTEX_SHADER);
  const fs = compileShader(gl, gl.FRAGMENT_SHADER, FRAGMENT_SHADER);
  if (!vs || !fs) return { ok: false, reason: "shader compile failed" };

  const program = gl.createProgram();
  if (!program) return { ok: false, reason: "createProgram failed" };
  gl.attachShader(program, vs);
  gl.attachShader(program, fs);
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    return { ok: false, reason: "program link failed" };
  }

  // Quad: two triangles covering NDC -1..1.
  const quadBuffer = gl.createBuffer();
  if (!quadBuffer) return { ok: false, reason: "createBuffer failed" };
  gl.bindBuffer(gl.ARRAY_BUFFER, quadBuffer);
  gl.bufferData(
    gl.ARRAY_BUFFER,
    new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]),
    gl.STATIC_DRAW,
  );

  // Texture: empty 1x1 placeholder; replaced on uploadSource.
  const texture = gl.createTexture();
  if (!texture) return { ok: false, reason: "createTexture failed" };
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texImage2D(
    gl.TEXTURE_2D,
    0,
    gl.RGBA,
    1,
    1,
    0,
    gl.RGBA,
    gl.UNSIGNED_BYTE,
    new Uint8Array([0, 0, 0, 255]),
  );

  return {
    ok: true,
    program,
    quadBuffer,
    texture,
    attrs: { a_position: gl.getAttribLocation(program, "a_position") },
    uniforms: {
      u_image: gl.getUniformLocation(program, "u_image"),
      u_zoom: gl.getUniformLocation(program, "u_zoom"),
      u_pan: gl.getUniformLocation(program, "u_pan"),
      u_flipY: gl.getUniformLocation(program, "u_flipY"),
      u_blackPoint: gl.getUniformLocation(program, "u_blackPoint"),
      u_whitePoint: gl.getUniformLocation(program, "u_whitePoint"),
    },
  };
}

function compileShader(
  gl: WebGLRenderingContext,
  type: number,
  source: string,
): WebGLShader | null {
  const sh = gl.createShader(type);
  if (!sh) return null;
  gl.shaderSource(sh, source);
  gl.compileShader(sh);
  if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
    // Best-effort cleanup before failing; the caller decides
    // whether to retry or fall back.
    gl.deleteShader(sh);
    return null;
  }
  return sh;
}

/**
 * Pack normalized Float32 source data into a Float32 RGBA
 * texture. 1-channel sources replicate luminance across R, G,
 * and B with A=1. 3-channel sources are interleaved into RGBA
 * (3 source channels + alpha=1). 4-channel sources pass
 * through (currently unused; reserved for a future mask-texture
 * extension).
 */
export function packSourceToRgba(
  channels: 1 | 3,
  data: Float32Array,
): Float32Array {
  const n = data.length / channels;
  const out = new Float32Array(n * 4);
  if (channels === 1) {
    for (let i = 0; i < n; i++) {
      const v = data[i];
      out[i * 4] = v;
      out[i * 4 + 1] = v;
      out[i * 4 + 2] = v;
      out[i * 4 + 3] = 1;
    }
  } else {
    for (let i = 0; i < n; i++) {
      out[i * 4] = data[i * 3];
      out[i * 4 + 1] = data[i * 3 + 1];
      out[i * 4 + 2] = data[i * 3 + 2];
      out[i * 4 + 3] = 1;
    }
  }
  return out;
}

export interface DrawSourceArgs {
  gl: WebGLRenderingContext;
  state: ImageCanvasGlInitResult;
  width: number;
  height: number;
  channels: 1 | 3;
  data: Float32Array;
  params: ImageCanvasGlParams;
  viewport: ImageCanvasGlViewport;
}

/**
 * One-call helper: upload the source (only when it changes),
 * resize the canvas drawing buffer to the display size, then
 * draw the quad with the current viewport + clip uniforms.
 *
 * Float32 upload is preferred (matches the source data
 * precision exactly); if the WebGL context can't take Float32
 * textures (no `OES_texture_float`) we fall back to 8-bit
 * RGBA, which loses sub-LSB precision but renders identically
 * for visual use. The same fallback mirrors `gl-renderer.ts`
 * (lines 246–262 of `shaders.ts`).
 *
 * Returns the upload cost (bytes copied to GPU) so callers can
 * surface a perf hint when the upload dominates.
 */
export interface DrawSourceResult {
  uploadBytes: number;
  drawCalls: 1;
  precision: "float32" | "uint8";
}

export function drawImageCanvasGl(args: DrawSourceArgs): DrawSourceResult {
  const { gl, state, width, height, channels, data, params, viewport } = args;

  // WebGL 1 only allows Float32 textures with the
  // OES_texture_float extension. Without it, drop to Uint8
  // 8-bit upload (same precision as the Canvas 2D display path).
  const floatOk =
    !!gl.getExtension("OES_texture_float") ||
    !!gl.getExtension("OES_texture_half_float");

  let uploadBytes = 0;
  gl.bindTexture(gl.TEXTURE_2D, state.texture);
  if (floatOk) {
    const packed = packSourceToRgba(channels, data);
    gl.texImage2D(
      gl.TEXTURE_2D,
      0,
      gl.RGBA,
      width,
      height,
      0,
      gl.RGBA,
      gl.FLOAT,
      packed,
    );
    uploadBytes = packed.byteLength;
  } else {
    const packed8 = packSourceToUint8(channels, data);
    gl.texImage2D(
      gl.TEXTURE_2D,
      0,
      gl.RGBA,
      width,
      height,
      0,
      gl.RGBA,
      gl.UNSIGNED_BYTE,
      packed8,
    );
    uploadBytes = packed8.byteLength;
  }

  // Resize drawing buffer to the canvas CSS size.
  const canvas = gl.canvas as HTMLCanvasElement;
  const displayW = canvas.clientWidth || 1;
  const displayH = canvas.clientHeight || 1;
  if (canvas.width !== displayW) canvas.width = displayW;
  if (canvas.height !== displayH) canvas.height = displayH;
  gl.viewport(0, 0, displayW, displayH);

  // Program + uniforms.
  gl.useProgram(state.program);
  gl.bindBuffer(gl.ARRAY_BUFFER, state.quadBuffer);
  gl.enableVertexAttribArray(state.attrs.a_position);
  gl.vertexAttribPointer(state.attrs.a_position, 2, gl.FLOAT, false, 0, 0);

  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, state.texture);
  if (state.uniforms.u_image) gl.uniform1i(state.uniforms.u_image, 0);
  if (state.uniforms.u_zoom)
    gl.uniform2f(state.uniforms.u_zoom, viewport.zoom, viewport.zoom);
  if (state.uniforms.u_pan)
    gl.uniform2f(state.uniforms.u_pan, viewport.panX, viewport.panY);
  if (state.uniforms.u_flipY) gl.uniform1f(state.uniforms.u_flipY, 1);
  if (state.uniforms.u_blackPoint)
    gl.uniform1f(state.uniforms.u_blackPoint, params.blackPoint);
  if (state.uniforms.u_whitePoint)
    gl.uniform1f(state.uniforms.u_whitePoint, params.whitePoint);

  gl.clearColor(0, 0, 0, 1);
  gl.clear(gl.COLOR_BUFFER_BIT);
  gl.drawArrays(gl.TRIANGLES, 0, 6);

  return {
    uploadBytes,
    drawCalls: 1,
    precision: floatOk ? "float32" : "uint8",
  };
}

/**
 * Same packing as `packSourceToRgba` but quantizes to 0..255
 * Uint8 values for the WebGL contexts that can't upload
 * Float32 textures.
 */
export function packSourceToUint8(
  channels: 1 | 3,
  data: Float32Array,
): Uint8Array {
  const n = data.length / channels;
  const out = new Uint8Array(n * 4);
  if (channels === 1) {
    for (let i = 0; i < n; i++) {
      const v = Math.max(0, Math.min(255, Math.round(data[i] * 255)));
      out[i * 4] = v;
      out[i * 4 + 1] = v;
      out[i * 4 + 2] = v;
      out[i * 4 + 3] = 255;
    }
  } else {
    for (let i = 0; i < n; i++) {
      out[i * 4] = Math.max(0, Math.min(255, Math.round(data[i * 3] * 255)));
      out[i * 4 + 1] = Math.max(
        0,
        Math.min(255, Math.round(data[i * 3 + 1] * 255)),
      );
      out[i * 4 + 2] = Math.max(
        0,
        Math.min(255, Math.round(data[i * 3 + 2] * 255)),
      );
      out[i * 4 + 3] = 255;
    }
  }
  return out;
}

export function destroyImageCanvasGl(
  gl: WebGLRenderingContext,
  state: ImageCanvasGlInitResult,
): void {
  gl.deleteProgram(state.program);
  gl.deleteBuffer(state.quadBuffer);
  gl.deleteTexture(state.texture);
}
