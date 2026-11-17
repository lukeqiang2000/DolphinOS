# DolphinOS 开机体验层报告（boot-experience 分支）

> 任务：补做用户 2026-11-16 定案、此前未实现的开机体验层——GRUB 海豚主题、
> 开机动画（Plymouth）、引导菜单品牌化；并追加 2026-11-17 20:32 定案的
> Live 自动登录。开发在 `boot-experience` 分支，**未合并 main、未触发构建**
> （Alpha 7 构建由另一路负责，避免撞车）；真机构建验证随 Alpha 8 并入时做。
> 起因：用户 2026-11-17 20:24 在 VirtualBox 试跑 Alpha 6，开机仍是 Debian
> live-build 默认菜单（黄色安全帽 +「Debian GNU/Linux none (sid)」字样），
> 且该菜单无鼠标（GRUB 文本菜单本就只能键盘操作，属正常，见 §2 末）。

## 1. 勘察结论：原来的菜单是怎么生成的

以 Alpha 7 实际 ISO（downloads/44064380731）提取物 + live-build 源码
（salsa.debian.org/live-team/live-build，与构建用的 20250814 同线）实证：

- 构建用 `--bootloaders "grub-pc,grub-efi"`：**BIOS 与 UEFI 都走 GRUB**，
  ISO 里根本没有 isolinux 目录（文件清单实查：只有 /boot/grub/*、
  /EFI/boot/*.efi、grub_eltorito）。用户截图里那个菜单就是 GRUB 的
  gfxterm 图形菜单，不是 isolinux。
- 黄色安全帽 +「Debian GNU/Linux none (sid) amd64」+「Built: ...」是
  **一整张背景图** /boot/grub/splash.png（800x600），由 live-build 的
  binary_bootloader_splash 步骤把默认 splash.svg 套占位符后用
  rsvg-convert 转出来的；「none」是 sid 的 os-release VERSION_ID。
- 菜单条目「Live system (amd64)」等是 binary_grub_cfg **在脚本代码里
  写死拼接**、注入 grub.cfg 模板的 `@LINUX_LIVE@` 占位行生成的；
  「Utilities...」「Verify integrity of the boot medium」等则在
  grub.cfg 模板文件里。两处都要改，手法不同（见 §3）。
- 主题文件 /boot/grub/live-theme/theme.txt、config.cfg、theme.cfg 都是
  live-build 自带模板，项目此前没有任何 config/bootloaders 覆盖。
- 覆盖机制（源码实证）：binary_grub_cfg 先拷 live-build 模板、再用
  `config/bootloaders/grub-pc/` 下的用户文件 `cp -af` 盖上，最后统一做
  占位符 sed 替换；同一套配置同时服务 grub-pc 与 grub-efi。
  splash 源文件的查找顺序里 `config/bootloaders/splash.svg` 优先级最高，
  且 GRUB（800x600）与 syslinux（640x480）共用这一份源。
- binary_hooks 在 binary 阶段末尾运行（grub_cfg 第 58 步、splash 第 65
  步、hooks 第 66 步、打 ISO 第 77 步），工作目录为 binary/——这是唯一
  能改到最终 grub.cfg 里脚本生成标签的口子。
- 「鼠标也没有」：GRUB 引导菜单（包括 Debian 原装与本次主题）只支持
  键盘（方向键+回车），这是引导器层面的固有限制，不是缺陷；鼠标要进
  登录屏/桌面后才有。本次不改变这一点。

## 2. 海豚素材怎么来的

新增生成器 `image/boot-assets/generate.py`（Pillow 实现）：一套海豚轮廓
控制点（吻→额隆→背→背鳍→尾柄→尾鳍→腹→喉，共 23 点）经 Catmull-Rom
样条采样成平滑线稿，同一套点列产出三处素材，保证各处海豚一致：

1. `image/config/bootloaders/splash.svg`——GRUB/syslinux 共用背景源：
   深蓝渐变底（#0d63d8→#05224d）+ 白色线稿海豚 + 波浪线 +
   「DolphinOS / ALPHA」字样（DejaVu Sans，构建 chroot 内必备字体）。
2. Plymouth 12 帧动画 PNG（1024x640 透明底，约 42KB/帧）：海豚正弦
   游动（横移±24px、起伏±9px、倾斜±2.2°）+ 吻部气泡升起渐隐 + 帧内
   烘焙的 DolphinOS/ALPHA 字样。
3. GRUB 选中项九宫格切片 select_*.png（24x24 圆角 Dolphin 蓝条切 9 块）。

改素材流程：改 generate.py 后重跑 `python3 image/boot-assets/generate.py`，
产物已一并提交，不要手改生成的 SVG/PNG。

## 3. GRUB 海豚主题（BIOS + UEFI 一套覆盖）

新增/覆盖（路径均相对 image/）：

| 文件 | 作用 |
|---|---|
| `config/bootloaders/splash.svg` | 背景源（见 §2），构建时转 splash.png |
| `config/bootloaders/grub-pc/grub.cfg` | 模板覆盖：子菜单改「工具...」、条目改「UEFI 固件设置」「校验启动介质完整性」「高级安装选项 ...」；占位符与默认模板逐字一致（@LINUX_LIVE@ 等，模拟替换验证零残留） |
| `config/bootloaders/grub-pc/live-theme/theme.txt` | Dolphin 配色：未选项 #b9d6ff、选中白字 + select_* 蓝条切片、倒计时条 Dolphin 蓝；菜单区下移（top 58%）避开 splash 里的标题 |
| `config/hooks/live/0010-grub-branding.hook.binary` | binary hook：对生成好的 grub.cfg 做 sed，把脚本写死的「Live system (amd64)」→「DolphinOS Alpha (amd64)」、「fail-safe mode」→「安全模式」 |

config.cfg / theme.cfg 未覆盖（默认逻辑够用：有 splash.png 就启用
live-theme）。**踩坑预防（本地模拟时实证发现）**：binary_grub_cfg 对
grub.cfg 是按「行含 @LINUX_LIVE@」整行注入+删除的，注释里只要出现该
占位符字样就会把条目重复注入一遍——本分支 grub.cfg 的注释已避开全部
占位符字样，Alpha 8 合并后改注释时也要守这条。

## 4. Plymouth 开机动画

规格落地：简笔画海豚动画、**无进度条**（不注册 BootProgress 回调、
画面无进度条元素）、深蓝渐变背景与 GRUB 背景同色系，菜单→动画→
登录三段色调不断层。

| 文件 | 作用 |
|---|---|
| `config/package-lists/dolphinos.list.chroot` | 新增 `plymouth` 包（script.so 插件在主包内，已验包内容） |
| `config/includes.chroot/usr/share/plymouth/themes/dolphin/dolphin.plymouth` | 主题描述符（ModuleName=script） |
| `.../dolphin/dolphin.script` | script 主题：背景渐变 + 12 帧循环（refresh 约 50/s、每 3 次换帧、一圈约 0.7s） |
| `.../dolphin/frame-00..11.png` | 动画帧（generate.py 生成） |
| `config/includes.chroot/etc/plymouth/plymouthd.conf` | [Daemon] Theme=dolphin、ShowDelay=0 |
| `config/hooks/live/0100-plymouth-theme.hook.chroot` | chroot 收尾：把 default.plymouth 的 alternatives 指到 dolphin 并回读校验，再 update-initramfs -u 重建 initramfs |
| `build.sh` | --bootappend-live 增加 `quiet splash`（boot=live components 原样保留；校验介质条目由 live-build 自动剔除 quiet/splash，fail-safe 自带 nosplash，均不受影响） |

接线依据（实证，非印象）：

- plymouth 24.x 包的 initramfs hook 取主题的方式是
  `update-alternatives --query default.plymouth`，**不是**读
  plymouthd.conf；且它把整个主题目录拷进 initramfs。所以 hook 的核心
  动作是切 alternatives，plymouthd.conf 只作运行时对齐。
- plymouth-set-default-theme 在当前 plymouth 包里**已不附带**（包内容
  实查无此文件），hook 改为优先用它（若在）、否则直接
  update-alternatives --install/--set，等价且不依赖缺失脚本。
- chroot 阶段顺序：内核/initramfs 先生成（chroot_linux-image），
  includes 落地后 hooks 才运行——hook 里重建的 initramfs 是最终版。
- plymouth-quit.service 在包里由 multi-user.target.wants 预启用
  （包内容实查），greetd 起来后动画会被正常终止；但「动画→登录」
  是否完全无黑闪，未经真机验证（见 §7）。

已知限制：script 主题未实现密码询问/消息回调——Live 系统自动登录、
无 LUKS 解锁环节，正常启动用不到；启动日志类消息在动画界面不显示
（按 Esc 可切 details 文本界面）。后续若装机版要加密盘，再补 dialog。

## 5. isolinux / syslinux 侧

当前构建 BIOS 走 GRUB（§1），isolinux 未启用，以下为预埋覆盖——将来
若把 --bootloaders 改回 syslinux 即刻生效，且 splash 与 GRUB 共用同一
份 `config/bootloaders/splash.svg`（构建时按 640x480 另行转换）：

| 文件 | 作用 |
|---|---|
| `config/bootloaders/syslinux_common/menu.cfg` | 标题改「DolphinOS Alpha Boot Menu」（保留 @OPTIONAL_INSTALLER_INCLUDE@） |
| `config/bootloaders/syslinux_common/live.cfg.in` | 条目改「DolphinOS Alpha (@FLAVOUR@)」等（占位符与模板逐字一致；vesamenu 位图字体无 CJK，故保持英文） |
| `config/bootloaders/syslinux_common/stdmenu.cfg` | 选中项底色改 Dolphin 蓝 #1370f3（沿用模板的 alpha 写法） |

## 6. Live 自动登录（2026-11-17 20:32 追加定案）

**口径**：Live 模式开机自动以 live 用户 `user` 直接进桌面、不停登录屏；
登录屏只在登出后出现；密码登录只针对装到硬盘后的系统。本项与开机
主题/动画一起进 **Alpha 8**（Alpha 7 已打包完成，来不及）。

**改法**（只动一个文件）：`config/includes.chroot/etc/greetd/config.toml`
新增一节，default_session（greeter）保持不动：

```toml
[initial_session]
command = "/usr/local/bin/dolphin-session"
user = "user"
```

会话命令与密码登录成功后 greeter 请求启动的那条完全一致
（apps/greeter/src/greetd.rs 里 StartSession 的 cmd 即
`/usr/local/bin/dolphin-session`，含 labwc 渲染失败 pixman 兜底）。

**语义实证**（greetd 0.10.3-7 包内 greetd(5) 手册页原文核对，非印象）：

- initial_session 即手册所称 "auto-login" 一节：greetd 首次启动时
  直接以指定用户运行该命令，**不经 greeter、不验密码**。
- 它**只在本次开机后 greetd 第一次运行时执行一次**：靠 runfile
  （位于每次重启都会清空的位置）防重入——手册明言此设计就是为了
  「保证登出正常工作」：初始会话退出后 greetd 转入 default_session
  （登录屏），不会再次自动登录；greeter 自身重启也不会重触发。
- default_session 的手册定义亦印证：它在「没有其他会话在跑时」自动
  启动，例如用户会话终止时——即登出后出登录屏。

**装到硬盘后怎么去掉自动登录（给安装环节的建议，本分支未动安装器）**：

安装器（Live 一键安装）把 live 文件系统落盘时，对目标系统的
`/etc/greetd/config.toml` 做一步处理即可，二选一：

1. **删节**（推荐）：删除 `[initial_session]` 整节，其余不动。装机后
   greetd 每次都走 default_session 密码登录，与现有登录链完全一致。
2. **换文件**：安装包内置一份「已安装版」config.toml（无 initial_session
   节）覆盖目标文件。适合以后两形态差异变多时。

同时提醒安装环节注意：live 用户 `user`（密码 live）是 Live 专用账号，
装机流程本就要引导用户建自己的账号；删自动登录应与「装机后不保留
live 账号或至少改掉其默认密码」一起做，不要只删节留账号。另外
OOBE 首次设置向导（dolphin-oobe）目前在 Live 桌面里以应用形态存在，
装机后首次启动是否要先走 OOBE 再进登录，属安装流程设计，未在本分支
臆造。

## 7. 验证记录（本地已做）

| 项 | 方法 | 结果 |
|---|---|---|
| greetd config.toml 语法 | python3 tomllib 解析 | 通过；initial_session/default_session 内容断言通过 |
| greetd initial_session 语义 | greetd 0.10.3-7 包内 greetd(5) 手册页 + 包内示例 config 对照 | 与 §6 所述一致（auto-login、runfile 单次、登出转 greeter） |
| splash.svg 语法/渲染 | XML 解析 + CairoSVG 按 800x600 渲染目检 | 通过：渐变、海豚线稿、标题位置均符合设计（另用 ImageMagick 预览核对几何，其 MSVG 引擎不认渐变填充、以 CairoSVG 结果为准） |
| Plymouth 帧图 | generate.py 生成后逐帧存在性 + 目检 frame-03 | 12 帧齐全，海豚/气泡/字样正常 |
| dolphin.script | 括号平衡检查 + 对照 plymouth 上游源码核 API | SetRefreshFunction/SetImage/Math.Int/哈希索引 `frames[i]`/`[]` 均为上游 script 插件实证存在的用法 |
| plymouth 接线 | plymouth 24.004.60 包内容实查 | initramfs hook 按 alternatives 取主题、script.so 在主包内、plymouth-quit 由 multi-user.target.wants 预启用，均确认 |
| grub.cfg 模板覆盖 | 按 binary_grub_cfg 的 sed 清单本地模拟替换 | 占位符零残留；子菜单/校验/UEFI 标签品牌化正确；发现并修复「注释含占位符会被重复注入条目」的坑（§3） |
| binary hook 改标签 | 拿 Alpha 7 ISO 的真实 grub.cfg 跑 hook 的 sed | 「Live system (amd64)」→「DolphinOS Alpha (amd64)」、「(amd64 fail-safe mode)」→「(amd64 安全模式)」均正确 |
| lb config 轻量校验 | 用 live-build 源码树在本机跑通 config 阶段（含改后的 build.sh 参数） | 通过：config/binary 里 LB_BOOTAPPEND_LIVE 含 quiet splash，failsafe 保持 nosplash；仅有的报错是本机代理环境变量与 apt 配置不一致，与本分支无关 |
| shell 语法 | sh -n 两个新 hook + build.sh | 通过 |

## 8. 未验证项（必须如实保留）

- **未做真机构建**：本分支没有触发 Actions 构建（按任务书与 Alpha 7
  避让）。GRUB 主题实际渲染（gfxterm 下 theme.txt 版式、select 切片
  显示、Unifont 中文标签字形）、splash.svg 经构建 chroot 内
  rsvg-convert 的实际出图，都要等 Alpha 8 构建后看真机/VirtualBox。
- **Plymouth 动画未在真实启动中验证**：initramfs 是否如期收录 dolphin
  主题、VirtualBox（VMSVGA）/VMware 下帧率与渲染器回退（DRM 不可用时
  framebuffer）、动画→greetd 登录的交接是否有黑闪，均未验。
- **自动登录未真机验证**：initial_session 在本镜像的 greetd 版本
  （sid 的 greetd）上的实际行为、runfile 在 live 文件系统（overlay+
  tmpfs /run）下的单次语义、登出后是否如期回到 dolphin-greeter，
  都要等 Alpha 8 试跑确认。
- **syslinux 预埋覆盖未验证**：当前构建不启用 syslinux，menu.cfg 等
  只做了模板逐字对照，没有实际生成过 isolinux 菜单。
- x32/ARM/LoongArch 三架构的引导层未动（本分支只保证 x64；GRUB 主题
  文件本身与架构无关，splash/主题可复用，但各架构 bootloader 模板
  目录不同，等对应架构开工时再接）。

## 9. Alpha 8 并入清单

1. 把本分支合并进 main（与 Alpha 7 的提交同线，无文件冲突预期：
   本分支只新增 config/bootloaders、hooks、plymouth includes 与素材，
   改动 build.sh 一行参数、包列表一行、greetd config 一节）。
2. 触发 build-iso 构建 Alpha 8，ISO 落地后先做镜像内静态核对：
   - 解 ISO 看 /boot/grub/splash.png 是否为海豚图（非安全帽）；
   - /boot/grub/grub.cfg 条目是否为 DolphinOS Alpha 字样；
   - 解 squashfs 看 /usr/share/plymouth/themes/dolphin 是否在位、
     /etc/plymouth/plymouthd.conf 是否 Theme=dolphin；
   - 解 /live/initrd.img 查 plymouth 主题是否被收录（找 dolphin 字样/
     default.plymouth 指向）；
   - /etc/greetd/config.toml 含 [initial_session] 节。
3. 用户真机/VirtualBox 试跑验收：GRUB 菜单外观与中文标签、开机动画
   播放与无进度条、自动登录直达桌面、登出后回到登录屏、密码登录
   （user/live）仍可用。
4. /os 下载页同步换 Alpha 8 条目（按既定口径：每版日志带下载链接）。
