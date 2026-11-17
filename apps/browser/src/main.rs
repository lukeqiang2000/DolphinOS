//! DolphinOS 自研浏览器（Slint 壳）。
//!
//! 默认构建不含 WebKitGTK 内嵌引擎（本机缺 libwebkit2gtk-4.1-dev），
//! 导航时用 xdg-open 交给系统浏览器打开，并如实显示状态文本。
//! 启用 `--features webkit` 后，导航改走 webkitview 模块的内嵌 WebView。

mod logic;
#[cfg(feature = "webkit")]
mod webkitview;

use logic::{normalize_url, History};
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;

slint::include_modules!();

const HOME_URL: &str = "https://duckduckgo.com/";

/// 把历史栈状态同步到界面属性。
fn sync_ui(ui: &AppWindow, history: &History) {
    let current = history.current().unwrap_or("").to_string();
    ui.set_current_url(current.clone().into());
    ui.set_address_text(current.into());
    ui.set_page_title(
        if history.current().is_none() {
            "新标签页".into()
        } else {
            history.current().unwrap_or("").into()
        },
    );
    ui.set_can_back(history.can_back());
    ui.set_can_forward(history.can_forward());
}

/// 执行一次导航：归一化 URL → 入历史栈 → 按构建形态真正打开页面。
fn navigate(ui: &AppWindow, history: &Rc<RefCell<History>>, raw: &str) {
    let url = normalize_url(raw);
    history.borrow_mut().push(url.clone());
    sync_ui(ui, &history.borrow());

    #[cfg(feature = "webkit")]
    {
        ui.set_status_text(format!("正在加载（WebKitGTK 内嵌引擎）：{url}").into());
        webkitview::open_url(&url);
    }

    #[cfg(not(feature = "webkit"))]
    {
        match Command::new("xdg-open").arg(&url).spawn() {
            Ok(_) => {
                ui.set_status_text(
                    "本机未启用 WebKitGTK 内嵌引擎，已用系统浏览器打开".into(),
                );
            }
            Err(e) => {
                ui.set_status_text(
                    format!("本机未启用 WebKitGTK 内嵌引擎；xdg-open 打开失败：{e}").into(),
                );
            }
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    let history = Rc::new(RefCell::new(History::new()));
    sync_ui(&ui, &history.borrow());
    ui.set_status_text("就绪：在地址栏输入网址或搜索词后回车".into());

    // 地址栏回车 / 主页按钮等触发的导航
    {
        let history = history.clone();
        let ui_handle = ui.as_weak();
        ui.on_navigate(move |raw| {
            if let Some(ui) = ui_handle.upgrade() {
                navigate(&ui, &history, raw.as_str());
            }
        });
    }

    // 后退
    {
        let history = history.clone();
        let ui_handle = ui.as_weak();
        ui.on_go_back(move || {
            if let Some(ui) = ui_handle.upgrade() {
                let mut h = history.borrow_mut();
                if h.can_back() {
                    h.back();
                    let url = h.current().unwrap_or("").to_string();
                    drop(h);
                    sync_ui(&ui, &history.borrow());
                    ui.set_status_text(format!("后退到：{url}").into());
                }
            }
        });
    }

    // 前进
    {
        let history = history.clone();
        let ui_handle = ui.as_weak();
        ui.on_go_forward(move || {
            if let Some(ui) = ui_handle.upgrade() {
                let mut h = history.borrow_mut();
                if h.can_forward() {
                    h.forward();
                    let url = h.current().unwrap_or("").to_string();
                    drop(h);
                    sync_ui(&ui, &history.borrow());
                    ui.set_status_text(format!("前进到：{url}").into());
                }
            }
        });
    }

    // 刷新：对当前页重新走一遍打开动作（不新增历史条目）
    {
        let history = history.clone();
        let ui_handle = ui.as_weak();
        ui.on_reload(move || {
            if let Some(ui) = ui_handle.upgrade() {
                if let Some(url) = history.borrow().current().map(|s| s.to_string()) {
                    #[cfg(feature = "webkit")]
                    webkitview::open_url(&url);
                    #[cfg(not(feature = "webkit"))]
                    {
                        let _ = Command::new("xdg-open").arg(&url).spawn();
                    }
                    ui.set_status_text(format!("已刷新：{url}").into());
                }
            }
        });
    }

    // 主页
    {
        let history = history.clone();
        let ui_handle = ui.as_weak();
        ui.on_go_home(move || {
            if let Some(ui) = ui_handle.upgrade() {
                navigate(&ui, &history, HOME_URL);
            }
        });
    }

    ui.run()
}
