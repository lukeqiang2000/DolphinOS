# dolphin-photos — DolphinOS 照片查看器

Rust + Slint 自研照片查看器，图像解码用 `image` crate（png/jpeg/gif/bmp/webp）。二进制名 `dolphin-photos`。

## 功能

- 扫描目录（不递归）：只收图片扩展名（png/jpg/jpeg/gif/bmp/webp，不区分大小写），按名称不区分大小写排序。
- 用 `image::image_dimensions` 读每张图的尺寸；读不了的损坏文件跳过并计数，在信息栏提示跳过数量。
- 中央大图区显示当前照片预览：经 `make_thumbnail` 等比缩放（最长边 1600px）生成 RGBA 像素后转 `slint::SharedPixelBuffer` 显示，`image-fit: contain` 保持比例。
- 底部栏：上一张 / 下一张按钮（首尾循环），信息显示「第 i / n 张 · 文件名 · WxH」。
- 键盘：左右方向键切换上一张/下一张（`FocusScope` 的 `key-pressed`）。
- 「打开文件夹」按钮：用 `xdg-open` 在文件管理器里打开当前照片目录。
- 照片目录由命令行参数指定：`dolphin-photos [目录]`，缺省为 `~/Pictures`（不存在时用当前目录）。

## 用法

```sh
cargo run -p dolphin-photos -- [照片目录]
dolphin-photos ~/Pictures
```

## 结构

- `src/logic.rs`：纯逻辑（`scan_photos`、`PhotoIndex` 循环导航、`make_thumbnail`），含单元测试（临时目录内用 `image` crate 生成小 PNG 做测试图）。
- `src/main.rs`：Slint 窗口接线、预览图生成与信息栏刷新。
- `ui/app-window.slint`：界面（大图区 + 底部导航栏 + 键盘处理）。

## 未做到的部分

- 无文件夹选择对话框：「打开文件夹」只是用系统文件管理器打开当前目录，换目录要靠命令行参数重启。
- 无递归扫描子目录、无幻灯片播放、无缩放/旋转/拖拽平移查看。
- GIF 只显示解码后的第一帧（`image::open` 语义），无动画播放；无 EXIF 方向自动校正、无 RAW 支持。
- 无缩略图条/网格总览，切换大图时同步解码，超大图集逐张切换可能有轻微卡顿。
