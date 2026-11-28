import { test } from "node:test";
import assert from "node:assert/strict";
import { decodeTiff } from "../src/lib/tiff-decode.ts";

// Build a minimal little-endian 16-bit grayscale TIFF matching the
// decoder's tag mapping (256 width, 257 height, 258 samples, 277 bps,
// 273 strip offset, 279 strip byte count).
function makeTiff16Gray(width, height, pixels) {
  const ifdOffset = 8;
  const entries = 6;
  const stripOffset = ifdOffset + 2 + entries * 12 + 4;
  const buf = new Uint8Array(stripOffset + pixels.length * 2);
  buf[0] = 0x49;
  buf[1] = 0x49;
  buf[2] = 42;
  buf[3] = 0;
  const w32 = (o, v) => {
    buf[o] = v & 0xff;
    buf[o + 1] = (v >> 8) & 0xff;
    buf[o + 2] = (v >> 16) & 0xff;
    buf[o + 3] = (v >> 24) & 0xff;
  };
  const w16 = (o, v) => {
    buf[o] = v & 0xff;
    buf[o + 1] = (v >> 8) & 0xff;
  };
  w32(4, ifdOffset);
  w16(ifdOffset, entries);
  const entry = (i, tag, count, value) => {
    const e = ifdOffset + 2 + i * 12;
    w16(e, tag);
    w16(e + 2, 1); // type SHORT
    w32(e + 4, count);
    if (count > 1) w32(e + 8, value);
    else w16(e + 8, value);
  };
  entry(0, 256, 1, width);
  entry(1, 257, 1, height);
  entry(2, 258, 1, 1); // samples
  entry(3, 277, 1, 16); // bps
  entry(4, 273, 1, stripOffset);
  entry(5, 279, 1, pixels.length * 2);
  w32(ifdOffset + 2 + entries * 12, 0); // next IFD
  pixels.forEach((v, i) => w16(stripOffset + i * 2, v));
  return buf;
}

test("decodeTiff decodes a 16-bit grayscale TIFF to 0..1 floats", () => {
  const tiff = makeTiff16Gray(2, 2, [0, 32768, 65535, 16384]);
  const out = decodeTiff(tiff);
  assert.ok(out);
  assert.equal(out.width, 2);
  assert.equal(out.height, 2);
  assert.equal(out.channels, 1);
  assert.equal(out.data.length, 4);
  assert.equal(out.data[0], 0);
  assert.ok(Math.abs(out.data[1] - 32768 / 65535) < 1e-6);
  assert.equal(out.data[2], 1);
});

test("decodeTiff rejects non-TIFF and non-16-bit input", () => {
  assert.equal(decodeTiff(new Uint8Array([1, 2, 3])), null);
  const notTiff = new Uint8Array(64);
  assert.equal(decodeTiff(notTiff), null);
  const tiff8 = makeTiff16Gray(1, 1, [0]);
  tiff8[8 + 2 + 3 * 12 + 8] = 8; // bps entry value -> 8
  tiff8[8 + 2 + 3 * 12 + 9] = 0;
  assert.equal(decodeTiff(tiff8), null);
});
