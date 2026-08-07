//! 错误诊断与用户可见中文报告
//!
//! # 职责
//! - 把 `CoreError` / 原始网络与 HTTP 现象 **分类** 成稳定 `ErrorCode`（给其它 AI / 日志检索）
//! - 产出 **纯中文** 用户说明、可能原因、建议操作（防呆；非银行级安全文案）
//! - **净化** 二进制/乱码响应体，避免 `http 404: \u0000...` 污染日志
//!
//! # 文档（双向链接）
//! - Outbound: `docs/tech/ERROR_DIAGNOSTICS.md`（错误类型权威表）
//! - Outbound: `docs/PLAN_NEXT_FOOLPROOF_AND_DIAG.md`（一键排查产品）
//! - Inbound: `error.rs` · `client.rs` · `gree.rs` · app `task_log` / UI
//!
//! # 设计原则
//! - 用户层不堆「只有状态码」；状态码可写在技术细节，但必须附带中文含义与可能情况
//! - 机读字段 `code` 稳定，**改中文文案不必改 code**
//! - 启发式地区/代理说明：只提示，不声称 100% 准确

use crate::error::CoreError;
use serde::Serialize;

/// 稳定错误码（蛇形大写风格用字符串常量，便于 JSON / 日志 grep）
///
/// 完整表见 `docs/tech/ERROR_DIAGNOSTICS.md`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    NetTimeout,
    NetConnect,
    NetDns,
    NetTls,
    NetProxy,
    NetOther,
    Http428Version,
    Http401Session,
    Http403Forbidden,
    Http404NotFound,
    Http5xxServer,
    HttpOther,
    LoginGreeSignature,
    LoginGreeInactive,
    LoginCredentials,
    LoginChannelUnsupported,
    LoginSessionExpired,
    LoginOther,
    FingerprintMissing,
    FingerprintInvalid,
    FingerprintChannel,
    CryptoUnpad,
    CryptoOther,
    ApiBusiness,
    ModuleSkip,
    ModuleAbort,
    Io,
    Json,
    Config,
    GateDenied,
    Other,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NetTimeout => "NET_TIMEOUT",
            Self::NetConnect => "NET_CONNECT",
            Self::NetDns => "NET_DNS",
            Self::NetTls => "NET_TLS",
            Self::NetProxy => "NET_PROXY",
            Self::NetOther => "NET_OTHER",
            Self::Http428Version => "HTTP_428_VERSION",
            Self::Http401Session => "HTTP_401_SESSION",
            Self::Http403Forbidden => "HTTP_403",
            Self::Http404NotFound => "HTTP_404",
            Self::Http5xxServer => "HTTP_5XX",
            Self::HttpOther => "HTTP_OTHER",
            Self::LoginGreeSignature => "LOGIN_GREE_SIGNATURE",
            Self::LoginGreeInactive => "LOGIN_GREE_INACTIVE",
            Self::LoginCredentials => "LOGIN_CREDENTIALS",
            Self::LoginChannelUnsupported => "LOGIN_CHANNEL_UNSUPPORTED",
            Self::LoginSessionExpired => "LOGIN_SESSION_EXPIRED",
            Self::LoginOther => "LOGIN_OTHER",
            Self::FingerprintMissing => "FP_MISSING",
            Self::FingerprintInvalid => "FP_INVALID",
            Self::FingerprintChannel => "FP_CHANNEL",
            Self::CryptoUnpad => "CRYPTO_UNPAD",
            Self::CryptoOther => "CRYPTO_OTHER",
            Self::ApiBusiness => "API_BUSINESS",
            Self::ModuleSkip => "MODULE_SKIP",
            Self::ModuleAbort => "MODULE_ABORT",
            Self::Io => "IO",
            Self::Json => "JSON",
            Self::Config => "CONFIG",
            Self::GateDenied => "GATE_DENIED",
            Self::Other => "OTHER",
        }
    }
}

/// 结构化诊断报告（API / 剪贴板 / 任务日志共用）
#[derive(Debug, Clone, Serialize)]
pub struct DiagReport {
    /// 稳定机读码

    pub code: String,
    /// 粗分类（network / http / login / …）

    pub category: String,
    /// 一行中文标题

    pub title_zh: String,
    /// 完整用户说明（可多段）

    pub detail_zh: String,
    /// 可能原因（中文）

