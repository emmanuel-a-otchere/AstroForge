import {
  VERTEX_SHADER,
  MTF_STRETCH_SHADER,
  SCNR_SHADER,
  IDENTITY_SHADER,
  DIFFERENCE_SHADER,
  COMPOSITE_STARS_SHADER,
  createProgram,
  createQuadBuffer,
  createFloatTexture,
  uploadToTexture,
  createUint8Texture,
  type ShaderProgram,
} from "./shaders";

export interface PreviewParams {
  blackPoint: number;
  midtones: number;
  highlights: number;
  strength: number;
  scnrStrength: number;
  scnrMethod: number; // 0 = min, 1 = average
  // P1.5-M4-T4 (#162) — Pure Expert UI per-stage params.
  // These fields are exposed in `ParameterSidebar.svelte` for the
  // denoise / colour_calibration / sharpen_deconvolution stage types.
  // The corresponding shader uniforms are not yet wired into the GL
  // renderer (those shaders ship with P1.5-M6). Storing them here
  // means the sidebar can edit them today and they will activate
  // automatically when the matching shader path lands.
  denoiseStrength?: number;
  denoiseThreshold?: number;
  colorSaturation?: number;
  sharpenStrength?: number;
  sharpenRadius?: number;
}

export interface ViewportState {
  zoom: number;
  panX: number;
  panY: number;
}

export class WebGLRenderer {
  private gl: WebGLRenderingContext;
  private quadBuffer: WebGLBuffer;
  private programs: Record<string, ShaderProgram> = {};
  private imageTexture: WebGLTexture | null = null;
  private textureB: WebGLTexture | null = null;
  private imageWidth = 0;
  private imageHeight = 0;
  private canvasWidth = 0;
  private canvasHeight = 0;
  /** True when the renderer is drawing into a half-resolution buffer for
   *  cheap interaction-time previews. See `setReducedResolution` and the
   *  P1.5-M2-T8 / #152 spec. */
  private reducedRes = false;
  /** Display size (the CSS / HTML attribute size the user sees).
   *  Distinct from `canvasWidth/Height`, which are the GL drawing-buffer
   *  size and may be downscaled during reduced-resolution interaction. */
  private displayWidth = 0;
  private displayHeight = 0;
  private viewport: ViewportState = { zoom: 1, panX: 0, panY: 0 };
  private params: PreviewParams = {
    blackPoint: 0,
    midtones: 0.25,
    highlights: 1,
    strength: 0,
    scnrStrength: 0,
    scnrMethod: 0,
  };
  private needsRender = true;
  private renderPending = false;
  private debounceTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(canvas: HTMLCanvasElement) {
    const gl = canvas.getContext("webgl", {
      premultipliedAlpha: false,
      preserveDrawingBuffer: false,
      antialias: false,
    });
    if (!gl) throw new Error("WebGL not supported");
    this.gl = gl as WebGLRenderingContext;

    this.quadBuffer = createQuadBuffer(this.gl);
    this.initPrograms();
  }

  private initPrograms(): void {
    this.programs.identity = createProgram(
      this.gl, VERTEX_SHADER, IDENTITY_SHADER,
      ["u_image", "u_zoom", "u_pan", "u_flipY"],
      ["a_position"]
    )!;

    this.programs.mtf = createProgram(
      this.gl, VERTEX_SHADER, MTF_STRETCH_SHADER,
      ["u_image", "u_blackPoint", "u_midtones", "u_highlights", "u_strength", "u_zoom", "u_pan", "u_flipY"],
      ["a_position"]
    )!;

    this.programs.scnr = createProgram(
      this.gl, VERTEX_SHADER, SCNR_SHADER,
      ["u_image", "u_strength", "u_method", "u_zoom", "u_pan", "u_flipY"],
      ["a_position"]
    )!;

    this.programs.difference = createProgram(
      this.gl, VERTEX_SHADER, DIFFERENCE_SHADER,
      ["u_imageA", "u_imageB", "u_scale", "u_zoom", "u_pan", "u_flipY"],
      ["a_position"]
    )!;

    this.programs.composite = createProgram(
      this.gl, VERTEX_SHADER, COMPOSITE_STARS_SHADER,
      ["u_starless", "u_stars", "u_strength", "u_colorBoost", "u_zoom", "u_pan", "u_flipY"],
      ["a_position"]
    )!;
  }

