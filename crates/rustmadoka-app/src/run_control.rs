//! In-process run status hub (multi-account) + 完整流式监视行缓冲。
//!
//! # 职责
//! - 设置页进度条：`RunStatusSnapshot`（粗粒度 round/total/message）
//! - **主页运行面板 / 程序运行面板终端风格**：`stream_lines` 完整行缓冲
//!   （刷图每轮开局/结算各一行，可数百行；与进度条不是同一产品面）
//!
//! Docs: `docs/tech/UI_ROUTING_AND_TASK_LOGS.md` · `docs/tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md` §3–§4
//! · `docs/PLAN_RUSTMADOKA_FULL_REWRITE.md`

use rustmadoka_core::RunControlFlags;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 单条监视流行（主页面板完整日志用）
#[derive(Debug, Clone, Serialize)]
pub struct StreamLine {
    pub seq: u64,
    pub ts: String,
    pub group: String,
    pub alias: String,
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunStatusSnapshot {
    pub busy: bool,
    pub paused: bool,
    pub session_id: Option<String>,
    pub group: Option<String>,
    pub alias: Option<String>,
    pub kind: Option<String>,
    pub account_key: Option<String>,
    #[serde(default)]
    pub game_id_hash: Option<String>,
    pub message: String,
    pub round: i64,
    pub total: i64,
    pub current_key: Option<String>,
    pub current_name: Option<String>,
    pub current_status: Option<String>,
    #[serde(default)]
    pub last_report: Option<String>,
    #[serde(default)]
    pub last_report_ok: Option<bool>,
}

impl Default for RunStatusSnapshot {
    fn default() -> Self {
        Self {
            busy: false,
            paused: false,
            session_id: None,
            group: None,
            alias: None,
            kind: None,
            account_key: None,
            game_id_hash: None,
            message: String::new(),
            round: 0,
            total: 0,
            current_key: None,
            current_name: None,
            current_status: None,
            last_report: None,
            last_report_ok: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RunStatusBundle {
    pub busy_any: bool,
    pub run: RunStatusSnapshot,
    pub runs: Vec<RunStatusSnapshot>,
    /// 完整监视流（主页面板用；可按 group 过滤后返回）
    #[serde(default)]
    pub stream_lines: Vec<StreamLine>,
}

struct RunEntry {
    flags: Option<Arc<RunControlFlags>>,
    snap: RunStatusSnapshot,
}

struct Inner {
    by_key: HashMap<String, RunEntry>,
    /// 全局流式行（按 seq 递增）；主页按用户组过滤展示
    stream: Vec<StreamLine>,
    stream_seq: u64,
}

/// 监视流最大行数（刷图 100 次 × 多行；超出丢最旧）
const STREAM_CAP: usize = 8000;

#[derive(Clone, Default)]
pub struct RunHub {
    inner: Arc<Mutex<Inner>>,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            by_key: HashMap::new(),
            stream: Vec::new(),
            stream_seq: 0,
        }
    }
}

impl RunHub {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::default())),
        }
    }

    fn push_stream_locked(g: &mut Inner, group: &str, alias: &str, kind: &str, text: impl Into<String>) {
        g.stream_seq = g.stream_seq.saturating_add(1);
        g.stream.push(StreamLine {
            seq: g.stream_seq,
            ts: chrono::Local::now().format("%H:%M:%S").to_string(),
            group: group.to_string(),
            alias: alias.to_string(),
            kind: kind.to_string(),
            text: text.into(),
        });
        if g.stream.len() > STREAM_CAP {
            let drop_n = g.stream.len() - STREAM_CAP;
            g.stream.drain(0..drop_n);
        }
    }

    /// 追加完整监视行（主页面板流；与进度条快照分离）
    pub fn append_stream_line(
        &self,
        group: &str,
        alias: &str,
        kind: &str,
        text: impl Into<String>,
    ) {
        let mut g = self.inner.lock().unwrap();
        Self::push_stream_locked(&mut g, group, alias, kind, text);
    }

    pub fn begin(
        &self,
        flags: Arc<RunControlFlags>,
        session_id: String,
        group: String,
        alias: String,
        kind: String,
        account_key: String,
    ) {
        let game_id_hash = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(account_key.as_bytes());
            Some(hex::encode(h.finalize())[..16].to_string())
        };
        let mut g = self.inner.lock().unwrap();
        Self::push_stream_locked(
            &mut g,
            &group,
            &alias,
            &kind,
            format!("—— 任务开始 · {kind} · 会话 {session_id} ——"),
        );
        g.by_key.insert(
            account_key.clone(),
            RunEntry {
                flags: Some(flags),
                snap: RunStatusSnapshot {
                    busy: true,
                    paused: false,
                    session_id: Some(session_id),
                    group: Some(group),
                    alias: Some(alias),
                    kind: Some(kind),
                    account_key: Some(account_key),
                    game_id_hash,
                    message: "running".into(),
                    round: 0,
                    total: 0,
                    current_key: None,
                    current_name: None,
                    current_status: Some("running".into()),
                    last_report: None,
                    last_report_ok: None,
                },
            },
        );
    }

    /// 运行中更新进度条快照 **并** 追加完整监视流行。
    /// Docs: docs/tech/UI_ROUTING_AND_TASK_LOGS.md · C7 · MULTI_GROUP §3
    pub fn update_progress(
        &self,
        account_key: &str,
        round: i64,
        total: i64,
        current_key: impl Into<String>,
        current_name: impl Into<String>,
        status: impl Into<String>,
        message: impl Into<String>,
    ) {
        let current_key = current_key.into();
        let current_name = current_name.into();
        let status = status.into();
        let message = message.into();
        let mut g = self.inner.lock().unwrap();
        let Some(e) = g.by_key.get_mut(account_key) else {
            return;
        };
        e.snap.busy = true;
        e.snap.round = round;
        e.snap.total = total;
        e.snap.current_key = Some(current_key.clone());
        e.snap.current_name = Some(current_name.clone());
        e.snap.current_status = Some(status.clone());
        e.snap.message = message.clone();
        let group = e.snap.group.clone().unwrap_or_default();
        let alias = e.snap.alias.clone().unwrap_or_default();
        let kind = e.snap.kind.clone().unwrap_or_default();
        let step = if total > 0 {
            format!("[{round}/{total}] ")
        } else {
            String::new()
        };
        let line = format!("{step}{current_name} · {status} · {message}");
        Self::push_stream_locked(&mut g, &group, &alias, &kind, line);
    }

    pub fn end_with_report(&self, account_key: &str, report: impl Into<String>, ok: bool) {
        let mut g = self.inner.lock().unwrap();
        let report = report.into();
        let Some(e) = g.by_key.get_mut(account_key) else {
            return;
        };
        e.flags = None;
        e.snap.busy = false;
        e.snap.paused = false;
        e.snap.current_status = Some(if ok { "success" } else { "error" }.into());
        e.snap.message = report.clone();
        e.snap.last_report = Some(report.clone());
        e.snap.last_report_ok = Some(ok);
        let group = e.snap.group.clone().unwrap_or_default();
        let alias = e.snap.alias.clone().unwrap_or_default();
        let kind = e.snap.kind.clone().unwrap_or_default();
        let tag = if ok { "成功结束" } else { "结束(有错误)" };
        Self::push_stream_locked(
            &mut g,
            &group,
            &alias,
            &kind,
            format!("—— {tag} · {report} ——"),
        );
    }

    pub fn clear_report(&self) {
        let mut g = self.inner.lock().unwrap();
        for e in g.by_key.values_mut() {
            e.snap.last_report = None;
            e.snap.last_report_ok = None;
            if e.flags.is_none() {
                e.snap.message.clear();
            }
        }
    }

    pub fn pause(&self) -> Result<(), String> {
        let mut g = self.inner.lock().unwrap();
        let e = g
            .by_key
            .values_mut()
            .find(|e| e.flags.is_some())
            .ok_or_else(|| "no running task".to_string())?;
        let f = e
            .flags
            .as_ref()
            .ok_or_else(|| "no running task".to_string())?;
        f.request_pause();
        e.snap.paused = true;
        e.snap.message = "pause requested".into();
        Ok(())
    }

    pub fn resume(&self) -> Result<(), String> {
        let mut g = self.inner.lock().unwrap();
        let e = g
            .by_key
            .values_mut()
            .find(|e| e.flags.is_some())
            .ok_or_else(|| "no running task".to_string())?;
        let f = e
            .flags
            .as_ref()
            .ok_or_else(|| "no running task".to_string())?;
        f.request_resume();
        e.snap.paused = false;
        e.snap.message = "resumed".into();
        Ok(())
    }

    pub fn abort(&self) -> Result<(), String> {
        let mut g = self.inner.lock().unwrap();
        let e = g
            .by_key
            .values_mut()
            .find(|e| e.flags.is_some())
            .ok_or_else(|| "no running task".to_string())?;
        let f = e
            .flags
            .as_ref()
            .ok_or_else(|| "no running task".to_string())?;
        f.request_abort();
        e.snap.paused = false;
        e.snap.message = "abort requested".into();
        Ok(())
    }

    pub fn bundle(&self, prefer_alias: Option<&str>) -> RunStatusBundle {
        self.bundle_prefer(prefer_alias, None)
    }

    pub fn bundle_prefer(
        &self,
        prefer_alias: Option<&str>,
        prefer_account_key: Option<&str>,
    ) -> RunStatusBundle {
        self.bundle_with_stream(prefer_alias, prefer_account_key, None)
    }

    /// `filter_group`: 主页流只返回该用户组相关行（MULTI_GROUP 主页隔离）
    pub fn bundle_with_stream(
        &self,
        prefer_alias: Option<&str>,
        prefer_account_key: Option<&str>,
        filter_group: Option<&str>,
    ) -> RunStatusBundle {
        let g = self.inner.lock().unwrap();
        let mut runs: Vec<RunStatusSnapshot> = g.by_key.values().map(|e| e.snap.clone()).collect();
        runs.sort_by(|a, b| b.busy.cmp(&a.busy));
        let busy_any = runs.iter().any(|r| r.busy);
        let run = prefer_account_key
            .and_then(|k| runs.iter().find(|r| r.account_key.as_deref() == Some(k)))
            .or_else(|| {
                prefer_alias.and_then(|a| runs.iter().find(|r| r.alias.as_deref() == Some(a)))
            })
            .or_else(|| runs.iter().find(|r| r.busy))
            .cloned()
            .unwrap_or_default();
        let stream_lines: Vec<StreamLine> = match filter_group {
            Some(fg) => g
                .stream
                .iter()
                .filter(|l| l.group == fg)
                .cloned()
                .collect(),
            None => g.stream.clone(),
        };
        RunStatusBundle {
            busy_any,
            run,
            runs,
            stream_lines,
        }
    }

    pub fn snapshot(&self) -> RunStatusSnapshot {
        self.bundle(None).run
    }

    /// 清空监视流（主页「清空显示」用；不影响任务本身）
    pub fn clear_stream(&self) {
        let mut g = self.inner.lock().unwrap();
        g.stream.clear();
    }

    /// 取出 `seq > after_seq` 的完整监视行（程序运行面板终端镜像用）。
    ///
    /// # 用途
    /// 黑色程序运行面板终端与浏览器网页前端主页共用同一 `stream` 缓冲；
    /// 终端按 seq 增量打印，避免 2 秒摘要把刷图逐轮过程压成一行。
    ///
    /// Docs: `docs/tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md` §3（PROC-MONITOR-TERMINAL）
    pub fn stream_lines_after(&self, after_seq: u64) -> Vec<StreamLine> {
        let g = self.inner.lock().unwrap();
        g.stream
            .iter()
            .filter(|l| l.seq > after_seq)
            .cloned()
            .collect()
    }

    /// 当前流末 seq（无行则为 0）
    pub fn stream_tail_seq(&self) -> u64 {
        let g = self.inner.lock().unwrap();
        g.stream.last().map(|l| l.seq).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustmadoka_core::RunControlFlags;
    use std::sync::Arc;

    #[test]
    fn stream_lines_after_returns_only_new_seq() {
        let hub = RunHub::new();
        let flags = Arc::new(RunControlFlags::default());
        hub.begin(
            flags,
            "sess-1".into(),
            "g1".into(),
            "a1".into(),
            "daily".into(),
            "en:code".into(),
        );
        hub.update_progress("en:code", 1, 5, "k", "名", "running", "轮1");
        hub.update_progress("en:code", 2, 5, "k", "名", "running", "轮2");
        let all = hub.stream_lines_after(0);
        assert!(all.len() >= 3, "begin + 2 progress");
        let mid = all[0].seq;
        let rest = hub.stream_lines_after(mid);
        assert!(rest.iter().all(|l| l.seq > mid));
        assert_eq!(hub.stream_tail_seq(), all.last().unwrap().seq);
    }

    #[test]
    fn bundle_filter_group_isolates_stream() {
        let hub = RunHub::new();
        hub.append_stream_line("组A", "x", "daily", "仅A");
        hub.append_stream_line("组B", "y", "daily", "仅B");
        let a = hub.bundle_with_stream(None, None, Some("组A"));
        assert!(a.stream_lines.iter().all(|l| l.group == "组A"));
        assert_eq!(a.stream_lines.len(), 1);
    }
}
