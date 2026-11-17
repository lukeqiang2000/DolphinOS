#!/usr/bin/env python3
# DolphinOS 开机体验素材生成器
#
# 用同一套海豚轮廓控制点（Catmull-Rom 样条采样）生成三处素材，保证
# GRUB 菜单背景、GRUB 选中框与 Plymouth 开机动画里的海豚长得一致：
#   1. image/config/bootloaders/splash.svg
#      —— live-build 的 binary_bootloader_splash 会把它转成 800x600 的
#         boot/grub/splash.png（GRUB 背景图；若将来切回 syslinux 则转
#         640x480 的 isolinux/splash.png，同一文件复用）。
#   2. image/config/bootloaders/grub-pc/live-theme/select_*.png
#      —— GRUB 主题里选中项的九宫格切片（Dolphin 蓝圆角条）。
#   3. image/config/includes.chroot/usr/share/plymouth/themes/dolphin/frame-XX.png
#      —— Plymouth script 主题的 12 帧游动动画（透明底，背景渐变由
#         dolphin.script 用 Window.SetBackground*Color 画）。
#
# 重新生成：python3 image/boot-assets/generate.py
# 依赖：Pillow（PIL）与 DejaVu Sans 字体（构建机/开发机标配）。

import math
import os

from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
IMAGE_DIR = os.path.dirname(HERE)  # .../image
SPLASH_SVG = os.path.join(IMAGE_DIR, "config", "bootloaders", "splash.svg")
GRUB_THEME_DIR = os.path.join(IMAGE_DIR, "config", "bootloaders", "grub-pc", "live-theme")
PLYMOUTH_DIR = os.path.join(
    IMAGE_DIR, "config", "includes.chroot", "usr", "share", "plymouth", "themes", "dolphin"
)

FONT_BOLD = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
FONT_REGULAR = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"

# ---------------------------------------------------------------------------
# 海豚轮廓：局部坐标（1000x700，朝左），闭合控制点列，经 Catmull-Rom 采样。
# 顺序：吻尖 → 额隆 → 头顶 → 背 → 背鳍 → 背后段 → 尾柄 → 尾鳍上叶 →
#       尾鳍缺口 → 尾鳍下叶 → 尾柄下 → 腹 → 喉 → 颌下 → 吻下。
# ---------------------------------------------------------------------------
OUTLINE = [
    (60, 382), (120, 338), (185, 300), (275, 268), (400, 240),
    (505, 232), (548, 205), (575, 160), (592, 215), (615, 242),
    (730, 262), (838, 292), (905, 262), (978, 208), (940, 302),
    (972, 392), (880, 348), (760, 382), (600, 408), (430, 420),
    (300, 410), (168, 402), (84, 396),
]
# 胸鳍（独立折线）与嘴线（独立折线）、眼睛（圆心，局部坐标）
FIN = [(418, 402), (452, 466), (494, 486), (472, 410)]
MOUTH = [(76, 388), (150, 382), (208, 378)]
EYE = (214, 330)

LINE = (220, 238, 255)        # 线稿近白蓝
TITLE = (255, 255, 255)
SUBTITLE = (169, 204, 255)
ACCENT = (19, 112, 243)       # Dolphin 蓝 #1370f3


def sample_closed(points, steps=16):
    """闭合 Catmull-Rom 样条采样，返回平滑点列。"""
    n = len(points)
    out = []
    for i in range(n):
        p0 = points[(i - 1) % n]
        p1 = points[i]
        p2 = points[(i + 1) % n]
        p3 = points[(i + 2) % n]
        for s in range(steps):
            t = s / steps
            t2, t3 = t * t, t * t * t
            x = 0.5 * ((2 * p1[0]) + (-p0[0] + p2[0]) * t
                       + (2 * p0[0] - 5 * p1[0] + 4 * p2[0] - p3[0]) * t2
                       + (-p0[0] + 3 * p1[0] - 3 * p2[0] + p3[0]) * t3)
            y = 0.5 * ((2 * p1[1]) + (-p0[1] + p2[1]) * t
                       + (2 * p0[1] - 5 * p1[1] + 4 * p2[1] - p3[1]) * t2
                       + (-p0[1] + 3 * p1[1] - 3 * p2[1] + p3[1]) * t3)
            out.append((x, y))
    return out


def svg_path(points, close=True):
    d = "M " + " L ".join(f"{x:.1f} {y:.1f}" for x, y in points)
    return d + (" Z" if close else "")


