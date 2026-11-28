#!/usr/bin/env python3
"""UI-strategy isolation gate (Model B — see docs/BRANCHING.md).

Two front-end strategies coexist in this repo and must never import each
other; they meet only at the shared crates (astroforge-core / astroforge-ai)
and the on-disk data directory:

  web  : src/ + src-tauri/                          (Tauri 2 + Svelte 5, ships from main)
  egui : crates/astroforge-ui/ + crates/astroforge-render/   (native parity branch: egui/main)

Static mode (default; runs in CI on every push/PR):
  1. egui crates contain no reference to the web tree (no "src-tauri" /
     "astroforge-tauri" outside comments in .rs / Cargo.toml files)
  2. src-tauri does not depend on the egui crates
  3. web src/ contains no imports of ../crates (the Rust UI crates)
  4. every astroforge-* dependency of each UI crate is within its
     allow-list (web: core+ai; egui-render: core; egui-ui: core+ai+render)

PR mode (--pr N --repo owner/repo; needs GH_TOKEN with pull-requests read):
  5. a PR from an egui/* head branch into main is rejected unless labeled
     "cutover" (the only sanctioned egui->main flow, at retirement)
  6. a PR touching both the web and egui trees is rejected unless labeled
     "allow-cross-tree"

Exit code 0 = isolation holds; 1 = violation (message on stdout).
"""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

WEB_DIRS = ("src/", "src-vite/", "src-tauri/")
WEB_FILES = {
    "index.html", "package.json", "package-lock.json", "svelte.config.js",
    "vite.config.ts", "vitest.config.ts", "vitest.setup.ts", "tsconfig.json",
    ".prettierrc", ".prettierignore",
}
EGUI_DIRS = ("crates/astroforge-ui/", "crates/astroforge-render/")

# Allowed astroforge-* path/workspace deps per crate. Anything else = the
# UIs started depending on each other directly.
DEP_ALLOWLIST = {
    "src-tauri": {"astroforge-core", "astroforge-ai"},
    "crates/astroforge-render": {"astroforge-core"},
    "crates/astroforge-ui": {"astroforge-core", "astroforge-ai", "astroforge-render"},
}

WEB_REF_PATTERNS = ("src-tauri", "astroforge-tauri", "astroforge_tauri")
EGUI_REF_PATTERNS = ("crates/astroforge-ui", "crates/astroforge-render", "astroforge-ui", "astroforge-render")


def code_lines(path: Path):
    """Yield (lineno, text) skipping comment-only lines so doc comments that
    merely *mention* the other tree (e.g. 'ports src-tauri/src/commands_x.rs')
    do not count as references."""
    for i, raw in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
        s = raw.lstrip()
        if s.startswith("//") or s.startswith("#") or s.startswith("/*") or s.startswith("*"):
            continue
        yield i, raw


def find_refs(path: Path, patterns):
    hits = []
    if path.suffix not in (".rs", ".toml", ".ts", ".svelte", ".vue", ".js"):
        return hits
    for i, line in code_lines(path):
        for pat in patterns:
            if pat in line:
                hits.append(f"{path.relative_to(ROOT)}:{i}: {line.strip()[:120]}")
                break
    return hits


def dep_names(crate_dir: Path):
    """astroforge-* dependency names declared in a crate's Cargo.toml."""
    toml = crate_dir / "Cargo.toml"
    if not toml.exists():
        return set()
    names, section = set(), None
    for raw in toml.read_text(encoding="utf-8").splitlines():
        s = raw.strip()
        if s.startswith("["):
            section = s
            continue
        if section and section.startswith("[") and ("dependencies" in section):
            m = re.match(r"([A-Za-z0-9_-]+)\s*=", s)
            if m and m.group(1).startswith("astroforge"):
                names.add(m.group(1))
    return names


def static_check() -> list[str]:
    errors = []
    # 1. egui crates must not reference the web tree
    for egui_dir in EGUI_DIRS:
        for f in sorted((ROOT / egui_dir).rglob("*")):
            if f.is_file():
                for hit in find_refs(f, WEB_REF_PATTERNS):
                    errors.append(f"egui tree references web UI: {hit}")
    # 2/4. src-tauri must not depend on egui crates (covered by allow-list)
    # 3. web trees (src/ + src-vite/) must not reference the Rust UI crates
    for web_dir in WEB_DIRS:
        for f in sorted((ROOT / web_dir).rglob("*")):
            if f.is_file():
                for hit in find_refs(f, EGUI_REF_PATTERNS):
                    errors.append(f"web tree references egui crates: {hit}")
    # 4. per-crate astroforge-* dep allow-list
    for crate, allowed in DEP_ALLOWLIST.items():
        extra = dep_names(ROOT / crate) - allowed
        if extra:
            errors.append(
                f"{crate}/Cargo.toml depends on {sorted(extra)} — outside the "
                f"UI-isolation allow-list {sorted(allowed)} (docs/BRANCHING.md)"
            )
    return errors


def pr_check(number: int, repo: str) -> list[str]:
    out = subprocess.run(
        ["gh", "pr", "view", str(number), "--repo", repo,
         "--json", "labels,files,baseRefName,headRefName"],
        capture_output=True, text=True,
    )
    if out.returncode != 0:
        return [f"gh pr view failed: {out.stderr.strip()}"]
    pr = json.loads(out.stdout)
    labels = {l["name"] for l in pr.get("labels", [])}
    files = [f["path"] for f in pr.get("files", [])]
    base, head = pr.get("baseRefName", ""), pr.get("headRefName", "")
    errors = []

    def touches(paths, prefixes, exact):
        return any(p.startswith(prefixes) or p in exact for p in paths)

    web_touch = touches(files, WEB_DIRS, WEB_FILES)
    egui_touch = touches(files, EGUI_DIRS, frozenset())

    # 5. merge direction: egui/* -> main only at cutover
    if base == "main" and head.startswith("egui/") and "cutover" not in labels:
        errors.append(
            f"PR #{number} merges egui branch '{head}' into main. Model B forbids "
            "egui->main merges except the retirement cutover — label the PR "
            "'cutover' if this is the sanctioned switch (docs/BRANCHING.md)."
        )
    # 6. dual-tree PRs need an explicit opt-out
    if web_touch and egui_touch and "allow-cross-tree" not in labels:
        errors.append(
            f"PR #{number} touches both UI trees (web + egui). Label it "
            "'allow-cross-tree' only for sanctioned cross-tree work "
            "(docs/BRANCHING.md)."
        )
    return errors


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--pr", type=int, help="also gate this PR's merge direction / file set")
    ap.add_argument("--repo", help="owner/repo for --pr")
    args = ap.parse_args()

    errors = static_check()
    if args.pr:
        if not args.repo:
            print("--repo is required with --pr")
            return 1
        errors += pr_check(args.pr, args.repo)

    if errors:
        print("UI ISOLATION VIOLATION (Model B — docs/BRANCHING.md):")
        for e in errors:
            print(f"  - {e}")
        return 1
    print("UI isolation OK: web and egui trees share only astroforge-core/-ai (+ data dir).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
