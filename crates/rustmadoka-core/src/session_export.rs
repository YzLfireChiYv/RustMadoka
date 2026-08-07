//! 会话快照导出（E1）— full_login 后内存业务数据落盘。
//!
//! # 职责
//! - 将 `GameClient` 的 `init_data` / `game_config` / 已缓存 mst / 元数据写入目录
//! - 默认**不**写入 Gree 私钥、引继码、游戏密码（分析业务通常够用；本机明文仍合法 P9）
//! - 路径约定：`RustMadoka_data/exports/{alias}/{timestamp}/`
//!
//! # 文档
//! - `docs/tech/RUST_CODEBASE_AUDIT_AND_ROADMAP.md` §4.2 E1
//! - `docs/TASK_INVENTORY.md` §2
//! - `docs/tech/INIT_AND_RESPONSE_PAYLOADS.md`
//!
//! # 不变量
//! - 调用方须已完成 `full_login`（或 `GameClient::login`）
//! - 导出目录应 gitignore；不打进分发 exe（P8/P8b）

use crate::client::GameClient;
use crate::error::{CoreError, Result};
use crate::mst::MstCache;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// 导出选项
#[derive(Debug, Clone)]
pub struct SessionExportOptions {
    /// 是否写入游戏 sessionId（默认 false：分析业务通常不需要）

    pub include_session_id: bool,
    /// 是否写入 Gree uuid / 设备相关调试字段（默认 false；仍不含 privateKey 文件内容）

    pub include_device_debug: bool,
}

impl Default for SessionExportOptions {
    fn default() -> Self {
        Self {
            include_session_id: false,
            include_device_debug: false,
        }
    }
}

/// 调用方提供的账号侧元数据（不含密钥）
#[derive(Debug, Clone)]
pub struct SessionExportMeta {
    pub group: String,
    pub alias: String,
    pub channel: String,
    /// 可选：指纹 version / sm 摘要

    pub app_version: Option<String>,
    pub build_stamp: Option<String>,
}

/// 导出结果
#[derive(Debug, Clone)]
pub struct SessionExportResult {
    pub dir: PathBuf,
    pub manifest: Value,
}

impl MstCache {
    /// 已缓存 mst 的可序列化快照

    pub fn to_export_value(&self) -> Value {
        json!({
            "revision": self.revision,
            "style_list": self.style_list,
            "selection_ability_list": self.selection_ability_list,
            "character_list": self.character_list,
            "figure_list": self.figure_list,
            "on_demand_cache": self.export_on_demand_cache(),
        })
    }

    fn export_on_demand_cache(&self) -> Value {
        // 通过 public 访问：在 mst.rs 增加方法更干净；此处用 clone of known fields only
        // on-demand cache 为 private — 见 mst.rs 的 export helper
        self.on_demand_cache_json()
    }
}

/// 写入会话快照到 `base_exports/{alias}/{timestamp}/`
pub fn write_session_export(
    client: &GameClient,
    exports_root: &Path,
    meta: &SessionExportMeta,
    opts: &SessionExportOptions,
) -> Result<SessionExportResult> {
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let safe_alias = sanitize_path_segment(&meta.alias);
    let dir = exports_root.join(&safe_alias).join(&ts);
    fs::create_dir_all(&dir).map_err(|e| {
        CoreError::other(format!("创建导出目录失败 {}: {e}", dir.display()))
    })?;

    write_json(&dir.join("init_data.json"), &client.init_data)?;
    write_json(&dir.join("game_config.json"), &client.game_config)?;
    write_json(&dir.join("mst.json"), &client.mst.to_export_value())?;

    let mut meta_obj = json!({
        "schema": "automadoka-session-export-v1",
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "group": meta.group,
        "alias": meta.alias,
        "channel": meta.channel,
        "user_id": client.user_id,
        "app_version": meta.app_version,
        "build_stamp": meta.build_stamp,
        "fingerprint_version": client.fp.version,
        "fingerprint_channel": client.fp.channel,
        "files": [
            "manifest.json",
            "init_data.json",
            "game_config.json",
            "mst.json",
            "meta.json"
        ],
        "notes": [
            "full_login 后业务数据快照（E1）",
            "默认不含引继码、游戏密码、Gree privateKey",
            "目录位于 RustMadoka_data/exports，应 gitignore"
        ],
        "counts": {
            "init_data_is_null": client.init_data.is_null(),
            "game_config_is_null": client.game_config.is_null(),
            "mst_style": client.mst.style_list.len(),
            "mst_character": client.mst.character_list.len(),
            "mst_figure": client.mst.figure_list.len(),
            "mst_selection_ability": client.mst.selection_ability_list.len(),
            "party_count": client
                .init_data
                .get("partyDataList")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0),
        }
    });

    if opts.include_session_id {
        if let Some(obj) = meta_obj.as_object_mut() {
            obj.insert(
                "session_id".into(),
                json!(client.session_id.clone()),
            );
        }
    }
    if opts.include_device_debug {
        if let Some(obj) = meta_obj.as_object_mut() {
            obj.insert("uuid".into(), json!(client.uuid.clone()));
            let region = match client.gree.region {
                crate::gree::GreeRegion::Japan => "japan",
                crate::gree::GreeRegion::Global => "global",
            };
            obj.insert("gree_region".into(), json!(region));
        }
    }

    write_json(&dir.join("meta.json"), &meta_obj)?;
    // manifest 与 meta 同内容，便于只读 manifest
    write_json(&dir.join("manifest.json"), &meta_obj)?;

    Ok(SessionExportResult {
        dir,
        manifest: meta_obj,
    })
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|e| CoreError::other(format!("序列化 JSON 失败: {e}")))?;
    fs::write(path, text.as_bytes())
        .map_err(|e| CoreError::other(format!("写入 {} 失败: {e}", path.display())))?;
    Ok(())
}

fn sanitize_path_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' || (c as u32) > 127 {
            // 允许中文别名
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                out.push('_');
            } else {
                out.push(c);
            }
        } else if c == ' ' {
            out.push('_');
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "account".into()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_blocks_path_sep() {
        assert!(!sanitize_path_segment("a/../b").contains('/'));
        assert!(!sanitize_path_segment("a\\b").contains('\\'));
    }
}