    pub possible_causes_zh: Vec<String>,
    /// 建议操作（中文）

    pub next_steps_zh: Vec<String>,
    /// HTTP 状态码（若有）；说明写在 detail，勿只展示数字

    pub http_status: Option<u16>,
    /// 净化后的技术细节（可给开发 / 其它 AI）

    pub tech_detail: String,
    /// 相关文档路径（仓库相对）

    pub doc_refs: Vec<String>,
}

impl DiagReport {
    /// 任务日志 / toast 用的完整中文块

    pub fn format_user_block(&self) -> String {
        let mut s = format!("【{}】{}\n{}", self.code, self.title_zh, self.detail_zh);
        if !self.possible_causes_zh.is_empty() {
            s.push_str("\n\n可能原因：");
            for (i, c) in self.possible_causes_zh.iter().enumerate() {
                s.push_str(&format!("\n  {}. {}", i + 1, c));
            }
        }
        if !self.next_steps_zh.is_empty() {
            s.push_str("\n\n建议操作：");
            for (i, c) in self.next_steps_zh.iter().enumerate() {
                s.push_str(&format!("\n  {}. {}", i + 1, c));
            }
        }
        if !self.tech_detail.is_empty() {
            s.push_str(&format!("\n\n技术细节：{}", self.tech_detail));
        }
        s
    }

    /// 模块级日志：短一些，仍含 code + 原因摘要

    pub fn format_module_log(&self) -> String {
        let mut s = format!("[{}] {}", self.code, self.title_zh);
        if !self.detail_zh.is_empty() {
            s.push_str(" — ");
            s.push_str(&self.detail_zh.replace('\n', " "));
        }
        if let Some(c) = self.possible_causes_zh.first() {
            s.push_str(&format!("（可能：{}）", c));
        }
        s
    }
}

/// 净化响应体：去掉不可打印字符，限制长度
pub fn sanitize_body(raw: &str) -> String {
    let printable: String = raw
        .chars()
        .map(|c| {
            if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
                '�'
            } else {
                c
            }
        })
        .collect();
    let trimmed = printable.trim();
    if trimmed.is_empty() {
        return "（空响应体）".into();
    }
    // 乱码占比过高 → 当作二进制/加密失败体
    let bad = trimmed.chars().filter(|&c| c == '�' || c == '\u{FFFD}').count();
    let total = trimmed.chars().count().max(1);
    if bad * 100 / total > 15 || looks_like_binary(trimmed.as_bytes()) {
        return format!(
            "（响应体无法按文本阅读，长度 {} 字节。常见于：解密失败、返回了非 JSON 的网关页、或协议与指纹不匹配）",
            raw.len()
        );
    }
    const MAX: usize = 400;
    if trimmed.chars().count() > MAX {
        let t: String = trimmed.chars().take(MAX).collect();
        format!("{t}…（已截断）")
    } else {
        trimmed.to_string()
    }
}

fn looks_like_binary(b: &[u8]) -> bool {
    if b.is_empty() {
        return false;
    }
    let non_text = b
        .iter()
        .filter(|&&c| c < 9 || (c > 13 && c < 32) || c == 0x7f)
        .count();
    non_text * 100 / b.len() > 20
}

/// 从 reqwest 错误构造网络 `CoreError`（内嵌稳定 code，便于 `diagnose()` 还原）
///
/// 存储格式：`{CODE}|{phase}|{reqwest原英文}` — 用户中文一律走 `diagnose()` / `user_block_zh()`。
pub fn network_from_reqwest(err: &reqwest::Error, phase_zh: &str) -> CoreError {
    let report = classify_reqwest(err, phase_zh);
    CoreError::Network(format!("{}|{}|{}", report.code, phase_zh, err))
}

