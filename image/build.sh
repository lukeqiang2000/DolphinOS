#!/bin/sh
# DolphinOS 镜像构建入口（第一阶段：amd64 / Debian sid）
# 用法：在装有 live-build 的 Debian（或 Debian 容器/虚机）里以 root 运行 ./build.sh
# 本目录的 config/ 树（package-lists、hooks、includes.chroot）会被 lb config/lb build 自动读取。
set -e
cd "$(dirname "$0")"

lb clean

# 说明：
# - distribution sid：底座 Debian unstable（用户定案）。
# - archive-areas 含 non-free-firmware：保证无线网卡等固件可直接用。
# - --security false / --updates false：sid 没有 -security 与 -updates 套件，开着会因源不存在而失败。
# - --bootloaders "grub-pc,grub-efi"：GRUB 2 同时覆盖 BIOS 与 UEFI 启动（用户定案 GRUB 2）。
#   注意必须写成一个逗号分隔的单参数：lb config 的 --bootloaders 只取一个值，
#   写成两个词时第二个词会被静默丢弃，LB_BOOTLOADER_EFI 变空、EFI 引导镜像不生成，
#   最后 xorriso 写 hybrid ISO 时报 El Torito EFI 错误（首轮构建实测踩坑）。
# - 目标 Debian 版 live-build 的参数名；Ubuntu 老版 live-build（3.0~a57）参数略有差异
#   （--bootloader 单数等），在 Ubuntu 上构建时按 lb config --help 对照调整。
lb config \
    --mode debian \
    --distribution sid \
    --architectures amd64 \
    --archive-areas "main contrib non-free non-free-firmware" \
    --binary-images iso-hybrid \
    --bootloaders "grub-pc,grub-efi" \
    --security false \
    --updates false \
    --mirror-bootstrap "http://deb.debian.org/debian/" \
    --mirror-chroot "http://deb.debian.org/debian/" \
    --mirror-binary "http://deb.debian.org/debian/" \
    --debian-installer false \
    "$@"

lb build

echo "构建完成，产物："
ls -lh ./*.iso 2>/dev/null || true
