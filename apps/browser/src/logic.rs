//! 浏览器纯逻辑：URL 归一化与历史栈。不依赖 Slint / 系统，可独立单测。

/// 把任意字节按 RFC 3986 percent-encode 编码（查询参数用）。
/// 保留 unreserved 字符 `A-Z a-z 0-9 - _ . ~`，其余全部 `%XX`（大写十六进制）。
pub fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for &b in input.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// 判断一段（已去空白、无 scheme 的）文本是否「像域名」：
/// 含点、无空白，且点不在首尾。像 `example.com`、`sub.example.org/path`。
fn looks_like_domain(s: &str) -> bool {
    if s.is_empty() || s.chars().any(|c| c.is_whitespace()) {
        return false;
    }
    // 只看 host 部分（第一个 '/' 之前）
    let host = s.split('/').next().unwrap_or("");
    host.contains('.') && !host.starts_with('.') && !host.ends_with('.')
}

/// 把地址栏输入归一化为可导航 URL：
/// - 去首尾空白；
/// - 已有 `http://` / `https://` 前缀（不分大小写）原样保留；
/// - 像域名则补 `https://`；
/// - 其余当搜索词，走 DuckDuckGo 搜索（查询词 percent-encode）。
pub fn normalize_url(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "about:blank".to_string();
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return trimmed.to_string();
    }
    if looks_like_domain(trimmed) {
        return format!("https://{}", trimmed);
    }
    format!("https://duckduckgo.com/?q={}", percent_encode(trimmed))
}

/// 浏览器历史栈：entries + 当前下标。
/// push 新页面时截断「前进」部分（与主流浏览器一致）。
#[derive(Debug, Clone, Default)]
pub struct History {
    entries: Vec<String>,
    index: Option<usize>,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, url: impl Into<String>) {
        if let Some(idx) = self.index {
            self.entries.truncate(idx + 1);
        } else {
            self.entries.clear();
        }
        self.entries.push(url.into());
        self.index = Some(self.entries.len() - 1);
    }

    pub fn can_back(&self) -> bool {
        matches!(self.index, Some(idx) if idx > 0)
    }

    pub fn can_forward(&self) -> bool {
        matches!(self.index, Some(idx) if idx + 1 < self.entries.len())
    }

    pub fn current(&self) -> Option<&str> {
        self.index.and_then(|idx| self.entries.get(idx)).map(|s| s.as_str())
    }

    /// 后退一步，返回后退后的当前 URL；已在最旧一页时返回 None 且不动。
    pub fn back(&mut self) -> Option<&str> {
        if self.can_back() {
            self.index = self.index.map(|idx| idx - 1);
        }
        self.current()
    }

    /// 前进一步，返回前进后的当前 URL；已在最新一页时返回 None 语义上的当前页。
    pub fn forward(&mut self) -> Option<&str> {
        if self.can_forward() {
            self.index = self.index.map(|idx| idx + 1);
        }
        self.current()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_gets_https_prefix() {
        assert_eq!(normalize_url("example.com"), "https://example.com");
        assert_eq!(normalize_url("  sub.example.org/path "), "https://sub.example.org/path");
    }

    #[test]
    fn existing_scheme_is_preserved() {
        assert_eq!(normalize_url("http://example.com/a"), "http://example.com/a");
        assert_eq!(normalize_url("https://example.com"), "https://example.com");
        assert_eq!(normalize_url("HTTP://example.com"), "HTTP://example.com");
    }

    #[test]
    fn search_terms_are_encoded_to_duckduckgo() {
        assert_eq!(
            normalize_url("hello world"),
            "https://duckduckgo.com/?q=hello%20world"
        );
        // 无点的单词也当搜索词
        assert_eq!(
            normalize_url("dolphin"),
            "https://duckduckgo.com/?q=dolphin"
        );
        // 中文按 UTF-8 字节编码
        assert_eq!(
            normalize_url("海豚"),
            "https://duckduckgo.com/?q=%E6%B5%B7%E8%B1%9A"
        );
    }

    #[test]
    fn percent_encode_keeps_unreserved() {
        assert_eq!(percent_encode("aZ09-_.~"), "aZ09-_.~");
        assert_eq!(percent_encode("a+b&c"), "a%2Bb%26c");
    }

    #[test]
    fn history_back_forward() {
        let mut h = History::new();
        assert!(!h.can_back() && !h.can_forward());
        assert_eq!(h.current(), None);
        h.push("https://a.example");
        h.push("https://b.example");
        h.push("https://c.example");
        assert_eq!(h.current(), Some("https://c.example"));
        assert!(h.can_back() && !h.can_forward());
        assert_eq!(h.back(), Some("https://b.example"));
        assert_eq!(h.back(), Some("https://a.example"));
        assert!(!h.can_back());
        assert_eq!(h.forward(), Some("https://b.example"));
        assert!(h.can_forward());
    }

    #[test]
    fn history_push_truncates_forward_stack() {
        let mut h = History::new();
        h.push("https://a.example");
        h.push("https://b.example");
        h.push("https://c.example");
        h.back();
        h.back();
        h.push("https://d.example");
        assert_eq!(h.len(), 2);
        assert_eq!(h.current(), Some("https://d.example"));
        assert!(!h.can_forward(), "push 后前进栈应被截断");
        assert_eq!(h.back(), Some("https://a.example"));
    }
}