# ---------------------------------------------------------------------------
# 1) GRUB / syslinux 共用 splash.svg（800x600 画布）
# ---------------------------------------------------------------------------
def write_splash_svg():
    outline = sample_closed(OUTLINE)
    fin = sample_closed(FIN, steps=10)[:-6]  # 胸鳍近似平滑即可，不闭合
    svg = f"""<?xml version="1.0" encoding="UTF-8"?>
<!-- DolphinOS 引导菜单背景（live-build 自动转 PNG：GRUB 800x600 / syslinux 640x480）
     由 image/boot-assets/generate.py 生成，请勿手改本文件，改生成器后重跑。 -->
<svg xmlns="http://www.w3.org/2000/svg" width="800" height="600" viewBox="0 0 800 600">
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
  <ellipse cx="400" cy="150" rx="330" ry="150" fill="url(#glow)"/>
  <g transform="translate(192,21) scale(0.40)">
    <path d="{svg_path(outline)}" fill="#1370f3" fill-opacity="0.10"
          stroke="#dceeff" stroke-width="11" stroke-linejoin="round" stroke-linecap="round"/>
    <path d="{svg_path(fin, close=False)}" fill="none"
          stroke="#dceeff" stroke-width="10" stroke-linejoin="round" stroke-linecap="round"/>
    <path d="{svg_path(MOUTH, close=False)}" fill="none"
          stroke="#dceeff" stroke-width="8" stroke-linecap="round"/>
    <circle cx="{EYE[0]}" cy="{EYE[1]}" r="14" fill="#dceeff"/>
  </g>
  <path d="M250 232 C300 218 340 246 390 232 S480 218 530 232" fill="none"
        stroke="#7db4ff" stroke-opacity="0.45" stroke-width="3" stroke-linecap="round"/>
  <path d="M300 252 C335 242 365 262 400 252 S465 242 500 252" fill="none"
        stroke="#7db4ff" stroke-opacity="0.25" stroke-width="3" stroke-linecap="round"/>
  <text x="400" y="296" text-anchor="middle" font-family="DejaVu Sans"
        font-weight="bold" font-size="44" letter-spacing="1" fill="#ffffff">DolphinOS</text>
  <text x="400" y="326" text-anchor="middle" font-family="DejaVu Sans"
        font-size="17" letter-spacing="6" fill="#a9ccff">ALPHA</text>
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
# ---------------------------------------------------------------------------
def draw_dolphin_layer(scale, ox, oy, line_width):
    """在独立透明层上画海豚线稿（局部坐标 → 屏幕坐标）。"""
    layer = Image.new("RGBA", (1024, 640), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)

    def tx(points):
        return [(ox + x * scale, oy + y * scale) for x, y in points]

    d.line(tx(sample_closed(OUTLINE)), fill=LINE + (255,), width=line_width, joint="curve")
    d.line(tx(FIN), fill=LINE + (255,), width=line_width - 1, joint="curve")
    d.line(tx(MOUTH), fill=LINE + (255,), width=line_width - 2, joint="curve")
    ex, ey = ox + EYE[0] * scale, oy + EYE[1] * scale
    r = 8
    d.ellipse([ex - r, ey - r, ex + r, ey + r], fill=LINE + (255,))
    return layer


def write_frames(count=12):
    os.makedirs(PLYMOUTH_DIR, exist_ok=True)
    scale, ox, oy = 0.80, 97, 42
    layer0 = draw_dolphin_layer(scale, ox, oy, line_width=7)
    title_font = ImageFont.truetype(FONT_BOLD, 58)
    sub_font = ImageFont.truetype(FONT_REGULAR, 25)
    for i in range(count):
        phase = 2 * math.pi * i / count
        dx = round(24 * math.sin(phase))
        dy = round(9 * math.sin(phase + 0.9))
        angle = 2.2 * math.sin(phase)
        layer = layer0.rotate(angle, resample=Image.BICUBIC, center=(512, 300))
        frame = Image.new("RGBA", (1024, 640), (0, 0, 0, 0))
        frame.alpha_composite(layer, (dx, dy))
        d = ImageDraw.Draw(frame)
        # 气泡：从吻部附近升起、渐隐
        for j in range(4):
            p = (i * 11 + j * 29) % 110
            bx = 185 + j * 30 + 6 * math.sin(i * 0.7 + j)
            by = 305 - p
            r = 4 + (j % 3) * 2
            alpha = max(30, int(210 - p * 1.7))
            d.ellipse([bx - r, by - r, bx + r, by + r],
                      outline=(180, 215, 255, alpha), width=2)
        d.text((512, 522), "DolphinOS", font=title_font, anchor="mm",
               fill=TITLE + (235,))
        d.text((512, 580), "A L P H A", font=sub_font, anchor="mm",
               fill=SUBTITLE + (225,))
        path = os.path.join(PLYMOUTH_DIR, f"frame-{i:02d}.png")
        frame.save(path, optimize=True)
        print(f"frame-{i:02d}.png:", os.path.getsize(path), "bytes")


if __name__ == "__main__":
    write_splash_svg()
    write_select_pixmaps()
    write_frames()
