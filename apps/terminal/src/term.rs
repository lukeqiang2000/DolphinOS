//! 终端网格状态机：用 vte 的 Parser + Perform 做 VT 解析，本模块只维护
//! 固定尺寸的字符网格与光标，不从零写转义序列解析器。
//!
//! 支持的子集（其余序列一律安全忽略，不 panic）：
//! - print：写字符并推进光标，行尾自动折行，页尾自动滚屏
//! - execute：\r 回车（列归零）、\n 换行、\x08 退格、\t 制表
//! - CSI：光标移动 A/B/C/D、定位 H/f、列 G、行 d、清屏 J（0/1/2）、
//!   清行 K（0/1/2）；SGR（m）与其余 CSI 直接忽略

use vte::{Params, Parser, Perform};

/// 网格核心状态，实现 vte::Perform。
pub struct GridCore {
    pub cols: usize,
    pub rows: usize,
    cells: Vec<Vec<char>>,
    cx: usize,
    cy: usize,
}

impl GridCore {
    pub fn new(cols: usize, rows: usize) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        Self {
            cols,
            rows,
            cells: vec![vec![' '; cols]; rows],
            cx: 0,
            cy: 0,
        }
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let mut cells = vec![vec![' '; cols]; rows];
        for (y, row) in self.cells.iter().enumerate() {
            if y >= rows {
                break;
            }
            for (x, ch) in row.iter().enumerate() {
                if x >= cols {
                    break;
                }
                cells[y][x] = *ch;
            }
        }
        self.cells = cells;
        self.cols = cols;
        self.rows = rows;
        self.cx = self.cx.min(cols - 1);
        self.cy = self.cy.min(rows - 1);
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.cx, self.cy)
    }

    fn scroll_up(&mut self) {
        if self.rows == 0 {
            return;
        }
        self.cells.remove(0);
        self.cells.push(vec![' '; self.cols]);
    }

    fn newline(&mut self) {
        if self.cy + 1 >= self.rows {
            self.scroll_up();
        } else {
            self.cy += 1;
        }
    }

    fn clear_range(&mut self, y: usize, x0: usize, x1: usize) {
        if y >= self.rows {
            return;
        }
        let x1 = x1.min(self.cols);
        for x in x0.min(self.cols)..x1 {
            self.cells[y][x] = ' ';
        }
    }

    pub fn screen_text(&self) -> String {
        let mut out = String::new();
        for (i, row) in self.cells.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            let line: String = row.iter().collect();
            out.push_str(line.trim_end());
        }
        out
    }
}

/// 取第 idx 个参数的原始值（无参数时返回 0）。
fn param_raw(params: &Params, idx: usize) -> u16 {
    params
        .iter()
        .nth(idx)
        .and_then(|sub| sub.first().copied())
        .unwrap_or(0)
}

/// 取第 idx 个参数；为 0 或缺失时用默认值（光标移动/定位类序列的惯例）。
fn param_or(params: &Params, idx: usize, default: u16) -> u16 {
    let v = param_raw(params, idx);
    if v == 0 {
        default
    } else {
        v
    }
}

