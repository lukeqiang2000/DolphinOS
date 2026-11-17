//! DolphinOS 首次设置向导（Slint）。
//!
//! 界面只负责呈现与收集输入，步骤/校验/落盘全在 wizard.rs 的状态机里；
//! 完成时经 dolphin-config 写共享配置（oobe_done = true 等一并落盘）。

mod wizard;

use dolphin_config::{palette, Theme};
use std::cell::RefCell;
use std::rc::Rc;
use wizard::{BiometricChoice, WizardState};

slint::include_modules!();

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
    let ui = AppWindow::new()?;
    let state = Rc::new(RefCell::new(WizardState::new()));
    refresh(&ui, &state.borrow());

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

    ui.run()
}
