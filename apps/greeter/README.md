# dolphin-greeter（DolphinOS 自研图形登录屏）

Slint 图形登录屏，经 greetd IPC 协议与 greetd 通信完成登录。二进制名：`dolphin-greeter`。

## greetd 对接（部署方式）

1. 把 `/etc/greetd/config.toml` 的 `default_session.command` 改为本程序路径，例如：

   ```toml
   [terminal]
   vt = 1

   [default_session]
   command = "/usr/bin/dolphin-greeter"
   user = "_greetd"
   ```

2. greetd 启动本程序时注入环境变量 `GREETD_SOCK`（Unix socket 路径）；本程序经该 socket 与 greetd 通信：每条消息是 JSON，帧格式为 **u32 小端长度前缀 + JSON 字节**（greetd 真实 IPC 格式，见 `src/greetd.rs`）。
3. **密码校验由 greetd/PAM 在服务端完成**：本程序只按协议问答——`create_session` → 收到 `secret` 类 `auth_message` 时用 `post_auth_message_response` 回密码 → `start_session` 以 `cmd=["/usr/local/bin/dolphin-session"]` 启动会话（包装脚本拉起 labwc，渲染失败时自动 pixman 软件渲染重试）。本程序不保存、不比对密码。

## 已实现

- **`src/greetd.rs`**：请求/响应结构体（serde，按 `type` 字段 tag）：`create_session`、`post_auth_message_response`（response 可为 null）、`start_session`、`cancel_session`；响应 `success` / `error` / `auth_message`（visible|secret|info|error）三种解析。`GreetdClient` 用 `UnixStream` 连 `$GREETD_SOCK` 收发长度前缀帧。
- **`Authenticator` trait**：`GreetdAuthenticator` 走真实协议（create → secret 问答回密码 → start_session labwc，认证出错时发 cancel_session 收尾）；`DemoAuthenticator` **仅在带 `--demo` 参数时使用**，规则为用户名非空且密码长度 ≥ 4——**这是演示模式、不是真校验**，没有 greetd 时用来看界面流程，绝不能作为正式登录部署。
- **UI `ui/app-window.slint`**：居中登录卡——用户名 `LineEdit`、密码 `LineEdit`（`InputType.password`）、登录按钮、状态/错误文本；登录成功显示「验证通过，正在启动会话…」。
- **指纹/人脸预留**：两个按钮已在界面上，点击按 `src/biometric.rs` 的探测结果显示文案——探测 fprintd 守护/工具与 `/var/lib/fprint` 录入痕迹、Howdy 类组件路径（也认 `DOLPHIN_GREETER_FPRINTD`/`DOLPHIN_GREETER_FACE` 环境标记），分别给出「未检测到 fprintd 服务」「需 fprintd 录入后启用」「PAM 模块接入后启用」等不同文案。

## 未做到

- 指纹/人脸**不能真正登录**：未接 fprintd/Howdy 的 PAM 模块，只有探测与文案。
- 未在真实 greetd 会话里端到端验证（本机无 greetd 环境）；协议层靠单元测试覆盖序列化/解析，`GreetdAuthenticator` 的问答流程未做过真机联调。
- 多因子认证（第二次追问输入）暂回 null 让 PAM 失败收尾；会话选择（labwc 以外）、多用户列表、电源菜单未做。

## 测试

`cargo test -p dolphin-greeter`：5 个单元测试（协议消息 JSON 字段、帧长度前缀、三种响应解析、DemoAuthenticator 规则、生物识别文案含「未就绪」），全过。