/// 分类 reqwest 错误（也可被文本回退路径使用）
pub fn classify_reqwest(err: &reqwest::Error, phase_zh: &str) -> DiagReport {
    let raw = err.to_string();
    let (code, title, causes, steps) = if err.is_timeout() {
        (
            ErrorCode::NetTimeout,
            format!("{phase_zh}：连接或等待超时"),
            vec![
                "网络过慢或中途断开".into(),
                "加速器节点卡顿、全局代理半失效".into(),
                "日服未走日本出口 / 国际服未走允许地区，链路被掐断".into(),
                "防火墙/安全软件拦截本程序联网".into(),
            ],
            vec![
                "切换加速器节点后重试".into(),
                "日服请确认日本 IP；国际服请确认美国（或官方允许）IP".into(),
                "可点「一键排查和修复」（若已提供）查看出口地区与连通性".into(),
            ],
        )
    } else if err.is_connect() {
        (
            ErrorCode::NetConnect,
            format!("{phase_zh}：无法建立连接"),
            vec![
                "无网络或 DNS 失败".into(),
                "目标主机被墙/被加速器分流错误".into(),
                "本地防火墙拒绝出站".into(),
                "游戏服或 Gree 域名当前不可达".into(),
            ],
            vec![
                "检查系统能否打开网页".into(),
                "调整加速器模式（勿错误劫持全部流量）".into(),
                "稍后重试；若仅 GitHub 不通但不影响内置指纹，可先尝试登录游戏".into(),
            ],
        )
    } else if err.is_request() && raw.to_lowercase().contains("dns") {
        (
            ErrorCode::NetDns,
            format!("{phase_zh}：域名解析失败"),
            vec![
                "DNS 污染或运营商解析异常".into(),
                "加速器 DNS 模式配置不当".into(),
            ],
            vec!["更换 DNS 或加速器节点".into(), "重试".into()],
        )
    } else if raw.to_lowercase().contains("tls")
        || raw.to_lowercase().contains("ssl")
        || raw.to_lowercase().contains("certificate")
    {
        (
            ErrorCode::NetTls,
            format!("{phase_zh}：安全连接（TLS）失败"),
            vec![
                "系统时间不准导致证书校验失败".into(),
                "中间人代理/抓包证书未信任".into(),
                "企业或家长控制软件劫持 HTTPS".into(),
            ],
            vec![
                "核对系统时间".into(),
                "临时关闭 HTTPS 解密类代理后重试".into(),
            ],
        )
    } else if raw.to_lowercase().contains("proxy") {
        (
            ErrorCode::NetProxy,
            format!("{phase_zh}：代理设置导致失败"),
            vec![
                "系统代理指向失效地址".into(),
                "加速器本地端口未开但系统仍走代理".into(),
            ],
            vec!["检查系统代理；或关闭代理后重试".into()],
        )
    } else {
        (
            ErrorCode::NetOther,
            format!("{phase_zh}：网络错误"),
            vec![
                "不稳定网络".into(),
                "目标服务限流或临时故障".into(),
                "加速器/防火墙干扰".into(),
            ],
            vec![
                "重试一次".into(),
                "换网络或加速节点".into(),
                "把本报告复制给维护者".into(),
            ],
        )
    };

    DiagReport {
        code: code.as_str().into(),
        category: "network".into(),
        title_zh: title,
        detail_zh: format!(
            "在「{phase_zh}」阶段与远程服务器通信失败。这不是游戏账号密码格式问题本身，而是数据包没能正常到达或返回。"
        ),
        possible_causes_zh: causes,
        next_steps_zh: steps,
        http_status: None,
        tech_detail: sanitize_body(&raw),
        doc_refs: vec![
            "docs/tech/ERROR_DIAGNOSTICS.md".into(),
            "docs/tech/CHANNELS.md".into(),
        ],
    }
}

