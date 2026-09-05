#!/usr/bin/env bash
# STEP-0126: Linux x64 native rerun of the M12 provider corpus and the
# guest-visible http@0.2.0 runner corpus (RFC-0037 §9 platform evidence).
# Runs inside WSL2 Ubuntu-24.04 at /root/sico; offline after cargo fetch.
set -u
cd /root/sico || exit 1
cargo=/root/.cargo/bin/cargo
fail=0

echo "=== rustc ==="
"$cargo" --version 2>/dev/null || /root/.cargo/bin/rustc --version
uname -s -m

echo "=== fmt (workspace + runner) ==="
"$cargo" fmt --all -- --check || fail=1
"$cargo" fmt --manifest-path runner/sico-runner/Cargo.toml -- --check || fail=1

echo "=== clippy (provider, -D warnings) ==="
"$cargo" clippy --locked --offline -p sico-http-provider --all-targets --all-features -- -D warnings || fail=1

echo "=== provider corpus (48) ==="
"$cargo" test --locked --offline -p sico-http-provider || fail=1

echo "=== clippy (runner, -D warnings) ==="
"$cargo" clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --all-targets -- -D warnings || fail=1

echo "=== runner corpus (37 lib + 6 http2 + 37 runner, serial) ==="
"$cargo" test --locked --offline --manifest-path runner/sico-runner/Cargo.toml -- --test-threads=1 || fail=1

echo "=== frozen 0.1.0 oracle (STEP-0089) ==="
"$cargo" test --locked --offline -p sico-package --test package || fail=1

if [ "$fail" -eq 0 ]; then
    echo "STEP-0126 LINUX x64: ALL GREEN"
else
    echo "STEP-0126 LINUX x64: FAILED"
fi
exit "$fail"
