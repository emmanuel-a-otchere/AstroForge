import { test } from "node:test";
import assert from "node:assert/strict";
import { slugFromName, formatDate } from "../src/common/utils.js";

test("slugFromName lowercases and dashes non-alphanumerics", () => {
  assert.equal(slugFromName("M42 — Orion Nebula"), "m42-orion-nebula");
  assert.equal(slugFromName("  NGC 7000! "), "ngc-7000");
});

test("formatDate renders ISO dates and passes through junk", () => {
  assert.equal(formatDate(""), "—");
  assert.equal(formatDate("not-a-date"), "not-a-date");
  const out = formatDate("2026-11-27T10:00:00Z");
  assert.match(out, /2026/);
});