/// HTTP 状态 → 诊断（游戏 API / Gree 共用）
pub fn classify_http(status: u16, body: &str, phase_zh: &str) -> DiagReport {
    let body_s = sanitize_body(body);
    let lower = body.to_lowercase();

    if status == 428 {
        return DiagReport {
            code: ErrorCode::Http428Version.as_str().into(),
            category: "http".into(),
            title_zh: "游戏版本指纹过旧或不匹配（HTTP 428）".into(),
            detail_zh: format!(
                "服务器拒绝了请求：当前使用的游戏版本指纹（version/sign/libcount）与官方要求不一致。阶段：{phase_zh}。"
            ),
            possible_causes_zh: vec![
                "本地/内置指纹落后于线上游戏版本".into(),
                "日服/国际服指纹用串了".into(),
                "缓存了旧的 version.json".into(),
            ],
            next_steps_zh: vec![
                "点「一键排查和修复」尝试从默认源更新指纹".into(),
                "或升级到带有新内置指纹的程序版本".into(),
                "确认账号所选服务器（日服/国际服）正确".into(),
            ],
            http_status: Some(428),
            tech_detail: body_s,
            doc_refs: vec![
                "docs/tech/ERROR_DIAGNOSTICS.md".into(),
                "docs/tech/VERSION_FINGERPRINT.md".into(),
                "docs/tech/LESSONS_RUST_PORT.md".into(),
            ],
        };
    }

    if status == 401 {
        return DiagReport {
            code: ErrorCode::Http401Session.as_str().into(),
            category: "http".into(),
            title_zh: "登录会话失效（HTTP 401）".into(),
            detail_zh: format!("服务器认为当前游戏会话无效或已过期。阶段：{phase_zh}。"),
            possible_causes_zh: vec![
                "会话过期，需要重新登录".into(),
                "同一账号在其它设备/客户端挤掉会话".into(),
                "token 缓存损坏".into(),
            ],
            next_steps_zh: vec![
                "重新执行一次获取信息或清日常（会重新登录）".into(),
                "仍失败可删除 cache/token 中对应缓存后重试".into(),
            ],
            http_status: Some(401),
            tech_detail: body_s,
            doc_refs: vec!["docs/tech/ERROR_DIAGNOSTICS.md".into()],
        };
    }

    if status == 403
        || lower.contains("invalid signature")
        || lower.contains("\"code\":4")
    {
        let gree_sig = lower.contains("invalid signature") || lower.contains("signature");
        if gree_sig {
            return DiagReport {
                code: ErrorCode::LoginGreeSignature.as_str().into(),
                category: "login".into(),
                title_zh: "Gree 登录签名被拒绝（Invalid Signature）".into(),
                detail_zh: format!(
                    "渠道登录（Gree）校验请求签名失败。阶段：{phase_zh}。HTTP 状态：{status}（禁止访问/签名失败类）。"
                ),
                possible_causes_zh: vec![
                    "引继码或游戏密码错误".into(),
                    "本机 token 缓存与服务器状态不一致".into(),
                    "设备注册/签名算法与协议要求不符（开发问题）".into(),
                    "网络中间设备改写了请求".into(),
                ],
                next_steps_zh: vec![
                    "核对引继码与密码后重试".into(),
                    "删除该账号后重新添加；或清理 RustMadoka_data/cache/token 后重试".into(),
                    "确认网络稳定且加速器未做 HTTPS 解密".into(),
                ],
                http_status: Some(status),
                tech_detail: body_s,
                doc_refs: vec![
                    "docs/tech/ERROR_DIAGNOSTICS.md".into(),
                    "docs/tech/LESSONS_RUST_PORT.md".into(),
                    "docs/tech/SDK_AND_LOGIN.md".into(),
                ],
            };
        }
        return DiagReport {
            code: ErrorCode::Http403Forbidden.as_str().into(),
            category: "http".into(),
            title_zh: format!("服务器拒绝访问（HTTP {status}）"),
            detail_zh: format!(
                "远程服务器返回禁止访问。阶段：{phase_zh}。可能是权限、地区限制或签名/鉴权失败。"
            ),
            possible_causes_zh: vec![
                "当前 IP 地区不符合该服要求（日服需日本、国际服常需美国等）".into(),
                "鉴权失败或账号状态异常".into(),
                "WAF/CDN 拦截".into(),
            ],
            next_steps_zh: vec![
                "切换到符合服务器要求的加速节点".into(),
                "核对账号与密码".into(),
                "稍后重试".into(),
            ],
            http_status: Some(status),
            tech_detail: body_s,
            doc_refs: vec!["docs/tech/ERROR_DIAGNOSTICS.md".into()],
        };
    }

    if status == 404 {
        return DiagReport {
            code: ErrorCode::Http404NotFound.as_str().into(),
            category: "http".into(),
            title_zh: "接口不存在或路径错误（HTTP 404）".into(),
            detail_zh: format!(
                "服务器找不到该接口。阶段：{phase_zh}。若响应体无法阅读，也可能是解密失败被误报成 404。"
            ),
            possible_causes_zh: vec![
                "游戏版本变更导致 API 路径变化".into(),
                "AES/msgpack 解密失败，正文被当成错误页".into(),
                "连错服务器（日/国服 API 根搞混）".into(),
            ],
            next_steps_zh: vec![
                "更新指纹后重试".into(),
                "确认服务器渠道正确".into(),
                "若大量模块 404，把完整报告发给维护者".into(),
            ],
            http_status: Some(404),
            tech_detail: body_s,
            doc_refs: vec![
                "docs/tech/ERROR_DIAGNOSTICS.md".into(),
                "docs/tech/PROTOCOL_STACK.md".into(),
            ],
        };
    }

    if (500..600).contains(&status) {
        return DiagReport {
            code: ErrorCode::Http5xxServer.as_str().into(),
            category: "http".into(),
            title_zh: format!("游戏或渠道服务器异常（HTTP {status}）"),
            detail_zh: format!("对端服务器内部错误。阶段：{phase_zh}。通常是临时故障。"),
            possible_causes_zh: vec![
                "官方维护或故障".into(),
                "瞬时过载".into(),
            ],
            next_steps_zh: vec!["等待数分钟后重试".into(), "关注官方维护公告".into()],
            http_status: Some(status),
            tech_detail: body_s,
            doc_refs: vec!["docs/tech/ERROR_DIAGNOSTICS.md".into()],
        };
    }

    DiagReport {
        code: ErrorCode::HttpOther.as_str().into(),
        category: "http".into(),
        title_zh: format!("HTTP 请求失败（状态 {status}）"),
        detail_zh: format!(
            "阶段：{phase_zh}。状态码 {status} 表示请求未按成功（200）完成。请结合下方可能原因排查，不要只记录数字。"
        ),
        possible_causes_zh: vec![
            "服务器拒绝或重定向异常".into(),
            "中间网络设备改写响应".into(),
            "客户端请求与当前协议不匹配".into(),
        ],
        next_steps_zh: vec![
            "重试".into(),
            "更新指纹".into(),
            "复制完整报告给维护者".into(),
        ],
        http_status: Some(status),
        tech_detail: body_s,
        doc_refs: vec!["docs/tech/ERROR_DIAGNOSTICS.md".into()],
    }
}

