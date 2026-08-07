//! 任务执行日志落盘：进度实时写，完整日志定稿后可读。
//!
//! # 职责
//! - 一键清日常 / 单模块 / CLI 会话的索引、进度文件、定稿 JSON
//! - 与浏览器网页前端「任务日志」页、RunHub 进度分离（进度 ≠ 完整日志，C7）
//!
//! # 文档（双向链接）
//! - 路由/日志产品: `docs/tech/UI_ROUTING_AND_TASK_LOGS.md` · `docs/PLAN_UI_ROUTING_LOGS.md`
//! - **失败 message 中文与错误码:** `docs/tech/ERROR_DIAGNOSTICS.md` · `rustmadoka_core::diag`
//! - 多组/监视: `docs/tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md`
//!
//! # 约定
//! - `status=error` 且 `modules=[]` → 登录前/登录失败，原因在 **`message`**
//! - `message` 应由调用方写入 `err_zh` / `user_block_zh` 诊断块，勿只写英文 `Display`
//!
//! Outbound: `crates/rustmadoka-app/src/task_log.rs`

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskTrigger {
    OneClickDaily,
    SingleModule,
    Cli,
    Scheduled,
}

impl TaskTrigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OneClickDaily => "one_click_daily",
            Self::SingleModule => "single_module",
            Self::Cli => "cli",
            Self::Scheduled => "scheduled",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::OneClickDaily => "一键清日常",
            Self::SingleModule => "单独运行",
            Self::Cli => "CLI",
            Self::Scheduled => "定时任务",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Running,
    Paused,
    Success,
    Aborted,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleLogEntry {
    pub key: String,
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub log: String,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSession {
    pub id: String,
    pub trigger: TaskTrigger,
    pub group: String,
    pub alias: String,
    pub status: TaskStatus,
    pub started_at: String,
    #[serde(default)]
    pub finished_at: Option<String>,
    #[serde(default)]
    pub modules: Vec<ModuleLogEntry>,
    #[serde(default)]
    pub message: String,
    /// 是否已定稿（完成后才给前端完整日志，进行中请用 progress）
    #[serde(default)]
    pub finalized: bool,
    #[serde(default)]
    pub module_filter: Option<String>,
    /// 清日常开跑瞬间提取的配置快照（enabled + 扁平 config）；跑中用户改设置写磁盘，不改本次快照
    #[serde(default)]
    pub run_config_snapshot: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSessionSummary {
    pub id: String,
    pub trigger: TaskTrigger,
    pub group: String,
    pub alias: String,
    pub status: TaskStatus,
    pub started_at: String,
    #[serde(default)]
    pub finished_at: Option<String>,
    pub message: String,
    pub finalized: bool,
    /// 单独运行某模块时的模块键（列表筛选用）
    #[serde(default)]
    pub module_filter: Option<String>,
}

fn safe_seg(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c == ' ' {
                '_'
            } else {
                '%'
            }
        })
        .collect::<String>()
        + &format!("_{:x}", simple_hash(s))
}

fn simple_hash(s: &str) -> u32 {
    let mut h = 2166136261u32;
    for b in s.as_bytes() {
        h ^= *b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

pub fn account_dir(data_dir: &Path, group: &str, alias: &str) -> PathBuf {
    data_dir
        .join("task_logs")
        .join(safe_seg(group))
        .join(safe_seg(alias))
}

fn index_path(dir: &Path) -> PathBuf {
    dir.join("index.json")
}

fn session_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.json"))
}

fn progress_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.progress.json"))
}

pub fn begin_session(
    data_dir: &Path,
    group: &str,
    alias: &str,
    trigger: TaskTrigger,
    module_filter: Option<String>,
) -> Result<TaskSession> {
    begin_session_with_snapshot(data_dir, group, alias, trigger, module_filter, None)
}

/// 开跑时写入配置快照，便于日志查询「本次暂存的配置内容」。
pub fn begin_session_with_snapshot(
    data_dir: &Path,
    group: &str,
    alias: &str,
    trigger: TaskTrigger,
    module_filter: Option<String>,
    run_config_snapshot: Option<serde_json::Value>,
) -> Result<TaskSession> {
    let dir = account_dir(data_dir, group, alias);
    std::fs::create_dir_all(&dir)?;
    let id = uuid::Uuid::new_v4().to_string();
    let sess = TaskSession {
        id: id.clone(),
        trigger,
        group: group.into(),
        alias: alias.into(),
        status: TaskStatus::Running,
        started_at: chrono::Utc::now().to_rfc3339(),
        finished_at: None,
        modules: vec![],
        message: String::new(),
        finalized: false,
        module_filter,
        run_config_snapshot,
    };
    save_session(data_dir, &sess)?;
    write_progress(data_dir, &sess)?;
    push_index(data_dir, group, alias, &sess)?;
    Ok(sess)
}

