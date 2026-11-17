//! DolphinOS 共享配置库。
//!
//! 设置统一存放在 `~/.config/dolphinos/settings.toml`（可用环境变量
//! `DOLPHINOS_CONFIG_DIR` 覆盖目录，主要供测试与便携运行）。
//! Shell、设置应用、首次设置向导都通过本库读写同一份配置，保证
//! 「设置里改主题、桌面立即同款」这类联动有唯一真源。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 桌面主题（形态级皮肤，见 docs/SPEC.md 第 3 节）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Dolphin,
    Metro,
    Fluent,
    Pixel,
}

impl Theme {
    pub fn all() -> [Theme; 4] {
        [Theme::Dolphin, Theme::Metro, Theme::Fluent, Theme::Pixel]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::Dolphin => "Dolphin",
            Theme::Metro => "Metro",
            Theme::Fluent => "Fluent",
            Theme::Pixel => "Pixel",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Theme::Dolphin => "Dolphin（默认）",
            Theme::Metro => "Metro（仿 Windows 8）",
            Theme::Fluent => "Fluent（仿 Windows 10）",
            Theme::Pixel => "Pixel（仿 Google Pixel）",
        }
    }

    pub fn from_str_loose(s: &str) -> Option<Theme> {
        match s.trim().to_ascii_lowercase().as_str() {
            "dolphin" => Some(Theme::Dolphin),
            "metro" => Some(Theme::Metro),
            "fluent" => Some(Theme::Fluent),
            "pixel" => Some(Theme::Pixel),
            _ => None,
        }
    }

    /// 下一个主题（设置里「轮换」按钮与测试用）。
    pub fn next(&self) -> Theme {
        let all = Theme::all();
        let idx = all.iter().position(|t| t == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dolphin
    }
}

/// 打开动画三档（规格定案：翻转 → 弹入 → 无动画，逐级可关）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimationMode {
    Flip,
    Popup,
    None,
}

impl AnimationMode {
    pub fn all() -> [AnimationMode; 3] {
        [AnimationMode::Flip, AnimationMode::Popup, AnimationMode::None]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AnimationMode::Flip => "Flip",
            AnimationMode::Popup => "Popup",
            AnimationMode::None => "None",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AnimationMode::Flip => "翻转打开",
            AnimationMode::Popup => "弹入打开",
            AnimationMode::None => "无动画",
        }
    }

    pub fn from_str_loose(s: &str) -> Option<AnimationMode> {
        match s.trim().to_ascii_lowercase().as_str() {
            "flip" => Some(AnimationMode::Flip),
            "popup" => Some(AnimationMode::Popup),
            "none" => Some(AnimationMode::None),
            _ => None,
        }
    }

    /// 动画时长（毫秒）。无动画恒为 0。
    pub fn duration_ms(&self) -> u64 {
        match self {
            AnimationMode::Flip => 320,
            AnimationMode::Popup => 160,
            AnimationMode::None => 0,
        }
    }
}

impl Default for AnimationMode {
    fn default() -> Self {
        AnimationMode::Flip
    }
}

/// 全局设置（settings.toml 的内容）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: Theme,
    pub animation: AnimationMode,
    /// 界面语言，如 "zh-CN"、"en-US"。
    pub language: String,
    /// 本机账号显示名（首次设置向导写入，可空）。
    pub account_name: String,
    /// 隐私：是否允许匿名使用统计（默认关，隐私优先）。
    pub telemetry: bool,
    /// 隐私：是否允许崩溃报告上传（默认关）。
    pub crash_reports: bool,
    /// 首次设置向导是否已完成。
    pub oobe_done: bool,
    /// 数据导入来源路径（向导记录，可空）。
    pub import_source: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: Theme::Dolphin,
            animation: AnimationMode::Flip,
            language: "zh-CN".to_string(),
            account_name: String::new(),
            telemetry: false,
            crash_reports: false,
            oobe_done: false,
            import_source: String::new(),
        }
    }
}

/// 单个主题的配色与形状参数（数据层皮肤）。
/// Shell 与各应用按当前主题取这份参数渲染，至少保证配色/圆角/形状指数随主题切换。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub bg: &'static str,
    pub panel: &'static str,
    pub card: &'static str,
    pub card_border: &'static str,
    pub accent: &'static str,
    pub text: &'static str,
    pub subtext: &'static str,
    /// 卡片圆角（px）：Dolphin 大圆角 squircle、Metro 直角、Fluent 小圆角、Pixel 大圆角胶囊感。
    pub corner_radius: f32,
    /// 超椭圆指数 n（2=椭圆，越大越方）；Metro 直角用大指数近似方角。
    pub shape_exponent: f32,
}

