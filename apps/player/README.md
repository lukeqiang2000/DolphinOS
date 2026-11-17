# dolphin-player（DolphinOS 自研视频播放器）

Slint 界面 + 可插拔播放后端的视频播放器，二进制名 `dolphin-player`。
命令行参数可传视频文件或目录（可多个），全部收进播放列表。

## 实现说明

- **纯逻辑（`src/logic.rs`，不依赖 Slint）**：
  - `Playlist`：`add` / `add_dir`（只收 mp4/mkv/webm/avi/mov，不递归，
    按文件名排序）、`next` / `prev`（循环）、`remove`（自动修正当前
    索引）、`jump`。
  - 播放状态机 `PlayState { Stopped, Playing, Paused }` 与
    `PlayerCore`：持有 position/duration，`tick(dt)` 在 Playing 时
    推进进度，到尾自动切下一首并归零（列表为空/无当前项则转
    Stopped）；`format_time(secs)` 输出 `mm:ss`，满 1 小时输出
    `h:mm:ss`；音量 `set_volume` 钳制在 0..=100。
  - 后端接口 `trait Backend { play, pause, resume, stop, seek }`。
- **默认后端 `SimulatedBackend`**：只记录状态、不解码，让 UI 与状态
  机在没有 GStreamer 的环境也能跑通；无真实时长时 UI 给 180 秒
  占位时长演示进度条与自动切歌。
- **界面（Slint）**：左侧播放列表（当前项带 ▶ 标记高亮）、中央视频
  占位区（当前文件名 + 播放状态）、底部控制条：上一首、播放/暂停、
  停止、下一首、进度滑杆 + 起止时间文本、音量滑杆；250ms Timer
  驱动 `tick` 并刷新界面。

## GStreamer 后端（feature `gst`，默认关闭）

`src/gst_backend.rs` 是按 gstreamer 0.23 API 写的 playbin 骨架
（`ElementFactory::make("playbin")`、设置 `uri` 属性、
`set_state` 切换 Playing/Paused/Null、`seek_simple` 跳转）。

**真实状态：该 feature 未在本机验证、未能在本机构建。** 本机只有
libgstreamer 运行时库、没有 libgstreamer1.0-dev（pkg-config 找不到
gstreamer-1.0），gstreamer crate 的构建脚本会失败，因此默认构建
不依赖它。在装好 `libgstreamer1.0-dev` 的环境里可用
`cargo build -p dolphin-player --features gst` 构建并做真机验证；
启用后初始化失败会自动回退模拟后端。

## 测试

`cargo test -p dolphin-player`：覆盖列表循环 next/prev 与
remove/jump、add_dir 扩展名过滤与排序、状态机 tick 到尾自动切歌
与暂停不推进、format_time 与音量 clamp，共 4 个单元测试
（在 `src/logic.rs`）。

## 未做到 / 已知限制

- 默认（模拟）后端不产生真实画面与声音，中央区域只是占位显示；
  真实视频渲染（含把 GStreamer 视频流嵌入 Slint 窗口）未完成。
- 模拟后端的时长是占位值（180 秒），与文件真实时长无关；真实
  时长查询（GStreamer query_duration）未接入。
- 播放列表只能经命令行参数填充，界面内无打开文件/文件夹按钮，
  列表项不可点击切换（`jump` 逻辑已备好，未接 UI 点击）。
- gst feature 未在本机编译验证（见上），API 细节以 0.23 文档为准，
  首次在 -dev 环境构建时可能需要小修。
