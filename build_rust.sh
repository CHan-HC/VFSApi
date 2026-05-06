#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RUST_DIR="$SCRIPT_DIR/rust_src"
TARGET_DIR="$SCRIPT_DIR/harmony/entry/libs/arm64-v8a"

echo "=========================================="
echo "Building Rust library for HarmonyOS..."
echo "=========================================="

cd "$RUST_DIR"

export PATH="$HOME/.cargo/bin:$PATH"

cargo build --release --target aarch64-unknown-linux-ohos

echo ""
echo "=========================================="
echo "Copying library to HarmonyOS project..."
echo "=========================================="

mkdir -p "$TARGET_DIR"

cp "$RUST_DIR/target/aarch64-unknown-linux-ohos/release/libvfs_apis.so" "$TARGET_DIR/"

echo ""
echo "=========================================="
echo "Build completed successfully!"
echo "=========================================="
echo "Library copied to: $TARGET_DIR/libvfs_apis.so"
ls -lh "$TARGET_DIR/libvfs_apis.so"