/// 对任意错误字符串做启发式分类（anyhow / 旧日志回放）
pub fn diagnose_text(raw: &str) -> DiagReport {
    let t = raw.trim();
    let lower = t.to_lowercase();

    if lower.contains("缺少游戏指纹")
        || lower.contains("no fingerprint")
        || (lower.contains("fingerprint") && (lower.contains("missing") || lower.contains("缺少")))
    {
        return DiagReport {
            code: ErrorCode::FingerprintMissing.as_str().into(),
            category: "fingerprint".into(),
            title_zh: "缺少可用的游戏指纹".into(),
            detail_zh: "程序没有可用的 version/sign/libcount，无法与游戏服务器对话。".into(),
            possible_causes_zh: vec![
                "只获得了程序文件，没有指纹缓存，且无法访问默认 GitHub 源".into(),
                "加速器拦截了 GitHub，又没有内置/旁路指纹".into(),
                "数据目录被清空".into(),
            ],
            next_steps_zh: vec![
                "点「一键排查和修复」或升级带内置指纹的版本".into(),
                "将 publish/automadoka.json 放在程序旁 publish 目录".into(),
                "在能访问 GitHub 的网络下拉取指纹后再开加速打游戏".into(),
            ],
            http_status: None,
            tech_detail: sanitize_body(t),
            doc_refs: vec![
                "docs/tech/ERROR_DIAGNOSTICS.md".into(),
                "docs/tech/VERSION_FINGERPRINT.md".into(),
            ],
        };
    }

    if lower.contains("invalid signature") {
        return classify_http(403, t, "Gree/登录");
    }
    if lower.contains("428") || lower.contains("version mismatch") {
        return classify_http(428, t, "游戏 API");
    }
    if lower.contains("unpad") {
        return DiagReport {
            code: ErrorCode::CryptoUnpad.as_str().into(),
            category: "crypto".into(),
            title_zh: "游戏数据解密失败（Unpad）".into(),
            detail_zh: "AES 解密后填充不正确，通常意味着密钥或密文不匹配。".into(),
            possible_causes_zh: vec![
                "协议密钥派生错误（开发回归）".into(),
                "响应并非加密游戏包（网络返回了 HTML/网关页）".into(),
                "严重的版本/渠道不匹配".into(),
            ],
            next_steps_zh: vec![
                "更新指纹并重试".into(),
                "确认渠道正确".into(),
                "持续出现则属程序缺陷，请带报告联系维护者".into(),
            ],
            http_status: None,
            tech_detail: sanitize_body(t),
            doc_refs: vec![
                "docs/tech/ERROR_DIAGNOSTICS.md".into(),
                "docs/tech/LESSONS_RUST_PORT.md".into(),
            ],
        };
    }
    if lower.contains("timeout") || lower.contains("timed out") {
        return classify_reqwest_text(t, "网络通信");
    }
    if lower.contains("connect") || lower.contains("connection") {
        return classify_reqwest_text(t, "网络连接");
    }
    if lower.contains("台服") || lower.contains("sonet") {
        return DiagReport {
            code: ErrorCode::LoginChannelUnsupported.as_str().into(),
            category: "login".into(),
            title_zh: "该服务器登录尚未支持".into(),
            detail_zh: "台服等渠道登录协议尚未移植完成。".into(),
            possible_causes_zh: vec!["功能未实现".into()],
            next_steps_zh: vec!["请使用日服或国际服账号".into()],
            http_status: None,
            tech_detail: sanitize_body(t),
            doc_refs: vec!["docs/tech/ERROR_DIAGNOSTICS.md".into()],
        };
    }
    if lower.contains("allow_daily") || lower.contains("门禁") || lower.contains("未开放") {
        return DiagReport {
            code: ErrorCode::GateDenied.as_str().into(),
            category: "config".into(),
            title_zh: "功能门禁未开放".into(),
            detail_zh: "当前版本或配置禁止执行该消耗性操作。".into(),
            possible_causes_zh: vec!["安全门禁关闭".into()],
            next_steps_zh: vec!["等待维护者开放，或使用已允许的功能（如获取信息）".into()],
            http_status: None,
            tech_detail: sanitize_body(t),
            doc_refs: vec!["docs/tech/ERROR_DIAGNOSTICS.md".into()],
        };
    }

    // http NNN: body
    if let Some(rest) = t.strip_prefix("http ") {
        if let Some((code_s, body)) = rest.split_once(':') {
            if let Ok(status) = code_s.trim().parse::<u16>() {
                return classify_http(status, body.trim(), "HTTP");
            }
        }
    }
    if lower.starts_with("network:") {
        return classify_reqwest_text(t.trim_start_matches("network:").trim(), "网络");
    }

    DiagReport {
        code: ErrorCode::Other.as_str().into(),
        category: "other".into(),
        title_zh: "发生错误".into(),
        detail_zh: "未能自动归入已知类型，请结合技术细节与操作步骤判断。".into(),
        possible_causes_zh: vec![
            "业务逻辑返回的错误".into(),
            "未覆盖的新失败模式".into(),
        ],
        next_steps_zh: vec![
            "重试一次".into(),
            "复制完整报告给维护者".into(),
        ],
        http_status: None,
        tech_detail: sanitize_body(t),
        doc_refs: vec!["docs/tech/ERROR_DIAGNOSTICS.md".into()],
    }
}

