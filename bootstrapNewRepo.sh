#!/usr/bin/env zsh
#
# bootstrapNewRepo.sh - launch the interactive project scaffolder.
#
# This is a thin convenience wrapper. The real tool is the Rust `quickstarter`
# binary under `cli/`. On first run it builds the binary in release mode; after
# that it just runs it. The binary clones the latest template repository at
# runtime, so it always builds from the newest templates even if the binary
# itself is old.
#
# Any arguments are passed straight through to the binary, e.g.:
#   ./bootstrapNewRepo.sh --repo https://github.com/AvandarLabs/quickstarter.git
set -euo pipefail

SCRIPT_DIR=${0:A:h}
CLI_DIR="$SCRIPT_DIR/cli"
BIN="$CLI_DIR/target/release/quickstarter"

if ! command -v cargo >/dev/null 2>&1; then
  echo "Error: cargo (the Rust toolchain) is required to build the scaffolder." >&2
  echo "Install Rust from https://rustup.rs and try again." >&2
  exit 1
fi

if [[ ! -x "$BIN" ]]; then
  echo "Building the scaffolder (first run only)..."
  cargo build --release --manifest-path "$CLI_DIR/Cargo.toml"
fi

exec "$BIN" "$@"
