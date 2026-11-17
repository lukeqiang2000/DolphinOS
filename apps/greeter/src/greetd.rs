//! greetd IPC 协议客户端。
//!
//! 协议要点（greetd 官方 IPC）：greeter 通过环境变量 `GREETD_SOCK`
//! 指定的 Unix socket 与 greetd 通信；每条消息是 JSON，前缀 4 字节
//! 小端 u32 长度。密码校验由 greetd 服务端经 PAM 完成，greeter 只负责
//! 收用户名/密码、按协议问答、最后请求启动会话。
//!
//! 请求：
//! - `{"type":"create_session","username":..}`
//! - `{"type":"post_auth_message_response","response":.. 或 null}`
//! - `{"type":"start_session","cmd":["/usr/local/bin/dolphin-session"],"env":[]}`
//! - `{"type":"cancel_session"}`
//!
//! 响应：
//! - `{"type":"success"}`
//! - `{"type":"error","error_type":..,"description":..}`
//! - `{"type":"auth_message","auth_message_type":"visible|secret|info|error","auth_message":..}`

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

/// 发往 greetd 的请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    CreateSession { username: String },
    PostAuthMessageResponse { response: Option<String> },
    StartSession { cmd: Vec<String>, env: Vec<String> },
    CancelSession,
}

/// greetd 返回的响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Success,
    Error {
        #[serde(default)]
        error_type: Option<String>,
        #[serde(default)]
        description: Option<String>,
    },
    AuthMessage {
        auth_message_type: String,
        auth_message: String,
    },
}

