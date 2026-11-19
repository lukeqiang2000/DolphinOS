# DolphinOS 规格（已定案要点）

> 本文件记录用户逐项点选定案的规格，只记录结论，不重新选型。变更须经用户点选确认。

## 1. 底座与形态

- 底座：Debian unstable（sid），定制成完整发行版。
- 形态：支持 Live（光盘/U 盘直接进桌面体验），Live 里带一键安装到硬盘。
- 架构：x64、ARM、x32、LoongArch（龙芯 loong64）四架构；第一阶段只做 x64（amd64）。
- 引导器：GRUB 2（四架构全支持，配海豚主题）。
- 选型总判据：流畅、占用低。所有组件选型以此为准。

## 2. 窗口系统与桌面

- 窗口系统：Wayland；合成器 labwc（wlroots 系轻量堆叠合成器，支持 layer-shell）。
- 桌面 Shell：自研，用 Slint（Rust）编写，原生渲染、不用 WebView。
- 窗口形态：普通窗口，可拖动、缩放、并排（labwc 堆叠式）。
- 登录屏：自研图形登录屏（Slint），输密码进桌面；另支持指纹（fprintd）与人脸解锁（人脸方案开工时按设备定）。第一阶段先用 greetd + agreety 占位跑通登录链。

## 3. 主题（形态级皮肤，非只换配色）

- Shell 必须支持多主题切换；切主题 = 整套桌面形态随之改变。
- Dolphin（默认）：squircle 超椭圆轮廓（自编超椭圆公式生成多边形点列绘制，与网站同一套算法，见 apps/dolphin-shape）。
- Metro：整个系统模仿 Windows 8（开始屏幕磁贴、全屏应用）。
- Fluent：模仿 Windows 10（任务栏、开始菜单、Acrylic + Reveal）。
- Pixel：仿 Google Pixel（Material You 动态取色 + Pixel 桌面形态）。

## 4. Dolphin 主题桌面细节

- 布局：板块网格桌面（同网站首页卡片板块）。卡片两类：常用应用直接成卡点开运行，其余按板块收纳、点开进板块页。视觉向网站 Dolphin 主题看齐（Alpha 10 起）：同配色（底 #0f172a、卡片 #1e293b、强调 #6366f1、双色光晕背景）与自编 squircle 轮廓（与网站同一套超椭圆算法）。
- 打开动画（Alpha 10 改定，取代旧「卡片原地翻面」口径）：点卡片后卡片慢慢放大到窗口大小、放大过程中同时绕竖轴翻转、结束时正好变成打开的窗口；Slint 侧做放大翻转替身动画与 labwc 窗口出现时机对齐，时长 520ms（缓动 easeInOutCubic），翻到中点启动应用、后段替身淡出交棒。动画两级可关：关一级降为弹入，再关则无动画直接打开；其他界面过渡动画同样可在设置里关。
- 顶栏：一条很窄的顶栏，只放时间和状态图标。
- 已打开应用切换：只用 Alt+Tab，不做任务概览、不做卡片运行标记。
- 通知：仿 macOS 横幅式，下滑可展开看详情。
- 找应用入口（Alpha 10 改定，原左上角板块方案作废）：顶栏左上角一个海豚图标按钮（官方海豚标志，见第 5 节），点一下弹出已实现的应用搜索/启动面板（搜索框 + 全部应用列表）。
- 桌面卡片：可拖动排序、可拖进拖出板块、位置记住。
- 设置与电源：桌面一张系统板块卡，设置、关机、重启收在里面。
- 桌面可直接放文件（放网格空位）。

## 5. 官方标志、开机与首次设置

