//! DolphinOS 自研图形登录屏（Slint）。
//!
//! 默认经 greetd IPC 协议校验（密码校验在 greetd/PAM 服务端完成）；
//! 带 `--demo` 参数时走 DemoAuthenticator 演示规则（见 README，不是真校验）。

mod biometric;
mod greetd;

use greetd::{Authenticator, DemoAuthenticator, GreetdAuthenticator};
use std::cell::RefCell;
use std::rc::Rc;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let demo = std::env::args().any(|a| a == "--demo");

    let ui = AppWindow::new()?;
    if demo {
        ui.set_status_text("演示模式：任意非空用户名 + 至少 4 位密码即可通过（非真实校验）".into());
    } else {
        ui.set_status_text("请输入用户名与密码登录".into());
    }

    let authenticator: Rc<RefCell<Box<dyn Authenticator>>> =
        Rc::new(RefCell::new(if demo {
            Box::new(DemoAuthenticator) as Box<dyn Authenticator>
        } else {
            Box::new(GreetdAuthenticator::from_env()) as Box<dyn Authenticator>
        }));

    // 登录
    {
        let auth = authenticator.clone();
        let ui_handle = ui.as_weak();
        ui.on_try_login(move |user, password| {
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_status_text("正在验证…".into());
                match auth.borrow_mut().login(user.as_str(), password.as_str()) {
                    Ok(()) => {
                        ui.set_status_text("验证通过，正在启动会话…".into());
                    }
                    Err(e) => {
                        ui.set_status_text(format!("登录失败：{e}").into());
                    }
                }
            }
        });
    }

    // 指纹 / 人脸：预留按钮，点击按探测结果显示状态文案
    {
        let ui_handle = ui.as_weak();
        ui.on_fingerprint(move || {
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_status_text(biometric::fingerprint_status().into());
            }
        });
    }
    {
        let ui_handle = ui.as_weak();
        ui.on_face(move || {
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_status_text(biometric::face_status().into());
            }
        });
    }

    ui.run()
}
