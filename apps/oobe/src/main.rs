//! DolphinOS 首次设置向导（Slint）。
//!
//! 界面只负责呈现与收集输入，步骤/校验/落盘全在 wizard.rs 的状态机里；
//! 完成时经 dolphin-config 写共享配置（oobe_done = true 等一并落盘）。

mod wizard;

use dolphin_config::{palette, Theme};
use std::cell::RefCell;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::rc::Rc;
use wizard::{BiometricChoice, WizardState};

slint::include_modules!();

/// 当前是不是 Live 会话（与 Shell 同口径：内核命令行带 boot=live）。
fn is_live_session() -> bool {
    match std::fs::read_to_string("/proc/cmdline") {
        Ok(cmdline) => cmdline.split_whitespace().any(|t| t == "boot=live"),
        Err(_) => false,
    }
}

/// 枚举可安装的整盘设备（跳过 loop/ram/光盘/映射设备）。
fn enumerate_disks() -> Vec<InstallDisk> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir("/sys/block") else {
        return out;
    };
    let mut names: Vec<String> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| {
            !(n.starts_with("loop") || n.starts_with("ram") || n.starts_with("sr") || n.starts_with("dm-"))
        })
        .collect();
    names.sort();
    for name in names {
        let sectors: u64 = std::fs::read_to_string(format!("/sys/block/{name}/size"))
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        let gib = sectors as f64 * 512.0 / 1_073_741_824.0;
        let model = std::fs::read_to_string(format!("/sys/block/{name}/device/model"))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let label = if model.is_empty() {
            format!("/dev/{name} · {gib:.0} GB")
        } else {
            format!("/dev/{name} · {gib:.0} GB · {model}")
        };
        out.push(InstallDisk {
            value: format!("/dev/{name}").into(),
            label: label.into(),
        });
    }
    out
}

fn hex_to_color(hex: &str) -> slint::Color {
    let h = hex.trim_start_matches('#');
    if h.len() == 6 {
        let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(0);
        slint::Color::from_rgb_u8(r, g, b)
    } else {
        slint::Color::from_rgb_u8(0, 0, 0)
    }
}

/// 把状态机当前状态同步到界面。
fn refresh(ui: &AppWindow, state: &WizardState) {
    let idx = state.step.index();
    ui.set_step_index(idx as i32);
    ui.set_step_name(state.step.title().into());
    ui.set_progress_text(format!("第 {}/8 步", idx + 1).into());
    ui.set_can_go_back(idx > 0 && state.step != wizard::Step::Done);
    ui.set_selected_language(state.draft.language.clone().into());
    ui.set_selected_theme_name(state.draft.theme.display_name().into());
    ui.set_selected_biometric(
        match state.biometric_choice {
            BiometricChoice::Fingerprint => "指纹",
            BiometricChoice::Face => "人脸",
            BiometricChoice::Skip => "跳过",
            BiometricChoice::None => "未选择",
        }
        .into(),
    );
    let pal = palette(state.draft.theme);
    ui.set_preview_bg(hex_to_color(pal.bg));
    ui.set_preview_card(hex_to_color(pal.card));
    ui.set_preview_accent(hex_to_color(pal.accent));
    ui.set_hint_text(
        state
            .validation_message()
            .map(|s| s.to_string())
            .unwrap_or_default()
            .into(),
    );
}

/// 把界面输入框/勾选的当前值收回状态机草稿。
fn sync_from_ui(ui: &AppWindow, state: &mut WizardState) {
    state.network_name = ui.get_network_name().to_string();
    state.network_skipped = state.network_name.trim().is_empty();
    state.draft.account_name = ui.get_account_name().to_string();
    state.password = ui.get_account_password().to_string();
    state.draft.import_source = ui.get_import_path().to_string();
    state.draft.telemetry = ui.get_telemetry();
    state.draft.crash_reports = ui.get_crash_reports();
}