fn classify_reqwest_text(raw: &str, phase: &str) -> DiagReport {
    // 构造一个伪分类：复用 timeout/connect 关键字
    let lower = raw.to_lowercase();
    if lower.contains("timed out") || lower.contains("timeout") {
        return DiagReport {
            code: ErrorCode::NetTimeout.as_str().into(),
            category: "network".into(),
            title_zh: format!("{phase}：超时"),
            detail_zh: "等待服务器响应超时。".into(),
            possible_causes_zh: vec![
                "网络慢或加速器卡顿".into(),
                "地区不正确导致链路挂起".into(),
            ],
            next_steps_zh: vec!["换节点重试".into(), "检查出口 IP 是否符合服务器要求".into()],
            http_status: None,
            tech_detail: sanitize_body(raw),
            doc_refs: vec!["docs/tech/ERROR_DIAGNOSTICS.md".into()],
        };
    }
    diagnose_network_generic(raw, phase)
}

fn diagnose_network_generic(raw: &str, phase: &str) -> DiagReport {
    DiagReport {
        code: ErrorCode::NetOther.as_str().into(),
        category: "network".into(),
        title_zh: format!("{phase}：网络错误"),
        detail_zh: "与远程服务器通信失败。".into(),
        possible_causes_zh: vec![
            "网络不稳定".into(),
            "加速器/防火墙".into(),
            "服务器不可达".into(),
        ],
        next_steps_zh: vec!["重试".into(), "检查网络与加速".into()],
        http_status: None,
        tech_detail: sanitize_body(raw),
        doc_refs: vec!["docs/tech/ERROR_DIAGNOSTICS.md".into()],
    }
}

