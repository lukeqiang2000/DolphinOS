//! DolphinOS 照片查看器（Slint + image）。
//!
//! 用法：dolphin-photos [照片目录]（默认 ~/Pictures，不存在时用当前目录）

mod logic;

use logic::{scan_photos, PhotoIndex};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;

slint::include_modules!();

/// 预览图最长边（像素）：足够全屏预览，又不至于把原图整张搬进显存。
const PREVIEW_MAX: u32 = 1600;

fn default_dir() -> PathBuf {
    if let Some(arg) = std::env::args().nth(1) {
        return PathBuf::from(arg);
    }
    if let Some(home) = std::env::var_os("HOME") {
        let pictures = PathBuf::from(home).join("Pictures");
        if pictures.is_dir() {
            return pictures;
        }
    }
    PathBuf::from(".")
}

fn show_current(ui: &AppWindow, index: &PhotoIndex, dir: &std::path::Path, skipped: usize) {
    match index.current_item() {
        Some(item) => {
            match logic::make_thumbnail(&item.path, PREVIEW_MAX) {
                Some((w, h, pixels)) => {
                    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
                    buffer.make_mut_bytes().copy_from_slice(&pixels);
                    ui.set_current_image(Image::from_rgba8(buffer));
                    ui.set_has_image(true);
                    ui.set_status_text("".into());
                }
                None => {
                    ui.set_has_image(false);
                    ui.set_status_text(
                        format!("无法解码：{}", item.name).into(),
                    );
                }
            }
            let mut info = format!(
                "第 {} / {} 张 · {} · {}x{}",
                index.current + 1,
                index.len(),
                item.name,
                item.width,
                item.height
            );
            if skipped > 0 {
                info.push_str(&format!(" · 已跳过 {} 个损坏文件", skipped));
            }
            ui.set_info_text(info.into());
        }
        None => {
            ui.set_has_image(false);
            ui.set_info_text(format!("第 0 / 0 张 · {}", dir.display()).into());
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let dir = default_dir();
    let scan = scan_photos(&dir);
    let skipped = scan.skipped;
    let index = Rc::new(RefCell::new(PhotoIndex::new(scan.items)));
    let dir_rc = Rc::new(dir);

    let ui = AppWindow::new()?;
    show_current(&ui, &index.borrow(), &dir_rc, skipped);

    {
        let ui_w = ui.as_weak();
        let index = index.clone();
        let dir = dir_rc.clone();
        ui.on_prev_photo(move || {
            index.borrow_mut().prev();
            if let Some(ui) = ui_w.upgrade() {
                show_current(&ui, &index.borrow(), &dir, skipped);
            }
        });
    }
    {
        let ui_w = ui.as_weak();
        let index = index.clone();
        let dir = dir_rc.clone();
        ui.on_next_photo(move || {
            index.borrow_mut().next();
            if let Some(ui) = ui_w.upgrade() {
                show_current(&ui, &index.borrow(), &dir, skipped);
            }
        });
    }
    {
        let ui_w = ui.as_weak();
        let dir = dir_rc.clone();
        ui.on_open_folder(move || {
            if let Err(e) = Command::new("xdg-open").arg(dir.as_path()).spawn() {
                if let Some(ui) = ui_w.upgrade() {
                    ui.set_status_text(format!("打开文件夹失败：{}", e).into());
                }
            }
        });
    }

    ui.run()
}
