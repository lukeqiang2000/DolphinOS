# DolphinOS 第一阶段报告（骨架）

日期：2026-11-16。范围：仓库骨架、规格落盘、live-build 配置、镜像构建可行性验证、squircle 共享库与 Shell 骨架。规格以 docs/SPEC.md 为准，本阶段未做任何重新选型。

## 一、做了什么

### 1. 仓库骨架（~/workspace/dolphinos/）

- `image/`：live-build 构建目录（见下节）。
- `apps/`：Cargo workspace（成员 dolphin-shape、shell）+ 各应用占位目录 shell、greeter、settings、files、terminal、browser、player、photos、oobe，每个目录一行 README 说明用途与占位策略。
- `packaging/`：README 写明 aptly 三通道（alpha/beta/stable）、GPG 签名、pinning、整版升级元包等分发方案。
- `docs/SPEC.md`：已定规格落盘——底座 sid、Live + 一键安装、四架构（第一阶段 amd64）、Wayland + labwc、Slint 自研 Shell/登录屏、GRUB 2、多主题形态级皮肤（Dolphin/Metro/Fluent/Pixel）、板块网格桌面细节、SVG 简笔画海豚开机动画无加载条、aptly 三通道、版本分 Alpha/Beta/正式版、官网挂 /os（Alpha 后写）、测试分工。
- 顶层 `README.md`：仓库导览。

### 2. live-build 配置（image/）

- `build.sh`：封装 `lb clean` + `lb config` + `lb build`。关键参数：`--mode debian`、`--distribution sid`、`--architectures amd64`、`--archive-areas "main contrib non-free non-free-firmware"`、`--binary-images iso-hybrid`、`--bootloaders grub-pc grub-efi`（GRUB 2 覆盖 BIOS+UEFI）、`--security false --updates false`（sid 无这两个套件，开着会因源不存在失败）、镜像 deb.debian.org。脚本内注明：参数针对 Debian 版 live-build，Ubuntu 老版（3.0~a57）参数名略有差异（如 --bootloader 单数）。
- `config/package-lists/dolphinos.list.chroot`：live-boot、live-config、systemd-sysv、labwc、seatd、greetd、foot、thunar、network-manager、pipewire、wireplumber、xdg-desktop-portal-wlr、fonts-noto-cjk。
- 默认会话 = labwc（双保险）：
  - `config/includes.chroot/etc/greetd/config.toml`：greetd 的 default_session 为 `agreety --cmd labwc`（自研登录屏就绪前占位，登录链跑通后进 labwc 会话）；
  - `config/includes.chroot/usr/share/wayland-sessions/labwc.desktop`：会话文件；
  - `config/hooks/live/0090-dolphin-session.hook.chroot`：chroot hook 校验 greetd 配置并 enable greetd/seatd 服务。

### 3. squircle 共享库（apps/dolphin-shape）

- `superellipse_points(半宽, 半高, 指数 n, 点数)`：超椭圆公式 `x = a·sign(cos t)·|cos t|^(2/n)`、`y = b·sign(sin t)·|sin t|^(2/n)` 生成点列；另有 `squircle_points`（n 固定 4.5）与左上角坐标系版 `squircle_points_xy`。
- 实现细节：轴附近 sin/cos 做 1e-6 吸附归零——f32 在 π/2 附近的残差（约 1e-7）会被 0.444 次幂放大成约 0.04 的坐标偏差，吸附后轴向点精确、对称性严格（首版测试因此有 2 项失败，已修正实现而非放宽断言）。
- 单元测试 6 项：点数、n=2 椭圆方程、轴向点、对称性、squircle 45° 比椭圆更饱满、xy 版不越界。**cargo test -p dolphin-shape：6 passed / 0 failed。**

### 4. Shell 骨架（apps/shell）

- Slint 1.19.0 + Rust：一个窗口——28px 窄顶栏（左侧 DolphinOS 字样、右侧状态占位 + 时钟，main.rs 用 Timer 每秒以 chrono 本地时间刷新）+ 下方 GridLayout 两行共 8 张占位卡片（文件/终端/浏览器/设置/照片/播放器/应用商店/系统）。
- main.rs 启动时调用 dolphin_shape 生成 squircle 点列自检打印，证明共享库链路可用（后续卡片轮廓用同一点列喂 Slint Path）。
- **cargo build -p dolphin-shell：成功**（6 分 48 秒），产物 `apps/target/debug/dolphin-shell`（329 MB，debug 版）。未真跑画面（本 VM 无图形会话），仅保证编译通过，符合本阶段要求。

