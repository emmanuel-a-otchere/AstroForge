// WebGL shader sources for AstroForge live preview pipeline.
// All shaders operate on float textures (RGBA16F/RGBA32F) in linear space.

// ── Vertex Shader ───────────────────────────────────────────────────────────
// Full-screen quad with zoom/pan uniforms.
export const VERTEX_SHADER = `
  attribute vec2 a_position;
  varying vec2 v_uv;
  uniform vec2 u_zoom;       // zoom factor (1.0 = fit)
  uniform vec2 u_pan;        // pan offset in UV space
  uniform bool u_flipY;      // flip Y for image display

  void main() {
    v_uv = a_position * 0.5 + 0.5;
    if (u_flipY) v_uv.y = 1.0 - v_uv.y;
    v_uv = (v_uv - 0.5) / u_zoom + 0.5 + u_pan;
    gl_Position = vec4(a_position, 0.0, 1.0);
  }
`;

// ── MTF (Midtones Transfer Function) Stretch ──────────────────────────────
// Implements the PixInsight-style MTF transform:
//   M(x) = ((x - BP) / (1 - BP)) ^ (1 / gamma)
// where gamma is derived from the midtones balance point.
// Also applies white point clipping.
export const MTF_STRETCH_SHADER = `
  precision highp float;
  uniform sampler2D u_image;
  uniform float u_blackPoint;   // 0..1, pixels below this are clipped to 0
  uniform float u_midtones;     // 0..1, midtones balance (0.25 = neutral)
  uniform float u_highlights;   // 0..1, white point (1 = no clip)
  uniform float u_strength;     // 0..1, blend between original and stretched
  varying vec2 v_uv;

  float mtf(float x, float m) {
    // PixInsight MTF formula.
    //
    // PARITY: this formula is mirrored verbatim in Rust at
    // crates/astroforge-core/src/stretching.rs (midtone_transfer).
    // If you change one, change the other and update both test suites:
    //   - Rust: test_mtf_* in crates/astroforge-core/src/stretching.rs
    //   - (no JS test suite yet — Rust tests pin the contract)
    if (x <= 0.0) return 0.0;
    if (x >= 1.0) return 1.0;
    return ((m - 1.0) * x) / ((2.0 * m - 1.0) * x - m);
  }

  void main() {
    vec4 src = texture2D(u_image, v_uv);
    vec3 linear = src.rgb;

    // Apply black point clipping
    vec3 clipped = (linear - vec3(u_blackPoint)) / (1.0 - u_blackPoint);
    clipped = max(clipped, vec3(0.0));

    // Apply MTF per channel
    vec3 stretched;
    stretched.r = mtf(clipped.r, u_midtones);
    stretched.g = mtf(clipped.g, u_midtones);
    stretched.b = mtf(clipped.b, u_midtones);

    // Apply highlight clipping
    stretched = min(stretched, vec3(u_highlights));

    // Blend by strength
    vec3 result = mix(linear, stretched, u_strength);

    gl_FragColor = vec4(result, src.a);
  }
`;

// ── SCNR (Subtractive Colour Noise Reduction) "Green-be-Gone" ───────────────
// Reduces green channel dominance common in narrowband and OSC data.
// Two modes: "min" (subtract to min of R/B) and "average" (subtract to avg of R/B).
export const SCNR_SHADER = `
  precision highp float;
  uniform sampler2D u_image;
  uniform float u_strength;    // 0..1, blend
  uniform int u_method;        // 0 = min(R,B), 1 = average(R,B)
  varying vec2 v_uv;

  void main() {
    vec4 src = texture2D(u_image, v_uv);
    vec3 color = src.rgb;

    float target;
    if (u_method == 0) {
      target = min(color.r, color.b);
    } else {
      target = (color.r + color.b) * 0.5;
    }

    float greenExcess = max(color.g - target, 0.0);
    vec3 corrected = color;
    corrected.g = color.g - greenExcess * u_strength;

    gl_FragColor = vec4(corrected, src.a);
  }
`;

// ── Identity / Passthrough Shader ───────────────────────────────────────────
// Used when no effects are active (original image display).
export const IDENTITY_SHADER = `
  precision highp float;
  uniform sampler2D u_image;
  varying vec2 v_uv;

  void main() {
    gl_FragColor = texture2D(u_image, v_uv);
  }
`;

// ── Difference Map Shader ───────────────────────────────────────────────────
// Shows the absolute difference between two textures (for star separation
// verification: exact complementary layers should sum to zero difference).
export const DIFFERENCE_SHADER = `
  precision highp float;
  uniform sampler2D u_imageA;
  uniform sampler2D u_imageB;
  uniform float u_scale;      // amplification factor for visibility
  varying vec2 v_uv;

  void main() {
    vec4 a = texture2D(u_imageA, v_uv);
    vec4 b = texture2D(u_imageB, v_uv);
    vec3 diff = abs(a.rgb - b.rgb) * u_scale;
    gl_FragColor = vec4(diff, 1.0);
  }
`;

