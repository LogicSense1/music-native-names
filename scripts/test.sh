#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}"
mkdir -p "$CARGO_TARGET_DIR"
cargo fmt --all -- --check
cargo test --locked --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
xcrun clang++ -std=c++17 -Wall -Wextra -Werror tests/native_duration.cpp -o "$CARGO_TARGET_DIR/native-duration-test"
"$CARGO_TARGET_DIR/native-duration-test"
# Compile the actual bridge for Music's arm64e ABI; never attach to Music in CI.
xcrun clang++ -std=c++17 -arch arm64e -fsyntax-only tests/native_syntax.mm
printf 'Rust checks, native duration assertions, and arm64e bridge syntax passed.\n'
