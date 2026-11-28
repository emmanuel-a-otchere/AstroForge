import { test } from "node:test";
import assert from "node:assert/strict";
import {
  slugFromName,
  formatDate,
  formatDateLong,
  parseRaDeg,
  parseDecDeg,
} from "../src/common/utils.js";

test("slugFromName lowercases and dashes non-alphanumerics", () => {
  assert.equal(slugFromName("M42 — Orion Nebula"), "m42-orion-nebula");
  assert.equal(slugFromName("  NGC 7000! "), "ngc-7000");
});

test("formatDate renders ISO dates via date-fns and passes through junk", () => {
  assert.equal(formatDate(""), "—");
  assert.equal(formatDate("not-a-date"), "not-a-date");
  assert.equal(formatDate("2026-11-27T10:00:00Z"), "2026-11-27");
  assert.match(formatDateLong("2026-11-27T10:00:00Z"), /Nov 27, 2026/);
});

test("parseRaDeg handles hms, colon, and plain-degree forms", () => {
  assert.equal(parseRaDeg("6h"), 90);
  assert.ok(Math.abs(parseRaDeg("05:35:17.2") - 83.821) < 0.001);
  assert.equal(parseRaDeg("83.5"), 83.5);
  assert.equal(parseRaDeg(""), null);
  assert.equal(parseRaDeg("abc"), null);
});

test("parseDecDeg handles signed dms and plain degrees", () => {
  assert.ok(Math.abs(parseDecDeg("+36d45m") - 36.75) < 0.001);
  assert.ok(Math.abs(parseDecDeg("-36:45:00") + 36.75) < 0.001);
  assert.equal(parseDecDeg("-12.5"), -12.5);
  assert.equal(parseDecDeg(null), null);
});