pub fn save_session(data_dir: &Path, sess: &TaskSession) -> Result<()> {
    let dir = account_dir(data_dir, &sess.group, &sess.alias);
    std::fs::create_dir_all(&dir)?;
    let t = serde_json::to_string_pretty(sess)?;
    std::fs::write(session_path(&dir, &sess.id), t)?;
    Ok(())
}

pub fn write_progress(data_dir: &Path, sess: &TaskSession) -> Result<()> {
    let dir = account_dir(data_dir, &sess.group, &sess.alias);
    let snap = json_progress(sess);
    std::fs::write(progress_path(&dir, &sess.id), serde_json::to_string_pretty(&snap)?)?;
    Ok(())
}

fn json_progress(sess: &TaskSession) -> Value {
    serde_json::json!({
        "id": sess.id,
        "status": sess.status,
        "trigger": sess.trigger,
        "group": sess.group,
        "alias": sess.alias,
        "started_at": sess.started_at,
        "modules": sess.modules.iter().map(|m| {
            serde_json::json!({
                "key": m.key,
                "name": m.name,
                "status": m.status,
            })
        }).collect::<Vec<_>>(),
        "message": sess.message,
        "finalized": sess.finalized,
    })
}

pub fn finalize_session(
    data_dir: &Path,
    sess: &mut TaskSession,
    status: TaskStatus,
    message: impl Into<String>,
) -> Result<()> {
    sess.status = status.clone();
    sess.message = message.into();
    sess.finished_at = Some(chrono::Utc::now().to_rfc3339());
    sess.finalized = true;
    save_session(data_dir, sess)?;
    let dir = account_dir(data_dir, &sess.group, &sess.alias);
    let _ = std::fs::remove_file(progress_path(&dir, &sess.id));
    push_index(data_dir, &sess.group, &sess.alias, sess)?;
    // Windows 系统 toast：默认关；配置在 notifications/system_toast.json
    let ok = matches!(status, TaskStatus::Success);
    let title = if ok {
        "RustMadoka · 任务完成"
    } else {
        "RustMadoka · 任务结束"
    };
    let body = format!(
        "{} / {} · {}\n{}",
        sess.group,
        sess.alias,
        sess.trigger.as_str(),
        if sess.message.is_empty() {
            match status {
                TaskStatus::Success => "成功".to_string(),
                TaskStatus::Aborted => "已中止".to_string(),
                TaskStatus::Error => "错误".to_string(),
                _ => format!("{status:?}"),
            }
        } else {
            sess.message.chars().take(180).collect()
        }
    );
    crate::system_toast::notify_task_finished(data_dir, ok, title, &body);
    Ok(())
}

fn push_index(data_dir: &Path, group: &str, alias: &str, sess: &TaskSession) -> Result<()> {
    let dir = account_dir(data_dir, group, alias);
    let mut list: Vec<TaskSessionSummary> = if index_path(&dir).is_file() {
        serde_json::from_str(&std::fs::read_to_string(index_path(&dir))?).unwrap_or_default()
    } else {
        vec![]
    };
    list.retain(|x| x.id != sess.id);
    list.insert(
        0,
        TaskSessionSummary {
            id: sess.id.clone(),
            trigger: sess.trigger.clone(),
            group: sess.group.clone(),
            alias: sess.alias.clone(),
            status: sess.status.clone(),
            started_at: sess.started_at.clone(),
            finished_at: sess.finished_at.clone(),
            message: sess.message.clone(),
            finalized: sess.finalized,
            module_filter: sess.module_filter.clone(),
        },
    );
    // Keep index short.
    if list.len() > 500 {
        list.truncate(500);
    }
    std::fs::write(index_path(&dir), serde_json::to_string_pretty(&list)?)?;
    Ok(())
}

pub fn list_sessions(
    data_dir: &Path,
    group: &str,
    alias: &str,
    trigger: Option<&str>,
) -> Result<Vec<TaskSessionSummary>> {
    let dir = account_dir(data_dir, group, alias);
    if !index_path(&dir).is_file() {
        return Ok(vec![]);
    }
    let list: Vec<TaskSessionSummary> =
        serde_json::from_str(&std::fs::read_to_string(index_path(&dir))?)?;
    Ok(list
        .into_iter()
        .filter(|s| {
            if let Some(t) = trigger {
                if t.is_empty() {
                    return true;
                }
                return s.trigger.as_str() == t;
            }
            true
        })
        .collect())
}