impl Perform for GridCore {
    fn print(&mut self, c: char) {
        if self.cols == 0 || self.rows == 0 {
            return;
        }
        if self.cx >= self.cols {
            self.cx = 0;
            self.newline();
        }
        let (x, y) = (self.cx, self.cy);
        self.cells[y][x] = c;
        self.cx += 1;
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\r' => self.cx = 0,
            b'\n' => self.newline(),
            0x08 => self.cx = self.cx.saturating_sub(1),
            b'\t' => {
                let next = ((self.cx / 8) + 1) * 8;
                self.cx = next.min(self.cols.saturating_sub(1));
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char) {
        match action {
            // 光标上移
            'A' => {
                let n = param_or(params, 0, 1).max(0) as usize;
                self.cy = self.cy.saturating_sub(n);
            }
            // 光标下移
            'B' => {
                let n = param_or(params, 0, 1).max(0) as usize;
                self.cy = (self.cy + n).min(self.rows.saturating_sub(1));
            }
            // 光标右移
            'C' => {
                let n = param_or(params, 0, 1).max(0) as usize;
                self.cx = (self.cx + n).min(self.cols.saturating_sub(1));
            }
            // 光标左移
            'D' => {
                let n = param_or(params, 0, 1).max(0) as usize;
                self.cx = self.cx.saturating_sub(n);
            }
            // 光标定位（行;列，均从 1 起）
            'H' | 'f' => {
                let row = param_or(params, 0, 1).max(1) as usize;
                let col = param_or(params, 1, 1).max(1) as usize;
                self.cy = (row - 1).min(self.rows.saturating_sub(1));
                self.cx = (col - 1).min(self.cols.saturating_sub(1));
            }
            // 绝对列
            'G' => {
                let col = param_or(params, 0, 1).max(1) as usize;
                self.cx = (col - 1).min(self.cols.saturating_sub(1));
            }
            // 绝对行
            'd' => {
                let row = param_or(params, 0, 1).max(1) as usize;
                self.cy = (row - 1).min(self.rows.saturating_sub(1));
            }
            // 清屏
            'J' => match param_raw(params, 0) {
                0 => {
                    self.clear_range(self.cy, self.cx, self.cols);
                    for y in (self.cy + 1)..self.rows {
                        self.clear_range(y, 0, self.cols);
                    }
                }
                1 => {
                    for y in 0..self.cy {
                        self.clear_range(y, 0, self.cols);
                    }
                    self.clear_range(self.cy, 0, self.cx + 1);
                }
                2 | 3 => {
                    for y in 0..self.rows {
                        self.clear_range(y, 0, self.cols);
                    }
                }
                _ => {}
            },
            // 清行
            'K' => match param_raw(params, 0) {
                0 => self.clear_range(self.cy, self.cx, self.cols),
                1 => self.clear_range(self.cy, 0, self.cx + 1),
                2 => self.clear_range(self.cy, 0, self.cols),
                _ => {}
            },
            // SGR（颜色/样式）与其它 CSI：安全忽略
            _ => {}
        }
    }
}

/// 对外终端类型：持有 vte Parser，把字节流喂给内部网格。
pub struct TermGrid {
    core: GridCore,
    parser: Parser,
}

impl TermGrid {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            core: GridCore::new(cols, rows),
            parser: Parser::new(),
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.core, bytes);
    }

    pub fn feed_str(&mut self, s: &str) {
        self.feed(s.as_bytes());
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.core.resize(cols, rows);
    }

    pub fn screen_text(&self) -> String {
        self.core.screen_text()
    }

    pub fn cursor(&self) -> (usize, usize) {
        self.core.cursor()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_crlf_two_lines() {
        let mut t = TermGrid::new(100, 30);
        t.feed_str("hello\r\nworld");
        let text = t.screen_text();
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some("hello"));
        assert_eq!(lines.next(), Some("world"));
    }

    #[test]
    fn ansi_position_and_clear_screen() {
        let mut t = TermGrid::new(100, 30);
        t.feed_str("garbage garbage");
        // 清屏（2J）+ 光标定位到第 2 行第 3 列（1 起）后写字
        t.feed_str("\x1b[2J\x1b[2;3Hhi");
        let text = t.screen_text();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "");
        assert_eq!(lines[1], "  hi");
        assert!(!text.contains("garbage"));
    }

    #[test]
    fn backspace_and_carriage_return_overwrite() {
        let mut t = TermGrid::new(100, 30);
        t.feed_str("abc\x08X");
        assert_eq!(t.screen_text().lines().next(), Some("abX"));
        // \r 回到行首覆盖
        t.feed_str("\rZ");
        assert_eq!(t.screen_text().lines().next(), Some("ZbX"));
    }

    #[test]
    fn cursor_moves_and_line_erase() {
        let mut t = TermGrid::new(100, 30);
        t.feed_str("hello\x1b[2D\x1b[K");
        assert_eq!(t.screen_text().lines().next(), Some("hel"));
        // SGR 与未知序列应被安全忽略
        t.feed_str("\x1b[1;31m\x1b[?25l!");
        assert_eq!(t.screen_text().lines().next(), Some("hel!"));
    }
}
