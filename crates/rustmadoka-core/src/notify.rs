//! 功能隔离的通知历史（落盘）
//!
//! **隔离原则：** 每个功能一个文件，只能在该功能入口查看本功能历史。
//! 路径：`{data_dir}/notifications/{feature}.json`
//!
//! 文档: docs/HANDOFF.md · docs/logs/（设置自动保存与通知）
//! 产品约束：
//! - 浏览器 toast 不得抢焦点（由前端保证）
//! - 变更明细只记录**更改后**的状态
//! - 可配置最多存储条数；可「仅保留最近 N 条」清理更早历史

use crate::error::{CoreError, Result as CoreResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// 清日常 / 账号设置 功能键（配置自动保存通知落在此）
pub const FEATURE_SETTINGS: &str = "settings";

const SCHEMA: u32 = 1;
const DEFAULT_MAX_KEEP: usize = 500;
const MIN_MAX_KEEP: usize = 20;
const MAX_MAX_KEEP: usize = 5000;

/// 单条变更的**结果态**（不强制记录旧值）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeAfter {
    pub key: String,
    /// 人话标签，如「领取登陆奖励」

    pub label: String,
    /// 更改后的值

    pub after: Value,
}

/// 一条通知
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyEntry {
    pub id: String,
    /// ISO-8601 UTC

    pub at: String,
    /// 如 `config_save` / `config_error`

    pub category: String,
    pub title: String,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub alias: String,
    #[serde(default)]
    pub changes: Vec<ChangeAfter>,
    #[serde(default)]
    pub message: String,
}

/// 单功能通知文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureNotifyFile {
    #[serde(default = "schema_default")]
    pub schema: u32,
    pub feature: String,
    /// 最多保留条数（写入时自动裁剪）

    #[serde(default = "default_max_keep")]
    pub max_keep: usize,
    /// 新在前

    #[serde(default)]
    pub entries: Vec<NotifyEntry>,
}

fn schema_default() -> u32 {
    SCHEMA
}
fn default_max_keep() -> usize {
    DEFAULT_MAX_KEEP
}

fn clamp_max_keep(n: usize) -> usize {
    n.clamp(MIN_MAX_KEEP, MAX_MAX_KEEP)
}

/// 仅允许安全 feature 名（防路径穿越）
pub fn sanitize_feature(feature: &str) -> CoreResult<String> {
    let f = feature.trim();
    if f.is_empty()
        || f.len() > 64
        || !f
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(CoreError::Abort(format!("invalid feature name: {feature}")));
    }
    Ok(f.to_string())
}

fn file_path(data_dir: &Path, feature: &str) -> CoreResult<PathBuf> {
    let f = sanitize_feature(feature)?;
    Ok(data_dir.join("notifications").join(format!("{f}.json")))
}

impl FeatureNotifyFile {
    pub fn empty(feature: &str) -> Self {
        Self {
            schema: SCHEMA,
            feature: feature.to_string(),
            max_keep: DEFAULT_MAX_KEEP,
            entries: Vec::new(),
        }
    }

    pub fn load(data_dir: &Path, feature: &str) -> CoreResult<Self> {
        let path = file_path(data_dir, feature)?;
        if !path.is_file() {
            return Ok(Self::empty(feature));
        }
        let text = std::fs::read_to_string(&path)?;
        let mut file: Self = serde_json::from_str(&text)?;
        file.feature = sanitize_feature(feature)?;
        file.max_keep = clamp_max_keep(file.max_keep);
        Ok(file)
    }

    pub fn save(&self, data_dir: &Path) -> CoreResult<()> {
        let path = file_path(data_dir, &self.feature)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, text)?;
        Ok(())
    }

    fn trim_to_max(&mut self) {
        let max = clamp_max_keep(self.max_keep);
        self.max_keep = max;
        if self.entries.len() > max {
            self.entries.truncate(max);
        }
    }

    /// 追加一条（新在前），并按 max_keep 裁剪

    pub fn push(&mut self, entry: NotifyEntry) {
        self.entries.insert(0, entry);
        self.trim_to_max();
    }

    /// 设置最多存储条数并立即裁剪

    pub fn set_max_keep(&mut self, n: usize) {
        self.max_keep = clamp_max_keep(n);
        self.trim_to_max();
    }

    /// 清除「最近 keep 条之前」的历史，只保留最新 keep 条

    pub fn keep_latest(&mut self, keep: usize) {
        let k = keep.max(0);
        if self.entries.len() > k {
            self.entries.truncate(k);
        }
    }

    /// 筛选列表（仍新在前）。`group` 用于隔离用户组：B 组看不到 A 组改设置通知。

    pub fn query(
        &self,
        category: Option<&str>,
        alias: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<NotifyEntry> {
        self.query_filtered(category, alias, None, limit)
    }

    pub fn query_filtered(
        &self,
        category: Option<&str>,
        alias: Option<&str>,
        group: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<NotifyEntry> {
        let mut out: Vec<NotifyEntry> = self
            .entries
            .iter()
            .filter(|e| {
                if let Some(c) = category {
                    if !c.is_empty() && e.category != c {
                        return false;
                    }
                }
                if let Some(a) = alias {
                    if !a.is_empty() && e.alias != a {
                        return false;
                    }
                }
                if let Some(g) = group {
                    if !g.is_empty() && e.group != g {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();
        if let Some(lim) = limit {
            if out.len() > lim {
                out.truncate(lim);
            }
        }
        out
    }
}

/// 构造一条设置保存成功通知
pub fn make_config_save_entry(
    group: &str,
    alias: &str,
    changes: Vec<ChangeAfter>,
    title: impl Into<String>,
) -> NotifyEntry {
    let n = changes.len();
    let message = if n == 0 {
        "配置已保存".into()
    } else if n == 1 {
        format!("{} → {}", changes[0].label, format_after(&changes[0].after))
    } else {
        format!("已保存 {n} 项设置")
    };
    NotifyEntry {
        id: uuid::Uuid::new_v4().to_string(),
        at: chrono::Utc::now().to_rfc3339(),
        category: "config_save".into(),
        title: title.into(),
        group: group.into(),
        alias: alias.into(),
        changes,
        message,
    }
}

fn format_after(v: &Value) -> String {
    match v {
        Value::Bool(b) => {
            if *b {
                "开启".into()
            } else {
                "关闭".into()
            }
        }
        Value::String(s) => {
            if s.is_empty() {
                "（空）".into()
            } else {
                s.clone()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::Null => "（空）".into(),
        other => other.to_string(),
    }
}

/// 写入设置类通知并落盘
pub fn append_settings_notify(
    data_dir: &Path,
    group: &str,
    alias: &str,
    changes: Vec<ChangeAfter>,
) -> CoreResult<NotifyEntry> {
    let mut file = FeatureNotifyFile::load(data_dir, FEATURE_SETTINGS)?;
    let entry = make_config_save_entry(group, alias, changes, "设置已保存");
    file.push(entry.clone());
    file.save(data_dir)?;
    Ok(entry)
}
