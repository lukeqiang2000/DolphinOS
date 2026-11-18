#!/usr/bin/env python3
# DolphinOS 开机体验素材生成器
#
# Alpha 10 起，海豚标志统一使用用户提供的官方版本
# （仓库根 assets/dolphin-logo.svg，用户 2026-11-18 提供、令全系统
# 换用）。硬规则：海豚本体的路径/比例/配色一律原样，只许等比缩放与
# 栅格化；官方 SVG 仅去掉了一层独立的白色背景 rect（让背景透明），
# 本体零改动。本生成器只做：栅格化、整图平移/合成、以及在海豚之外
# 添加水花/文字等独立元素，绝不重绘海豚本体。
#
# 产出：
#   1. image/config/bootloaders/splash.svg
#      —— GRUB 菜单背景：品牌蓝渐变 + 内嵌官方标志栅格图（data URI）
#         + DolphinOS/ALPHA 字样；live-build 转 800x600 splash.png。
#   2. image/config/bootloaders/grub-pc/live-theme/select_*.png
#      —— GRUB 主题选中项九宫格切片（Dolphin 蓝圆角条，与海豚无关）。
#   3. image/config/includes.chroot/usr/share/plymouth/themes/dolphin/
#      animation-XXXX.png 与 throbber-XXXX.png（同一批帧、两组名字）
#      —— Plymouth two-step 主题 12 帧：官方海豚沿跃起弧线上下运动
#         （整图平移，本体零改动），低点时下方水花环扩散；无加载条。
#         script 插件在 plymouth 26.134.222 里必崩（见 ALPHA10-REPORT.md），
#         故动画走 two-step 的帧序列实现。
#   4. apps/{shell,greeter,oobe}/ui/dolphin.png
#      —— 三个自研应用的品牌图（顶栏海豚按钮/登录屏/向导），512px 宽。
#
# 重新生成：python3 image/boot-assets/generate.py
# 依赖：Pillow、cairosvg（或 rsvg-convert）与 DejaVu Sans 字体。

import base64
import io
import math
import os
import subprocess
import sys

from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
IMAGE_DIR = os.path.dirname(HERE)  # .../image
REPO_DIR = os.path.dirname(IMAGE_DIR)  # 仓库根
LOGO_SVG = os.path.join(REPO_DIR, "assets", "dolphin-logo.svg")
SPLASH_SVG = os.path.join(IMAGE_DIR, "config", "bootloaders", "splash.svg")
GRUB_THEME_DIR = os.path.join(IMAGE_DIR, "config", "bootloaders", "grub-pc", "live-theme")
PLYMOUTH_DIR = os.path.join(
    IMAGE_DIR, "config", "includes.chroot", "usr", "share", "plymouth", "themes", "dolphin"
)
APP_UI_DIRS = [
    os.path.join(REPO_DIR, "apps", app, "ui") for app in ("shell", "greeter", "oobe")
]

FONT_BOLD = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
FONT_REGULAR = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"

TITLE = (255, 255, 255)
SUBTITLE = (169, 204, 255)
ACCENT = (19, 112, 243)  # Dolphin 蓝 #1370f3
SPLASH_RING = (168, 208, 255)  # 水花浅蓝


def load_logo_png(width):
    """把官方标志 SVG 栅格化为指定宽度的透明 PNG（等比缩放，本体零改动）。"""
    try:
        import cairosvg

        data = cairosvg.svg2png(url=LOGO_SVG, output_width=width)
        return Image.open(io.BytesIO(data)).convert("RGBA")
    except ImportError:
        pass
    # 退路：rsvg-convert
    proc = subprocess.run(
        ["rsvg-convert", "-w", str(width), LOGO_SVG],
        capture_output=True, check=True,
    )
    return Image.open(io.BytesIO(proc.stdout)).convert("RGBA")


# ---------------------------------------------------------------------------
# 1) GRUB / syslinux 共用 splash.svg（800x600 画布）
# ---------------------------------------------------------------------------
def write_splash_svg():
    logo = load_logo_png(400)
    buf = io.BytesIO()
    logo.save(buf, format="PNG", optimize=True)
    b64 = base64.b64encode(buf.getvalue()).decode("ascii")
    svg = f"""<?xml version="1.0" encoding="UTF-8"?>
<!-- DolphinOS 引导菜单背景（live-build 自动转 PNG：GRUB 800x600 / syslinux 640x480）
     由 image/boot-assets/generate.py 生成，请勿手改本文件，改生成器后重跑。
     海豚为用户提供的官方标志（assets/dolphin-logo.svg）栅格化内嵌，本体零改动。 -->
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="800" height="600" viewBox="0 0 800 600">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#0d63d8"/>
      <stop offset="0.45" stop-color="#0b4da8"/>
      <stop offset="1" stop-color="#05224d"/>
    </linearGradient>
    <radialGradient id="glow" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="#ffffff" stop-opacity="0.22"/>
      <stop offset="1" stop-color="#ffffff" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <rect width="800" height="600" fill="url(#bg)"/>
  <ellipse cx="400" cy="165" rx="330" ry="170" fill="url(#glow)"/>
  <image x="200" y="30" width="400" height="267" xlink:href="data:image/png;base64,{b64}"/>
  <text x="400" y="356" text-anchor="middle" font-family="DejaVu Sans"
        font-weight="bold" font-size="44" letter-spacing="1" fill="#ffffff">DolphinOS</text>
  <text x="400" y="388" text-anchor="middle" font-family="DejaVu Sans"
        font-size="17" letter-spacing="6" fill="#a9ccff">ALPHA</text>
  <path d="M250 522 C300 508 340 536 390 522 S480 508 530 522" fill="none"
        stroke="#7db4ff" stroke-opacity="0.45" stroke-width="3" stroke-linecap="round"/>
  <path d="M300 545 C335 535 365 555 400 545 S465 535 500 545" fill="none"
        stroke="#7db4ff" stroke-opacity="0.25" stroke-width="3" stroke-linecap="round"/>
</svg>
"""
    os.makedirs(os.path.dirname(SPLASH_SVG), exist_ok=True)
    with open(SPLASH_SVG, "w", encoding="utf-8") as f:
        f.write(svg)
    print("splash.svg:", SPLASH_SVG, len(svg), "bytes")


