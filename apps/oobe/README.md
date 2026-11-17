# dolphin-oobe（DolphinOS 首次设置向导）

Slint 八步向导，结果经共享库 `dolphin-config` 写 `~/.config/dolphinos/settings.toml`，与 Shell/设置应用共用同一份配置。二进制名：`dolphin-oobe`。

## 已实现

- **状态机 `src/wizard.rs`**（纯逻辑、可单测）：`Step` 八步——Language、Network、Account、Biometric、ThemeStep、Privacy、Import、Done。`WizardState` 持 `Settings` 草稿 + `network_name`/`network_skipped` + `biometric_choice`（None/Fingerprint/Face/Skip）+ 内存密码。
  - `next()`/`back()` 带边界（首步不能退、Import 步须走 finish、Done 终局）。
  - `can_next()` 校验：Account 步要求 `account_name` 非空；密码可空（稍后设置），填了须 ≥ 4 位。**密码只在内存校验，finish 时立即清零、任何路径都不落盘**（有测试断言落盘文件不含密码）。
  - `finish()` = `finish_to(settings_path())`：把草稿落盘并置 `oobe_done = true`，theme/language/account_name/telemetry/crash_reports/import_source 一并写入；`finish_to(path)` 为可注入路径版本供测试。
- **UI `ui/app-window.slint`**（std-widgets 的 Button/LineEdit/CheckBox）：顶部进度（第 x/8 步 + 步名），中部按步切换——语言（中/英/日三选项）、网络（SSID 输入，留空 = 跳过）、账号（用户名 + 密码框）、指纹/人脸（三选项）、主题（四主题按钮，点选即时用 `dolphin-config::palette` 配色刷新预览色块）、隐私（使用统计/崩溃报告两个 CheckBox，默认关）、数据导入（路径输入，留空 = 跳过）、完成页；底部上一步/下一步（Import 步为「完成」），校验不过时在底部显示提示文本。

## 未做到

- 网络步只记录 SSID 文案，不真正扫描/连接 Wi-Fi；生物识别步只记录意向，不做录入；数据导入只记录来源路径，不执行拷贝。
- 主题预览是配色块级预览（背景/卡片/强调色），不是完整的桌面形态预览图。
- 界面在本机仅通过 Slint 编译与单元测试验证，未做真机 GUI 手测；与开机 SVG 海豚动画的无缝衔接未做（动画本身不在本应用内）。

## 测试

`cargo test -p dolphin-oobe`：3 个单元测试（步骤前进后退边界、Account 校验拦截与放行、finish 后用 `load_settings_from` 读临时路径验证落盘字段且密码不落盘），全过。
