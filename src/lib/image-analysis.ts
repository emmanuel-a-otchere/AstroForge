export interface ImageMetadata {
  fileName: string;
  fileSize: number;
  focalLength: number | null;
  exposureTime: number | null;
  filter: string | null;
  objectType: string | null;
  frameType: string;
  width: number | null;
  height: number | null;
  source: "fits" | "exif" | "filename" | "unknown";
}

export interface AnalysisResult {
  focalLength: number | null;
  focalLengthSource: string;
  detectedObjectType: "deep_sky" | "planetary" | "lunar" | null;
  detectionConfidence: "high" | "medium" | "low";
  detectionReason: string;
  frames: ImageMetadata[];
  lightCount: number;
  darkCount: number;
  flatCount: number;
  biasCount: number;
  avgExposure: number | null;
  totalFrames: number;
}

const FITS_EXTENSIONS = [".fits", ".fit"];
const RAW_EXTENSIONS = [".dng", ".cr2", ".nef", ".arw"];
const IMAGE_EXTENSIONS = [".tif", ".tiff", ".png", ".jpg", ".jpeg"];

const FITS_RECORD_SIZE = 80;
const FITS_BLOCK_SIZE = 2880;

function isFitsFile(name: string): boolean {
  const lower = name.toLowerCase();
  return FITS_EXTENSIONS.some((ext) => lower.endsWith(ext));
}

function isRawFile(name: string): boolean {
  const lower = name.toLowerCase();
  return RAW_EXTENSIONS.some((ext) => lower.endsWith(ext));
}

function isImageFile(name: string): boolean {
  const lower = name.toLowerCase();
  return IMAGE_EXTENSIONS.some((ext) => lower.endsWith(ext));
}