  setImageData(width: number, height: number, data: Float32Array): void {
    this.imageWidth = width;
    this.imageHeight = height;
    if (this.imageTexture) {
      this.gl.deleteTexture(this.imageTexture);
    }
    this.imageTexture = createFloatTexture(this.gl, width, height, data);
    this.needsRender = true;
  }

  setImageFromCanvas(source: HTMLCanvasElement): void {
    this.imageWidth = source.width;
    this.imageHeight = source.height;
    if (this.imageTexture) {
      this.gl.deleteTexture(this.imageTexture);
    }
    this.gl.bindTexture(this.gl.TEXTURE_2D, null);
    // Create from canvas element
    const texture = this.gl.createTexture();
    if (!texture) return;
    this.gl.bindTexture(this.gl.TEXTURE_2D, texture);
    this.gl.texParameteri(this.gl.TEXTURE_2D, this.gl.TEXTURE_WRAP_S, this.gl.CLAMP_TO_EDGE);
    this.gl.texParameteri(this.gl.TEXTURE_2D, this.gl.TEXTURE_WRAP_T, this.gl.CLAMP_TO_EDGE);
    this.gl.texParameteri(this.gl.TEXTURE_2D, this.gl.TEXTURE_MIN_FILTER, this.gl.LINEAR);
    this.gl.texParameteri(this.gl.TEXTURE_2D, this.gl.TEXTURE_MAG_FILTER, this.gl.LINEAR);
    this.gl.texImage2D(this.gl.TEXTURE_2D, 0, this.gl.RGBA, this.gl.RGBA, this.gl.UNSIGNED_BYTE, source);
    this.imageTexture = texture;
    this.needsRender = true;
  }

  setImageFromImageData(imageData: ImageData): void {
    this.imageWidth = imageData.width;
    this.imageHeight = imageData.height;
    if (this.imageTexture) {
      this.gl.deleteTexture(this.imageTexture);
    }
    const tex = createUint8Texture(this.gl, imageData.width, imageData.height, new Uint8Array(imageData.data.buffer));
    this.imageTexture = tex;
    this.needsRender = true;
  }

  setSecondImageData(width: number, height: number, data: Float32Array): void {
    if (this.textureB) {
      this.gl.deleteTexture(this.textureB);
    }
    this.textureB = createFloatTexture(this.gl, width, height, data);
    this.needsRender = true;
  }

  setParams(params: Partial<PreviewParams>): void {
    this.params = { ...this.params, ...params };
    this.needsRender = true;
  }

  setViewport(viewport: Partial<ViewportState>): void {
    this.viewport = { ...this.viewport, ...viewport };
    this.needsRender = true;
  }

  resize(width: number, height: number): void {
    this.displayWidth = width;
    this.displayHeight = height;
    // Drawing-buffer size may be downscaled during reduced-res interaction;
    // the CSS pixel size stays at the requested display size so the canvas
    // continues to fill the container.
    this.applyBufferSize();
    this.needsRender = true;
  }

  /**
   * Toggle the reduced-resolution interaction mode (P1.5-M2-T8 / #152).
   *
   * When `enabled` is true, the GL drawing buffer drops to half the display
   * dimensions; the canvas's CSS size stays the same so the layout is
   * unaffected, only the render is cheaper. The browser scales the
   * lower-res buffer up to the display size, which is visually fine for
   * the pan/zoom interaction phase — the user is moving the view, not
   * inspecting fine detail.
   *
   * Call `setReducedResolution(false)` (or wait for the debouncer to fire)
   * before the final commit-quality render.
   */
  setReducedResolution(enabled: boolean): void {
    if (this.reducedRes === enabled) return;
    this.reducedRes = enabled;
    this.applyBufferSize();
    this.needsRender = true;
  }

  isReducedResolution(): boolean {
    return this.reducedRes;
  }

  private applyBufferSize(): void {
    const w = this.reducedRes
      ? Math.max(1, Math.floor(this.displayWidth / 2))
      : this.displayWidth;
    const h = this.reducedRes
      ? Math.max(1, Math.floor(this.displayHeight / 2))
      : this.displayHeight;
    this.canvasWidth = w;
    this.canvasHeight = h;
    this.gl.canvas.width = w;
    this.gl.canvas.height = h;
    this.gl.viewport(0, 0, w, h);
  }

