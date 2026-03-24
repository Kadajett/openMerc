#!/usr/bin/env bash
set -e

# Build release
cargo build --release

# Install binary to /usr/local/bin
BIN_NAME=openmerc
TARGET="target/release/$BIN_NAME"
if [ -f "$TARGET" ]; then
  sudo cp "$TARGET" /usr/local/bin/$BIN_NAME
  echo "Installed $BIN_NAME to /usr/local/bin"
else
  echo "Binary not found at $TARGET"
  exit 1
fi