function parseFitsHeader(data: ArrayBuffer): Map<string, string> {
  const cards = new Map<string, string>();
  const bytes = new Uint8Array(data.slice(0, Math.min(data.byteLength, 8640)));
  let offset = 0;

  while (offset + FITS_RECORD_SIZE <= bytes.length) {
    const record = new TextDecoder("ascii", { fatal: false }).decode(
      bytes.subarray(offset, offset + FITS_RECORD_SIZE)
    );

    if (record.trim().startsWith("END")) break;

    if (record.length >= 8) {
      const key = record.substring(0, 8).trim();
      if (key) {
        const valPart = record.length > 10 ? record.substring(10) : "";
        const value = valPart
          .replace(/^=\s*/, "")
          .trim()
          .replace(/^["']|["']$/g, "")
          .trim();
        if (value) cards.set(key, value);
      }
    }

    offset += FITS_RECORD_SIZE;
    if (offset % FITS_BLOCK_SIZE === 0) {
      const blockEnd = bytes.subarray(offset, Math.min(offset + FITS_BLOCK_SIZE, bytes.length));
      if (blockEnd.every((b) => b === 0x20) || blockEnd.length === 0) break;
    }
  }

  return cards;
}

async function readExifFocalLength(file: File): Promise<number | null> {
  try {
    if (typeof file.arrayBuffer !== "function") return null;
    const buffer = await file.arrayBuffer();

    if (isRawFile(file.name) || /\.(tif|tiff)$/i.test(file.name)) {
      return extractTiffFocalLength(buffer);
    }
    if (/\.(jpg|jpeg)$/i.test(file.name)) {
      return extractJpegFocalLength(buffer);
    }
    return null;
  } catch {
    return null;
  }
}

function extractTiffFocalLength(buffer: ArrayBuffer): number | null {
  const view = new DataView(buffer);
  if (view.byteLength < 8) return null;

  const littleEndian = view.getUint16(0, false) === 0x4d4d ? false : true;
  const ifdOffset = view.getUint32(4, littleEndian);
  if (ifdOffset + 2 > view.byteLength) return null;

  const entryCount = view.getUint16(ifdOffset, littleEndian);
  for (let i = 0; i < entryCount; i++) {
    const entryOffset = ifdOffset + 2 + i * 12;
    if (entryOffset + 12 > view.byteLength) continue;
    const tag = view.getUint16(entryOffset, littleEndian);
    if (tag === 0x920a) {
      const valueOffset = view.getUint32(entryOffset + 8, littleEndian);
      if (valueOffset + 2 > view.byteLength) return null;
      return view.getUint16(valueOffset, littleEndian) || null;
    }
  }
  return null;
}

function extractJpegFocalLength(buffer: ArrayBuffer): number | null {
  const view = new DataView(buffer);
  let offset = 2;

  while (offset < view.byteLength - 4) {
    if (view.getUint16(offset, false) !== 0xff00) break;
    const marker = view.getUint8(offset + 1);

    if (marker === 0xe1) {
      const segLength = view.getUint16(offset + 2, false);
      const segStart = offset + 4;
      if (segStart + 6 > view.byteLength) break;
      const magic = new TextDecoder().decode(buffer.slice(segStart, segStart + 4));
      if (magic === "Exif") {
        const tiffStart = segStart + 6;
        const tiffBuffer = buffer.slice(tiffStart);
        const fl = extractTiffFocalLength(tiffBuffer);
        if (fl) return fl;
      }
      offset += 2 + segLength;
    } else if ((marker >= 0xd0 && marker <= 0xd7) || marker === 0x01 || marker === 0xd8) {
      offset += 2;
    } else if (marker === 0xd9) {
      break;
    } else {
      const segLength = view.getUint16(offset + 2, false);
      offset += 2 + segLength;
    }
  }
  return null;
}

function guessFrameTypeFromName(name: string): string {
  const lower = name.toLowerCase();
  if (lower.includes("dark")) return "DARK";
  if (lower.includes("flat")) return "FLAT";
  if (lower.includes("bias") || lower.includes("offset")) return "BIAS";
  return "LIGHT";
}

function guessObjectFromFits(cards: Map<string, string>): string | null {
  const obj = cards.get("OBJECT");
  return obj || null;
}

function guessObjectFromName(name: string): string | null {
  const lower = name.toLowerCase();
  const planetary = ["jupiter", "saturn", "mars", "venus", "mercury", "neptune", "uranus", "pluto"];
  const lunar = ["moon", "lunar"];
  for (const p of planetary) {
    if (lower.includes(p)) return p.charAt(0).toUpperCase() + p.slice(1);
  }
  for (const l of lunar) {
    if (lower.includes(l)) return "Moon";
  }
  return null;
}

function inferObjectType(
  frames: ImageMetadata[]
): { type: "deep_sky" | "planetary" | "lunar"; confidence: "high" | "medium" | "low"; reason: string } | null {
  const lights = frames.filter((f) => f.frameType === "LIGHT");
  if (lights.length === 0) return null;

  const exposures = lights
    .map((f) => f.exposureTime)
    .filter((e): e is number => e !== null);
  const avgExposure =
    exposures.length > 0 ? exposures.reduce((a, b) => a + b, 0) / exposures.length : null;

  const objectNames = lights
    .map((f) => f.objectType)
    .filter((o): o is string => o !== null)
    .map((o) => o.toLowerCase());

  const hasPlanetaryName = objectNames.some(
    (o) =>
      ["jupiter", "saturn", "mars", "venus", "mercury", "neptune", "uranus", "pluto"].some((p) =>
        o.includes(p)
      )
  );
  const hasLunarName = objectNames.some((o) => o.includes("moon") || o.includes("lunar"));

  if (hasLunarName) {
    return { type: "lunar", confidence: "high", reason: 'FITS OBJECT or filename indicates "Moon"' };
  }
  if (hasPlanetaryName) {
    return { type: "planetary", confidence: "high", reason: "FITS OBJECT or filename indicates a planet" };
  }

  if (avgExposure !== null) {
    if (avgExposure < 2 && lights.length > 500) {
      return {
        type: "planetary",
        confidence: "high",
        reason: `Short exposures (avg ${avgExposure.toFixed(1)}s) with ${lights.length} frames`,
      };
    }
    if (avgExposure > 10 && lights.length < 500) {
      return {
        type: "deep_sky",
        confidence: "high",
        reason: `Long exposures (avg ${avgExposure.toFixed(1)}s) with ${lights.length} frames`,
      };
    }
    if (avgExposure < 2) {
      return {
        type: "planetary",
        confidence: "medium",
        reason: `Short exposures (avg ${avgExposure.toFixed(1)}s)`,
      };
    }
    if (avgExposure > 10) {
      return {
        type: "deep_sky",
        confidence: "medium",
        reason: `Long exposures (avg ${avgExposure.toFixed(1)}s)`,
      };
    }
  }

  return {
    type: "deep_sky",
    confidence: "low",
    reason: "Defaulting to deep-sky (insufficient metadata to determine otherwise)",
  };
}

export async function analyzeFiles(files: File[]): Promise<AnalysisResult> {
  const frames: ImageMetadata[] = [];

  for (const file of files) {
    const meta: ImageMetadata = {
      fileName: file.name,
      fileSize: file.size,
      focalLength: null,
      exposureTime: null,
      filter: null,
      objectType: null,
      frameType: guessFrameTypeFromName(file.name),
      width: null,
      height: null,
      source: "unknown",
    };

    if (isFitsFile(file.name)) {
      try {
        const buffer = await file.arrayBuffer();
        const cards = parseFitsHeader(buffer);

        const focalLen = cards.get("FOCALLEN") || cards.get("FOCAL");
        if (focalLen) {
          const parsed = parseFloat(focalLen);
          if (!isNaN(parsed) && parsed > 0) meta.focalLength = parsed;
        }

        const exptime = cards.get("EXPTIME");
        if (exptime) {
          const parsed = parseFloat(exptime);
          if (!isNaN(parsed)) meta.exposureTime = parsed;
        }

        const filter = cards.get("FILTER");
        if (filter) meta.filter = filter;

        const object = guessObjectFromFits(cards) || guessObjectFromName(file.name);
        if (object) meta.objectType = object;

        const imagetyp = cards.get("IMAGETYP");
        if (imagetyp) {
          const upper = imagetyp.toUpperCase();
          if (upper.includes("LIGHT")) meta.frameType = "LIGHT";
          else if (upper.includes("DARK")) meta.frameType = "DARK";
          else if (upper.includes("FLAT")) meta.frameType = "FLAT";
          else if (upper.includes("BIAS") || upper.includes("OFFSET")) meta.frameType = "BIAS";
        }

        const naxis1 = cards.get("NAXIS1");
        const naxis2 = cards.get("NAXIS2");
        if (naxis1) meta.width = parseInt(naxis1, 10) || null;
        if (naxis2) meta.height = parseInt(naxis2, 10) || null;

        meta.source = "fits";
      } catch {
        // header parse failed, keep filename-based guesses
      }
    } else if (isRawFile(file.name) || /\.(tif|tiff)$/i.test(file.name)) {
      const fl = await readExifFocalLength(file);
      if (fl && fl > 0) {
        meta.focalLength = fl;
        meta.source = "exif";
      }
      const obj = guessObjectFromName(file.name);
      if (obj) meta.objectType = obj;
    } else {
      const obj = guessObjectFromName(file.name);
      if (obj) meta.objectType = obj;
    }

    frames.push(meta);
  }

  const focalLengths = frames
    .map((f) => f.focalLength)
    .filter((f): f is number => f !== null && f > 0);

  const focalLength =
    focalLengths.length > 0
      ? Math.round(focalLengths.reduce((a, b) => a + b, 0) / focalLengths.length)
      : null;

  const focalLengthSource =
    focalLengths.length > 0
      ? frames.find((f) => f.focalLength !== null)?.source === "fits"
        ? `Detected from FITS headers (${focalLengths.length} frame${focalLengths.length !== 1 ? "s" : ""})`
        : `Detected from EXIF data (${focalLengths.length} frame${focalLengths.length !== 1 ? "s" : ""})`
      : "";

  const objectDetection = inferObjectType(frames);

  const lightCount = frames.filter((f) => f.frameType === "LIGHT").length;
  const darkCount = frames.filter((f) => f.frameType === "DARK").length;
  const flatCount = frames.filter((f) => f.frameType === "FLAT").length;
  const biasCount = frames.filter((f) => f.frameType === "BIAS").length;

  const exposures = frames
    .filter((f) => f.frameType === "LIGHT")
    .map((f) => f.exposureTime)
    .filter((e): e is number => e !== null);
  const avgExposure =
    exposures.length > 0 ? exposures.reduce((a, b) => a + b, 0) / exposures.length : null;

  return {
    focalLength,
    focalLengthSource,
    detectedObjectType: objectDetection?.type ?? null,
    detectionConfidence: objectDetection?.confidence ?? "low",
    detectionReason: objectDetection?.reason ?? "",
    frames,
    lightCount,
    darkCount,
    flatCount,
    biasCount,
    avgExposure,
    totalFrames: frames.length,
  };
}