/// 把一条请求编码成线上帧：u32 小端长度 + JSON 字节。
pub fn encode_frame(req: &Request) -> std::io::Result<Vec<u8>> {
    let payload = serde_json::to_vec(req)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

/// 解析一条响应 JSON。
pub fn parse_response(bytes: &[u8]) -> Result<Response, String> {
    serde_json::from_slice(bytes).map_err(|e| format!("响应解析失败: {e}"))
}

/// 与 greetd 的一条连接（一个 UnixStream）。
pub struct GreetdClient {
    stream: UnixStream,
}

impl GreetdClient {
    /// 连接 `$GREETD_SOCK` 指向的 socket。
    pub fn connect_env() -> Result<Self, String> {
        let path = std::env::var("GREETD_SOCK")
            .map_err(|_| "环境变量 GREETD_SOCK 未设置，不在 greetd 会话中".to_string())?;
        Self::connect(Path::new(&path))
    }

    pub fn connect(path: &Path) -> Result<Self, String> {
        let stream = UnixStream::connect(path)
            .map_err(|e| format!("连接 greetd socket {} 失败: {e}", path.display()))?;
        Ok(GreetdClient { stream })
    }

    pub fn send(&mut self, req: &Request) -> Result<(), String> {
        let frame = encode_frame(req).map_err(|e| e.to_string())?;
        self.stream
            .write_all(&frame)
            .and_then(|_| self.stream.flush())
            .map_err(|e| format!("发送 greetd 请求失败: {e}"))
    }

    pub fn recv(&mut self) -> Result<Response, String> {
        let mut len_buf = [0u8; 4];
        self.stream
            .read_exact(&mut len_buf)
            .map_err(|e| format!("读取 greetd 响应长度失败: {e}"))?;
        let len = u32::from_le_bytes(len_buf) as usize;
        let mut payload = vec![0u8; len];
        self.stream
            .read_exact(&mut payload)
            .map_err(|e| format!("读取 greetd 响应失败: {e}"))?;
        parse_response(&payload)
    }

    /// 发一条请求并读一条响应。
    pub fn roundtrip(&mut self, req: &Request) -> Result<Response, String> {
        self.send(req)?;
        self.recv()
    }
}

/// 登录校验抽象：真实实现走 greetd 协议，演示实现只做本地规则检查。
pub trait Authenticator {
    /// 校验通过返回 Ok；失败返回给用户看的错误文案。
    fn login(&mut self, user: &str, password: &str) -> Result<(), String>;
}

/// 真实校验：经 greetd 协议完成 PAM 问答并启动 labwc 会话。
pub struct GreetdAuthenticator {
    socket_path: Option<std::path::PathBuf>,
}

impl GreetdAuthenticator {
    pub fn from_env() -> Self {
        GreetdAuthenticator {
            socket_path: std::env::var("GREETD_SOCK").ok().map(std::path::PathBuf::from),
        }
    }

    pub fn new(socket_path: std::path::PathBuf) -> Self {
        GreetdAuthenticator {
            socket_path: Some(socket_path),
        }
    }
}

impl Authenticator for GreetdAuthenticator {
    fn login(&mut self, user: &str, password: &str) -> Result<(), String> {
        let path = self
            .socket_path
            .clone()
            .ok_or_else(|| "未连接到 greetd（缺少 GREETD_SOCK）".to_string())?;
        let mut client = GreetdClient::connect(&path)?;

        // 1) 创建会话
        let mut resp = client.roundtrip(&Request::CreateSession {
            username: user.to_string(),
        })?;

        // 2) 按 auth_message 问答：secret/visible 需要用户输入（这里用密码），
        //    info/error 只需回 null 确认；success 表示无需更多问答。
        let mut password_sent = false;
        for _ in 0..8 {
            match resp {
                Response::Success => break,
                Response::Error { description, .. } => {
                    let _ = client.send(&Request::CancelSession);
                    return Err(description.unwrap_or_else(|| "认证失败".to_string()));
                }
                Response::AuthMessage {
                    auth_message_type,
                    auth_message,
                } => {
                    let answer = match auth_message_type.as_str() {
                        "secret" | "visible" => {
                            if password_sent {
                                // 第二次还要输入：多半是密码错后重试或二次因子，
                                // 本界面只有一个密码框，回 null 让 PAM 失败收尾。
                                None
                            } else {
                                password_sent = true;
                                Some(password.to_string())
                            }
                        }
                        _ => {
                            let _ = auth_message; // info/error 提示文本，界面层暂不转发
                            None
                        }
                    };
                    resp = client.roundtrip(&Request::PostAuthMessageResponse { response: answer })?;
                }
            }
        }

        // 3) 启动会话：走 dolphin-session 包装脚本（内部拉起 labwc，
        //    渲染器初始化失败时自动改用 WLR_RENDERER=pixman 软件渲染重试，
        //    并把 labwc 输出落盘到 /tmp/labwc.log）。不要直接写裸 labwc，
        //    否则无 3D 虚拟机里的 pixman 兜底会丢。
        match client.roundtrip(&Request::StartSession {
            cmd: vec!["/usr/local/bin/dolphin-session".to_string()],
            env: vec![],
        })? {
            Response::Success => Ok(()),
            Response::Error { description, .. } => {
                Err(description.unwrap_or_else(|| "启动会话失败".to_string()))
            }
            Response::AuthMessage { .. } => Err("启动会话时收到意外的认证追问".to_string()),
        }
    }
}

/// 演示校验（仅 `--demo` 时使用）：不连 greetd、不经 PAM，
/// 只检查用户名非空且密码长度 >= 4。**不是真校验**，见 README。
pub struct DemoAuthenticator;

impl Authenticator for DemoAuthenticator {
    fn login(&mut self, user: &str, password: &str) -> Result<(), String> {
        if user.trim().is_empty() {
            return Err("用户名不能为空".to_string());
        }
        if password.chars().count() < 4 {
            return Err("演示模式：密码长度至少 4 位".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_json_fields_are_correct() {
        let v: serde_json::Value =
            serde_json::to_value(Request::CreateSession { username: "luke".into() }).unwrap();
        assert_eq!(v["type"], "create_session");
        assert_eq!(v["username"], "luke");

        let v: serde_json::Value =
            serde_json::to_value(Request::PostAuthMessageResponse { response: None }).unwrap();
        assert_eq!(v["type"], "post_auth_message_response");
        assert!(v["response"].is_null());

        let v: serde_json::Value = serde_json::to_value(Request::StartSession {
            cmd: vec!["/usr/local/bin/dolphin-session".into()],
            env: vec![],
        })
        .unwrap();
        assert_eq!(v["type"], "start_session");
        assert_eq!(v["cmd"][0], "/usr/local/bin/dolphin-session");
        assert_eq!(v["env"].as_array().unwrap().len(), 0);

        let v: serde_json::Value = serde_json::to_value(Request::CancelSession).unwrap();
        assert_eq!(v["type"], "cancel_session");
    }

    #[test]
    fn frame_is_u32le_length_prefixed() {
        let frame = encode_frame(&Request::CancelSession).unwrap();
        let len = u32::from_le_bytes(frame[0..4].try_into().unwrap()) as usize;
        assert_eq!(len, frame.len() - 4);
        let text = std::str::from_utf8(&frame[4..]).unwrap();
        assert!(text.contains("cancel_session"));
    }

    #[test]
    fn response_parsing_three_kinds() {
        assert_eq!(
            parse_response(br#"{"type":"success"}"#).unwrap(),
            Response::Success
        );
        match parse_response(br#"{"type":"error","error_type":"auth_error","description":"bad password"}"#)
            .unwrap()
        {
            Response::Error { error_type, description } => {
                assert_eq!(error_type.as_deref(), Some("auth_error"));
                assert_eq!(description.as_deref(), Some("bad password"));
            }
            other => panic!("应为 error，实际 {other:?}"),
        }
        match parse_response(
            br#"{"type":"auth_message","auth_message_type":"secret","auth_message":"Password: "}"#,
        )
        .unwrap()
        {
            Response::AuthMessage { auth_message_type, auth_message } => {
                assert_eq!(auth_message_type, "secret");
                assert_eq!(auth_message, "Password: ");
            }
            other => panic!("应为 auth_message，实际 {other:?}"),
        }
    }

    #[test]
    fn demo_authenticator_rules() {
        let mut a = DemoAuthenticator;
        assert!(a.login("", "1234").is_err(), "空用户名应拒绝");
        assert!(a.login("   ", "1234").is_err(), "纯空白用户名应拒绝");
        assert!(a.login("luke", "123").is_err(), "密码短于 4 位应拒绝");
        assert!(a.login("luke", "1234").is_ok());
        assert!(a.login("luke", "correct horse").is_ok());
    }
}
