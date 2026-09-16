// CR-07 C-A2 — Side-by-side composite export.
//
// Renders a single off-screen canvas containing
// version A on the left and version B on the right,
// each labeled, with a small mode note in the gap.
// Returns a PNG blob the caller can download.
//
// This is a presentation-only export; it does NOT
// attempt to replicate the on-screen canvas state
// (zoom, pan, region, histogram overlay) at pixel
// fidelity. The audit accepts "side-by-side JPEG/PNG
// export" as the §30 acceptance criterion; this
// implementation covers the headline case.
//
// Honest flags:
//   * Single-frame composite. Does not encode the
//     comparison mode (overlay, blink, split) as
//     animation frames.
//   * PNG only. JPEG support is trivial to add.
//   * No "analytical report" export (§30 second
//     bullet). That ships in a future slice.
//   * Both versions are rendered at their natural
//     pixel dimensions, scaled to fit the requested
//     export width. The export does NOT apply
//     zoom / pan / region state.

import { readImageArtifact } from "./astroforge-api";

export interface CompositeExportOptions {
  /** Version id of the left ("A") image. */
  versionAId: string;
  /** Version id of the right ("B") image. */
  versionBId: string;
  /** Display label for A (e.g. "Linear", "AI enhanced"). */
  labelA: string;
  /** Display label for B. */
  labelB: string;
  /** Width of each panel in pixels. Total export
   *  width is `2 * panelWidth + gutter`. */
  panelWidth?: number;
  /** Pixel gap between A and B. Default 16. */
  gutter?: number;
  /** Background color (CSS-style hex or rgba).
   *  Default black. */
  background?: string;
  /** Optional mode note rendered above the panels
   *  (e.g. "Overlay 50%"). Default empty (no
   *  banner). */
  modeNote?: string;
}

export interface CompositeExportResult {
  blob: Blob;
  width: number;
  height: number;
}

const DEFAULTS = {
  panelWidth: 1024,
  gutter: 16,
  background: "#000000",
};

/** Build a side-by-side composite image of two
 *  Image Versions and return it as a PNG blob. */
export async function exportComparisonComposite(
  opts: CompositeExportOptions,
): Promise<CompositeExportResult> {
  const panelWidth = opts.panelWidth ?? DEFAULTS.panelWidth;
  const gutter = opts.gutter ?? DEFAULTS.gutter;
  const background = opts.background ?? DEFAULTS.background;

  // Load both artifacts in parallel. Each is a
  // 16-bit TIFF; the existing ImageCanvas decoder
  // handles the byte-layout but is component-bound.
  // Here we decode inline (smaller surface) using
  // the same TIFF parser approach.
  const [resA, resB] = await Promise.all([
    readImageArtifact(opts.versionAId),
    readImageArtifact(opts.versionBId),
  ]);

  const bytesA = base64ToBytes(resA.base64_data);
  const bytesB = base64ToBytes(resB.base64_data);

  const imgA = decodeTiffRgba(bytesA, resA.width, resA.height, resA.channels);
  const imgB = decodeTiffRgba(bytesB, resB.width, resB.height, resB.channels);

  if (!imgA || !imgB) {
    throw new Error(
      "Composite export: failed to decode one or both artifacts as TIFF.",
    );
  }

  // Compute per-panel height so the panels share
  // a common export height (height of the taller
  // image, scaled to match the width).
  const aspectA = resA.height / resA.width;
  const aspectB = resB.height / resB.width;
  const maxAspect = Math.max(aspectA, aspectB);
  const panelHeight = Math.round(panelWidth * maxAspect);

  // Banner height (only if modeNote is set).
  const bannerHeight = opts.modeNote ? 28 : 0;
  const totalHeight = panelHeight + bannerHeight;
  const totalWidth = panelWidth * 2 + gutter;

  const canvas = document.createElement("canvas");
  canvas.width = totalWidth;
  canvas.height = totalHeight;
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    throw new Error("Composite export: failed to acquire 2D canvas context.");
  }

  // Background.
  ctx.fillStyle = background;
  ctx.fillRect(0, 0, totalWidth, totalHeight);

  // Banner (if modeNote).
  if (opts.modeNote) {
    ctx.fillStyle = "rgba(255,255,255,0.85)";
    ctx.font = "14px sans-serif";
    ctx.textBaseline = "middle";
    ctx.textAlign = "center";
    ctx.fillText(opts.modeNote, totalWidth / 2, bannerHeight / 2);
  }

  // Paint A and B using ImageBitmap -> drawImage.
  const bmpA = await bitmapFromRgba(imgA, resA.width, resA.height);
  const bmpB = await bitmapFromRgba(imgB, resB.width, resB.height);
  ctx.drawImage(bmpA, 0, bannerHeight, panelWidth, panelHeight);
  ctx.drawImage(
    bmpB,
    panelWidth + gutter,
    bannerHeight,
    panelWidth,
    panelHeight,
  );

  // Labels under each panel.
  ctx.fillStyle = "rgba(255,255,255,0.85)";
  ctx.font = "16px sans-serif";
  ctx.textBaseline = "top";
  ctx.textAlign = "left";
  ctx.fillText(`A: ${opts.labelA}`, 4, bannerHeight + 4);
  ctx.textAlign = "left";
  ctx.fillText(
    `B: ${opts.labelB}`,
    panelWidth + gutter + 4,
    bannerHeight + 4,
  );

  const blob: Blob = await new Promise((resolve, reject) => {
    canvas.toBlob(
      (b) =>
        b ? resolve(b) : reject(new Error("Canvas toBlob returned null.")),
      "image/png",
    );
  });

  return { blob, width: totalWidth, height: totalHeight };
}

