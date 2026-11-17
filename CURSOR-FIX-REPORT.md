# DolphinOS 虚拟机光标不可见修复报告

时间：2026-11-17 晚（Muse 子代理执行）
分支：`boot-experience`（基于 Alpha 7，未合并 main、未触发构建）

## 问题

用户 2026-11-17 在 VirtualBox 试跑 Alpha 6 反馈「一直没有鼠标」：右键菜单能点出来（点击有效，18:14 已证实），但屏幕上看不见光标。典型原因是 wlroots 在虚拟机里硬件光标平面（hardware cursor plane）渲染失败——合成器以为光标已由硬件平面显示，实际没画出来，于是光标不可见但指针逻辑正常。

## 改动（1 个文件）

`image/config/includes.chroot/usr/local/bin/dolphin-session`：

- 在脚本开头（LOG_FILE 定义后、第一次启动 labwc 之前）加一行 `export WLR_NO_HARDWARE_CURSORS=1`，强制 wlroots 用软件光标。
- export 一次即覆盖两条启动路径：默认渲染首启、以及失败后 `WLR_RENDERER=pixman` 的兜底重试（重试只是在命令前加环境变量前缀，不影响已 export 的变量）。
- 文件头注释新增「背景二（光标）」说明来由；原有兜底逻辑（10 秒快速失败判定、退出码透传、日志路径）一字未动。

## 本地验证

- `sh -n` 语法检查通过；文件仍为 755 可执行。
- 模拟验证（假 labwc 打印自身环境后退出）：
  - 情形 A（首启成功）：labwc 拿到 `WLR_NO_HARDWARE_CURSORS=1`、无 `WLR_RENDERER`，脚本透传退出码 0。
  - 情形 B（首启快速失败 rc=3）：自动重试，第二次 labwc 同时拿到 `WLR_NO_HARDWARE_CURSORS=1` 与 `WLR_RENDERER=pixman`，透传退出码 0。
  两条路径都确认带上了软件光标开关。

## 光标主题核查结论（实查，非推测）

- 配置层：`image/config/package-lists/dolphinos.list.chroot` 里**没有显式列**任何图标/光标主题包，`image/config/` 与 `build.sh` 里也没有 `XCURSOR_THEME`/`XCURSOR_SIZE` 设置。
- 镜像层（实读 Alpha 7 的 `check7/filesystem.squashfs` 内 dpkg status 与文件列表）：**光标主题实际不缺**——`adwaita-icon-theme` 51.0-1 已装（包列表没写、是依赖传递带入的），`/usr/share/icons/Adwaita/cursors/` 有 69 个光标文件，`/usr/share/icons/default/index.theme` 默认主题解析链也在；另有 `hicolor-icon-theme`、`libxcursor1`、`libwayland-cursor0`。
- 结论：「主题/尺寸缺失」不是这次光标不可见的根因，不需要为此加包；根因仍指向硬件光标平面，本次的软件光标开关对症。
- Alpha 8 建议项（未擅自改）：把 `adwaita-icon-theme` 显式写进包列表——它现在靠传递依赖进来，哪天上游依赖关系变了可能悄悄消失，显式声明更稳妥。是否采纳请主代理/用户定。

## 未验证项

- 真机 VirtualBox 里光标是否实际出现，要等 Alpha 8 构建（并入 boot-experience 分支）后由用户试跑确认；本次只做到脚本层与模拟验证。
- VMware 等其他虚拟机同理未验（原理相同，预期一并受益）。
