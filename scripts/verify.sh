#!/usr/bin/env bash
# Numeris — local verification script
#
# Runs every check that CI runs (minus the OS matrix) so a developer can
# verify a change in one command before pushing:
#
#   1. cargo fmt --all --check     (formatting)
#   2. cargo clippy --all-targets -- -D warnings   (lints; warnings are errors)
#   3. cargo test --all            (full engine suite incl. NIST StRD Longley
#                                   and golden validation fixtures)
#   4. cd apps/desktop && bun install   (frontend dependencies)
#   5. bunx tsc --noEmit           (TypeScript typecheck)
#   6. bunx vite build             (frontend production build)
#
# Requires: Rust stable toolchain, bun >= 1.1.
# Exit code is non-zero if any step fails (set -euo pipefail).

set -euo pipefail

# Always run from the repository root, regardless of where the script was
# invoked from. The Rust workspace manifest lives at the repo root, so
# plain cargo commands resolve the whole workspace
# (crates/numeris-* + apps/desktop/src-tauri).
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "==> Numeris verification (docs/DEVELOPER_GUIDE.md §3)"

echo "==> [1/6] cargo fmt --all --check"
cargo fmt --all --check

echo "==> [2/6] cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets -- -D warnings

echo "==> [3/6] cargo test --all  (engine + NIST StRD + golden fixtures)"
cargo test --all

# The frontend lives in apps/desktop. CI mirrors exactly these steps
# (.github/workflows/ci.yml); bun install falls back to a plain install if
# the lockfile is not yet frozen, matching CI's behavior.
echo "==> [4/6] bun install (apps/desktop)"
(
  cd apps/desktop
  bun install --frozen-lockfile || bun install
)

echo "==> [5/6] bunx tsc --noEmit (frontend typecheck)"
(
  cd apps/desktop
  bunx tsc --noEmit
)

echo "==> [6/6] bunx vite build (frontend production build)"
(
  cd apps/desktop
  bunx vite build
)

echo "ALL CHECKS PASSED"
