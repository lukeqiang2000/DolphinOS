//! DolphinOS 自研视频播放器（Slint 界面）。
//!
//! 默认后端是 SimulatedBackend（只跑状态机与 UI，不真正解码）；
//! 启用 feature "gst" 时优先用 GStreamer playbin 后端，初始化失败
//! 自动回退模拟后端。命令行参数可传视频文件或目录，全部收进列表。

mod logic;
#[cfg(feature = "gst")]
mod gst_backend;

use logic::{Backend, PlayerCore, SimulatedBackend};
use slint::{ModelRc, Timer, TimerMode, VecModel};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

slint::include_modules!();

fn make_backend() -> Box<dyn Backend> {
    #[cfg(feature = "gst")]
    {
        match gst_backend::GstBackend::new() {
            Ok(b) => return Box::new(b),
            Err(e) => {
                eprintln!("GStreamer 后端初始化失败，回退模拟后端：{}", e);
            }
        }
    }
    Box::new(SimulatedBackend::new())
}

fn playlist_text(core: &PlayerCore) -> String {
    let mut out = String::new();
    for (i, p) in core.playlist.items.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let marker = if core.playlist.current == Some(i) {
            "▶ "
        } else {
            "   "
        };
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| p.display().to_string());
        out.push_str(marker);
        out.push_str(&name);
    }
    if core.playlist.items.is_empty() {
        out.push_str("（列表为空，点上方「打开文件」选视频/音乐，或用命令行参数传入）");
    }
    out
}

fn sync_ui(ui: &AppWindow, core: &PlayerCore) {
    ui.set_playlist_text(playlist_text(core).into());
    let current_name = core
        .playlist
        .current_path()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "未选择文件".to_string());
    ui.set_current_name(current_name.into());
    ui.set_state_text(core.state.label().into());
    ui.set_position_text(logic::format_time(core.position_secs as u64).into());
    ui.set_duration_text(logic::format_time(core.duration_secs as u64).into());
    ui.set_progress_value(core.position_secs as f32);
    ui.set_progress_max(core.duration_secs.max(0.0) as f32);
    ui.set_volume(core.volume as f32);
    ui.set_playing(core.state == logic::PlayState::Playing);
}

/// 模拟后端没有真实时长：开始播放且时长未知时给一个占位时长，
/// 让进度条与自动切歌逻辑在无 GStreamer 环境也能演示。
fn ensure_demo_duration(core: &mut PlayerCore) {
    if core.duration_secs <= 0.0 {
        core.duration_secs = 180.0;
    }
}

/// 「打开文件」面板状态：当前目录 + 与 UI 模型平行的路径表。
struct BrowserState {
    dir: PathBuf,
    paths: Vec<PathBuf>,
}

/// 把 state.dir 的目录内容刷进 UI 模型（目录名带 / 后缀区分）。
fn refresh_browser(ui: &AppWindow, state: &mut BrowserState) {
    let entries = logic::list_media_dir(&state.dir);
    state.paths = entries.iter().map(|e| e.path.clone()).collect();
    let model: Vec<BrowserEntry> = entries
        .iter()
        .map(|e| {
            let mut label = e
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| e.path.display().to_string());
            if e.is_dir {
                label.push('/');
            }
            BrowserEntry {
                label: label.into(),
                is_dir: e.is_dir,
            }
        })
        .collect();
    ui.set_browser_entries(ModelRc::new(VecModel::from(model)));
    ui.set_browser_path(state.dir.display().to_string().into());
}

