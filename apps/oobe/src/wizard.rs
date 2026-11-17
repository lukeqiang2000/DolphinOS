//! 首次设置向导状态机（纯逻辑，不依赖 Slint）。
//!
//! 8 步：语言 → 网络 → 账号 → 生物识别 → 主题 → 隐私 → 数据导入 → 完成。
//! 草稿直接持一份 dolphin-config 的 `Settings`，`finish()` 时补
//! `oobe_done = true` 并落盘到共享配置路径，Shell/设置应用随后读到同一份。

use dolphin_config::{save_settings_to, settings_path, Settings};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Language,
    Network,
    Account,
    Biometric,
    ThemeStep,
    Privacy,
    Import,
    Done,
}

impl Step {
    pub fn all() -> [Step; 8] {
        [
            Step::Language,
            Step::Network,
            Step::Account,
            Step::Biometric,
            Step::ThemeStep,
            Step::Privacy,
            Step::Import,
            Step::Done,
        ]
    }

    /// 0..=7
    pub fn index(&self) -> usize {
        Self::all().iter().position(|s| s == self).unwrap_or(0)
    }

    pub fn from_index(idx: usize) -> Step {
        Self::all()[idx.min(7)]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Step::Language => "语言",
            Step::Network => "网络",
            Step::Account => "账号",
            Step::Biometric => "指纹 / 人脸",
            Step::ThemeStep => "主题",
            Step::Privacy => "隐私",
            Step::Import => "数据导入",
            Step::Done => "完成",
        }
    }
}

/// 生物识别选择（本步只记录意向，真正录入在登录后由系统设置完成）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiometricChoice {
    None,
    Fingerprint,
    Face,
    Skip,
}

impl Default for BiometricChoice {
    fn default() -> Self {
        BiometricChoice::Skip
    }
}

#[derive(Debug, Clone)]
pub struct WizardState {
    pub step: Step,
    /// 设置草稿：语言/主题/账号名/隐私/导入来源都先写这里，finish 时落盘。
    pub draft: Settings,
    /// 网络名（Wi-Fi SSID 等，仅向导内记录，不进 Settings）。
    pub network_name: String,
    pub network_skipped: bool,
    pub biometric_choice: BiometricChoice,
    /// 账号密码：只在内存里做长度校验，校验后即丢弃，**不落盘**。
    pub password: String,
}

impl Default for WizardState {
    fn default() -> Self {
        WizardState {
            step: Step::Language,
            draft: Settings::default(),
            network_name: String::new(),
            network_skipped: false,
            biometric_choice: BiometricChoice::default(),
            password: String::new(),
        }
    }
}