  refit(): void {
    if (!this.imageWidth || !this.canvasWidth) return;
    const scaleX = this.canvasWidth / this.imageWidth;
    const scaleY = this.canvasHeight / this.imageHeight;
    this.viewport.zoom = Math.min(scaleX, scaleY);
    this.viewport.panX = 0;
    this.viewport.panY = 0;
    this.needsRender = true;
  }

  private bindQuad(program: ShaderProgram): void {
    this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.quadBuffer);
    this.gl.enableVertexAttribArray(program.attributes["a_position"]);
    this.gl.vertexAttribPointer(program.attributes["a_position"], 2, this.gl.FLOAT, false, 0, 0);
  }

  private setViewportUniforms(program: ShaderProgram): void {
    const zoomLoc = program.uniforms["u_zoom"];
    const panLoc = program.uniforms["u_pan"];
    const flipLoc = program.uniforms["u_flipY"];
    if (zoomLoc) this.gl.uniform2f(zoomLoc, this.viewport.zoom, this.viewport.zoom);
    if (panLoc) this.gl.uniform2f(panLoc, this.viewport.panX, this.viewport.panY);
    if (flipLoc) this.gl.uniform1i(flipLoc, 1);
  }

  render(mode: "identity" | "mtf" | "scnr" | "difference" | "composite" = "mtf"): void {
    if (!this.imageTexture || !this.needsRender) return;
    this.needsRender = false;

    const program = this.programs[mode];
    if (!program) return;

    this.gl.useProgram(program.program);
    this.bindQuad(program);
    this.setViewportUniforms(program);

    this.gl.activeTexture(this.gl.TEXTURE0);
    this.gl.bindTexture(this.gl.TEXTURE_2D, this.imageTexture);
    this.gl.uniform1i(program.uniforms["u_image"], 0);

    if (mode === "mtf") {
      this.gl.uniform1f(program.uniforms["u_blackPoint"], this.params.blackPoint);
      this.gl.uniform1f(program.uniforms["u_midtones"], this.params.midtones);
      this.gl.uniform1f(program.uniforms["u_highlights"], this.params.highlights);
      this.gl.uniform1f(program.uniforms["u_strength"], this.params.strength);
    } else if (mode === "scnr") {
      this.gl.uniform1f(program.uniforms["u_strength"], this.params.scnrStrength);
      this.gl.uniform1i(program.uniforms["u_method"], this.params.scnrMethod);
    } else if (mode === "difference") {
      if (this.textureB) {
        this.gl.activeTexture(this.gl.TEXTURE1);
        this.gl.bindTexture(this.gl.TEXTURE_2D, this.textureB);
        this.gl.uniform1i(program.uniforms["u_imageB"], 1);
      }
      this.gl.uniform1f(program.uniforms["u_scale"], 1.0);
    } else if (mode === "composite") {
      if (this.textureB) {
        this.gl.activeTexture(this.gl.TEXTURE1);
        this.gl.bindTexture(this.gl.TEXTURE_2D, this.textureB);
        this.gl.uniform1i(program.uniforms["u_stars"], 1);
      }
      this.gl.uniform1f(program.uniforms["u_strength"], this.params.strength);
      this.gl.uniform1f(program.uniforms["u_colorBoost"], 1.0);
    }

    this.gl.clearColor(0, 0, 0, 1);
    this.gl.clear(this.gl.COLOR_BUFFER_BIT);
    this.gl.drawArrays(this.gl.TRIANGLES, 0, 6);
  }

  requestDebouncedRender(mode: "identity" | "mtf" | "scnr" | "difference" | "composite" = "mtf", delayMs = 150): void {
    this.needsRender = true;
    if (this.debounceTimer) clearTimeout(this.debounceTimer);
    this.debounceTimer = setTimeout(() => {
      this.render(mode);
    }, delayMs);
  }

  isDestroyed = false;
  destroy(): void {
    this.isDestroyed = true;
    if (this.debounceTimer) clearTimeout(this.debounceTimer);
    if (this.imageTexture) this.gl.deleteTexture(this.imageTexture);
    if (this.textureB) this.gl.deleteTexture(this.textureB);
    this.gl.deleteBuffer(this.quadBuffer);
    for (const p of Object.values(this.programs)) {
      this.gl.deleteProgram(p.program);
    }
  }
}
