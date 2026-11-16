//! DolphinOS 桌面 Shell 骨架（Slint）。
//!
//! 当前形态：窄顶栏（时间 + 状态占位）+ 下方板块网格占位卡片。
//! 正式形态按 docs/SPEC.md：layer-shell 贴附 labwc、卡片翻转打开动画、
//! 左上角找应用入口、多主题形态级皮肤等，后续迭代补齐。

use slint::{Timer, TimerMode};
use std::time::Duration;

slint::include_modules!();

fn now_hhmm() -> slint::SharedString {
    chrono::Local::now().format("%H:%M").to_string().into()
}

fn main() -> Result<(), slint::PlatformError> {
    // squircle 点列自检：确认共享库链路可用。
    // 后续卡片轮廓将用同一份点列喂 Slint Path 绘制（与网站同一套算法）。
    let pts = dolphin_shape::squircle_points(100.0, 100.0, 64);
    println!(
        "dolphin-shape: squircle 点列 {} 点，首点 {:?}",
        pts.len(),
        pts[0]
    );

    let ui = AppWindow::new()?;
    ui.set_clock(now_hhmm());

    // 顶栏时钟：每秒刷新一次。
    let ui_handle = ui.as_weak();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_millis(1000), move || {
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_clock(now_hhmm());
        }
    });

    ui.run()
}
