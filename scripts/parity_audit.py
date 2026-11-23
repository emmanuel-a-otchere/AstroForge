#!/usr/bin/env python3
"""Parity audit: egui api surface vs the Tauri command surface.

Compares every command name invoked from the Svelte side
(`invoke("...")` in src/lib/astroforge-api.ts + the wizard screens)
against the direct-call functions in crates/astroforge-ui/src/api/*.rs.

Exit code 1 when gaps exist, so CI can gate on it.

Usage: python3 scripts/parity_audit.py
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
TS_DIR = ROOT / "src"
RUST_API = ROOT / "crates" / "astroforge-ui" / "src" / "api"

# Commands invoked anywhere in the Svelte/TS tree.
INVOKE_RE = re.compile(r'invoke\s*(?:<[^>]*>)?\(\s*"([a-z_][a-z0-9_]*)"')


def ts_commands() -> set[str]:
    found: set[str] = set()
    for path in TS_DIR.rglob("*.ts"):
        found |= set(INVOKE_RE.findall(path.read_text(errors="ignore")))
    for path in TS_DIR.rglob("*.svelte"):
        found |= set(INVOKE_RE.findall(path.read_text(errors="ignore")))
    return found


# Rust api functions: `pub fn name(` in crates/astroforge-ui/src/api/.
RUST_FN_RE = re.compile(r"pub fn ([a-z_][a-z0-9_]*)\(")


def rust_fns() -> set[str]:
    found: set[str] = set()
    for path in RUST_API.glob("*.rs"):
        found |= set(RUST_FN_RE.findall(path.read_text(errors="ignore")))
    return found


def main() -> int:
    ts = ts_commands()
    rust = rust_fns()

    missing = sorted(ts - rust)
    extra = sorted(rust - ts)

    print(f"Tauri commands invoked from the web front-end : {len(ts)}")
    print(f"egui api functions (crates/astroforge-ui)    : {len(rust)}")
    print()

    if missing:
        print(f"MISSING in egui api ({len(missing)}):")
        for name in missing:
            print(f"  - {name}")
        print()
    else:
        print("No gaps: every web command has a direct-call equivalent.")
        print()

    if extra:
        print(f"egui-only helpers (no web equivalent — expected for UI glue): {len(extra)}")
        for name in extra:
            print(f"  + {name}")
        print()

    # Allow-listed gaps, documented:
    # - pipeline_run_session: inline MVP-pipeline command flagged
    #   during the P3a port; allowed absent until the Process
    #   workspace grows the legacy one-shot runner.
    # - recipe_feed_list: DEAD CALL in src/lib/recipeFeed.ts — no
    #   such command exists in src-tauri (main.rs) or astroforge-core.
    #   Invoking it fails at runtime with "command not found". The
    #   egui api cannot port a command that has no implementation;
    #   fix or delete the call in recipeFeed.ts.
    ALLOWED_MISSING = {"pipeline_run_session", "recipe_feed_list"}
    real_missing = [m for m in missing if m not in ALLOWED_MISSING]

    if real_missing:
        print(f"RESULT: {len(real_missing)} command(s) still missing parity.")
        return 1
    print("RESULT: parity OK (within the documented allow-list).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