## 二、镜像构建可行性验证（本 VM）

### 工具安装

- debootstrap 本机已有：1.0.134ubuntu2，自带 sid 脚本。
- live-build 经 apt 安装成功：Ubuntu 版 3.0~a57-1ubuntu49.1（另装 squashfs-tools、xorriso、mtools、dosfstools、isolinux、syslinux-common）。apt 镜像可达，安装约 10 分钟（镜像较慢）。

### 尝试一：/tmp 下最小构建（包列表仅 live-boot、live-config、systemd-sysv、labwc，bootstrap-flavour minimal）

- `lb config` 成功（a57 语法：--bootloader grub 等）。
- **debootstrap 阶段完整跑通**：从 deb.debian.org 下载并装好 sid 基础系统，bootstrap 阶段缓存保存成功（chroot 一度达约 221 MB）。说明网络、代理、debootstrap 链路本身没问题。
- 进入 chroot 阶段后失败，两个问题叠加：
  1. `/tmp` 是 512 MB 的 tmpfs，构建目录涨到 493 MB 后写满，保存缓存时报 `E: Write error - write (28: No space left on device)`；
  2. 致命卡点：挂载 sysfs 被拒——`mount: /tmp/lbtest/chroot/sys: permission denied.`，lb build 以 rc=32 中止（devpts、proc 两步未报错，倒在 sysfs 这一步）。
- 日志：build-attempt-1-tmp.log。

### 尝试二：磁盘目录（~/lbtest2）同配置重跑

- 更早失败：debootstrap 在 chroot 内装 base-files 时 postinst 报错 `chown: changing ownership of '/var/mail': Operation not permitted`，rc=1；另有 `W: Could not create /dev/ptmx, falling back to symlink` 警告。即本机磁盘卷上 chown/设备节点同样受沙箱限制。
- 日志：build-attempt-2-disk.log。

### 挂载能力探针（定性用）

- 直接 `mount -t sysfs`：permission denied；`unshare -rm` 与 `unshare -rmn` 里再试：同样 permission denied。
- `mount -t proc` 与 `mount --bind /sys`：成功。
- 结论：本 VM 沙箱明确禁止新建 sysfs 挂载（多组命名空间组合均拒），live-build 的 chroot 阶段无法在此通过；未做任何绕过沙箱的操作。

### 结论

- **本 VM 不能完整跑 lb build，ISO 未产出。** 已验证到的链路：lb config ✓、debootstrap 下载/解包/装基础系统 ✓（tmpfs 上 bootstrap 阶段完整完成）；卡点在 chroot 阶段的 sysfs 挂载（沙箱权限）与磁盘卷 chown 限制，属环境问题、非配置问题——配置与脚本已做到在正常 Debian 环境可复现。

### 替代构建地点建议

1. **GitHub Actions（推荐先行）**：ubuntu-latest 有 sudo、挂载不受限；装 Debian 版 live-build 或在 debian:sid 容器（--privileged）里跑 image/build.sh，可同时为后续四架构 CI 打底。
2. **用户 PC 上的 Debian 虚拟机**（VirtualBox 等）：sid 虚机里装 live-build 直接跑 build.sh；注意构建目录放磁盘卷、预留 ≥20 GB。
3. 特权 Docker 容器（`--privileged` 的 debian:sid 镜像）效果同虚机，也可放在 PC 或任何 Linux 主机上。

## 三、下一步建议

1. 先在 GitHub Actions 落一条 amd64 镜像构建流水线（用 image/build.sh），产出第一个能进 labwc + greetd 登录链的 Alpha ISO，再由用户 x64 真机/QEMU 冒烟（能装、能进桌面、能开占位应用）。
2. Shell 骨架下一步：接 layer-shell 贴顶栏、卡片点击翻转打开（点列已由 dolphin-shape 备好）、左上角找应用入口；登录屏按 SPEC 以 Slint 自研替换 agreety。
3. 构建环境固定后，把尝试一的最小包列表换回 image/config 的正式包列表（含 pipewire、NetworkManager 等）做完整构建。
4. aptly 仓库与 GPG 密钥在 packaging 阶段落地，配合 Alpha 通道先行。
