#!/usr/bin/env bash
# STEP-0109: Linux x64 native runner parity corpus (M11).
# Runs the same scheduler + runner evidence as the Windows validators.
# Requires: native Linux x64 (WSL2 acceptable; cross-compiled binaries are
# NOT evidence), rustup with the pinned toolchain.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export RUSTUP_TOOLCHAIN=1.98.0-x86_64-unknown-linux-gnu
WORK="$ROOT/target/evidence/step-0109/linux"
mkdir -p "$WORK"
rm -rf "$WORK"
mkdir -p "$WORK"

{
  echo "== environment =="
  uname -a
  rustc --version
  ldd --version | head -1
} | tee "$WORK/environment.txt"

cd "$ROOT"

echo "== fmt/clippy =="
cargo fmt --all --check || exit 1
cargo clippy --workspace --all-targets --offline --locked -- -D warnings || exit 1

echo "== scheduler unit corpus =="
cargo test --offline --locked --manifest-path runner/sico-runner/Cargo.toml --lib scheduler || exit 1

echo "== runner integration suite (release, single-threaded) =="
cargo build --release --offline --locked --manifest-path runner/sico-runner/Cargo.toml || exit 1
cargo test --release --offline --locked --manifest-path runner/sico-runner/Cargo.toml \
  --test runner -- --test-threads=1 --nocapture 2>&1 | tee "$WORK/runner-tests.txt"
STATUS=${PIPESTATUS[0]}
[ "$STATUS" -eq 0 ] || { echo "runner-integration-tests-failed|STEP-0109"; exit 1; }

echo "== evidence markers =="
for marker in SCHEDULER_SCALE CHAIN_CANCEL_1024 CHANNEL_RELAY_1GIB GRANT_MATRIX_100 DAP_TERMINATE_20 SCHEDULER_TEARDOWN_100; do
  grep -q "$marker" "$WORK/runner-tests.txt" || { echo "missing evidence: $marker"; exit 1; }
done

echo "STEP_0109_OK platform=linux-x64 corpus=parity evidence=$WORK"
