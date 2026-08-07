//! 数据文件夹布局版本与目录保证（正式自用 · 向后兼容）
//!
//! # 职责
//! - 启动时创建约定子目录（不存在则建；**不删除**已有用户文件）
//! - 写入/升级 `layout.json` 的 `layout_schema` 整数
//! - 新版本只**增加**字段与目录；禁止静默改名/搬迁导致主人丢号
//!
//! # 文档
//! - `docs/tech/DATA_FOLDER_LAYOUT.md`
//! - `docs/NORMS.md` P32 · P1b
//!
//! # Outbound 对照
//! - 用户组：`account::Store` → `users/{name}.json`（schema 2）
//! - 占用：`occupancy.rs` · `owner_lock.rs`

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 当前程序认识的数据文件夹布局版本。
/// - 1：users/ 混装 + cache/task_logs
/// - 2：增加 accounts/ · groups/ 树与可复制 settings 旁路（Store 权威仍可读 users/）
/// 升版本时：只允许「多认文件/多字段」；必须能读旧 schema 并**无损**打开。
pub const LAYOUT_SCHEMA: u32 = 2;

/// 布局清单文件名（旁路数据文件夹根）
pub const LAYOUT_FILE: &str = "layout.json";

/// 相对数据文件夹根的约定子目录（正式自用布局 v1）
pub const STANDARD_DIRS: &[&str] = &[
    "users",
    "accounts",
    "groups",
    "cache",
    "cache/token",
    "cache/mst",
    "cache/parties",
    "cache/device_by_account",
    "task_logs",
    "notifications",
    "exports",
    "wire",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutManifest {
    /// 布局 schema 整数（本文件真源）
    #[serde(default = "default_schema")]
    pub layout_schema: u32,
    /// 程序写入时的产品名
    #[serde(default)]
    pub product: String,
    /// 最近一次 ensure 的 UTC RFC3339
    #[serde(default)]
    pub ensured_at: String,
    /// 创建本清单时的程序版本（信息性）
    #[serde(default)]
    pub app_version: String,
    /// 备注：人类可读
    #[serde(default)]
    pub note: String,
}

fn default_schema() -> u32 {
    1
}

fn layout_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join(LAYOUT_FILE)
}

/// 读取清单；文件不存在返回 None。
pub fn load_manifest(data_dir: &Path) -> Result<Option<LayoutManifest>> {
    let p = layout_path(data_dir);
    if !p.is_file() {
        return Ok(None);
    }
    let t = std::fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
    let m: LayoutManifest = serde_json::from_str(&t)?;
    Ok(Some(m))
}

/// 若磁盘 layout_schema **大于**本程序认识的版本，拒绝写入以免破坏未来格式。
/// 小于或等于：允许打开（旧夹自动补目录并升到当前 schema 标记）。
pub fn check_layout_readable(data_dir: &Path) -> Result<()> {
    if let Some(m) = load_manifest(data_dir)? {
        if m.layout_schema > LAYOUT_SCHEMA {
            bail!(
                "数据文件夹 layout_schema={} 高于本程序支持的 {}。请升级 RustMadoka 程序后再打开。\n  路径: {}",
                m.layout_schema,
                LAYOUT_SCHEMA,
                data_dir.display()
            );
        }
    }
    Ok(())
}

/// 启动时调用：建目录、写/更新 layout.json。**永不删除** users/cache/token 等用户数据。
pub fn ensure_data_layout(data_dir: &Path, app_version: &str) -> Result<LayoutManifest> {
    std::fs::create_dir_all(data_dir)
        .with_context(|| format!("create data dir {}", data_dir.display()))?;
    check_layout_readable(data_dir)?;

    for rel in STANDARD_DIRS {
        let p = data_dir.join(rel);
        std::fs::create_dir_all(&p).with_context(|| format!("create {}", p.display()))?;
    }

    let prev = load_manifest(data_dir)?;
    let mut note = "RustMadoka data layout v2: users(compat) + accounts/ + groups/{settings,cards} + cache + task_logs; device_id by migration; occupancy_heartbeat ≠ owner.lock".to_string();
    if let Some(ref old) = prev {
        if old.layout_schema < LAYOUT_SCHEMA {
            note = format!(
                "upgraded layout_schema {} → {}; user files kept",
                old.layout_schema, LAYOUT_SCHEMA
            );
        } else if old.layout_schema == LAYOUT_SCHEMA {
            note = old.note.clone();
            if note.is_empty() {
                note = "layout ok".into();
            }
        }
    }

    let m = LayoutManifest {
        layout_schema: LAYOUT_SCHEMA,
        product: "RustMadoka".into(),
        ensured_at: chrono::Utc::now().to_rfc3339(),
        app_version: app_version.to_string(),
        note,
    };
    let p = layout_path(data_dir);
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&m)?)?;
    std::fs::rename(&tmp, &p).or_else(|_| {
        std::fs::copy(&tmp, &p)?;
        std::fs::remove_file(&tmp)
    })?;
    Ok(m)
}
