#!/bin/sh
# DolphinOS Alpha 7 CI 容器内构建脚本（在 debian:sid 特权容器里运行，
# 工作区挂载在 /work）。做三件事：
#   1. 装齐 live-build 与 Rust/Slint/feature 构建依赖；
#   2. cargo build --release 编译全部自研应用，strip 后注入
#      image/config/includes.chroot/usr/bin/（不进 git，只在 CI 工作区）；
#      browser 的 webkit feature 与 player 的 gst feature 各试编一次，
#      成功就换用 feature 版二进制，失败降级用默认版并打标记，不卡死 ISO。
#   3. 跑 image/build.sh 构建 ISO。
set -e
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

apt-get update
apt-get install -y --no-install-recommends \
    live-build debootstrap debian-archive-keyring ca-certificates \
    squashfs-tools xorriso mtools dosfstools isolinux syslinux-common \
    grub-pc-bin grub-efi-amd64-bin \
    cargo rustc build-essential pkg-config \
    libfontconfig1-dev libxkbcommon-dev libssl-dev \
    libgtk-3-dev libsoup-3.0-dev libwebkit2gtk-4.1-dev \
    libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev

echo "=== rustc/cargo 版本 ==="
rustc --version || true
cargo --version || true

cd /work/apps
echo "=== cargo build --release（默认构建，全部应用）==="
cargo build --release

BIN_DIR=/work/image/config/includes.chroot/usr/bin
mkdir -p "$BIN_DIR"
APPS="dolphin-shell dolphin-settings dolphin-greeter dolphin-files dolphin-terminal dolphin-browser dolphin-player dolphin-photos dolphin-oobe"
for b in $APPS; do
    strip "target/release/$b"
    cp "target/release/$b" "$BIN_DIR/$b"
    chmod +x "$BIN_DIR/$b"
done
echo "=== 默认构建二进制已注入 ==="
ls -lh "$BIN_DIR"

echo "=== 试编 browser webkit feature ==="
if cargo build --release -p dolphin-browser --features webkit; then
    echo "FEATURE_WEBKIT_RESULT=OK"
    strip target/release/dolphin-browser
    cp target/release/dolphin-browser "$BIN_DIR/dolphin-browser"
    chmod +x "$BIN_DIR/dolphin-browser"
else
    echo "FEATURE_WEBKIT_RESULT=FAIL（降级：ISO 使用默认版 dolphin-browser，导航走 xdg-open）"
fi

echo "=== 试编 player gst feature ==="
if cargo build --release -p dolphin-player --features gst; then
    echo "FEATURE_GST_RESULT=OK"
    strip target/release/dolphin-player
    cp target/release/dolphin-player "$BIN_DIR/dolphin-player"
    chmod +x "$BIN_DIR/dolphin-player"
else
    echo "FEATURE_GST_RESULT=FAIL（降级：ISO 使用默认版 dolphin-player，模拟后端）"
fi

echo "=== 最终注入的二进制 ==="
for b in $APPS; do
    if [ -x "$BIN_DIR/$b" ]; then
        ls -lh "$BIN_DIR/$b"
    else
        echo "缺失二进制: $b" >&2
        exit 1
    fi
done

cd /work/image
chmod +x build.sh
./build.sh