fn main() -> Result<(), slint::PlatformError> {
    let args: Vec<String> = std::env::args().collect();
    // --autostart：桌面 autostart 每次登录都会拉起；向导已完成过就静默
    // 退出（装盘系统只在新用户首次登录时弹；Live 主目录不落盘、每次弹）。
    if args.iter().any(|a| a == "--autostart") && dolphin_config::load_settings().oobe_done {
        return Ok(());
    }
    let install_mode = args.iter().any(|a| a == "--install");

    let ui = AppWindow::new()?;
    let state = Rc::new(RefCell::new(WizardState::new()));
    refresh(&ui, &state.borrow());
    ui.set_is_live(is_live_session());
    ui.set_install_disks(slint::ModelRc::new(slint::VecModel::from(enumerate_disks())));
    if install_mode {
        ui.set_show_install(true);
        ui.set_install_user("user".into());
    }

    // 下一步
    {
        let state = state.clone();
        let ui_handle = ui.as_weak();
        ui.on_next_step(move || {
            if let Some(ui) = ui_handle.upgrade() {
                let mut st = state.borrow_mut();
                sync_from_ui(&ui, &mut st);
                if st.next() {
                    ui.set_hint_text("".into());
                }
                refresh(&ui, &st);
            }
        });
    }

    // 上一步
    {
        let state = state.clone();
        let ui_handle = ui.as_weak();
        ui.on_back_step(move || {
            if let Some(ui) = ui_handle.upgrade() {
                let mut st = state.borrow_mut();
                sync_from_ui(&ui, &mut st);
                st.back();
                refresh(&ui, &st);
            }
        });
    }

    // 完成（Import 步的「完成」按钮）：落盘共享配置
    {
        let state = state.clone();
        let ui_handle = ui.as_weak();
        ui.on_finish(move || {
            if let Some(ui) = ui_handle.upgrade() {
                let mut st = state.borrow_mut();
                sync_from_ui(&ui, &mut st);
                match st.finish() {
                    Ok(()) => refresh(&ui, &st),
                    Err(e) => ui.set_hint_text(format!("保存设置失败：{e}").into()),
                }
            }
        });
    }

    // 语言选择
    {
        let state = state.clone();
        let ui_handle = ui.as_weak();
        ui.on_select_language(move |code| {
            if let Some(ui) = ui_handle.upgrade() {
                let mut st = state.borrow_mut();
                st.draft.language = code.to_string();
                refresh(&ui, &st);
            }
        });
    }

    // 主题选择：点选即时刷新预览配色
    {
        let state = state.clone();
        let ui_handle = ui.as_weak();
        ui.on_select_theme(move |name| {
            if let Some(ui) = ui_handle.upgrade() {
                let mut st = state.borrow_mut();
                if let Some(theme) = Theme::from_str_loose(name.as_str()) {
                    st.draft.theme = theme;
                }
                refresh(&ui, &st);
            }
        });
    }

    // 生物识别选择
    {
        let state = state.clone();
        let ui_handle = ui.as_weak();
        ui.on_select_biometric(move |kind| {
            if let Some(ui) = ui_handle.upgrade() {
                let mut st = state.borrow_mut();
                st.biometric_choice = match kind.as_str() {
                    "fingerprint" => BiometricChoice::Fingerprint,
                    "face" => BiometricChoice::Face,
                    _ => BiometricChoice::Skip,
                };
                refresh(&ui, &st);
            }
        });
    }

    // 进安装页：预填用户名（向导账号步填过的优先，其次当前登录名兜底 user）
    {
        let state = state.clone();
        let ui_handle = ui.as_weak();
        ui.on_open_install(move || {
            if let Some(ui) = ui_handle.upgrade() {
                let mut st = state.borrow_mut();
                sync_from_ui(&ui, &mut st);
                let name = st.draft.account_name.trim().to_string();
                ui.set_install_user(if name.is_empty() { "user".into() } else { name.into() });
                ui.set_show_install(true);
            }
        });
    }

    // 离开安装页回向导
    {
        let ui_handle = ui.as_weak();
        ui.on_close_install(move || {
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_show_install(false);
            }
        });
    }

    // 开始安装：把参数写进运行时目录的配置文件（sudo 会过滤环境变量，
    // 引擎支持配置文件入参），再以 sudo -n 拉起引擎、逐行回读进度。
    {
        let ui_handle = ui.as_weak();
        ui.on_start_install(move || {
            let Some(ui) = ui_handle.upgrade() else { return };
            let disk = ui.get_install_disk().to_string();
            let user = ui.get_install_user().to_string();
            let password = ui.get_install_password().to_string();
            if disk.is_empty() || user.is_empty() || password.chars().count() < 4 {
                return;
            }
            let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
            let conf_path = format!("{runtime}/dolphin-install.conf");
            let conf = format!(
                "DOLPHIN_INSTALL_DISK={disk}\nDOLPHIN_INSTALL_USER={user}\nDOLPHIN_INSTALL_PASSWORD={password}\nDOLPHIN_INSTALL_YES=1\n"
            );
            if std::fs::write(&conf_path, conf).is_err() {
                ui.set_install_log("写入安装配置失败".into());
                ui.set_install_failed(true);
                return;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&conf_path, std::fs::Permissions::from_mode(0o600));
            }
            ui.set_install_running(true);
            ui.set_install_failed(false);
            ui.set_install_done(false);
            ui.set_install_log("正在启动安装引擎…".into());

            let weak = ui.as_weak();
            std::thread::spawn(move || {
                let child = Command::new("sudo")
                    .args(["-n", "/usr/local/bin/dolphin-install", &conf_path])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn();
                let mut log = String::new();
                let status = match child {
                    Ok(mut child) => {
                        if let Some(out) = child.stdout.take() {
                            for line in BufReader::new(out).lines() {
                                let Ok(line) = line else { break };
                                log.push_str(&line);
                                log.push('\n');
                                let snapshot = log.clone();
                                let _ = weak.upgrade_in_event_loop(move |ui| {
                                    ui.set_install_log(snapshot.into());
                                });
                            }
                        }
                        child.wait().ok()
                    }
                    Err(e) => {
                        log.push_str(&format!("启动安装引擎失败：{e}\n"));
                        None
                    }
                };
                let ok = status.map(|s| s.success()).unwrap_or(false);
                let final_log = log;
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    ui.set_install_log(final_log.into());
                    ui.set_install_running(false);
                    ui.set_install_done(ok);
                    ui.set_install_failed(!ok);
                });
                // 配置含密码：装完（无论成败）立即删
                let _ = std::fs::remove_file(&conf_path);
            });
        });
    }

    // 安装完成后的重启
    ui.on_reboot_now(move || {
        let _ = Command::new("sudo").args(["-n", "systemctl", "reboot"]).spawn();
    });

    // 关闭窗口（暂不安装 / 装盘系统的完成页关闭）
    {
        let ui_handle = ui.as_weak();
        ui.on_finish_close(move || {
            if let Some(ui) = ui_handle.upgrade() {
                let _ = ui.hide();
            }
        });
    }

    ui.run()
}
