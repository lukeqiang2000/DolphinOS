//! Shell 纯逻辑：桌面应用清单、板块分组、桌面顺序持久化、搜索过滤、启动命令解析。
//! 与 UI 解耦，保证可单测。

use serde::{Deserialize, Serialize};

/// 一个可启动应用（桌面卡片的数据源）。
#[derive(Debug, Clone, PartialEq)]
pub struct AppEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub exec: &'static str,
    /// 所属板块（分组名）。
    pub section: &'static str,
    /// 常用应用：直接成卡排在前面；其余按板块顺序跟在后面。
    pub frequent: bool,
}

/// 系统默认应用清单（顺序即默认桌面顺序：常用在前，其余按板块）。
pub fn default_apps() -> Vec<AppEntry> {
    vec![
        AppEntry { id: "files", name: "文件", exec: "dolphin-files", section: "常用", frequent: true },
        AppEntry { id: "browser", name: "浏览器", exec: "dolphin-browser", section: "常用", frequent: true },
        AppEntry { id: "terminal", name: "终端", exec: "dolphin-terminal", section: "常用", frequent: true },
        AppEntry { id: "settings", name: "设置", exec: "dolphin-settings", section: "常用", frequent: true },
        AppEntry { id: "photos", name: "照片", exec: "dolphin-photos", section: "媒体", frequent: false },
        AppEntry { id: "player", name: "播放器", exec: "dolphin-player", section: "媒体", frequent: false },
        AppEntry { id: "store", name: "应用商店", exec: "gnome-software", section: "系统", frequent: false },
        AppEntry { id: "oobe", name: "首次设置", exec: "dolphin-oobe", section: "系统", frequent: false },
    ]
}

/// 桌面布局（持久化到 ~/.config/dolphinos/desktop.toml）：只记应用 id 顺序。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesktopLayout {
    pub order: Vec<String>,
}

impl Default for DesktopLayout {
    fn default() -> Self {
        DesktopLayout {
            order: default_apps().iter().map(|a| a.id.to_string()).collect(),
        }
    }
}

/// 把保存的顺序套到默认清单上：保存里有的按保存序在前，未知 id 丢弃，
/// 清单里新增的应用按默认序追加到后面（升级加了新应用也不会丢）。
pub fn apply_saved_order(saved: &[String]) -> Vec<AppEntry> {
    let apps = default_apps();
    let mut out: Vec<AppEntry> = Vec::with_capacity(apps.len());
    let mut seen: Vec<&str> = Vec::new();
    for id in saved {
        if seen.contains(&id.as_str()) {
            continue;
        }
        if let Some(app) = apps.iter().find(|a| &a.id == id) {
            out.push(app.clone());
            seen.push(app.id);
        }
    }
    for app in apps {
        if !seen.contains(&app.id) {
            out.push(app);
        }
    }
    out
}

/// 在顺序表里把 from 处的 id 移到 to 处（拖动排序的逻辑核心；越界自动夹紧）。
/// UI 目前用相邻交换实现调序，本函数保留为布局 API（有单测覆盖）。
#[allow(dead_code)]
pub fn move_id(order: &mut Vec<String>, from: usize, to: usize) {
    if order.is_empty() || from >= order.len() {
        return;
    }
    let to = to.min(order.len() - 1);
    if from == to {
        return;
    }
    let item = order.remove(from);
    order.insert(to, item);
}

/// 搜索过滤：名称或 exec 包含关键词（不区分大小写）；空关键词返回全部。
pub fn search_filter<'a>(apps: &'a [AppEntry], query: &str) -> Vec<&'a AppEntry> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return apps.iter().collect();
    }
    apps.iter()
        .filter(|a| {
            a.name.to_ascii_lowercase().contains(&q) || a.exec.to_ascii_lowercase().contains(&q)
        })
        .collect()
}

/// 解析启动命令：自研应用优先找与当前可执行文件同目录的二进制（打包/便携运行），
/// 找不到再退回 PATH 里的同名命令。返回要执行的程序路径/名。
pub fn resolve_exec(exec: &str) -> String {
    if exec.starts_with("dolphin-") {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let sibling = dir.join(exec);
                if sibling.exists() {
                    return sibling.to_string_lossy().into_owned();
                }
            }
        }
    }
    exec.to_string()
}

pub fn layout_path() -> std::path::PathBuf {
    dolphin_config::config_dir().join("desktop.toml")
}

pub fn load_layout_from(path: &std::path::Path) -> DesktopLayout {
    match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text).unwrap_or_default(),
        Err(_) => DesktopLayout::default(),
    }
}

pub fn save_layout_to(path: &std::path::Path, layout: &DesktopLayout) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(layout)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_order_applied_and_unknown_dropped() {
        let saved = vec!["player".to_string(), "files".to_string(), "ghost".to_string()];
        let apps = apply_saved_order(&saved);
        assert_eq!(apps.len(), default_apps().len(), "未知 id 应丢弃、新应用应补齐");
        assert_eq!(apps[0].id, "player");
        assert_eq!(apps[1].id, "files");
        assert!(apps.iter().all(|a| a.id != "ghost"));
        // 默认清单里其余应用都还在
        for def in default_apps() {
            assert!(apps.iter().any(|a| a.id == def.id), "缺了 {}", def.id);
        }
    }

    #[test]
    fn saved_order_dedups() {
        let saved = vec!["files".to_string(), "files".to_string()];
        let apps = apply_saved_order(&saved);
        assert_eq!(apps.iter().filter(|a| a.id == "files").count(), 1);
    }

    #[test]
    fn move_id_reorders_and_clamps() {
        let mut order = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        move_id(&mut order, 0, 2);
        assert_eq!(order, vec!["b", "c", "a"]);
        move_id(&mut order, 2, 99); // 已在末尾且目标越界夹紧，无变化
        assert_eq!(order, vec!["b", "c", "a"]);
        move_id(&mut order, 9, 0); // from 越界不动
        assert_eq!(order, vec!["b", "c", "a"]);
    }

    #[test]
    fn search_matches_name_and_exec() {
        let apps = default_apps();
        assert_eq!(search_filter(&apps, "").len(), apps.len());
        assert_eq!(search_filter(&apps, "照片").len(), 1);
        assert_eq!(search_filter(&apps, "PHOTOS").len(), 1);
        assert_eq!(search_filter(&apps, "不存在的东西").len(), 0);
    }

    #[test]
    fn layout_roundtrip() {
        let path = std::env::temp_dir()
            .join(format!("dolphinos-shell-test-{}", std::process::id()))
            .join("desktop.toml");
        let layout = DesktopLayout { order: vec!["player".into(), "files".into()] };
        save_layout_to(&path, &layout).unwrap();
        assert_eq!(load_layout_from(&path), layout);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn frequent_apps_come_first_by_default() {
        let apps = default_apps();
        let first_non_frequent = apps.iter().position(|a| !a.frequent).unwrap();
        assert!(apps[..first_non_frequent].iter().all(|a| a.frequent));
        assert!(first_non_frequent >= 4, "常用应用至少 4 个直接成卡");
    }
}
