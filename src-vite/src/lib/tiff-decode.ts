// Minimal uncompressed 16-bit TIFF decoder (grayscale or RGB), matching
// the Svelte ImageCanvas path: Float32Array, one plane per channel,
// normalised to 0..1.

export interface DecodedTiff {
  width: number;
  height: number;
  channels: number;
  data: Float32Array;
}

export function decodeTiff(buf: Uint8Array): DecodedTiff | null {
  if (buf.length < 8 || (buf[0] !== 0x49 && buf[0] !== 0x4d)) return null;
  const little = buf[0] === 0x49;
  const r16 = (o: number) =>
    little ? buf[o] | (buf[o + 1] << 8) : (buf[o] << 8) | buf[o + 1];
  const r32 = (o: number) =>
    little
      ? buf[o] | (buf[o + 1] << 8) | (buf[o + 2] << 16) | (buf[o + 3] << 24)
      : (buf[o] << 24) | (buf[o + 1] << 16) | (buf[o + 2] << 8) | buf[o + 3];
  if (r16(2) !== 42) return null;
  const ifd = r32(4);
  const nEntries = r16(ifd);
  let width = 0;
  let height = 0;
  let samples = 1;
  let bps = 16;
  let stripOffset = 0;
  let stripByteCount = 0;
  for (let i = 0; i < nEntries; i++) {
    const e = ifd + 2 + i * 12;
    const tag = r16(e);
    const count = r32(e + 4);
    if (tag === 256) width = count > 1 ? r32(e + 8) : r16(e + 8);
    else if (tag === 257) height = count > 1 ? r32(e + 8) : r16(e + 8);
    else if (tag === 258) samples = r16(e + 8);
    else if (tag === 277) bps = r16(e + 8);
    else if (tag === 273) stripOffset = r32(e + 8);
    else if (tag === 279) stripByteCount = r32(e + 8);
  }
  if (bps !== 16 || (samples !== 1 && samples !== 3)) return null;
  if (!width || !height || !stripOffset) return null;
  const px = width * height;
  const expected = px * samples * 2;
  if (!stripByteCount) stripByteCount = expected;
  if (stripByteCount < expected) return null;
  const data = new Float32Array(px * samples);
  let p = 0;
  for (let i = 0; i < expected; i += 2) {
    const v = little
      ? buf[stripOffset + i] | (buf[stripOffset + i + 1] << 8)
      : (buf[stripOffset + i] << 8) | buf[stripOffset + i + 1];
    data[p++] = v / 65535;
  }
  return { width, height, channels: samples, data };
}
