# dolphin-terminal（DolphinOS 自研终端）

Slint 界面 + portable-pty + vte 的真实可用终端骨架，二进制名 `dolphin-terminal`。

## 实现说明

- **PTY**：`portable-pty` 打开 pty 对（100 列 × 30 行），在 slave 侧启动
  `$SHELL`（缺省 `/bin/bash`），工作目录为 `$HOME`；子进程随主进程管理。
- **输出路径**：读线程从 pty master 读字节，经 `std::sync::mpsc` 送回主
  线程；Slint `Timer` 每 40ms 轮询，把字节喂给网格并刷新界面的
  `screen` 文本属性。
- **VT 解析**：不从零写解析器。用 `vte` 的 `Parser` + `Perform`
  （`src/term.rs`）：print 写字符（行尾折行、页尾滚屏）；execute 处理
  `\r`、`\n`、`\x08`、`\t`；CSI 处理光标移动 A/B/C/D、定位 H/f、
  绝对列 G、绝对行 d、清屏 J（0/1/2）、清行 K（0/1/2）；SGR（m）与
  其余所有序列安全忽略，不 panic。网格提供
  `screen_text() -> String`（每行 trim_end 后以 `\n` 拼接）。
- **输入路径**：界面 `FocusScope` 的 `key-pressed` 回调把按键文本与
  Ctrl 状态转发给 Rust 编码后写回 pty master：回车发 `\r`、退格发
  `\x7f`、Ctrl+字母发控制码、方向键/Home/End/PgUp/PgDn 发对应 CSI
  序列、普通字符发其 UTF-8、私有区特殊键忽略。
- **界面**：整窗等宽字体（`font-family: "monospace"`）文本区显示网格
  内容。

## 测试

`cargo test -p dolphin-terminal`：覆盖喂入 `hello\r\nworld` 的两行
内容、ANSI 清屏+定位后的内容、退格/回车覆盖行为、光标左移+清行与
SGR/未知序列忽略，共 4 个单元测试（在 `src/term.rs`）。

## 未做到 / 已知限制

- 网格尺寸固定 100×30，尚未随窗口大小动态 resize（`TermGrid::resize`
  已实现但未接窗口尺寸事件），pty 尺寸也未同步调整。
- 无颜色/样式渲染：界面把网格渲染为单一纯文本，SGR 属性被忽略，
  彩色输出只显示文字。
- 无滚动回看（scrollback）、无选区复制粘贴、无鼠标支持。
- 未在真机 Wayland/labwc 会话里做交互验收，仅通过单元测试验证
  网格逻辑；GUI 本身在本开发环境未实际打开窗口验证。
