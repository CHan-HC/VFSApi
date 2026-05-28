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

# HarmonyOS SDK 路径
SDK_HOME="${HARMONY_SDK_HOME:-/Applications/DevEco-Studio.app/Contents/sdk/default}"
CLANG="$SDK_HOME/openharmony/native/llvm/bin/aarch64-unknown-linux-ohos-clang"
CLANGXX="$SDK_HOME/openharmony/native/llvm/bin/aarch64-unknown-linux-ohos-clang++"
SYSROOT="$SDK_HOME/openharmony/native/sysroot"

# 设置 C 交叉编译环境变量（用于 ring / aws-lc-sys 等 C 依赖）
# cc crate 将 target triple 的 '-' 替换为 '_' 来查找环境变量
export CC_aarch64_unknown_linux_ohos="$CLANG"
export CXX_aarch64_unknown_linux_ohos="$CLANGXX"
export CFLAGS_aarch64_unknown_linux_ohos="--sysroot=$SYSROOT"

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
