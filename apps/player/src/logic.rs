//! 播放器纯逻辑：播放列表、播放状态机、时间格式化、音量。
//! 不依赖 Slint 与具体播放后端，可独立单元测试。

use std::path::{Path, PathBuf};

/// 常见视频扩展名（小写比较）。
const VIDEO_EXTS: &[&str] = &["mp4", "mkv", "webm", "avi", "mov"];

fn is_video(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| VIDEO_EXTS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

#[derive(Debug, Clone, Default)]
pub struct Playlist {
    pub items: Vec<PathBuf>,
    pub current: Option<usize>,
}

impl Playlist {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, path: PathBuf) {
        self.items.push(path);
        if self.current.is_none() {
            self.current = Some(0);
        }
    }

    /// 收进一个目录里的视频文件（不递归），按文件名排序。
    /// 路径本身是文件时按单文件处理。返回新增数量。
    pub fn add_dir(&mut self, dir: &Path) -> usize {
        if dir.is_file() {
            if is_video(dir) {
                self.add(dir.to_path_buf());
                return 1;
            }
            return 0;
        }
        let mut found: Vec<PathBuf> = match std::fs::read_dir(dir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_file() && is_video(p))
                .collect(),
            Err(_) => return 0,
        };
        found.sort_by(|a, b| {
            a.file_name()
                .unwrap_or_default()
                .cmp(b.file_name().unwrap_or_default())
        });
        let n = found.len();
        for p in found {
            self.add(p);
        }
        n
    }

    pub fn current_path(&self) -> Option<&PathBuf> {
        self.current.and_then(|i| self.items.get(i))
    }

    /// 下一首（循环：末尾回到开头）。空列表返回 None。
    pub fn next(&mut self) -> Option<&PathBuf> {
        if self.items.is_empty() {
            self.current = None;
            return None;
        }
        let next = match self.current {
            Some(i) => (i + 1) % self.items.len(),
            None => 0,
        };
        self.current = Some(next);
        self.current_path()
    }

    /// 上一首（循环：开头回到末尾）。空列表返回 None。
    pub fn prev(&mut self) -> Option<&PathBuf> {
        if self.items.is_empty() {
            self.current = None;
            return None;
        }
        let prev = match self.current {
            Some(0) | None => self.items.len() - 1,
            Some(i) => i - 1,
        };
        self.current = Some(prev);
        self.current_path()
    }

    pub fn remove(&mut self, idx: usize) -> Option<PathBuf> {
        if idx >= self.items.len() {
            return None;
        }
        let removed = self.items.remove(idx);
        match self.current {
            None => {}
            Some(cur) if idx < cur => self.current = Some(cur - 1),
            Some(cur) if idx == cur => {
                if self.items.is_empty() {
                    self.current = None;
                } else if cur >= self.items.len() {
                    self.current = Some(self.items.len() - 1);
                }
            }
            _ => {}
        }
        Some(removed)
    }

    pub fn jump(&mut self, idx: usize) -> Option<&PathBuf> {
        if idx < self.items.len() {
            self.current = Some(idx);
            self.current_path()
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

impl PlayState {
    pub fn label(&self) -> &'static str {
        match self {
            PlayState::Stopped => "已停止",
            PlayState::Playing => "播放中",
            PlayState::Paused => "已暂停",
        }
    }
}

/// 播放后端接口：UI 与状态机只依赖这个 trait。
pub trait Backend {
    fn play(&mut self, path: &Path);
    fn pause(&mut self);
    fn resume(&mut self);
    fn stop(&mut self);
    fn seek(&mut self, secs: f64);
}

/// 模拟后端：只记录状态，供没有 GStreamer 的环境跑通 UI 与状态机，
/// 也是单元测试与默认构建使用的后端。
#[derive(Debug, Default)]
pub struct SimulatedBackend {
    pub state: PlayState,
    pub current: Option<PathBuf>,
    pub position_secs: f64,
    pub seek_count: usize,
}

impl SimulatedBackend {
    pub fn new() -> Self {
        Self {
            state: PlayState::Stopped,
            current: None,
            position_secs: 0.0,
            seek_count: 0,
        }
    }
}

impl Backend for SimulatedBackend {
    fn play(&mut self, path: &Path) {
        self.current = Some(path.to_path_buf());
        self.position_secs = 0.0;
        self.state = PlayState::Playing;
    }
    fn pause(&mut self) {
        if self.state == PlayState::Playing {
            self.state = PlayState::Paused;
        }
    }
    fn resume(&mut self) {
        if self.state == PlayState::Paused {
            self.state = PlayState::Playing;
        }
    }
    fn stop(&mut self) {
        self.state = PlayState::Stopped;
        self.position_secs = 0.0;
    }
    fn seek(&mut self, secs: f64) {
        self.position_secs = secs.max(0.0);
        self.seek_count += 1;
    }
}

/// 播放核心：状态机 + 列表 + 进度 + 音量。后端以 trait object 注入。
pub struct PlayerCore {
    pub playlist: Playlist,
    pub state: PlayState,
    pub position_secs: f64,
    pub duration_secs: f64,
    pub volume: u32,
    pub backend: Box<dyn Backend>,
}

impl PlayerCore {
    pub fn new(backend: Box<dyn Backend>) -> Self {
        Self {
            playlist: Playlist::new(),
            state: PlayState::Stopped,
            position_secs: 0.0,
            duration_secs: 0.0,
            volume: 80,
            backend,
        }
    }

    pub fn play_current(&mut self) {
        if let Some(path) = self.playlist.current_path().cloned() {
            self.backend.play(&path);
            self.state = PlayState::Playing;
            self.position_secs = 0.0;
        }
    }

    pub fn toggle_play_pause(&mut self) {
        match self.state {
            PlayState::Playing => {
                self.backend.pause();
                self.state = PlayState::Paused;
            }
            PlayState::Paused => {
                self.backend.resume();
                self.state = PlayState::Playing;
            }
            PlayState::Stopped => self.play_current(),
        }
    }

    pub fn stop(&mut self) {
        self.backend.stop();
        self.state = PlayState::Stopped;
        self.position_secs = 0.0;
    }

    pub fn next(&mut self) {
        if self.playlist.next().is_some() {
            self.play_current();
        }
    }

    pub fn prev(&mut self) {
        if self.playlist.prev().is_some() {
            self.play_current();
        }
    }

    pub fn seek(&mut self, secs: f64) {
        let clamped = if self.duration_secs > 0.0 {
            secs.clamp(0.0, self.duration_secs)
        } else {
            secs.max(0.0)
        };
        self.position_secs = clamped;
        self.backend.seek(clamped);
    }

    pub fn set_volume(&mut self, vol: i32) {
        self.volume = vol.clamp(0, 100) as u32;
    }

    /// 推进播放进度。Playing 时前进 dt 秒；到达结尾自动切下一首并
    /// 归零（列表为空/无当前项则停为 Stopped）。
    pub fn tick(&mut self, dt: f64) {
        if self.state != PlayState::Playing || dt <= 0.0 {
            return;
        }
        self.position_secs += dt;
        if self.duration_secs > 0.0 && self.position_secs >= self.duration_secs {
            if self.playlist.items.is_empty() || self.playlist.current.is_none() {
                self.state = PlayState::Stopped;
                self.position_secs = 0.0;
                self.backend.stop();
            } else {
                // 循环切到下一首（单曲时即重播自身）
                let _ = self.playlist.next();
                self.position_secs = 0.0;
                if let Some(path) = self.playlist.current_path().cloned() {
                    self.backend.play(&path);
                    self.state = PlayState::Playing;
                }
            }
        }
    }
}

/// 秒数格式化：<1 小时 "mm:ss"，>=1 小时 "h:mm:ss"。
pub fn format_time(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn names(pl: &Playlist) -> Vec<String> {
        pl.items
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn playlist_next_prev_wraps() {
        let mut pl = Playlist::new();
        pl.add(PathBuf::from("a.mp4"));
        pl.add(PathBuf::from("b.mp4"));
        pl.add(PathBuf::from("c.mp4"));
        assert_eq!(pl.current, Some(0));
        pl.next();
        assert_eq!(pl.current, Some(1));
        pl.next();
        assert_eq!(pl.current, Some(2));
        pl.next();
        assert_eq!(pl.current, Some(0), "末尾应循环回开头");
        pl.prev();
        assert_eq!(pl.current, Some(2), "开头 prev 应循环到末尾");
        pl.jump(1);
        assert_eq!(pl.current_path().unwrap(), &PathBuf::from("b.mp4"));
        pl.remove(1);
        assert_eq!(names(&pl), vec!["a.mp4", "c.mp4"]);
    }

    #[test]
    fn add_dir_filters_extensions_and_sorts() {
        let dir = std::env::temp_dir().join(format!(
            "dolphin-player-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["b.mp4", "a.mkv", "c.txt", "d.webm", "e.mp3", "f.MOV"] {
            let mut f = std::fs::File::create(dir.join(name)).unwrap();
            f.write_all(b"x").unwrap();
        }
        let mut pl = Playlist::new();
        let n = pl.add_dir(&dir);
        assert_eq!(n, 4);
        assert_eq!(names(&pl), vec!["a.mkv", "b.mp4", "d.webm", "f.MOV"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tick_advances_and_switches_at_end() {
        let mut core = PlayerCore::new(Box::new(SimulatedBackend::new()));
        core.playlist.add(PathBuf::from("one.mp4"));
        core.playlist.add(PathBuf::from("two.mp4"));
        core.duration_secs = 10.0;
        core.play_current();
        assert_eq!(core.state, PlayState::Playing);
        core.tick(4.0);
        assert_eq!(core.position_secs, 4.0);
        assert_eq!(core.playlist.current, Some(0));
        core.tick(7.0);
        assert_eq!(core.playlist.current, Some(1), "到尾应自动切下一首");
        assert_eq!(core.position_secs, 0.0);
        assert_eq!(core.state, PlayState::Playing);
        // 暂停时 tick 不推进
        core.toggle_play_pause();
        assert_eq!(core.state, PlayState::Paused);
        core.tick(5.0);
        assert_eq!(core.position_secs, 0.0);
    }

    #[test]
    fn format_time_and_volume_clamp() {
        assert_eq!(format_time(0), "00:00");
        assert_eq!(format_time(65), "01:05");
        assert_eq!(format_time(599), "09:59");
        assert_eq!(format_time(3600), "1:00:00");
        assert_eq!(format_time(3661), "1:01:01");
        assert_eq!(format_time(36000), "10:00:00");

        let mut core = PlayerCore::new(Box::new(SimulatedBackend::new()));
        core.set_volume(150);
        assert_eq!(core.volume, 100);
        core.set_volume(-20);
        assert_eq!(core.volume, 0);
        core.set_volume(42);
        assert_eq!(core.volume, 42);
    }
}
