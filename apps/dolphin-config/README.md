dolphin-config：DolphinOS 共享配置库（库 crate，无二进制）。

职责：作为全系统设置的唯一真源，Shell、设置应用（dolphin-settings）、首次设置向导（dolphin-oobe）都通过本库读写同一份配置。

- 配置文件：`~/.config/dolphinos/settings.toml`（环境变量 `DOLPHINOS_CONFIG_DIR` 可覆盖目录，供测试/便携运行）。
- 内容：主题（Dolphin/Metro/Fluent/Pixel）、打开动画三档（Flip/Popup/None）、语言、账号显示名、隐私开关（telemetry/crash_reports，默认关）、向导完成标记、数据导入来源。
- 容错：文件缺失或损坏时回退默认值，绝不因配置坏掉让桌面起不来。
- `palette(theme)`：每个主题一份配色 + 圆角 + 超椭圆指数（数据层皮肤），UI 按当前主题取用。
- 测试：默认值、保存/读取往返、损坏文件回退、部分 TOML 补默认、主题解析与轮换、四主题圆角互异。
