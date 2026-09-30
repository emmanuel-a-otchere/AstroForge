// CR-10 P1.6.2.4 / Slice P1.6.2.4 tests:
// node-catalog tooltip helper. The slice adds
// `shortSummary(description, fallback)` to node-catalog.ts
// for the per-node hover tooltip text. This file pins
// the truncation policy: 7 words max, fallback for
// empty / null descriptions, no mid-word truncation.

import { describe, it, expect } from "vitest";
import { shortSummary } from "../node-catalog";

describe("shortSummary (CR-10 P1.6.2.4)", () => {
  it("returns the fallback when description is null", () => {
    expect(shortSummary(null, "Colour Calibration")).toBe("Colour Calibration");
  });

  it("returns the fallback when description is empty", () => {
    expect(shortSummary("", "Crop / Rotate")).toBe("Crop / Rotate");
  });

  it("returns the fallback when description is whitespace-only", () => {
    expect(shortSummary("   \t  ", "Stretch")).toBe("Stretch");
  });

  it("returns the description verbatim when 7 words or fewer", () => {
    expect(shortSummary("Remove green cast from star fields.", "Fallback"))
      .toBe("Remove green cast from star fields.");
  });

  it("truncates to the first 7 words + ellipsis when over the limit", () => {
    expect(
      shortSummary(
        "Apply a series of carefully chosen non-linear transformations to the linear input image",
        "Fallback",
      ),
    ).toBe("Apply a series of carefully chosen non-linear...");
  });

  it("truncates 8-word descriptions to 7 words plus ellipsis", () => {
    expect(shortSummary("one two three four five six seven eight", "Fallback"))
      .toBe("one two three four five six seven...");
  });

  it("does not collapse repeated whitespace into a single token", () => {
    // The helper splits on /\s+/ so any run of whitespace
    // is one word boundary; the result is clean.
    expect(shortSummary("a  b  c", "Fallback")).toBe("a b c");
  });

  it("returns the fallback verbatim, never an empty string", () => {
    expect(shortSummary(null, "")).toBe("");
    expect(shortSummary("   ", "")).toBe("");
  });

  it("trims leading and trailing whitespace before counting words", () => {
    expect(shortSummary("   one two three   ", "Fallback"))
      .toBe("one two three");
  });

  it("handles very long descriptions without truncation glitches", () => {
    const long = ("word" + " ").repeat(500).trim();
    const summary = shortSummary(long, "Fallback");
    // Exactly 7 tokens + "..." = 7 words followed by ellipsis.
    expect(summary.endsWith("...")).toBe(true);
    const sevenWords = summary.slice(0, -3).split(" ");
    expect(sevenWords).toHaveLength(7);
    for (const w of sevenWords) expect(w).toBe("word");
  });
});
