#!/usr/bin/env bash
set -e

# Build the release binary
cargo build --release

# Install to /usr/local/bin
install -Dm755 target/release/openmerc /usr/local/bin/openmerc

echo "openmerc installed to /usr/local/bin"