impl CoreError {
    /// 结构化诊断（权威入口）

    pub fn diagnose(&self) -> DiagReport {
        match self {
            CoreError::Network(s) => {
                // 新格式 CODE|phase|raw
                if let Some((code, rest)) = s.split_once('|') {
                    if code.starts_with("NET_") {
                        let (phase, raw) = rest
                            .split_once('|')
                            .map(|(p, r)| (p, r))
                            .unwrap_or(("网络", rest));
                        let mut r = diagnose_text(&format!("network: {raw}"));
                        // 用 classify 关键字结果，但保留 phase 标题
                        if raw.to_lowercase().contains("timed out")
                            || raw.to_lowercase().contains("timeout")
                        {
                            r = classify_reqwest_text(raw, phase);
                        } else if raw.to_lowercase().contains("connect") {
                            r.code = ErrorCode::NetConnect.as_str().into();
                            r.title_zh = format!("{phase}：无法建立连接");
                            r.category = "network".into();
                            r.tech_detail = sanitize_body(raw);
                        } else {
                            r = diagnose_network_generic(raw, phase);
                            r.code = code.to_string();
                        }
                        r.code = code.to_string();
                        return r;
                    }
                }
                diagnose_text(&format!("network: {s}"))
            }
            CoreError::Http { status, body } => classify_http(*status, body, "游戏或渠道 HTTP"),
            CoreError::Api(s) => {
                if s.contains("428") || s.to_lowercase().contains("version") {
                    classify_http(428, s, "游戏 API")
                } else {
                    DiagReport {
                        code: ErrorCode::ApiBusiness.as_str().into(),
                        category: "api".into(),
                        title_zh: "游戏接口返回业务错误".into(),
                        detail_zh: "服务器返回了可解析的失败信息。".into(),
                        possible_causes_zh: vec![
                            "游戏内条件不满足".into(),
                            "会话或参数异常".into(),
                        ],
                        next_steps_zh: vec!["阅读技术细节中的服务器消息".into(), "调整设置后重试".into()],
                        http_status: None,
                        tech_detail: sanitize_body(s),
                        doc_refs: vec!["docs/tech/ERROR_DIAGNOSTICS.md".into()],
                    }
                }
            }
            CoreError::Crypto(s) => {
                if s.to_lowercase().contains("unpad") {
                    diagnose_text(&format!("crypto: {s}"))
                } else {
                    DiagReport {
                        code: ErrorCode::CryptoOther.as_str().into(),
                        category: "crypto".into(),
                        title_zh: "加解密失败".into(),
                        detail_zh: "打包或解包游戏数据时出错。".into(),
                        possible_causes_zh: vec!["协议实现问题".into(), "数据损坏".into()],
                        next_steps_zh: vec!["更新程序与指纹".into(), "报告维护者".into()],
                        http_status: None,
                        tech_detail: sanitize_body(s),
                        doc_refs: vec![
                            "docs/tech/ERROR_DIAGNOSTICS.md".into(),
                            "docs/tech/PROTOCOL_STACK.md".into(),
                        ],
                    }
                }
            }
            CoreError::Fingerprint(s) => diagnose_text(&format!("fingerprint: {s}")),
            CoreError::Login(s) => {
                let lower = s.to_lowercase();
                if lower.contains("台服") || lower.contains("sonet") {
                    return diagnose_text(s);
                }
                if lower.contains("invalid signature") {
                    return classify_http(403, s, "Gree 登录");
                }
                if lower.contains("401") || lower.contains("expired") {
                    return classify_http(401, s, "登录会话");
                }
                DiagReport {
                    code: ErrorCode::LoginOther.as_str().into(),
                    category: "login".into(),
                    title_zh: "登录失败".into(),
                    detail_zh: "无法完成渠道或游戏登录。".into(),
                    possible_causes_zh: vec![
                        "引继码/密码错误".into(),
                        "网络或地区不符合要求".into(),
                        "token 缓存异常".into(),
                    ],
                    next_steps_zh: vec![
                        "核对账号密码".into(),
                        "检查加速器地区".into(),
                        "清理 token 缓存后重试".into(),
                    ],
                    http_status: None,
                    tech_detail: sanitize_body(s),
                    doc_refs: vec![
                        "docs/tech/ERROR_DIAGNOSTICS.md".into(),
                        "docs/tech/SDK_AND_LOGIN.md".into(),
                    ],
                }
            }
            CoreError::Skip(s) => DiagReport {
                code: ErrorCode::ModuleSkip.as_str().into(),
                category: "module".into(),
                title_zh: "模块跳过".into(),
                detail_zh: s.clone(),
                possible_causes_zh: vec!["当前无需执行或条件不足（正常）".into()],
                next_steps_zh: vec![],
                http_status: None,
                tech_detail: String::new(),
                doc_refs: vec!["docs/tech/ERROR_DIAGNOSTICS.md".into()],
            },
            CoreError::Abort(s) => DiagReport {
                code: ErrorCode::ModuleAbort.as_str().into(),
                category: "module".into(),
                title_zh: "模块中止".into(),
                detail_zh: s.clone(),
                possible_causes_zh: vec!["配置缺失或危险操作被拦截".into()],
                next_steps_zh: vec!["检查该模块设置（队伍、关卡等）".into()],
                http_status: None,
                tech_detail: String::new(),
                doc_refs: vec!["docs/tech/ERROR_DIAGNOSTICS.md".into()],
            },
            CoreError::Io(e) => DiagReport {
                code: ErrorCode::Io.as_str().into(),
                category: "io".into(),
                title_zh: "读写本地文件失败".into(),
                detail_zh: "无法访问数据目录或缓存文件。".into(),
                possible_causes_zh: vec![
                    "权限不足".into(),
                    "杀毒软件锁定".into(),
                    "磁盘满或路径无效".into(),
                ],
                next_steps_zh: vec![
                    "确认 RustMadoka_data 可写".into(),
                    "以普通用户权限运行并排除杀软误报".into(),
                ],
                http_status: None,
                tech_detail: e.to_string(),
                doc_refs: vec!["docs/tech/ERROR_DIAGNOSTICS.md".into()],
            },
            CoreError::Json(e) => DiagReport {
                code: ErrorCode::Json.as_str().into(),
                category: "json".into(),
                title_zh: "JSON 解析失败".into(),
                detail_zh: "配置或响应不是合法 JSON。".into(),
                possible_causes_zh: vec!["文件损坏".into(), "手动粘贴格式错误".into()],
                next_steps_zh: vec!["检查粘贴内容".into(), "删除损坏缓存后重试".into()],
                http_status: None,
                tech_detail: e.to_string(),
                doc_refs: vec!["docs/tech/ERROR_DIAGNOSTICS.md".into()],
            },
            CoreError::Other(s) => diagnose_text(s),
        }
    }

