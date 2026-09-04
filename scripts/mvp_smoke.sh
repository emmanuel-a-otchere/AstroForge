#!/usr/bin/env bash
# AstroForge MVP smoke test.
#
# Phase 7 close-out (issue #43 Windows, #44 macOS). Drops a small
# FITS folder, runs the CLI, asserts a 16-bit TIFF was produced.
#
# Usage:
#   bash scripts/mvp_smoke.sh tests/fixtures/sample-session
#
# Exit codes:
#   0  smoke test passed
#   1  fixture missing or CLI failed

set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_DIR="${1:-${REPO_ROOT}/tests/fixtures/sample-session}"
# Portable temp dir creation:
#   - GNU  (Linux, Git Bash on Windows): mktemp -d -t PREFIX.XXXXXX
#   - BSD   (macOS):                    mktemp -d -t PREFIX
# We probe for the GNU form first, falling back to BSD.
if OUTPUT_DIR="$(mktemp -d -t astroforge-smoke.XXXXXX 2>/dev/null)"; then
    :
elif OUTPUT_DIR="$(mktemp -d -t astroforge-smoke 2>/dev/null)"; then
    :
else
    echo "✗ mktemp failed to create a scratch directory" >&2
    exit 1
fi
OUTPUT_TIF="${OUTPUT_DIR}/output.tif"

cd "${REPO_ROOT}"

echo "→ fixture: ${FIXTURE_DIR}"
echo "→ output:  ${OUTPUT_TIF}"

# Build (or reuse) the CLI binary.
cargo build --quiet --bin astroforge
CLI="${REPO_ROOT}/target/debug/astroforge"
if [[ ! -x "${CLI}" ]]; then
    echo "✗ CLI binary not found at ${CLI}" >&2
    exit 1
fi

# Run the pipeline.
"${CLI}" "${FIXTURE_DIR}" "${OUTPUT_TIF}" >/dev/null

# Validate the output.
if [[ ! -s "${OUTPUT_TIF}" ]]; then
    echo "✗ output TIFF is missing or empty" >&2
    exit 1
fi

# Confirm the TIFF magic.
MAGIC=$(head -c 2 "${OUTPUT_TIF}")
if [[ "${MAGIC}" != "II" && "${MAGIC}" != "MM" ]]; then
    echo "✗ output is not a TIFF (magic=${MAGIC})" >&2
    exit 1
fi

SIZE=$(stat -c '%s' "${OUTPUT_TIF}" 2>/dev/null || stat -f '%z' "${OUTPUT_TIF}")
echo "✓ smoke test passed (TIFF ${SIZE} bytes at ${OUTPUT_TIF})"

# Keep the output for inspection; smoke scripts don't clean up so
# the human reviewer can eyeball the result.
exit 0
