dolphin-settings：DolphinOS 设置应用（Slint + Rust），二进制名 `dolphin-settings`。

已实现：
- 主题切换：Dolphin / Metro / Fluent / Pixel 四选一，即时保存。
- 打开动画三档：翻转 / 弹入 / 无动画，即时保存（Shell 读取同一配置生效）。
- 语言：简体中文 / English，即时保存。
- 隐私开关：匿名使用统计、崩溃报告上传，默认关，即时保存。
- 所有设置经共享库 dolphin-config 写入 `~/.config/dolphinos/settings.toml`，与 Shell、首次设置向导共用同一份；界面底部显示配置文件路径与保存状态。
- 纯逻辑在 src/logic.rs（选项清单、下标校验、与共享库的保存/重读往返均有单测）。

未做到（如实）：
- 「更新」页目前是说明文字（apt 小更新 + 整版升级通道的口径），没有在应用内直接触发检查更新。
- 改主题后已运行的 Shell 不会热切换，需重启 Shell 生效。
- 网络、声音、显示等系统级设置页未做（后续接 NetworkManager/ PipeWire 等系统服务时补）。