pub fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::Dolphin => Palette {
            bg: "#0b0e11",
            panel: "#10151b",
            card: "#1a2129",
            card_border: "#2b3644",
            accent: "#1370f3",
            text: "#e8eef4",
            subtext: "#8fa3b8",
            corner_radius: 18.0,
            shape_exponent: 4.5,
        },
        Theme::Metro => Palette {
            bg: "#0f2b46",
            panel: "#0b2036",
            card: "#1370f3",
            card_border: "#2f7fe0",
            accent: "#ffb900",
            text: "#ffffff",
            subtext: "#cfe3f7",
            corner_radius: 0.0,
            shape_exponent: 12.0,
        },
        Theme::Fluent => Palette {
            bg: "#202020",
            panel: "#2c2c2c",
            card: "#3a3a3a",
            card_border: "#4d4d4d",
            accent: "#4cc2ff",
            text: "#ffffff",
            subtext: "#b3b3b3",
            corner_radius: 4.0,
            shape_exponent: 3.0,
        },
        Theme::Pixel => Palette {
            bg: "#0e1512",
            panel: "#16211b",
            card: "#1e2b23",
            card_border: "#33503e",
            accent: "#7dd3a8",
            text: "#eef7f0",
            subtext: "#a7c4ae",
            corner_radius: 24.0,
            shape_exponent: 4.0,
        },
    }
}

/// 配置目录：`$DOLPHINOS_CONFIG_DIR` 或 `~/.config/dolphinos`。
pub fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("DOLPHINOS_CONFIG_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(".config").join("dolphinos")
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.toml")
}

/// 从指定路径读设置；文件不存在或解析失败时返回默认值（不报错，桌面不能因配置坏掉起不来）。
pub fn load_settings_from(path: &std::path::Path) -> Settings {
    match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

/// 把设置写到指定路径（自动建目录）。
pub fn save_settings_to(path: &std::path::Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, text)
}

pub fn load_settings() -> Settings {
    load_settings_from(&settings_path())
}

pub fn save_settings(settings: &Settings) -> std::io::Result<()> {
    save_settings_to(&settings_path(), settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_settings_file(tag: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("dolphinos-config-test-{}-{}", std::process::id(), tag))
            .join("settings.toml")
    }

    #[test]
    fn defaults_are_dolphin_flip_zh() {
        let s = Settings::default();
        assert_eq!(s.theme, Theme::Dolphin);
        assert_eq!(s.animation, AnimationMode::Flip);
        assert_eq!(s.language, "zh-CN");
        assert!(!s.telemetry, "隐私统计默认必须关");
        assert!(!s.oobe_done);
    }

    #[test]
    fn save_load_roundtrip() {
        let path = temp_settings_file("roundtrip");
        let s = Settings {
            theme: Theme::Metro,
            animation: AnimationMode::Popup,
            language: "en-US".into(),
            account_name: "luke".into(),
            telemetry: true,
            crash_reports: false,
            oobe_done: true,
            import_source: "/mnt/usb".into(),
        };
        save_settings_to(&path, &s).expect("写设置失败");
        let back = load_settings_from(&path);
        assert_eq!(back, s);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn corrupt_file_falls_back_to_default() {
        let path = temp_settings_file("corrupt");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, "这不是 toml {{{").unwrap();
        assert_eq!(load_settings_from(&path), Settings::default());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn partial_toml_keeps_other_defaults() {
        let path = temp_settings_file("partial");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, "theme = \"Pixel\"\n").unwrap();
        let s = load_settings_from(&path);
        assert_eq!(s.theme, Theme::Pixel);
        assert_eq!(s.animation, AnimationMode::Flip);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn theme_parse_and_cycle() {
        assert_eq!(Theme::from_str_loose(" metro "), Some(Theme::Metro));
        assert_eq!(Theme::from_str_loose("unknown"), None);
        assert_eq!(Theme::Dolphin.next(), Theme::Metro);
        assert_eq!(Theme::Pixel.next(), Theme::Dolphin);
        assert_eq!(AnimationMode::from_str_loose("NONE"), Some(AnimationMode::None));
        assert_eq!(AnimationMode::None.duration_ms(), 0);
        assert!(AnimationMode::Flip.duration_ms() > AnimationMode::Popup.duration_ms());
    }

    #[test]
    fn palettes_differ_per_theme() {
        let radii: Vec<f32> = Theme::all().iter().map(|t| palette(*t).corner_radius).collect();
        assert_eq!(palette(Theme::Metro).corner_radius, 0.0, "Metro 必须直角");
        assert!(palette(Theme::Pixel).corner_radius > palette(Theme::Fluent).corner_radius);
        let mut sorted = radii.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        sorted.dedup();
        assert_eq!(sorted.len(), 4, "四个主题圆角应互不相同: {radii:?}");
    }
}
