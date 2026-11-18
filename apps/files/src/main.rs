//! DolphinOS 文件管理器（Slint）。
//!
//! 用法：dolphin-files [起始路径]（默认 $HOME）

mod logic;

use logic::{format_size, list_dir, NavState};
use slint::{ModelRc, VecModel};
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;

slint::include_modules!();

fn refresh(
    ui: &AppWindow,
    nav: &NavState,
    entries: &Rc<RefCell<Vec<logic::DirEntry>>>,
    show_hidden: bool,
    error: Option<String>,
) {
    let current = nav.current.clone();
    ui.set_current_path(current.to_string_lossy().to_string().into());
    ui.set_can_back(nav.can_back());
    ui.set_can_forward(nav.can_forward());

    match list_dir(&current, show_hidden) {
        Ok(list) => {
            let model: Vec<FileEntry> = list
                .iter()
                .map(|e| FileEntry {
                    name: e.name.clone().into(),
                    path: e.path.to_string_lossy().to_string().into(),
                    is_dir: e.is_dir,
                    size_text: if e.is_dir {
                        "".into()
                    } else {
                        format_size(e.size).into()
                    },
                    icon: if e.is_dir { "▶".into() } else { "·".into() },
                })
                .collect();
            let count = list.len();
            *entries.borrow_mut() = list;
            ui.set_entries(ModelRc::new(VecModel::from(model)));
            ui.set_selected_index(-1);
            let status = match error {
                Some(err) => format!("{} · {}", err, current.display()),
                None => format!("{} 项 · {}", count, current.display()),
            };
            ui.set_status_text(status.into());
        }
        Err(e) => {
            ui.set_entries(ModelRc::new(VecModel::from(Vec::<FileEntry>::new())));
            entries.borrow_mut().clear();
            ui.set_status_text(format!("读取目录失败：{} · {}", e, current.display()).into());
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let start: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/"));

    let nav = Rc::new(RefCell::new(NavState::new(start)));
    let entries: Rc<RefCell<Vec<logic::DirEntry>>> = Rc::new(RefCell::new(Vec::new()));
    let show_hidden = Rc::new(RefCell::new(false));

    let ui = AppWindow::new()?;
    refresh(&ui, &nav.borrow(), &entries, false, None);

    // 后退
    {
        let ui_w = ui.as_weak();
        let nav = nav.clone();
        let entries = entries.clone();
        let show_hidden = show_hidden.clone();
        ui.on_navigate_back(move || {
            if nav.borrow_mut().back() {
                if let Some(ui) = ui_w.upgrade() {
                    refresh(&ui, &nav.borrow(), &entries, *show_hidden.borrow(), None);
                }
            }
        });
    }
    // 前进
    {
        let ui_w = ui.as_weak();
        let nav = nav.clone();
        let entries = entries.clone();
        let show_hidden = show_hidden.clone();
        ui.on_navigate_forward(move || {
            if nav.borrow_mut().forward() {
                if let Some(ui) = ui_w.upgrade() {
                    refresh(&ui, &nav.borrow(), &entries, *show_hidden.borrow(), None);
                }
            }
        });
    }
    // 上级
    {
        let ui_w = ui.as_weak();
        let nav = nav.clone();
        let entries = entries.clone();
        let show_hidden = show_hidden.clone();
        ui.on_navigate_up(move || {
            if nav.borrow_mut().up() {
                if let Some(ui) = ui_w.upgrade() {
                    refresh(&ui, &nav.borrow(), &entries, *show_hidden.borrow(), None);
                }
            }
        });
    }
    // 隐藏文件开关
    {
        let ui_w = ui.as_weak();
        let nav = nav.clone();
        let entries = entries.clone();
        let show_hidden = show_hidden.clone();
        ui.on_toggle_hidden(move |on| {
            *show_hidden.borrow_mut() = on;
            if let Some(ui) = ui_w.upgrade() {
                refresh(&ui, &nav.borrow(), &entries, on, None);
            }
        });
    }
    // 单击选中
    {
        let ui_w = ui.as_weak();
        ui.on_entry_selected(move |idx| {
            if let Some(ui) = ui_w.upgrade() {
                ui.set_selected_index(idx);
            }
        });
    }
    // 双击：目录进入，文件 xdg-open
    {
        let ui_w = ui.as_weak();
        let nav = nav.clone();
        let entries = entries.clone();
        let show_hidden = show_hidden.clone();
        ui.on_entry_activated(move |idx| {
            let entry = entries.borrow().get(idx as usize).cloned();
            let Some(entry) = entry else { return };
            if entry.is_dir {
                nav.borrow_mut().visit(entry.path.clone());
                if let Some(ui) = ui_w.upgrade() {
                    refresh(&ui, &nav.borrow(), &entries, *show_hidden.borrow(), None);
                }
            } else {
                match Command::new("xdg-open").arg(&entry.path).spawn() {
                    Ok(_) => {
                        if let Some(ui) = ui_w.upgrade() {
                            ui.set_status_text(
                                format!("已打开 {}", entry.path.display()).into(),
                            );
                        }
                    }
                    Err(e) => {
                        if let Some(ui) = ui_w.upgrade() {
                            refresh(
                                &ui,
                                &nav.borrow(),
                                &entries,
                                *show_hidden.borrow(),
                                Some(format!("打开失败：{}", e)),
                            );
                        }
                    }
                }
            }
        });
    }

    ui.run()
}
