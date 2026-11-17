//! dolphin-photos 纯逻辑：图片扫描、索引导航、缩略图生成。
//! 不依赖 Slint，可独立单元测试。

use std::path::{Path, PathBuf};

pub const IMAGE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "bmp", "webp"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotoItem {
    pub path: PathBuf,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

pub fn is_image_path(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => {
            let lower = ext.to_lowercase();
            IMAGE_EXTENSIONS.contains(&lower.as_str())
        }
        None => false,
    }
}

#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    pub items: Vec<PhotoItem>,
    /// 因尺寸读取失败被跳过的图片文件数量。
    pub skipped: usize,
}

/// 扫描目录（不递归），只收图片扩展名，按名称不区分大小写排序。
/// 用 `image::image_dimensions` 读尺寸，读不了的跳过并计数。
pub fn scan_photos(dir: &Path) -> ScanResult {
    let mut result = ScanResult::default();
    let read_dir = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return result,
    };
    let mut candidates: Vec<PathBuf> = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_file() && is_image_path(&path) {
            candidates.push(path);
        }
    }
    candidates.sort_by(|a, b| {
        let an = a
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let bn = b
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        an.cmp(&bn)
    });
    for path in candidates {
        match image::image_dimensions(&path) {
            Ok((width, height)) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                result.items.push(PhotoItem {
                    path,
                    name,
                    width,
                    height,
                });
            }
            Err(_) => {
                result.skipped += 1;
            }
        }
    }
    result
}

/// 照片索引：当前张 + 循环的上/下一张导航。
#[derive(Debug, Clone, Default)]
pub struct PhotoIndex {
    pub items: Vec<PhotoItem>,
    pub current: usize,
}

impl PhotoIndex {
    pub fn new(items: Vec<PhotoItem>) -> Self {
        Self { items, current: 0 }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn current_item(&self) -> Option<&PhotoItem> {
        self.items.get(self.current)
    }

    pub fn current_path(&self) -> Option<&Path> {
        self.current_item().map(|i| i.path.as_path())
    }

    /// 下一张（循环：末张回到第一张）。空列表返回 false。
    pub fn next(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }
        self.current = (self.current + 1) % self.items.len();
        true
    }

    /// 上一张（循环：第一张回到末张）。空列表返回 false。
    pub fn prev(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }
        if self.current == 0 {
            self.current = self.items.len() - 1;
        } else {
            self.current -= 1;
        }
        true
    }

    /// 跳到第 idx 张（0 起）；越界返回 false 且不移动。
    pub fn jump(&mut self, idx: usize) -> bool {
        if idx < self.items.len() {
            self.current = idx;
            true
        } else {
            false
        }
    }
}

/// 生成缩略图：等比缩放使宽高均不超过 `max`，返回 (宽, 高, RGBA 像素)。
/// 读图失败返回 None。
pub fn make_thumbnail(path: &Path, max: u32) -> Option<(u32, u32, Vec<u8>)> {
    let img = image::open(path).ok()?;
    // image::thumbnail 会把小图也放大到贴合边界；这里只缩小、不放大。
    if img.width() <= max && img.height() <= max {
        let rgba = img.to_rgba8();
        let (w, h) = (rgba.width(), rgba.height());
        return Some((w, h, rgba.into_raw()));
    }
    let thumb = img.thumbnail(max, max);
    let rgba = thumb.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    Some((w, h, rgba.into_raw()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dolphin-photos-test-{}-{}",
            std::process::id(),
            tag
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_png(path: &Path, w: u32, h: u32) {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb([200, 100, 50]));
        img.save(path).unwrap();
    }

    #[test]
    fn scan_filters_extensions_and_sorts_by_name() {
        let dir = temp_dir("scan");
        write_png(&dir.join("b.png"), 8, 6);
        write_png(&dir.join("A.JPG"), 4, 4);
        // 损坏的 .png：扩展名对但读不出尺寸，应被跳过并计数
        fs::write(dir.join("broken.png"), b"not an image").unwrap();
        fs::write(dir.join("notes.txt"), b"hello").unwrap();
        fs::write(dir.join("noext"), b"x").unwrap();
        let result = scan_photos(&dir);
        let names: Vec<&str> = result.items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["A.JPG", "b.png"]);
        assert_eq!(result.items[0].width, 4);
        assert_eq!(result.items[1].width, 8);
        assert_eq!(result.items[1].height, 6);
        assert_eq!(result.skipped, 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn photo_index_wraps_around() {
        let mk = |n: &str| PhotoItem {
            path: PathBuf::from(n),
            name: n.to_string(),
            width: 1,
            height: 1,
        };
        let mut idx = PhotoIndex::new(vec![mk("1.png"), mk("2.png"), mk("3.png")]);
        assert_eq!(idx.current_path(), Some(Path::new("1.png")));
        assert!(idx.next());
        assert_eq!(idx.current, 1);
        assert!(idx.next());
        assert_eq!(idx.current, 2);
        assert!(idx.next());
        assert_eq!(idx.current, 0, "末张 next 应循环回第一张");
        assert!(idx.prev());
        assert_eq!(idx.current, 2, "第一张 prev 应循环回末张");
        assert!(idx.jump(1));
        assert_eq!(idx.current, 1);
        assert!(!idx.jump(3));
        assert_eq!(idx.current, 1, "越界 jump 不应移动");
        let mut empty = PhotoIndex::new(vec![]);
        assert!(!empty.next());
        assert!(!empty.prev());
        assert_eq!(empty.current_path(), None);
    }

    #[test]
    fn thumbnail_does_not_exceed_max() {
        let dir = temp_dir("thumb");
        let src = dir.join("big.png");
        write_png(&src, 200, 100);
        let (w, h, pixels) = make_thumbnail(&src, 50).expect("缩略图应生成成功");
        assert!(w <= 50 && h <= 50, "缩略图尺寸 {}x{} 超过上限 50", w, h);
        assert_eq!(w, 50);
        assert_eq!(h, 25);
        assert_eq!(pixels.len(), (w * h * 4) as usize);
        // 小图不放大
        let small = dir.join("small.png");
        write_png(&small, 10, 10);
        let (w2, h2, _) = make_thumbnail(&small, 50).unwrap();
        assert_eq!((w2, h2), (10, 10));
        assert!(make_thumbnail(&dir.join("missing.png"), 50).is_none());
        let _ = fs::remove_dir_all(&dir);
    }
}
