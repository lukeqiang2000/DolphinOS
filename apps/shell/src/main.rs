//! DolphinOS 桌面 Shell（Slint）正式版。
//!
//! 功能：28px 窄顶栏（时间 + 状态图标）、板块网格桌面（常用应用直接成卡、
//! 其余按板块顺序排列）、卡片顺序调整并持久化、左上角找应用入口（搜索展开）、
//! 系统板块卡（设置/关机/重启）、多主题数据层皮肤（配色/圆角随 dolphin-config 主题）、
//! 打开动画三档（翻转/弹入/无动画，取自共享设置）。
//! 动画与启动的时序逻辑在本文件，桌面数据逻辑在 logic.rs（可单测）。

mod logic;

use slint::{Color, ComponentHandle, ModelRc, Timer, TimerMode, VecModel};
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;
use std::time::{Duration, Instant};

slint::include_modules!();

use dolphin_config::AnimationMode;
use logic::AppEntry;

fn now_hhmm() -> slint::SharedString {
    chrono::Local::now().format("%H:%M").to_string().into()
}

fn parse_hex(s: &str) -> Color {
    let h = s.trim_start_matches('#');
    if h.len() == 6 {
        let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(0);
        Color::from_rgb_u8(r, g, b)
    } else {
        Color::from_rgb_u8(0, 0, 0)
    }
}

/// 真正拉起一个应用进程（找不到/起不来只回状态文本，不让 Shell 崩）。
/// exec 可带参数（如 "dolphin-oobe --install"）：程序名走同目录优先解析，
/// 参数原样传给子进程。
fn spawn_app(ui: &AppWindow, exec: &str) {
    let (prog, args) = logic::parse_exec(exec);
    let prog = logic::resolve_exec(&prog);
    match Command::new(&prog).args(&args).spawn() {
        Ok(_) => ui.set_status_text(format!("已打开 {exec}").into()),
        Err(e) => ui.set_status_text(format!("打开 {exec} 失败：{e}（可能尚未打包进系统）").into()),
    }
}

/// squircle 轮廓的 Slint Path 命令串（点列与网站 clip-path 同一套
/// 超椭圆算法，左上角原点坐标系）。
fn squircle_path(w: f32, h: f32) -> String {
    let pts = dolphin_shape::squircle_points_xy(w, h, 48);
    let mut s = String::new();
    for (i, (x, y)) in pts.iter().enumerate() {
        s.push_str(&format!("{} {:.1} {:.1} ", if i == 0 { "M" } else { "L" }, x, y));
    }
    s.push('Z');
    s
}

fn cards_model(entries: &[AppEntry]) -> ModelRc<DesktopCard> {
    let items: Vec<DesktopCard> = entries
        .iter()
        .enumerate()
        .map(|(i, a)| DesktopCard {
            idx: i as i32,
            name: a.name.into(),
            exec: a.exec.into(),
            section: a.section.into(),
            frequent: a.frequent,
        })
        .collect();
    ModelRc::new(VecModel::from(items))
}

fn launcher_model(entries: &[AppEntry], query: &str) -> ModelRc<DesktopCard> {
    let items: Vec<DesktopCard> = logic::search_filter(entries, query)
        .iter()
        .map(|a| {
            let idx = entries.iter().position(|e| e.id == a.id).unwrap_or(0);
            DesktopCard {
                idx: idx as i32,
                name: a.name.into(),
                exec: a.exec.into(),
                section: a.section.into(),
                frequent: a.frequent,
            }
        })
        .collect();
    ModelRc::new(VecModel::from(items))
}

/// 一次打开动画的进行状态。
struct AnimState {
    exec: String,
    idx: usize,
    mode: AnimationMode,
    start: Instant,
    launched: bool,
}

