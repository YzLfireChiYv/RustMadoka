//! 统一错误类型
//!
//! # 文档
//! - 用户可见中文与错误码分类：`diag.rs` · `docs/tech/ERROR_DIAGNOSTICS.md` §3
//! - 业务码→Skip：`from_game_api_errors` · `docs/tech/W2_WIRE_ANALYSIS_AND_REWRITE_LIST.md` §2 · L13
//! - 协议坑：`docs/tech/LESSONS_RUST_PORT.md`
//!
//! # 约定
//! - `Display` 保留简短机读串（兼容旧日志）
//! - **展示给用户 / 任务日志** 请用 `user_block_zh()` / `module_log_zh()` / `diagnose()`
//! - HTTP 勿只抛状态码数字；构造时走 `diag::classify_http` 或 `CoreError::http_status`

use serde_json::Value;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {
    /// 网络层；优先用 `diag::network_from_reqwest` 构造（内嵌 NET_* code）

    #[error("network: {0}")]
    Network(String),
    /// HTTP 非成功；body 应尽量已是可展示短文本（构造前可 `sanitize`）

    #[error("http {status}: {body}")]
    Http { status: u16, body: String },
    #[error("api: {0}")]
    Api(String),
    #[error("crypto: {0}")]
    Crypto(String),
    #[error("fingerprint: {0}")]
    Fingerprint(String),
    #[error("login: {0}")]
    Login(String),
    /// 模块有意跳过（一键日常不抬 `ok=false`）。条件不满足、无可领、活动未开等。
    /// 与 `Abort`/`Api` 边界见 `docs/tech/ERROR_DIAGNOSTICS.md` §模块结果；**非**游戏结局全集。

    #[error("module skip: {0}")]
    Skip(String),
    /// 模块配置/前置不足等中止（计 `DailyReport.aborted`，抬 `ok=false`，仍继续后续模块）

    #[error("module abort: {0}")]
    Abort(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("other: {0}")]
    Other(String),
}

impl CoreError {
    pub fn other(s: impl Into<String>) -> Self {
        Self::Other(s.into())
    }

    /// 构造 HTTP 错误：body 自动净化，避免乱码进日志

    pub fn http_status(status: u16, body: impl AsRef<[u8]>, phase_zh: &str) -> Self {
        let raw = String::from_utf8_lossy(body.as_ref());
        let report = crate::diag::classify_http(status, &raw, phase_zh);
        // Display 仍带 status；完整中文走 diagnose()
        Self::Http {
            status,
            body: format!("{} | {}", report.code, crate::diag::sanitize_body(&raw)),
        }
    }

    /// 将游戏 API 回包 `errors[]` 映射为 Skip（业务条件）或 Api（未识别）。
    ///
    /// W2/W3 · L13/C20：HTTP 200 + 业务码表示「条件不满足」时应为跳过，禁止默认长诊断错误块。
    /// 文档：`docs/tech/W2_WIRE_ANALYSIS_AND_REWRITE_LIST.md` §2 · `docs/tech/LESSONS_RUST_PORT.md` L13

    pub fn from_game_api_errors(errs: &[Value]) -> Self {
        if errs.is_empty() {
            return Self::Api("empty errors array".into());
        }
        let mut parts = Vec::new();
        let mut prefer_skip: Option<String> = None;
        for e in errs {
            let code = e.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
            let domain = e
                .get("domain")
                .and_then(|d| d.as_str())
                .unwrap_or("");
            let reason = e
                .get("reason")
                .and_then(|r| r.as_str())
                .unwrap_or("");
            parts.push(format!("code={code} domain={domain} reason={reason}"));
            if prefer_skip.is_none() {
                if let Some(zh) = business_code_to_skip_zh(code, reason) {
                    prefer_skip = Some(zh);
                }
            }
        }
        if let Some(zh) = prefer_skip {
            return Self::Skip(zh);
        }
        Self::Api(parts.join("; "))
    }
}

/// 已知业务码 → 中文 Skip 说明。
///
/// **证据范围：** 表项来自本机 wire / 模块实跑样本，**不是**官方完整错误码手册。
/// 未列出的码 → `None` → 上层 `Api`（展示为错误），可能误伤「本应跳过」的条件；
/// 已列出的码也可能在其它接口语境下含义不同（例如 19001 在团战发车外的语义未穷尽）。
/// 新码须对照 wire 与游戏步骤再增，禁止凭猜测批量扩表。
fn business_code_to_skip_zh(code: i64, reason: &str) -> Option<String> {
    let zh = match code {
        // super_sweep 等：关卡游玩条件不满足（W2 EN 样本）
        18027 => "关卡游玩条件不满足，已跳过",
        // mission receive：无可领（W3）
        18044 => "当前没有可领取的任务，已跳过",
        // solo_raid skip：扫荡条件不足（W2 18054 + clearedDifficulty=0）
        18054 => "总力战不满足扫荡条件（可能尚未通关可扫难度），已跳过",
        // multi_raid initialize：本号 wire 曾 19001；暂按业务拒绝跳过，有团战编成号上可再证
        19001 => "团战发车被服务器拒绝（业务码 19001），已跳过",
        _ => {
            // 弱匹配：仅当 reason 明确含游玩条件时；其它 reason 不猜测
            if reason.contains("プレイ条件") || reason.contains("游玩条件") {
                "关卡游玩条件不满足，已跳过"
            } else {
                return None;
            }
        }
    };
    Some(format!("{zh}（code={code}）"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn business_18054_is_skip() {
        let errs = vec![json!({
            "code": 18054,
            "domain": "solo_raid_skip_quest_battle_logic",
            "reason": "dummy"
        })];
        match CoreError::from_game_api_errors(&errs) {
            CoreError::Skip(m) => assert!(m.contains("18054"), "{m}"),
            other => panic!("expected Skip, got {other}"),
        }
    }

    #[test]
    fn unknown_code_is_api() {
        let errs = vec![json!({ "code": 99999, "domain": "x", "reason": "y" })];
        match CoreError::from_game_api_errors(&errs) {
            CoreError::Api(_) => {}
            other => panic!("expected Api, got {other}"),
        }
    }
}
