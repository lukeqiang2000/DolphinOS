//! WebKitGTK 内嵌引擎入口（仅 feature = "webkit" 时编译）。
//!
//! 用 gtk 0.18 + webkit2gtk 2.0 的真实 API：初始化 GTK、建一个独立
//! 顶层窗口、把 WebView 作为子控件加入并 load_uri 加载目标 URL。
//!
//! 注意：本机（开发 VM）只有 libwebkit2gtk-4.1 运行时、没有 -dev 包
//! （pkg-config 找不到 webkit2gtk-4.1），gtk/webkit2gtk 的 -sys crate
//! 在本机构建不了，因此这条路径未在本机编译/运行验证；需在装有
//! libwebkit2gtk-4.1-dev 的机器上 `cargo build --features webkit` 验证。

use gtk::prelude::*;
use webkit2gtk::{WebView, WebViewExt};

/// 打开一个 WebKit 窗口加载 `url`。首次调用时初始化 GTK 并进入主循环。
pub fn open_url(url: &str) {
    if gtk::is_initialized() == false {
        if let Err(e) = gtk::init() {
            eprintln!("dolphin-browser: GTK 初始化失败: {e}");
            return;
        }
    }

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("Dolphin 浏览器");
    window.set_default_size(1024, 720);

    let webview = WebView::new();
    webview.load_uri(url);

    window.add(&webview);
    window.show_all();
    gtk::main();
}