fn main() -> Result<(), slint::PlatformError> {
    // squircle 点列自检：确认共享库链路可用（卡片轮廓与网站同一套超椭圆算法）。
    let pts = dolphin_shape::squircle_points(100.0, 100.0, 64);
    println!("dolphin-shape: squircle 点列 {} 点，首点 {:?}", pts.len(), pts[0]);

    let settings = dolphin_config::load_settings();
    let palette = dolphin_config::palette(settings.theme);

    // 桌面顺序：读持久化布局，套到默认清单上；再按会话过滤
    // （安装完整版入口只在 Live 会话出现）。
    let layout = logic::load_layout_from(&logic::layout_path());
    let ordered = logic::apply_saved_order(&layout.order);
    let entries = Rc::new(RefCell::new(logic::visible_apps(&ordered, logic::is_live_session())));

    let ui = AppWindow::new()?;
    // 桌面 Shell 占满整个输出：Alpha 8 在 VirtualBox 里浮动窗口（1280x800）
    // 一次都没出现，改全屏是最彻底的兜底——窗口不再依赖合成器的摆放，
    // 在任何分辨率下都完整可见；顶栏与卡片仍在 Shell 内部绘制。
    ui.window().set_fullscreen(true);
    ui.set_clock(now_hhmm());
    ui.set_theme_name(settings.theme.as_str().into());
    ui.set_c_bg(parse_hex(palette.bg));
    ui.set_c_panel(parse_hex(palette.panel));
    ui.set_c_card(parse_hex(palette.card));
    ui.set_c_border(parse_hex(palette.card_border));
    ui.set_c_accent(parse_hex(palette.accent));
    ui.set_c_text(parse_hex(palette.text));
    ui.set_c_sub(parse_hex(palette.subtext));
    ui.set_corner_radius(palette.corner_radius);
    // Dolphin 主题用 squircle 轮廓（与网站同一套算法生成的 Path 点列），
    // 其余主题继续用圆角矩形。
    let is_dolphin = settings.theme.as_str() == "Dolphin";
    ui.set_use_squircle(is_dolphin);
    if is_dolphin {
        ui.set_card_path(squircle_path(logic::CARD_W, logic::CARD_H).into());
        ui.set_syscard_path(squircle_path(688.0, 120.0).into());
    }
    ui.set_cards(cards_model(&entries.borrow()));
    ui.set_launcher_cards(launcher_model(&entries.borrow(), ""));

    // 动画 QA 钩子：DOLPHIN_SHELL_ANIM_PREVIEW=1 时把放大翻转替身冻结在
    // 中点帧，供截屏核对（正常运行不设此变量，无任何影响）。
    if std::env::var("DOLPHIN_SHELL_ANIM_PREVIEW").is_ok() {
        let (x, y, w, h, s, o) = logic::anim_frame(1, 0.5);
        ui.set_anim_x(x);
        ui.set_anim_y(y);
        ui.set_anim_w(w);
        ui.set_anim_h(h);
        ui.set_anim_scale_x(s);
        ui.set_anim_opacity(o);
        ui.set_anim_name("浏览器".into());
        ui.set_anim_active(true);
    }

    // 顶栏时钟：每秒刷新。
    {
        let weak = ui.as_weak();
        let clock_timer = Timer::default();
        clock_timer.start(TimerMode::Repeated, Duration::from_millis(1000), move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_clock(now_hhmm());
            }
        });
        // Timer 必须存活到程序结束：泄漏到静态生命周期（Shell 常驻进程，可接受）。
        std::mem::forget(clock_timer);
    }

    // 打开动画定时器：常驻 16ms 一拍，无动画进行时第一行就返回。
    let anim: Rc<RefCell<Option<AnimState>>> = Rc::new(RefCell::new(None));
    {
        let anim_ref = anim.clone();
        let weak = ui.as_weak();
        let anim_timer = Timer::default();
        anim_timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
            let Some(ui) = weak.upgrade() else { return };
            let mut slot = anim_ref.borrow_mut();
            let Some(state) = slot.as_mut() else { return };
            let total = state.mode.duration_ms();
            if total == 0 {
                *slot = None;
                return;
            }
            let p = (state.start.elapsed().as_millis() as f64 / total as f64).min(1.0);
            match state.mode {
                AnimationMode::Flip => {
                    // 放大+翻转替身（Alpha 10 新规格，取代旧「卡片原地翻面」）：
                    // 替身从卡片矩形缓动放大到窗口矩形、同时绕竖轴整翻一圈，
                    // 翻到中点（正反面交替瞬间）启动应用，后段淡出交棒真实窗口。
                    let (x, y, w, h, s, o) = logic::anim_frame(state.idx, p as f32);
                    ui.set_anim_x(x);
                    ui.set_anim_y(y);
                    ui.set_anim_w(w);
                    ui.set_anim_h(h);
                    ui.set_anim_scale_x(s);
                    ui.set_anim_opacity(o);
                    if p >= 0.5 && !state.launched {
                        state.launched = true;
                        let exec = state.exec.clone();
                        spawn_app(&ui, &exec);
                    }
                }
                AnimationMode::Popup => {
                    // 弹入：卡片轻微放大再回落，开始即启动应用。
                    let scale = 1.0 + 0.06 * (std::f64::consts::PI * p).sin();
                    ui.set_flip_scale(scale as f32);
                    if !state.launched {
                        state.launched = true;
                        let exec = state.exec.clone();
                        spawn_app(&ui, &exec);
                    }
                }
                AnimationMode::None => {}
            }
            if p >= 1.0 {
                ui.set_flip_scale(1.0);
                ui.set_flipping_index(-1);
                ui.set_anim_active(false);
                *slot = None;
            }
        });
        std::mem::forget(anim_timer);
    }

    // 启动应用（含打开动画三档）。
    {
        let entries_ref = entries.clone();
        let anim_ref = anim.clone();
        let weak = ui.as_weak();
        let mode = settings.animation;
        ui.on_launch_app(move |idx| {
            let Some(ui) = weak.upgrade() else { return };
            let (exec, name) = {
                let entries = entries_ref.borrow();
                let Some(app) = entries.get(idx as usize) else { return };
                (app.exec.to_string(), app.name.to_string())
            };
            match mode {
                AnimationMode::None => spawn_app(&ui, &exec),
                AnimationMode::Flip => {
                    // 放大翻转走替身层：先亮替身（首帧几何由定时器下一拍写入）
                    ui.set_anim_name(name.into());
                    ui.set_anim_active(true);
                    *anim_ref.borrow_mut() = Some(AnimState {
                        exec,
                        idx: idx as usize,
                        mode,
                        start: Instant::now(),
                        launched: false,
                    });
                }
                AnimationMode::Popup => {
                    ui.set_flipping_index(idx);
                    ui.set_flip_scale(1.0);
                    *anim_ref.borrow_mut() = Some(AnimState {
                        exec,
                        idx: idx as usize,
                        mode,
                        start: Instant::now(),
                        launched: false,
                    });
                }
            }
        });
    }

    // 卡片调序（◀ ▶）：改内存顺序、刷新模型、落盘 desktop.toml。
    {
        let entries_ref = entries.clone();
        let weak = ui.as_weak();
        ui.on_shift_card(move |from, to| {
            if from < 0 || to < 0 {
                return;
            }
            let (from, to) = (from as usize, to as usize);
            let mut entries = entries_ref.borrow_mut();
            if from >= entries.len() || to >= entries.len() || from == to {
                return;
            }
            entries.swap(from, to);
            let order: Vec<String> = entries.iter().map(|a| a.id.to_string()).collect();
            if let Some(ui) = weak.upgrade() {
                ui.set_cards(cards_model(&entries));
                let query = ui.get_search_text().to_string();
                ui.set_launcher_cards(launcher_model(&entries, &query));
            }
            drop(entries);
            let layout = logic::DesktopLayout { order };
            if let Err(e) = logic::save_layout_to(&logic::layout_path(), &layout) {
                eprintln!("保存桌面布局失败: {e}");
            }
        });
    }

    // 搜索：过滤找应用面板列表。
    {
        let entries_ref = entries.clone();
        let weak = ui.as_weak();
        ui.on_search_changed(move |query| {
            if let Some(ui) = weak.upgrade() {
                ui.set_launcher_cards(launcher_model(&entries_ref.borrow(), &query));
            }
        });
    }

    // 系统板块卡。
    {
        let weak = ui.as_weak();
        ui.on_open_settings(move || {
            if let Some(ui) = weak.upgrade() {
                spawn_app(&ui, "dolphin-settings");
            }
        });
    }
    ui.on_power_off(move || {
        let _ = Command::new("loginctl").arg("poweroff").spawn();
    });
    ui.on_reboot(move || {
        let _ = Command::new("loginctl").arg("reboot").spawn();
    });

    ui.run()
}