/// 完整日志：仅 finalized 后可读（进行中用 load_progress）
pub fn load_full_session(data_dir: &Path, group: &str, alias: &str, id: &str) -> Result<TaskSession> {
    let dir = account_dir(data_dir, group, alias);
    let sess: TaskSession = serde_json::from_str(
        &std::fs::read_to_string(session_path(&dir, id)).context("session not found")?,
    )?;
    if !sess.finalized {
        anyhow::bail!("任务尚未结束，完整日志暂不可用；请使用 progress 接口查看实时进度");
    }
    Ok(sess)
}

pub fn load_progress(data_dir: &Path, group: &str, alias: &str, id: &str) -> Result<Value> {
    let dir = account_dir(data_dir, group, alias);
    let p = progress_path(&dir, id);
    if p.is_file() {
        return Ok(serde_json::from_str(&std::fs::read_to_string(p)?)?);
    }
    // 无 progress 文件时若已定稿，返回摘要
    let sess = load_full_session(data_dir, group, alias, id)?;
    Ok(json_progress(&sess))
}

pub fn clear_sessions(
    data_dir: &Path,
    group: &str,
    alias: &str,
    only_one_click: bool,
    keep_latest: Option<usize>,
) -> Result<usize> {
    let dir = account_dir(data_dir, group, alias);
    let mut list = list_sessions(data_dir, group, alias, None)?;
    let before = list.len();
    if only_one_click {
        let remove: Vec<_> = list
            .iter()
            .filter(|s| s.trigger == TaskTrigger::OneClickDaily)
            .cloned()
            .collect();
        if let Some(k) = keep_latest {
            for (i, s) in remove.iter().enumerate() {
                if i >= k {
                    let _ = std::fs::remove_file(session_path(&dir, &s.id));
                    let _ = std::fs::remove_file(progress_path(&dir, &s.id));
                }
            }
            list = list
                .into_iter()
                .filter(|s| {
                    if s.trigger != TaskTrigger::OneClickDaily {
                        return true;
                    }
                    remove.iter().take(k).any(|x| x.id == s.id)
                })
                .collect();
        } else {
            for s in &remove {
                let _ = std::fs::remove_file(session_path(&dir, &s.id));
                let _ = std::fs::remove_file(progress_path(&dir, &s.id));
            }
            list.retain(|s| s.trigger != TaskTrigger::OneClickDaily);
        }
    } else if let Some(k) = keep_latest {
        for s in list.iter().skip(k) {
            let _ = std::fs::remove_file(session_path(&dir, &s.id));
            let _ = std::fs::remove_file(progress_path(&dir, &s.id));
        }
        list.truncate(k);
    } else {
        for s in &list {
            let _ = std::fs::remove_file(session_path(&dir, &s.id));
            let _ = std::fs::remove_file(progress_path(&dir, &s.id));
        }
        list.clear();
    }
    std::fs::create_dir_all(&dir)?;
    std::fs::write(index_path(&dir), serde_json::to_string_pretty(&list)?)?;
    Ok(before.saturating_sub(list.len()))
}

/// Auto-trim one-click logs; keep latest `keep` sessions.（旧口径；新产品按天清理见 `clear_sessions_older_than`）
pub fn auto_trim_one_click(data_dir: &Path, group: &str, alias: &str, keep: usize) -> Result<()> {
    let _ = clear_sessions(data_dir, group, alias, true, Some(keep))?;
    Ok(())
}

/// 按天清理：删除 **早于** `retain_days` 天之前开始的任务日志（含一键/单独/CLI）。
/// `retain_days=7` 表示只保留近 7 天；立即执行由调用方在打开自动清理时触发。
/// Docs: docs/tech/UI_ROUTING_AND_TASK_LOGS.md · 主人 2026-08-07 新规
pub fn clear_sessions_older_than(
    data_dir: &Path,
    group: &str,
    alias: &str,
    retain_days: u32,
    only_one_click: bool,
) -> Result<usize> {
    use chrono::{Duration, Utc};
    let cutoff = Utc::now() - Duration::days(retain_days as i64);
    let dir = account_dir(data_dir, group, alias);
    let mut list = list_sessions(data_dir, group, alias, None)?;
    let before = list.len();
    let mut removed = 0usize;
    list.retain(|s| {
        if only_one_click && s.trigger != TaskTrigger::OneClickDaily {
            return true;
        }
        let started = chrono::DateTime::parse_from_rfc3339(&s.started_at)
            .ok()
            .map(|d| d.with_timezone(&Utc));
        let keep = match started {
            Some(t) => t >= cutoff,
            None => true,
        };
        if !keep {
            let _ = std::fs::remove_file(session_path(&dir, &s.id));
            let _ = std::fs::remove_file(progress_path(&dir, &s.id));
            removed += 1;
        }
        keep
    });
    std::fs::create_dir_all(&dir)?;
    std::fs::write(index_path(&dir), serde_json::to_string_pretty(&list)?)?;
    let _ = before;
    Ok(removed)
}