# ---------------------------------------------------------------------------
# 2) GRUB 选中项九宫格切片（24x24 圆角蓝条切 9 块）
# ---------------------------------------------------------------------------
def write_select_pixmaps():
    os.makedirs(GRUB_THEME_DIR, exist_ok=True)
    img = Image.new("RGBA", (24, 24), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([0, 0, 23, 23], radius=8, fill=ACCENT + (255,))
    boxes = {
        "nw": (0, 0, 8, 8), "n": (8, 0, 16, 8), "ne": (16, 0, 24, 8),
        "w": (0, 8, 8, 16), "c": (8, 8, 16, 16), "e": (16, 8, 24, 16),
        "sw": (0, 16, 8, 24), "s": (8, 16, 16, 24), "se": (16, 16, 24, 24),
    }
    for name, box in boxes.items():
        img.crop(box).save(os.path.join(GRUB_THEME_DIR, f"select_{name}.png"))
    print("select_*.png: 9 files in", GRUB_THEME_DIR)


# ---------------------------------------------------------------------------
# 3) Plymouth 动画帧（1024x640 透明底，12 帧循环）
#    官方海豚整图沿跃起弧线平移（本体零改动）；低点时下方水花环扩散、
#    水滴扬起（独立元素）。two-step 主题把帧居中显示在品牌蓝渐变上。
# ---------------------------------------------------------------------------
def write_frames(count=12):
    os.makedirs(PLYMOUTH_DIR, exist_ok=True)
    logo = load_logo_png(620)  # 620x413
    title_font = ImageFont.truetype(FONT_BOLD, 58)
    sub_font = ImageFont.truetype(FONT_REGULAR, 25)
    cx = 512
    base_cy = 262  # 海豚中心基准高度
    water_y = 486  # 水面线（水花环中心）
    for i in range(count):
        phase = 2 * math.pi * i / count
        # 跃起弧线：i=3 最高（dy=-30），i=9 最低（dy=+30）
        dy = round(-30 * math.sin(phase))
        cy = base_cy + dy
        low = (dy + 30) / 60.0  # 0=最高点，1=最低点
        frame = Image.new("RGBA", (1024, 640), (0, 0, 0, 0))
        d = ImageDraw.Draw(frame)
        # 水花：椭圆环（低点时更宽更亮）+ 两侧水滴（低点时扬得更高）
        rx = 150 + 95 * low
        ry = 13 + 7 * low
        alpha = int(70 + 150 * low)
        d.ellipse([cx - rx, water_y - ry, cx + rx, water_y + ry],
                  outline=SPLASH_RING + (alpha,), width=3)
        rx2 = rx * 0.62
        d.ellipse([cx - rx2, water_y - ry * 0.6, cx + rx2, water_y + ry * 0.6],
                  outline=SPLASH_RING + (max(30, alpha - 70),), width=2)
        for k, off in enumerate((-150, -95, 95, 150)):
            lift = (8 + 22 * low) * (1.0 if k % 2 == 0 else 0.7)
            r = 4.5 - (k % 2)
            dxp = off * (0.8 + 0.35 * low)
            d.ellipse([cx + dxp - r, water_y - lift - r, cx + dxp + r, water_y - lift + r],
                      fill=SPLASH_RING + (alpha,))
        # 海豚本体：整图平移贴入，像素零改动
        frame.alpha_composite(logo, (cx - logo.width // 2, cy - logo.height // 2))
        d = ImageDraw.Draw(frame)
        d.text((512, 540), "DolphinOS", font=title_font, anchor="mm",
               fill=TITLE + (235,))
        d.text((512, 592), "A L P H A", font=sub_font, anchor="mm",
               fill=SUBTITLE + (225,))
        # two-step 主题按 animation-%04d / throbber-%04d 两组名字取帧，
        # 同一批帧写两份（序号从 1 起）。
        for prefix in ("animation", "throbber"):
            path = os.path.join(PLYMOUTH_DIR, f"{prefix}-{i + 1:04d}.png")
            frame.save(path, optimize=True)
            print(f"{prefix}-{i + 1:04d}.png:", os.path.getsize(path), "bytes")


# ---------------------------------------------------------------------------
# 4) 自研应用品牌图（Shell 顶栏按钮 / 登录屏 / 向导），512px 宽透明底
# ---------------------------------------------------------------------------
def write_app_logos():
    logo = load_logo_png(512)
    for ui_dir in APP_UI_DIRS:
        path = os.path.join(ui_dir, "dolphin.png")
        logo.save(path, optimize=True)
        print("app logo:", path, os.path.getsize(path), "bytes")


if __name__ == "__main__":
    if not os.path.exists(LOGO_SVG):
        sys.exit(f"official logo not found: {LOGO_SVG}")
    write_splash_svg()
    write_select_pixmaps()
    write_frames()
    write_app_logos()
