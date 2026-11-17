//! DolphinOS 设置应用：所有改动即时写入共享配置（dolphin-config），
//! Shell 下次启动（及读取同一配置的组件）即生效。

mod logic;

use std::cell::RefCell;
use std::rc::Rc;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let settings = Rc::new(RefCell::new(dolphin_config::load_settings()));

    let ui = AppWindow::new()?;
    {
        let s = settings.borrow();
        ui.set_theme_index(logic::theme_index(&s));
        ui.set_animation_index(logic::animation_index(&s));
        ui.set_language_index(logic::language_index(&s));
        ui.set_telemetry(s.telemetry);
        ui.set_crash_reports(s.crash_reports);
    }
    ui.set_config_path(dolphin_config::settings_path().to_string_lossy().into_owned().into());

    // 统一保存 + 状态回显。
    macro_rules! saver {
        () => {{
            let settings_ref = settings.clone();
            let weak = ui.as_weak();
            move || {
                if let Some(ui) = weak.upgrade() {
                    match logic::save(&settings_ref.borrow()) {
                        Ok(()) => ui.set_status_text("已保存".into()),
                        Err(e) => ui.set_status_text(format!("保存失败：{e}").into()),
                    }
                }
            }
        }};
    }

    {
        let settings_ref = settings.clone();
        let save = saver!();
        ui.on_theme_picked(move |idx| {
            if logic::set_theme(&mut settings_ref.borrow_mut(), idx) {
                save();
            }
        });
    }
    {
        let settings_ref = settings.clone();
        let save = saver!();
        ui.on_animation_picked(move |idx| {
            if logic::set_animation(&mut settings_ref.borrow_mut(), idx) {
                save();
            }
        });
    }
    {
        let settings_ref = settings.clone();
        let save = saver!();
        ui.on_language_picked(move |idx| {
            if logic::set_language(&mut settings_ref.borrow_mut(), idx) {
                save();
            }
        });
    }
    {
        let settings_ref = settings.clone();
        let save = saver!();
        let weak = ui.as_weak();
        ui.on_toggle_telemetry(move || {
            let new_val = {
                let mut s = settings_ref.borrow_mut();
                s.telemetry = !s.telemetry;
                s.telemetry
            };
            if let Some(ui) = weak.upgrade() {
                ui.set_telemetry(new_val);
            }
            save();
        });
    }
    {
        let settings_ref = settings.clone();
        let save = saver!();
        let weak = ui.as_weak();
        ui.on_toggle_crash(move || {
            let new_val = {
                let mut s = settings_ref.borrow_mut();
                s.crash_reports = !s.crash_reports;
                s.crash_reports
            };
            if let Some(ui) = weak.upgrade() {
                ui.set_crash_reports(new_val);
            }
            save();
        });
    }

    ui.run()
}
