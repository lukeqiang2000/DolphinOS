//! 指纹 / 人脸登录的就绪探测（预留入口）。
//!
//! 真实解锁要走 fprintd（指纹）/ Howdy 类服务（人脸）的 PAM 模块，
//! 本阶段只做「探测 + 文案」：检查守护进程/工具与录入痕迹是否存在，
//! 给出不同状态文案，按钮点击时如实提示，不假装能登录。

use std::path::Path;

/// 指纹登录状态文案。
pub fn fingerprint_status() -> String {
    // fprintd 守护/工具任一存在，视为「服务在」；再看有无录入痕迹目录。
    let service_present = ["/usr/lib/fprintd", "/usr/libexec/fprintd", "/usr/bin/fprintd-enroll"]
        .iter()
        .any(|p| Path::new(p).exists())
        || std::env::var_os("DOLPHIN_GREETER_FPRINTD").is_some();
    if !service_present {
        return "指纹登录未就绪：未检测到 fprintd 服务".to_string();
    }
    let enrolled_hint = Path::new("/var/lib/fprint").exists();
    if enrolled_hint {
        "指纹登录未就绪：fprintd 已安装，PAM 指纹模块接入后启用".to_string()
    } else {
        "指纹登录未就绪：需 fprintd 录入后启用".to_string()
    }
}

/// 人脸登录状态文案。
pub fn face_status() -> String {
    let service_present = ["/etc/howdy", "/usr/lib/security/howdy", "/usr/bin/howdy"]
        .iter()
        .any(|p| Path::new(p).exists())
        || std::env::var_os("DOLPHIN_GREETER_FACE").is_some();
    if service_present {
        "人脸登录未就绪：已检测到人脸服务，PAM 模块接入后启用".to_string()
    } else {
        "人脸登录未就绪：未检测到人脸识别服务（需 Howdy 类组件录入后启用）".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_are_not_ready_without_services() {
        // 测试机通常未装 fprintd/howdy；无论哪种文案都必须明确「未就绪」。
        assert!(fingerprint_status().contains("未就绪"));
        assert!(face_status().contains("未就绪"));
    }
}
