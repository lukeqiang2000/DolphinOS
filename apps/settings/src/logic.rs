//! 设置应用纯逻辑：选项清单、按下标改设置、保存/读取（路径可注入便于测试）。

use dolphin_config::{AnimationMode, Settings, Theme};

/// 可选语言（下标与 UI 列表一一对应）。
pub const LANGUAGES: [&str; 2] = ["zh-CN", "en-US"];
pub const LANGUAGE_NAMES: [&str; 2] = ["简体中文", "English"];

pub fn theme_names() -> Vec<String> {
    Theme::all().iter().map(|t| t.display_name().to_string()).collect()
}

pub fn animation_names() -> Vec<String> {
    AnimationMode::all().iter().map(|a| a.display_name().to_string()).collect()
}

pub fn theme_index(s: &Settings) -> i32 {
    Theme::all().iter().position(|t| *t == s.theme).unwrap_or(0) as i32
}

pub fn animation_index(s: &Settings) -> i32 {
    AnimationMode::all().iter().position(|a| *a == s.animation).unwrap_or(0) as i32
}

pub fn language_index(s: &Settings) -> i32 {
    LANGUAGES.iter().position(|l| *l == s.language).unwrap_or(0) as i32
}

/// 按下标设主题；越界不改并返回 false。
pub fn set_theme(s: &mut Settings, idx: i32) -> bool {
    if idx < 0 {
        return false;
    }
    match Theme::all().get(idx as usize) {
        Some(t) => {
            s.theme = *t;
            true
        }
        None => false,
    }
}

pub fn set_animation(s: &mut Settings, idx: i32) -> bool {
    if idx < 0 {
        return false;
    }
    match AnimationMode::all().get(idx as usize) {
        Some(a) => {
            s.animation = *a;
            true
        }
        None => false,
    }
}

pub fn set_language(s: &mut Settings, idx: i32) -> bool {
    if idx < 0 {
        return false;
    }
    match LANGUAGES.get(idx as usize) {
        Some(l) => {
            s.language = l.to_string();
            true
        }
        None => false,
    }
}

/// 保存到指定路径（测试用）；正式保存走 save()。
pub fn save_to(path: &std::path::Path, s: &Settings) -> std::io::Result<()> {
    dolphin_config::save_settings_to(path, s)
}

pub fn save(s: &Settings) -> std::io::Result<()> {
    dolphin_config::save_settings(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_lists_match_enums() {
        assert_eq!(theme_names().len(), 4);
        assert_eq!(animation_names().len(), 3);
        assert_eq!(LANGUAGES.len(), LANGUAGE_NAMES.len());
    }

    #[test]
    fn setters_validate_bounds() {
        let mut s = Settings::default();
        assert!(set_theme(&mut s, 1));
        assert_eq!(s.theme, Theme::Metro);
        assert!(!set_theme(&mut s, 99));
        assert_eq!(s.theme, Theme::Metro, "越界不能改掉现值");
        assert!(!set_animation(&mut s, -1));
        assert!(set_animation(&mut s, 2));
        assert_eq!(s.animation, AnimationMode::None);
        assert!(set_language(&mut s, 1));
        assert_eq!(s.language, "en-US");
        assert_eq!(language_index(&s), 1);
    }

    #[test]
    fn indices_roundtrip() {
        let mut s = Settings::default();
        for i in 0..4 {
            assert!(set_theme(&mut s, i));
            assert_eq!(theme_index(&s), i);
        }
        for i in 0..3 {
            assert!(set_animation(&mut s, i));
            assert_eq!(animation_index(&s), i);
        }
    }

    #[test]
    fn save_and_reload_via_shared_config() {
        let path = std::env::temp_dir()
            .join(format!("dolphinos-settings-test-{}", std::process::id()))
            .join("settings.toml");
        let mut s = Settings::default();
        set_theme(&mut s, 3);
        s.telemetry = false;
        s.account_name = "tester".into();
        save_to(&path, &s).unwrap();
        let back = dolphin_config::load_settings_from(&path);
        assert_eq!(back, s, "设置应用与共享库读到的必须是同一份");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