// ── Composite Shader (starless + stars) ─────────────────────────────────────
// Combines starless and stars layers with strength and colour-boost controls.
export const COMPOSITE_STARS_SHADER = `
  precision highp float;
  uniform sampler2D u_starless;
  uniform sampler2D u_stars;
  uniform float u_strength;     // 0..1, replace strength
  uniform float u_colorBoost;   // 0..2, star colour boost
  varying vec2 v_uv;

  void main() {
    vec3 starless = texture2D(u_starless, v_uv).rgb;
    vec3 stars = texture2D(u_stars, v_uv).rgb;

    // Apply colour boost to stars
    float starLum = dot(stars, vec3(0.299, 0.587, 0.114));
    vec3 boostedStars = mix(vec3(starLum), stars, u_colorBoost);

    // Blend: at strength=1, full replace; at strength=0, original starless only
    vec3 result = starless + boostedStars * u_strength;

    gl_FragColor = vec4(result, 1.0);
  }
`;

// ── Shader Program Helper ────────────────────────────────────────────────────
export interface ShaderProgram {
  program: WebGLProgram;
  uniforms: Record<string, WebGLUniformLocation | null>;
  attributes: Record<string, number>;
}

export function createShader(
  gl: WebGLRenderingContext,
  type: number,
  source: string
): WebGLShader | null {
  const shader = gl.createShader(type);
  if (!shader) return null;
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    console.error("Shader compile error:", gl.getShaderInfoLog(shader));
    gl.deleteShader(shader);
    return null;
  }
  return shader;
}

export function createProgram(
  gl: WebGLRenderingContext,
  vertexSource: string,
  fragmentSource: string,
  uniformNames: string[],
  attributeNames: string[]
): ShaderProgram | null {
  const vs = createShader(gl, gl.VERTEX_SHADER, vertexSource);
  const fs = createShader(gl, gl.FRAGMENT_SHADER, fragmentSource);
  if (!vs || !fs) return null;

  const program = gl.createProgram();
  if (!program) return null;
  gl.attachShader(program, vs);
  gl.attachShader(program, fs);
  gl.linkProgram(program);

  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    console.error("Program link error:", gl.getProgramInfoLog(program));
    gl.deleteProgram(program);
    return null;
  }

  gl.deleteShader(vs);
  gl.deleteShader(fs);

  const uniforms: Record<string, WebGLUniformLocation | null> = {};
  for (const name of uniformNames) {
    uniforms[name] = gl.getUniformLocation(program, name);
  }

  const attributes: Record<string, number> = {};
  for (const name of attributeNames) {
    attributes[name] = gl.getAttribLocation(program, name);
  }

  return { program, uniforms, attributes };
}

// ── Full-screen quad geometry ───────────────────────────────────────────────
export function createQuadBuffer(gl: WebGLRenderingContext): WebGLBuffer {
  const buffer = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
  gl.bufferData(
    gl.ARRAY_BUFFER,
    new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]),
    gl.STATIC_DRAW
  );
  return buffer;
}

// ── Texture helpers ──────────────────────────────────────────────────────────
export function createFloatTexture(
  gl: WebGLRenderingContext,
  width: number,
  height: number,
  data: Float32Array | null = null
): WebGLTexture | null {
  const texture = gl.createTexture();
  if (!texture) return null;
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);

  // Use RGBA16F for float textures if available, otherwise fall back
  const ext = gl.getExtension("OES_texture_float_linear") || gl.getExtension("OES_texture_half_float");
  const internalFormat = ext ? gl.RGBA : gl.RGBA;
  const format = gl.RGBA;
  const type = ext ? gl.FLOAT : gl.UNSIGNED_BYTE;

  gl.texImage2D(
    gl.TEXTURE_2D,
    0,
    internalFormat,
    width,
    height,
    0,
    format,
    type,
    data
  );

  return texture;
}

export function uploadToTexture(
  gl: WebGLRenderingContext,
  texture: WebGLTexture,
  width: number,
  height: number,
  data: Float32Array
): void {
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, width, height, gl.RGBA, gl.FLOAT, data);
}

export function createUint8Texture(
  gl: WebGLRenderingContext,
  width: number,
  height: number,
  data: Uint8Array | null = null
): WebGLTexture | null {
  const texture = gl.createTexture();
  if (!texture) return null;
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, width, height, 0, gl.RGBA, gl.UNSIGNED_BYTE, data);
  return texture;
}
