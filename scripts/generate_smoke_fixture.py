#!/usr/bin/env python3
"""Generate a synthetic FITS folder fixture for the MVP smoke test.

Phase 7 (issues #43/#44). Writes 3 light frames + 1 dark + 1 flat in
IEEE float (BITPIX=-32) format so the CLI's MVP pipeline can read
them with fits::read_f32_image.

Usage:
    python3 scripts/generate_smoke_fixture.py tests/fixtures/sample-session
"""
import os
import struct
import sys
from pathlib import Path

WIDTH = 32
HEIGHT = 32
BLOCK = 2880


def fits_card(key: str, value: str) -> bytes:
    card = f"{key:<8}= {value:<70}".encode("ascii")
    card = card[:80].ljust(80, b" ")
    return card


def write_fits(path: Path, frame_type: str, exptime: float) -> None:
    """Write a minimal but valid FITS file with BITPIX=-32 float data."""
    header = b""
    header += fits_card("SIMPLE", "T")
    header += fits_card("BITPIX", "-32")
    header += fits_card("NAXIS", "2")
    header += fits_card("NAXIS1", str(WIDTH))
    header += fits_card("NAXIS2", str(HEIGHT))
    header += fits_card("IMAGETYP", f"'{frame_type}'")
    header += fits_card("EXPTIME", f"{exptime:.1f}")
    header += fits_card("FILTER", "'L'")
    header += fits_card("END", "")
    while len(header) % BLOCK != 0:
        header += b" "
    assert len(header) % BLOCK == 0, "header must be padded to FITS block"

    pixels = WIDTH * HEIGHT
    data = bytearray()
    for i in range(pixels):
        # Gradient + per-frame offset so a stack has signal to find.
        v = ((i + pixels // 4) % 256) / 255.0
        if frame_type == "DARK":
            v = 0.05
        elif frame_type == "FLAT":
            v = 0.9
        data += struct.pack(">f", v)
    if len(data) % BLOCK != 0:
        data.extend(b"\x00" * (BLOCK - (len(data) % BLOCK)))

    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as f:
        f.write(header)
        f.write(bytes(data))


def main() -> int:
    target = Path(sys.argv[1] if len(sys.argv) > 1 else "tests/fixtures/sample-session")
    if target.exists():
        for entry in target.iterdir():
            if entry.is_file():
                entry.unlink()
    target.mkdir(parents=True, exist_ok=True)

    for i in range(3):
        write_fits(target / f"light_{i:03}.fits", "LIGHT", 120.0)
    write_fits(target / "dark_001.fits", "DARK", 120.0)
    write_fits(target / "flat_001.fits", "FLAT", 1.0)

    files = sorted(target.glob("*.fits"))
    print(f"wrote {len(files)} FITS files to {target}")
    for f in files:
        print(f"  {f.name}  ({f.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
