#!/usr/bin/env zsh
#
# bootstrapNewRepo.sh - launch the interactive project scaffolder.
#
# This is a thin convenience wrapper. The real tool is the Rust `quickstarter`
# binary under `cli/`. It always rebuilds the binary in release mode before
# running it: cargo is incremental, so an unchanged tree costs a fraction of a
# second, and a changed `cli/src` can never be silently ignored. The binary
# clones the latest template repository at runtime, so the templates are always
# the newest ones too.
#
# Any arguments are passed straight through to the binary. Every question has a
# flag, and anything you leave out is asked for; --yes turns the questions off.
# A project has one project type and any number of capabilities:
#   ./bootstrapNewRepo.sh --name "My App" --dir ~/src \
#     --project-type typescript:web --capability tanstack-router --yes
#   ./bootstrapNewRepo.sh --name "My Tool" --project-type rust:cli --yes
#   ./bootstrapNewRepo.sh --repo https://github.com/AvandarLabs/quickstarter.git
#   ./bootstrapNewRepo.sh --help
set -euo pipefail

SCRIPT_DIR=${0:A:h}
CLI_DIR="$SCRIPT_DIR/cli"
BIN="$CLI_DIR/target/release/quickstarter"

if ! command -v cargo >/dev/null 2>&1; then
  echo "Error: cargo (the Rust toolchain) is required to build the scaffolder." >&2
  echo "Install Rust from https://rustup.rs and try again." >&2
  exit 1
fi

# Always rebuild: never run a binary that predates the current `cli/src`.
echo "Building the scaffolder..."
cargo build --release --manifest-path "$CLI_DIR/Cargo.toml"

exec "$BIN" "$@"
