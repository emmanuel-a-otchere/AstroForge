#!/usr/bin/env python3
"""Generate the RT-11 (#429) sample dataset — a synthetic Seestar-style
session that contributors can run through the full pipeline (and the AI
denoise path) without any external data or licensing surface.

Provenance: authored in-repo by this script, deterministically (seeded
LCG so regeneration is byte-stable). Same license as the repo
(MIT OR Apache-2.0). The frames are synthetic by design — a real
astrophotograph would import someone's copyright; this imports nobody's.

Session shape (tests/fixtures/sample-dataset/):
  - 12 light frames  256x256, LIGHT, 30s, filter L, TELESCOP Seestar S50
    (synthetic star field + nebula gradient + per-frame dither + noise)
  - 4 dark frames    256x256, DARK  (small offset + noise)
  - 3 flat frames    256x256, FLAT   (vignette + noise)
  - 3 bias frames    256x256, BIAS   (offset only)

Usage:
    python3 scripts/generate_sample_dataset.py tests/fixtures/sample-dataset
"""
import math
import struct
import sys
from pathlib import Path

WIDTH = 256
HEIGHT = 256
BLOCK = 2880
SEED = 20261106


class LCG:
    """Deterministic PRNG — regenerating the dataset is byte-stable."""

    def __init__(self, seed: int) -> None:
        self.state = seed & 0xFFFFFFFF

    def next_u32(self) -> int:
        self.state = (1103515245 * self.state + 12345) & 0xFFFFFFFF
        return self.state

    def uniform(self) -> float:
        return self.next_u32() / 0xFFFFFFFF

    def gauss(self) -> float:
        # Irwin-Hall approx: sum of 3 uniforms is plenty for noise.
        return (self.uniform() + self.uniform() + self.uniform()) / 1.5 - 1.0


def fits_card(key: str, value: str) -> bytes:
    card = f"{key:<8}= {value:<70}".encode("ascii")
    card = card[:80].ljust(80, b" ")
    return card


def write_fits(path: Path, pixels: list[float], imagetyp: str, exptime: float,
               filter_name: str, frame_idx: int) -> None:
    header = b""
    header += fits_card("SIMPLE", "T")
    header += fits_card("BITPIX", "-32")
    header += fits_card("NAXIS", "2")
    header += fits_card("NAXIS1", str(WIDTH))
    header += fits_card("NAXIS2", str(HEIGHT))
    header += fits_card("IMAGETYP", f"'{imagetyp}'")
    header += fits_card("EXPTIME", f"{exptime:.1f}")
    header += fits_card("FILTER", f"'{filter_name}'")
    header += fits_card("TELESCOP", "'Seestar S50'")
    header += fits_card("INSTRUME", "'Seestar S50'")
    header += fits_card("OBJECT", "'M31 Sample'")
    header += fits_card("DATE-OBS", f"'2026-10-15T22:{frame_idx:02d}:00'")
    header += fits_card("XBINNING", "1")
    header += fits_card("YBINNING", "1")
    header += fits_card("CCD-TEMP", "15.0")
    header += fits_card("END", "")
    while len(header) % BLOCK != 0:
        header += b" "

    data = bytearray()
    for v in pixels:
        v = min(1.0, max(0.0, v))
        data += struct.pack(">f", v)
    if len(data) % BLOCK != 0:
        data.extend(b"\x00" * (BLOCK - (len(data) % BLOCK)))

    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as f:
        f.write(header)
        f.write(bytes(data))


def star_field(rng: LCG, n_stars: int) -> list[tuple[float, float, float, float]]:
    """(x, y, peak, sigma) for a synthetic field: a few bright stars,
    a spread of medium ones, mostly-faint background stars."""
    stars = []
    for _ in range(n_stars):
        x = rng.uniform() * WIDTH
        y = rng.uniform() * HEIGHT
        tier = rng.uniform()
        if tier > 0.93:
            peak, sigma = 0.9 + rng.uniform() * 0.1, 1.4
        elif tier > 0.6:
            peak, sigma = 0.35 + rng.uniform() * 0.3, 1.1
        else:
            peak, sigma = 0.08 + rng.uniform() * 0.15, 0.9
        stars.append((x, y, peak, sigma))
    return stars