- 官方海豚标志（Alpha 10 定，用户提供）：蓝色跃起海豚，唯一官方版本存仓库 `assets/dolphin-logo.svg`（仅去掉独立白色背景层，本体路径/比例/配色零改动，只许等比缩放与栅格化）。全系统统一用它：Shell 顶栏按钮、GRUB 主题、Plymouth 开机动画、登录屏、首次设置向导品牌图；取代 Alpha 9 的自绘实心海豚。素材由 `image/boot-assets/generate.py` 从该 SVG 生成。
- 开机画面：官方海豚动画（整图沿跃起弧线运动＋水花，形状不动），无加载条；风格参考 Google Pixel 手机首次开机动画。实现走 Plymouth two-step 帧序列（script 插件在 plymouth 26.134.222 必崩，Alpha 10 取证后弃用）。衔接：动画渐变终点色与桌面/向导/登录屏背景同为 #0f172a，向导第一页与登录屏都带同款海豚图——开机动画结束到向导（Live）/登录屏（装盘）不黑屏不闪断，视觉上是同一段流程。
- 硬件适配打包：镜像含 firmware-linux、firmware-linux-nonfree（non-free-firmware 分区）、amd64-microcode、intel-microcode，GPU/无线固件与 CPU 微码开箱齐全；NVIDIA 默认 nouveau，不预装闭源驱动。
- 首次设置向导（完整版）：语言、网络、账号、指纹/人脸录入、选主题、隐私选项、数据导入。
- 语言：安装时让用户自己选（安装流程含语言选择步）。
- 安装完整版（Alpha 10 定案 UX；Alpha 11 起安装器换 Calamares，用户定案）：Live 模式自动登录进桌面并自动弹首次设置向导；向导最后一页有「安装完整版」按钮（另留「暂不安装」），点击拉起 Calamares；当时不装的，桌面有一张「安装完整版」卡片可再进入安装（该入口只在 Live 会话显示，装到硬盘后的系统不显示）。安装走 Calamares（DolphinOS 自备 /etc/calamares 配置与品牌）：欢迎/语言/键盘/分区（整盘擦除＋手动）/用户/执行（unpackfs 解包 Live 根、建用户、删 Live 用户、装 GRUB、收尾去 Live 自动登录并卸 Live 包与安装器本体），BIOS 与 UEFI 均走 GRUB（UEFI 带 fallback 写入）。安装器界面有与系统四主题（Dolphin/Metro/Fluent/Pixel）对应的四款样式，启动器按当前所选主题自动匹配。Alpha 10 的自研安装页与 dolphin-install 引擎已退役（其 sudo -n 拉起失败且静默无报错，是本次替换的直接原因）。

## 6. 预装应用

- 应用商店：Bazaar（io.github.kolunmi.Bazaar，Flatpak 商店；Alpha 10 由 GNOME Software 改定。Debian sid 有现成 bazaar 包直接进镜像，镜像预置 Flathub 系统源）。
- 自研（Slint 界面，跟桌面一套设计）：Shell、登录屏、设置、文件管理器、终端（界面自研 + 成熟终端解析库打底）、浏览器、视频查看器、照片查看器、首次设置向导。
- 浏览器内核：WebKitGTK（Slint 自研界面 + WebKitGTK 内核）。
- 视频播放内核：GStreamer（与 WebKitGTK 共用一套多媒体栈）。
- 自研应用未就绪前用现成程序占位（如文件管理器先用 Thunar），换包对用户无感。

## 7. 更新与软件仓库

- 更新双轨：日常 apt 小更新 + 整版大升级通道（设置里检查更新触发）。
- 软件仓库：aptly 自建 apt 仓库，GPG 签名，公钥随系统预置；三个通道 alpha / beta / stable，对应版本三阶段。
- Debian 官方 sid 源直接引用；自有源用 pinning 抬高优先级，只盖自有包。
- 仓库托管在自有机器经隧道对外（deb 包体积小）；ISO 不放 R2（体积大），走 GitHub Releases + 官网直链。

## 8. 版本

- 版本号跟网站一样分三阶段：Alpha、Beta、正式版。

## 9. 官网

- 挂到现有网站的 /os 路径（不另开子域名）。
- 顺序：先做完 Alpha 版，再直接写官网。

## 10. 测试分工

- 用户本人只测 x64 真机。
- ARM、x32：QEMU 全系统模拟验功能 + CI 构建/冒烟（ARM 可借 GitHub Actions 原生 arm64 机器）。
- LoongArch：交叉编译 + QEMU 验功能，真机验证走 Beta 通道用户反馈。
- 每版发布前固定冒烟清单：能装、能进桌面、能开自研应用、能升级。
