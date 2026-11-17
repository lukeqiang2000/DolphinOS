//! DolphinOS 自研终端（Slint 界面）。
//!
//! 结构：portable-pty 打开一对 pty，在里面跑用户 shell；读线程把
//! master 输出经 mpsc 送回主线程，Slint Timer 周期轮询、喂给
//! term::TermGrid（vte 解析）后刷新界面文本；键盘输入经 FocusScope
//! 回调编码后写入 pty master。

mod term;

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use slint::{Timer, TimerMode};
use std::io::{Read, Write};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use term::TermGrid;

slint::include_modules!();

const COLS: u16 = 100;
const ROWS: u16 = 30;

/// 把 Slint 按键文本编码成要写入 pty 的字节。
/// 回车 → \r，退格 → \x7f，Ctrl+字母 → 控制码，方向键 → CSI 序列，
/// 其余特殊键（私有区码点）忽略，普通字符发其 UTF-8。
fn encode_key(text: &str, ctrl: bool) -> Vec<u8> {
    if ctrl {
        let mut chars = text.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            let lower = c.to_ascii_lowercase();
            if lower.is_ascii_lowercase() {
                return vec![lower as u8 - b'a' + 1];
            }
            if c == ' ' {
                return vec![0];
            }
        }
    }
    match text {
        "\n" | "\r" => return vec![b'\r'],
        // Slint 里 Backspace 的 text 是 \u{8}，终端惯例发 \x7f
        "\u{8}" => return vec![0x7f],
        // Delete 键
        "\u{7f}" => return b"\x1b[3~".to_vec(),
        "\t" => return vec![b'\t'],
        "\u{1b}" => return vec![0x1b],
        // 方向键 / Home / End / PgUp / PgDn（Slint 功能键码点）
        "\u{f700}" => return b"\x1b[A".to_vec(),
        "\u{f701}" => return b"\x1b[B".to_vec(),
        "\u{f703}" => return b"\x1b[C".to_vec(),
        "\u{f702}" => return b"\x1b[D".to_vec(),
        "\u{f729}" => return b"\x1b[H".to_vec(),
        "\u{f72b}" => return b"\x1b[F".to_vec(),
        "\u{f72c}" => return b"\x1b[5~".to_vec(),
        "\u{f72d}" => return b"\x1b[6~".to_vec(),
        _ => {}
    }
    // 其它特殊键落在 Unicode 私有区，没有文本意义，忽略。
    if text
        .chars()
        .any(|c| ('\u{e000}'..='\u{f8ff}').contains(&c))
    {
        return Vec::new();
    }
    text.as_bytes().to_vec()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());

    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: ROWS,
        cols: COLS,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = CommandBuilder::new(&shell);
    cmd.cwd(&home);
    let _child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader()?;
    let writer = Arc::new(Mutex::new(pair.master.take_writer()?));

    // 读线程：pty 输出 → mpsc → 主线程
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let ui = AppWindow::new()?;
    ui.set_screen("".into());

    // 按键 → 编码 → 写 pty master
    let writer_for_keys = Arc::clone(&writer);
    ui.on_keys_typed(move |text, ctrl| {
        let bytes = encode_key(text.as_str(), ctrl);
        if bytes.is_empty() {
            return;
        }
        if let Ok(mut w) = writer_for_keys.lock() {
            let _ = w.write_all(&bytes);
            let _ = w.flush();
        }
    });

    // Timer 轮询 pty 输出，喂网格并刷新界面
    let grid = Arc::new(Mutex::new(TermGrid::new(COLS as usize, ROWS as usize)));
    let ui_handle = ui.as_weak();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_millis(40), move || {
        let mut dirty = false;
        while let Ok(chunk) = rx.try_recv() {
            if let Ok(mut g) = grid.lock() {
                g.feed(&chunk);
            }
            dirty = true;
        }
        if dirty {
            if let (Some(ui), Ok(g)) = (ui_handle.upgrade(), grid.lock()) {
                ui.set_screen(g.screen_text().into());
            }
        }
    });

    ui.run()?;
    Ok(())
}
