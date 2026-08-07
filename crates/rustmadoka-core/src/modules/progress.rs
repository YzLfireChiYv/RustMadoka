//! 长任务进度事件 — 供 NDJSON/SSE 流式推送
//!
//! 文档: docs/tech/PHASE_R2_MODULE_PARITY.md §U1/U2 · docs/tech/HTTP_SERVER.md
//!
//! 统一模型 `{kind,key,name,round,total,status,message,done}`：
//! - 洗词条：每轮一行 `status=running`，结束 `done=true`
//! - 清日常：每模块开始/结束推一条；整次结束 `key=daily` + `done=true`

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

/// 进度事件（前端按 NDJSON 行解析）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    /// `"daily"` | `"wash"` | `"system"`

    pub kind: String,
    /// 模块 key（如 `loginbonus` / `super_wash`）或阶段名

    pub key: String,
    /// 中文显示名

    pub name: String,
    /// 当前轮次或当前模块序号（从 1）

    pub round: i64,
    /// 总轮次或模块总数

    pub total: i64,
    /// `running` | `success` | `skip` | `error` | `abort` | `info` | `done`

    pub status: String,
    /// 人话说明 / 模块日志摘要

    pub message: String,
    /// 整次任务是否结束

    pub done: bool,
}

impl ProgressEvent {
    pub fn info(kind: &str, key: &str, name: &str, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            key: key.into(),
            name: name.into(),
            round: 0,
            total: 0,
            status: "info".into(),
            message: message.into(),
            done: false,
        }
    }

    pub fn running(
        kind: &str,
        key: &str,
        name: &str,
        round: i64,
        total: i64,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            key: key.into(),
            name: name.into(),
            round,
            total,
            status: "running".into(),
            message: message.into(),
            done: false,
        }
    }

    pub fn module_done(
        kind: &str,
        key: &str,
        name: &str,
        round: i64,
        total: i64,
        status: &str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            key: key.into(),
            name: name.into(),
            round,
            total,
            status: status.into(),
            message: message.into(),
            done: false,
        }
    }

    pub fn finished(kind: &str, ok: bool, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            key: kind.into(),
            name: if ok { "完成".into() } else { "结束(有错误)".into() },
            round: 0,
            total: 0,
            status: if ok { "done".into() } else { "error".into() },
            message: message.into(),
            done: true,
        }
    }
}

/// 可选进度通道；无订阅方时静默
pub type ProgressTx = Option<UnboundedSender<ProgressEvent>>;

/// 发送进度；接收端已关则忽略
pub fn emit(tx: &ProgressTx, ev: ProgressEvent) {
    if let Some(t) = tx {
        let _ = t.send(ev);
    }
}