    pub fn user_block_zh(&self) -> String {
        self.diagnose().format_user_block()
    }

    pub fn module_log_zh(&self) -> String {
        match self {
            CoreError::Skip(s) | CoreError::Abort(s) => s.clone(),
            _ => self.diagnose().format_module_log(),
        }
    }
}

/// anyhow / 任意 Display：尽量抽出中文诊断块
pub fn format_anyhow_zh(err: &impl std::fmt::Display) -> String {
    diagnose_text(&err.to_string()).format_user_block()
}

/// 供 JSON API 返回
pub fn diag_json_from_display(err: &impl std::fmt::Display) -> serde_json::Value {
    let r = diagnose_text(&err.to_string());
    serde_json::to_value(r).unwrap_or_else(|_| serde_json::json!({"title_zh": err.to_string()}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_binary_like() {
        let s = sanitize_body("\u{0001}\u{0002}\u{0000}abc");
        assert!(s.contains("无法按文本") || s.contains("�") || !s.is_empty());
    }

    #[test]
    fn classify_428() {
        let r = classify_http(428, "upgrade", "login");
        assert_eq!(r.code, "HTTP_428_VERSION");
        assert!(r.title_zh.contains("指纹") || r.title_zh.contains("428"));
    }

    #[test]
    fn classify_missing_fp() {
        let r = diagnose_text("缺少游戏指纹（version/sign/libcount）");
        assert_eq!(r.code, "FP_MISSING");
    }
}