impl WizardState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前步是否允许前进。校验不过时界面用 validation_message() 显示原因。
    pub fn can_next(&self) -> bool {
        match self.step {
            // 账号步：显示名必填；密码可空（稍后设置），填了则至少 4 位。
            Step::Account => {
                !self.draft.account_name.trim().is_empty()
                    && (self.password.is_empty() || self.password.chars().count() >= 4)
            }
            Step::Done => false,
            // Import 步用 finish() 收尾，不走 next()。
            Step::Import => false,
            _ => true,
        }
    }

    pub fn validation_message(&self) -> Option<&'static str> {
        if self.can_next() {
            return None;
        }
        match self.step {
            Step::Account => {
                if self.draft.account_name.trim().is_empty() {
                    Some("请先填写账号名")
                } else {
                    Some("密码至少 4 位，或清空密码稍后再设置")
                }
            }
            _ => None,
        }
    }

    /// 前进一步；校验不过或已到 Import/Done 时返回 false 且不动。
    pub fn next(&mut self) -> bool {
        if !self.can_next() {
            return false;
        }
        let idx = self.step.index();
        if idx >= 7 {
            return false;
        }
        self.step = Step::from_index(idx + 1);
        true
    }

    /// 后退一步；在第一步时返回 false 且不动。
    pub fn back(&mut self) -> bool {
        let idx = self.step.index();
        if idx == 0 {
            return false;
        }
        self.step = Step::from_index(idx - 1);
        true
    }

    /// 完成向导：把草稿落盘到指定路径（测试用注入版本）。
    pub fn finish_to(&mut self, path: &Path) -> std::io::Result<()> {
        let mut out = self.draft.clone();
        out.oobe_done = true;
        save_settings_to(path, &out)?;
        self.draft = out;
        // 密码只做内存校验，收尾时立即丢弃，任何路径都不落盘。
        self.password.clear();
        self.step = Step::Done;
        Ok(())
    }

    /// 完成向导并写入共享配置默认路径。
    pub fn finish(&mut self) -> std::io::Result<()> {
        self.finish_to(&settings_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dolphin_config::{load_settings_from, Theme};

    fn temp_path(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!("dolphinos-oobe-test-{}-{}", std::process::id(), tag))
            .join("settings.toml")
    }

    #[test]
    fn step_navigation_boundaries() {
        let mut w = WizardState::new();
        assert_eq!(w.step, Step::Language);
        assert!(!w.back(), "第一步不能后退");
        assert_eq!(w.step, Step::Language);
        assert!(w.next());
        assert_eq!(w.step, Step::Network);
        assert!(w.back());
        assert_eq!(w.step, Step::Language);
        // 一路走到 Import：Language→Network→Account(需账号名)→Biometric→Theme→Privacy→Import
        assert!(w.next()); // Network
        w.draft.account_name = "luke".into();
        assert!(w.next()); // Account（校验在离开 Account 步时生效，先到 Account）
        assert_eq!(w.step, Step::Account);
        assert!(w.next()); // Biometric
        assert!(w.next()); // ThemeStep
        assert!(w.next()); // Privacy
        assert!(w.next()); // Import
        assert_eq!(w.step, Step::Import);
        assert!(!w.next(), "Import 步不能用 next 越过 finish");
        assert_eq!(w.step, Step::Import);
    }

    #[test]
    fn account_validation_blocks_and_allows() {
        let mut w = WizardState::new();
        w.step = Step::Account;
        // 账号名为空：拦截
        assert!(!w.can_next());
        assert!(!w.next());
        assert_eq!(w.step, Step::Account);
        assert!(w.validation_message().is_some());
        // 账号名有了、密码为空：放行
        w.draft.account_name = "luke".into();
        assert!(w.can_next());
        // 密码太短：拦截
        w.password = "abc".into();
        assert!(!w.can_next());
        assert!(!w.next());
        // 密码够长：放行
        w.password = "abcd".into();
        assert!(w.can_next());
        assert!(w.next());
        assert_eq!(w.step, Step::Biometric);
    }

    #[test]
    fn finish_persists_fields_to_disk() {
        let path = temp_path("finish");
        let mut w = WizardState::new();
        w.draft.language = "en-US".into();
        w.draft.theme = Theme::Metro;
        w.draft.account_name = "luke".into();
        w.draft.telemetry = true;
        w.draft.crash_reports = false;
        w.draft.import_source = "/mnt/usb/backup".into();
        w.password = "secret-password".into();
        w.biometric_choice = BiometricChoice::Fingerprint;
        w.finish_to(&path).expect("finish 落盘失败");

        assert_eq!(w.step, Step::Done);
        assert!(w.password.is_empty(), "finish 后密码必须已丢弃");

        let saved = load_settings_from(&path);
        assert!(saved.oobe_done, "oobe_done 必须落盘为 true");
        assert_eq!(saved.language, "en-US");
        assert_eq!(saved.theme, Theme::Metro);
        assert_eq!(saved.account_name, "luke");
        assert!(saved.telemetry);
        assert!(!saved.crash_reports);
        assert_eq!(saved.import_source, "/mnt/usb/backup");

        // 密码绝不出现在落盘文件里
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("secret-password"), "密码不得落盘");

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