fn main() -> Result<(), slint::PlatformError> {
    let mut core = PlayerCore::new(make_backend());
    for arg in std::env::args().skip(1) {
        let path = PathBuf::from(&arg);
        if path.is_dir() {
            core.playlist.add_dir(&path);
        } else if path.is_file() {
            core.playlist.add(path);
        }
    }
    let core = Rc::new(RefCell::new(core));

    let ui = AppWindow::new()?;
    sync_ui(&ui, &core.borrow());

    ui.on_play_pause({
        let core = Rc::clone(&core);
        let ui_handle = ui.as_weak();
        move || {
            let mut c = core.borrow_mut();
            c.toggle_play_pause();
            if c.state == logic::PlayState::Playing {
                ensure_demo_duration(&mut c);
            }
            if let Some(ui) = ui_handle.upgrade() {
                sync_ui(&ui, &c);
            }
        }
    });

    ui.on_stop({
        let core = Rc::clone(&core);
        let ui_handle = ui.as_weak();
        move || {
            let mut c = core.borrow_mut();
            c.stop();
            if let Some(ui) = ui_handle.upgrade() {
                sync_ui(&ui, &c);
            }
        }
    });

    ui.on_next_track({
        let core = Rc::clone(&core);
        let ui_handle = ui.as_weak();
        move || {
            let mut c = core.borrow_mut();
            c.next();
            ensure_demo_duration(&mut c);
            if let Some(ui) = ui_handle.upgrade() {
                sync_ui(&ui, &c);
            }
        }
    });

    ui.on_prev_track({
        let core = Rc::clone(&core);
        let ui_handle = ui.as_weak();
        move || {
            let mut c = core.borrow_mut();
            c.prev();
            ensure_demo_duration(&mut c);
            if let Some(ui) = ui_handle.upgrade() {
                sync_ui(&ui, &c);
            }
        }
    });

    ui.on_seek({
        let core = Rc::clone(&core);
        let ui_handle = ui.as_weak();
        move |secs| {
            let mut c = core.borrow_mut();
            c.seek(secs as f64);
            if let Some(ui) = ui_handle.upgrade() {
                sync_ui(&ui, &c);
            }
        }
    });

    ui.on_volume_changed({
        let core = Rc::clone(&core);
        move |vol| {
            core.borrow_mut().set_volume(vol as i32);
        }
    });

    // 「打开文件」面板（Alpha 10 修复：旧版 GUI 里没有任何打开
    // 文件的入口，只能靠命令行参数）。起始目录为用户主目录。
    let browser = Rc::new(RefCell::new(BrowserState {
        dir: std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/")),
        paths: Vec::new(),
    }));

    ui.on_toggle_browser({
        let browser = Rc::clone(&browser);
        let ui_handle = ui.as_weak();
        move || {
            if let Some(ui) = ui_handle.upgrade() {
                refresh_browser(&ui, &mut browser.borrow_mut());
                ui.set_browser_open(true);
            }
        }
    });

    ui.on_browser_up({
        let browser = Rc::clone(&browser);
        let ui_handle = ui.as_weak();
        move || {
            if let Some(ui) = ui_handle.upgrade() {
                let mut state = browser.borrow_mut();
                if let Some(parent) = state.dir.parent().map(|p| p.to_path_buf()) {
                    state.dir = parent;
                }
                refresh_browser(&ui, &mut state);
            }
        }
    });

    ui.on_browser_activate({
        let core = Rc::clone(&core);
        let browser = Rc::clone(&browser);
        let ui_handle = ui.as_weak();
        move |idx| {
            let Some(ui) = ui_handle.upgrade() else { return };
            let mut state = browser.borrow_mut();
            let Some(path) = state.paths.get(idx as usize).cloned() else { return };
            if path.is_dir() {
                state.dir = path;
                refresh_browser(&ui, &mut state);
                return;
            }
            drop(state);
            // 点文件：加入列表、跳到该项并立即播放，收起面板。
            let mut c = core.borrow_mut();
            c.playlist.add(path);
            let last = c.playlist.items.len() - 1;
            let _ = c.playlist.jump(last);
            c.play_current();
            ensure_demo_duration(&mut c);
            sync_ui(&ui, &c);
            ui.set_browser_open(false);
        }
    });

    // 播放进度推进：250ms 一拍
    let ui_handle = ui.as_weak();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_millis(250), move || {
        let mut c = core.borrow_mut();
        let before = (c.position_secs, c.playlist.current, c.state);
        c.tick(0.25);
        let after = (c.position_secs, c.playlist.current, c.state);
        if before != after {
            if let Some(ui) = ui_handle.upgrade() {
                sync_ui(&ui, &c);
            }
        }
    });

    ui.run()
}