def render_light(rng: LCG, stars: list[tuple[float, float, float, float]],
                 dither_x: float, dither_y: float, frame: int) -> list[float]:
    """Star field (Gaussian PSFs) + low-frequency nebula gradient +
    sky background + per-frame noise, with sub-pixel dithering."""
    pix = [0.0] * (WIDTH * HEIGHT)
    # Nebula: two smooth low-frequency lobes (M31-ish gradient).
    for y in range(HEIGHT):
        for x in range(WIDTH):
            nx, ny = x / WIDTH, y / HEIGHT
            lobe = (math.exp(-((nx - 0.45) ** 2 + (ny - 0.5) ** 2) * 18) * 0.30
                    + math.exp(-((nx - 0.62) ** 2 + (ny - 0.42) ** 2) * 40) * 0.15)
            dust = math.exp(-((nx - 0.5) ** 2 + (ny - 0.55) ** 2) * 60) * 0.10
            pix[y * WIDTH + x] = 0.02 + lobe - dust
    # Stars with sub-pixel dither (frame-local sky drift).
    for (sx, sy, peak, sigma) in stars:
        cx, cy = sx + dither_x, sy + dither_y
        x0, x1 = int(max(0, cx - 6)), int(min(WIDTH - 1, cx + 6))
        y0, y1 = int(max(0, cy - 6)), int(min(HEIGHT - 1, cy + 6))
        inv2s2 = 1.0 / (2.0 * sigma * sigma)
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                d2 = (x - cx) ** 2 + (y - cy) ** 2
                pix[y * WIDTH + x] += peak * math.exp(-d2 * inv2s2)
    # Noise + frame-varying sky offset.
    noise = 0.006 + (frame % 3) * 0.002
    for i in range(len(pix)):
        pix[i] += noise * rng.gauss()
    return pix


def render_dark(rng: LCG) -> list[float]:
    # Small thermal offset + hot pixels at deterministic positions.
    pix = [0.012 + 0.004 * rng.gauss() for _ in range(WIDTH * HEIGHT)]
    for i in range(0, WIDTH * HEIGHT, 997):
        pix[i] += 0.05
    return pix


def render_flat(rng: LCG) -> list[float]:
    # Vignette: brightness falls off toward the corners.
    pix = [0.0] * (WIDTH * HEIGHT)
    for y in range(HEIGHT):
        for x in range(WIDTH):
            nx, ny = (x - WIDTH / 2) / (WIDTH / 2), (y - HEIGHT / 2) / (HEIGHT / 2)
            r2 = nx * nx + ny * ny
            pix[y * WIDTH + x] = 0.85 - 0.25 * r2 + 0.01 * rng.gauss()
    return pix


def render_bias(rng: LCG) -> list[float]:
    return [0.005 + 0.002 * rng.gauss() for _ in range(WIDTH * HEIGHT)]


def main() -> int:
    target = Path(sys.argv[1] if len(sys.argv) > 1
                  else "tests/fixtures/sample-dataset")
    if target.exists():
        for entry in target.iterdir():
            if entry.is_file():
                entry.unlink()
    target.mkdir(parents=True, exist_ok=True)

    rng = LCG(SEED)
    stars = star_field(rng, 48)

    written = 0
    for i in range(12):
        dx = (i % 4) * 0.6 - 0.9  # ~1px dither box
        dy = (i // 4) * 0.6 - 0.6
        pix = render_light(rng, stars, dx, dy, i)
        write_fits(target / f"light_{i:03}.fits", pix, "LIGHT", 30.0, "L", i)
        written += 1
    for i in range(4):
        write_fits(target / f"dark_{i:03}.fits", render_dark(rng), "DARK", 30.0, "L", i)
        written += 1
    for i in range(3):
        write_fits(target / f"flat_{i:03}.fits", render_flat(rng), "FLAT", 1.0, "L", i)
        written += 1
    for i in range(3):
        write_fits(target / f"bias_{i:03}.fits", render_bias(rng), "BIAS", 0.0, "L", i)
        written += 1

    total_mb = sum(f.stat().st_size for f in sorted(target.glob("*.fits"))) / (1024 * 1024)
    print(f"wrote {written} FITS files ({total_mb:.1f} MB) to {target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