/** Trigger a browser download of a Blob. */
export function downloadBlob(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  // Defer revoke so the click handler completes.
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

// --- internals ---

function base64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const arr = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
  return arr;
}

/** Decode a 16-bit grayscale or RGB uncompressed
 *  TIFF into an RGBA Uint8ClampedArray suitable
 *  for ImageData. Returns null on malformed input.
 *
 *  Mirrors the decoder in ImageCanvas.svelte but
 *  returns 8-bit RGBA instead of Float32, since
 *  the export canvas doesn't need HDR data. */
function decodeTiffRgba(
  buf: Uint8Array,
  expectedWidth: number,
  expectedHeight: number,
  expectedChannels: number,
): Uint8ClampedArray | null {
  if (buf.length < 8) return null;
  const little = buf[0] === 0x49; // "II" vs "MM"
  if (!little && buf[0] !== 0x4d) return null;

  const r16 = (o: number): number =>
    little ? buf[o] | (buf[o + 1] << 8) : (buf[o] << 8) | buf[o + 1];
  const r32 = (o: number): number =>
    little
      ? buf[o] |
        (buf[o + 1] << 8) |
        (buf[o + 2] << 16) |
        (buf[o + 3] << 24)
      : (buf[o] << 24) |
        (buf[o + 1] << 16) |
        (buf[o + 2] << 8) |
        buf[o + 3];

  const magic = r16(2);
  if (magic !== 42) return null;
  const ifdOffset = r32(4);
  if (ifdOffset + 2 > buf.length) return null;

  const numEntries = r16(ifdOffset);
  let width = 0;
  let height = 0;
  let bitsPerSample = 16;
  let samplesPerPixel = expectedChannels;
  let stripOffset = 0;
  let stripBytes: number | null = null;

  for (let i = 0; i < numEntries; i++) {
    const e = ifdOffset + 2 + i * 12;
    if (e + 12 > buf.length) return null;
    const tag = r16(e);
    const typ = r16(e + 2);
    const count = r32(e + 4);
    const valueOffset = e + 8;
    let value: number;
    if (typ === 3 && count === 1) value = r16(valueOffset);
    else if (typ === 4 && count === 1) value = r32(valueOffset);
    else value = r32(valueOffset);

    if (tag === 256) width = value;
    else if (tag === 257) height = value;
    else if (tag === 258) bitsPerSample = value;
    else if (tag === 277) samplesPerPixel = value;
    else if (tag === 273) stripOffset = value;
    else if (tag === 279) stripBytes = value;
  }

  if (
    width !== expectedWidth ||
    height !== expectedHeight ||
    samplesPerPixel !== expectedChannels ||
    bitsPerSample !== 16
  ) {
    return null;
  }
  if (!stripBytes) return null;

  const totalPixels = width * height * samplesPerPixel;
  if (stripOffset + totalPixels * 2 > buf.length) return null;

  const rgba = new Uint8ClampedArray(width * height * 4);
  if (samplesPerPixel === 1) {
    // Grayscale -> RGBA. Scale 16-bit to 8-bit.
    for (let i = 0; i < width * height; i++) {
      const v = r16(stripOffset + i * 2);
      const o = i * 4;
      rgba[o] = v >> 8;
      rgba[o + 1] = v >> 8;
      rgba[o + 2] = v >> 8;
      rgba[o + 3] = 255;
    }
  } else {
    // RGB(A) -> RGBA. Scale 16-bit to 8-bit.
    const ch = samplesPerPixel;
    for (let i = 0; i < width * height; i++) {
      const o = i * 4;
      for (let c = 0; c < 3 && c < ch; c++) {
        const v = r16(stripOffset + (i * ch + c) * 2);
        rgba[o + c] = v >> 8;
      }
      rgba[o + 3] = 255;
    }
  }
  return rgba;
}

async function bitmapFromRgba(
  rgba: Uint8ClampedArray,
  width: number,
  height: number,
): Promise<ImageBitmap> {
  // The TS lib in this repo only exposes
  // ImageData(width, height) and
  // ctx.createImageData(width, height). Use
  // createImageData (from a throwaway canvas) and
  // copy the buffer in. Same effect as
  // `new ImageData(rgba, w, h)` on libs that
  // support it.
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    throw new Error("Composite export: failed to acquire 2D context for ImageData.");
  }
  const data = ctx.createImageData(width, height);
  data.data.set(rgba);
  return await createImageBitmap(data);
}