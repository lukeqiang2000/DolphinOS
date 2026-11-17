//! dolphin-files 纯逻辑：目录列举、排序、隐藏过滤、导航栈、大小格式化。
//! 不依赖 Slint，可独立单元测试。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified_unix: u64,
}

pub fn is_hidden_name(name: &str) -> bool {
    name.starts_with('.')
}

/// 列出目录条目。目录在前，同类按名称不区分大小写升序。
/// `show_hidden == false` 时过滤掉以 `.` 开头的名字。
pub fn list_dir(path: &Path, show_hidden: bool) -> std::io::Result<Vec<DirEntry>> {
    let mut entries: Vec<DirEntry> = Vec::new();
    for item in fs::read_dir(path)? {
        let item = match item {
            Ok(v) => v,
            Err(_) => continue,
        };
        let name = item.file_name().to_string_lossy().into_owned();
        if !show_hidden && is_hidden_name(&name) {
            continue;
        }
        let metadata = match item.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let is_dir = metadata.is_dir();
        let size = if is_dir { 0 } else { metadata.len() };
        let modified_unix = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        entries.push(DirEntry {
            name,
            path: item.path(),
            is_dir,
            size,
            modified_unix,
        });
    }
    sort_entries(&mut entries);
    Ok(entries)
}

pub fn sort_entries(entries: &mut [DirEntry]) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
}

/// 导航状态：当前目录 + 后退栈 + 前进栈（浏览器式语义）。
#[derive(Debug, Clone)]
pub struct NavState {
    pub current: PathBuf,
    pub back_stack: Vec<PathBuf>,
    pub forward_stack: Vec<PathBuf>,
}

impl NavState {
    pub fn new(start: PathBuf) -> Self {
        Self {
            current: start,
            back_stack: Vec::new(),
            forward_stack: Vec::new(),
        }
    }

    pub fn can_back(&self) -> bool {
        !self.back_stack.is_empty()
    }

    pub fn can_forward(&self) -> bool {
        !self.forward_stack.is_empty()
    }

    /// 访问新目录：当前压入后退栈，前进栈清空。
    pub fn visit(&mut self, path: PathBuf) {
        if path == self.current {
            return;
        }
        let old = std::mem::replace(&mut self.current, path);
        self.back_stack.push(old);
        self.forward_stack.clear();
    }

    /// 到上级目录；已在根目录时返回 false。
    pub fn up(&mut self) -> bool {
        match self.current.parent() {
            Some(parent) => {
                let p = parent.to_path_buf();
                self.visit(p);
                true
            }
            None => false,
        }
    }

    pub fn back(&mut self) -> bool {
        match self.back_stack.pop() {
            Some(prev) => {
                let old = std::mem::replace(&mut self.current, prev);
                self.forward_stack.push(old);
                true
            }
            None => false,
        }
    }

    pub fn forward(&mut self) -> bool {
        match self.forward_stack.pop() {
            Some(next) => {
                let old = std::mem::replace(&mut self.current, next);
                self.back_stack.push(old);
                true
            }
            None => false,
        }
    }
}

/// 人类可读大小：B / KB / MB / GB / TB，1024 进位，非 B 单位保留 1 位小数。
pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;
    if size < KB {
        format!("{} B", size)
    } else if size < MB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else if size < GB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size < TB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else {
        format!("{:.1} TB", size as f64 / TB as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dolphin-files-test-{}-{}",
            std::process::id(),
            tag
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn sorting_dirs_first_then_case_insensitive_name() {
        let dir = temp_dir("sort");
        fs::create_dir(dir.join("zeta")).unwrap();
        fs::create_dir(dir.join("Alpha")).unwrap();
        fs::write(dir.join("b.txt"), b"x").unwrap();
        fs::write(dir.join("A.txt"), b"x").unwrap();
        let entries = list_dir(&dir, false).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["Alpha", "zeta", "A.txt", "b.txt"]);
        assert!(entries[0].is_dir && entries[1].is_dir);
        assert!(!entries[2].is_dir);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hidden_files_filtered_by_default() {
        let dir = temp_dir("hidden");
        fs::write(dir.join(".secret"), b"x").unwrap();
        fs::write(dir.join("visible.txt"), b"x").unwrap();
        fs::create_dir(dir.join(".hdir")).unwrap();
        let without = list_dir(&dir, false).unwrap();
        assert_eq!(without.len(), 1);
        assert_eq!(without[0].name, "visible.txt");
        let with = list_dir(&dir, true).unwrap();
        assert_eq!(with.len(), 3);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn nav_stack_back_forward_semantics() {
        let mut nav = NavState::new(PathBuf::from("/a"));
        assert!(!nav.can_back());
        assert!(!nav.can_forward());
        nav.visit(PathBuf::from("/a/b"));
        nav.visit(PathBuf::from("/a/b/c"));
        assert_eq!(nav.current, PathBuf::from("/a/b/c"));
        assert!(nav.can_back());
        assert!(nav.back());
        assert_eq!(nav.current, PathBuf::from("/a/b"));
        assert!(nav.can_forward());
        assert!(nav.back());
        assert_eq!(nav.current, PathBuf::from("/a"));
        assert!(!nav.can_back());
        assert!(!nav.back());
        assert!(nav.forward());
        assert_eq!(nav.current, PathBuf::from("/a/b"));
        // 访问新目录后前进栈应清空
        nav.visit(PathBuf::from("/x"));
        assert!(!nav.can_forward());
        assert_eq!(nav.current, PathBuf::from("/x"));
        // up() 到父目录
        let mut nav2 = NavState::new(PathBuf::from("/a/b"));
        assert!(nav2.up());
        assert_eq!(nav2.current, PathBuf::from("/a"));
        assert!(nav2.up());
        assert_eq!(nav2.current, PathBuf::from("/"));
        assert!(!nav2.up());
    }

    #[test]
    fn format_size_boundaries() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1), "1 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_size(5 * 1024 * 1024 * 1024), "5.0 GB");
        assert_eq!(format_size(1024u64 * 1024 * 1024 * 1024), "1.0 TB");
    }
}
