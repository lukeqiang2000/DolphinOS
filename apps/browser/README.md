# dolphin-browser（DolphinOS 自研浏览器）

Slint 界面 + 地址栏逻辑的浏览器壳。二进制名：`dolphin-browser`。

## 已实现

- **纯逻辑 `src/logic.rs`**（与 UI 解耦、可单测）：
  - `normalize_url(input)`：去空白；已有 `http://`/`https://` 前缀原样保留；像域名（含点、无空白）补 `https://`；其余当搜索词，走 `https://duckduckgo.com/?q=<percent-encode>`。percent-encode 为自写小函数（保留 RFC 3986 unreserved 字符，其余 `%XX`），未引新依赖。
  - `History`：`entries + index` 历史栈，`push` 时截断前进栈，`back`/`forward`/`can_back`/`can_forward`/`current`。
- **UI `ui/app-window.slint`**：工具栏（后退/前进/刷新/主页按钮 + 地址栏 `LineEdit`，回车 `accepted` 触发导航）、标签标题区（显示当前页标题与 URL）、状态栏。
- **导航行为（默认构建）**：本机只有 libwebkit2gtk-4.1 运行时、无 `-dev`（pkg-config 找不到），gtk/webkit2gtk 的 `-sys` crate 在本机构建不了。因此 WebKit 集成放在默认关闭的 feature `webkit` 后；默认构建导航时用 `xdg-open` 交系统浏览器打开，状态栏如实显示「本机未启用 WebKitGTK 内嵌引擎，已用系统浏览器打开」（xdg-open 失败时降级为只显示失败状态）。后退/前进只切换历史栈与标题区，不会重新拉起系统浏览器页面。

## webkit feature（未在本机验证）

- `Cargo.toml`：`gtk = "0.18"`、`webkit2gtk = "2.0"` 均为 optional 依赖，`webkit = ["dep:gtk", "dep:webkit2gtk"]`。
- `src/webkitview.rs`：按 gtk 0.18 / webkit2gtk 2.0 API 写——`gtk::init`、`gtk::Window`、`WebView::new()` + `WebViewExt::load_uri`，导航时由 `main.rs` 在 feature 开启时调用。
- **真实状态：未在本机编译或运行验证**（缺 `libwebkit2gtk-4.1-dev`）。需在装有该 -dev 包的机器上 `cargo build -p dolphin-browser --features webkit` 验证；当前实现是「每次导航开一个独立 WebKit 窗口并进入 gtk 主循环」的最小形态，与 Slint 主窗口的内嵌整合（同窗口 WebView、标签页、进度/标题回传）尚未做。

## 未做到

- 内嵌渲染（见上）、多标签页、书签/历史持久化、下载管理、页面标题/加载进度回传均未实现。
- 界面在本机仅通过 Slint 编译与单元测试验证，未做真机 GUI 手测。

## 测试

`cargo test -p dolphin-browser`：6 个单元测试（normalize_url 三分支 + percent-encode、History 前进后退与 push 截断），全过。
