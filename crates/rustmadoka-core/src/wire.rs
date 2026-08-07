//! Full-session server wire recording + debug probes (W1 · 加重 debug).
//!
//! # Build
//! - feature **`wire_record`**: enabled on `RustMadoka_debug.exe`
//! - without feature: ordinary `RustMadoka.exe` does not write wire
//!
//! # Product rule (debug)
//! **无差别**记录与游戏服务器 / Gree SDK 的通讯：请求明文载荷、envelope、密文 b64、
//! 回包明文/密文、HTTP 状态、错误；另可写 `probe` 阶段探针。
//! 会话由 `ensure_started` 在首次登录/任务时自动打开（已打开则复用）。
//!
//! # Disk layout (wire_record only)
//! `RustMadoka_data/wire/{alias}/{session_id}/`
//! - `meta.json` · `events.jsonl` · `session_end.json`
//!
//! # Docs (bidirectional)
//! - `docs/tech/WIRE_AND_DEBUG_PROBES.md`（本模块权威说明）
//! - `docs/PLAN_RUSTMADOKA_FULL_REWRITE.md`
//! - `docs/logs/2026-08-07-dual-exe-wire.md`
//! - `docs/tech/W2_WIRE_ANALYSIS_AND_REWRITE_LIST.md`
//! - Outbound 调用点：`client.rs` · `gree.rs` · `wire_scope.rs`
//! - `scripts/build-win-dual.ps1`

use crate::error::{CoreError, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Whether this core build includes wire recording (dev package true).
pub fn is_built_with_wire() -> bool {
    cfg!(feature = "wire_record")
}

/// Whether a wire session is currently active.
pub fn is_active() -> bool {
    #[cfg(feature = "wire_record")]
    {
        imp::active()
    }
    #[cfg(not(feature = "wire_record"))]
    {
        false
    }
}

/// Start recording. Errors if this build has no `wire_record` feature.
/// Prefer [`ensure_started`] so concurrent tasks reuse one session.
pub fn start(
    data_dir: &Path,
    alias: &str,
    channel: &str,
    purpose: &str,
) -> Result<PathBuf> {
    #[cfg(feature = "wire_record")]
    {
        imp::start(data_dir, alias, channel, purpose)
    }
    #[cfg(not(feature = "wire_record"))]
    {
        let _ = (data_dir, alias, channel, purpose);
        Err(CoreError::Other(
            "ordinary build has no wire_record feature".into(),
        ))
    }
}

/// Debug：若本构建支持 wire 且当前无会话，则开启；已有会话则返回现有目录。
/// 普通版 no-op 返回 `None`。
/// Docs: `docs/tech/WIRE_AND_DEBUG_PROBES.md`
pub fn ensure_started(
    data_dir: &Path,
    alias: &str,
    channel: &str,
    purpose: &str,
) -> Option<PathBuf> {
    #[cfg(feature = "wire_record")]
    {
        if let Some(d) = imp::current_dir_active() {
            return Some(d);
        }
        match imp::start(data_dir, alias, channel, purpose) {
            Ok(d) => {
                record_probe(
                    "wire_session_start",
                    serde_json::json!({
                        "alias": alias,
                        "channel": channel,
                        "purpose": purpose,
                        "dir": d.display().to_string(),
                    }),
                );
                Some(d)
            }
            Err(e) => {
                tracing::warn!(error = %e, "wire ensure_started failed");
                None
            }
        }
    }
    #[cfg(not(feature = "wire_record"))]
    {
        let _ = (data_dir, alias, channel, purpose);
        None
    }
}

/// 测试探针（仅 wire 会话 active 时写入 events.jsonl，kind=probe）。
pub fn record_probe(name: &str, fields: Value) {
    #[cfg(feature = "wire_record")]
    {
        let mut extra = fields;
        if let Some(obj) = extra.as_object_mut() {
            obj.insert("probe".into(), Value::String(name.into()));
        } else {
            extra = serde_json::json!({ "probe": name, "data": extra });
        }
        imp::record_note("probe", extra);
    }
    #[cfg(not(feature = "wire_record"))]
    {
        let _ = (name, fields);
    }
}

pub fn stop() {
    #[cfg(feature = "wire_record")]
    imp::stop();
}

pub fn set_module_key(key: Option<&str>) {
    #[cfg(feature = "wire_record")]
    imp::set_module_key(key);
    #[cfg(not(feature = "wire_record"))]
    let _ = key;
}

#[allow(clippy::too_many_arguments)]
pub fn record_game_api(
    method: &str,
    full_url: &str,
    path: &str,
    request_payload: &Value,
    request_envelope: &Value,
    request_ciphertext: &[u8],
    http_status: u16,
    response_plain: Option<&Value>,
    response_ciphertext: Option<&[u8]>,
    error: Option<&str>,
) {
    record_game_api_timed(
        method,
        full_url,
        path,
        request_payload,
        request_envelope,
        request_ciphertext,
        http_status,
        response_plain,
        response_ciphertext,
        error,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn record_game_api_timed(
    method: &str,
    full_url: &str,
    path: &str,
    request_payload: &Value,
    request_envelope: &Value,
    request_ciphertext: &[u8],
    http_status: u16,
    response_plain: Option<&Value>,
    response_ciphertext: Option<&[u8]>,
    error: Option<&str>,
    duration_ms: Option<u64>,
) {
    #[cfg(feature = "wire_record")]
    imp::record_game_api_timed(
        method,
        full_url,
        path,
        request_payload,
        request_envelope,
        request_ciphertext,
        http_status,
        response_plain,
        response_ciphertext,
        error,
        duration_ms,
    );
    #[cfg(not(feature = "wire_record"))]
    {
        let _ = (
            method,
            full_url,
            path,
            request_payload,
            request_envelope,
            request_ciphertext,
            http_status,
            response_plain,
            response_ciphertext,
            error,
            duration_ms,
        );
    }
}

pub fn record_sdk_http(
    method: &str,
    url: &str,
    route: &str,
    request_body: Option<&str>,
    http_status: u16,
    response_json: &Value,
    error: Option<&str>,
) {
    #[cfg(feature = "wire_record")]
    imp::record_sdk_http(
        method,
        url,
        route,
        request_body,
        http_status,
        response_json,
        error,
    );
    #[cfg(not(feature = "wire_record"))]
    {
        let _ = (
            method,
            url,
            route,
            request_body,
            http_status,
            response_json,
            error,
        );
    }
}

pub fn record_note(event: &str, extra: Value) {
    #[cfg(feature = "wire_record")]
    imp::record_note(event, extra);
    #[cfg(not(feature = "wire_record"))]
    {
        let _ = (event, extra);
    }
}

pub fn current_dir() -> Option<PathBuf> {
    #[cfg(feature = "wire_record")]
    {
        imp::current_dir()
    }
    #[cfg(not(feature = "wire_record"))]
    {
        None
    }
}

#[cfg(feature = "wire_record")]
mod imp {
    use super::*;
    use base64::{engine::general_purpose::STANDARD as B64, Engine};
    use chrono::Utc;
    use serde_json::json;
    use std::fs::{self, File, OpenOptions};
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};

    struct WireInner {
        dir: PathBuf,
        events: File,
        seq: u64,
        module_key: Option<String>,
        alias: String,
        channel: String,
        purpose: String,
    }

    static WIRE: OnceLock<Mutex<Option<WireInner>>> = OnceLock::new();

    fn slot() -> &'static Mutex<Option<WireInner>> {
        WIRE.get_or_init(|| Mutex::new(None))
    }

    pub fn active() -> bool {
        slot().lock().map(|g| g.is_some()).unwrap_or(false)
    }

    /// Active session dir only (not last finished).
    pub fn current_dir_active() -> Option<PathBuf> {
        slot()
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|i| i.dir.clone()))
    }

    pub fn start(
        data_dir: &Path,
        alias: &str,
        channel: &str,
        purpose: &str,
    ) -> Result<PathBuf> {
        let safe_alias = sanitize_name(alias);
        let session_id = format!(
            "{}-{}",
            Utc::now().format("%Y%m%dT%H%M%S"),
            &uuid::Uuid::new_v4().to_string()[..8]
        );
        let dir = data_dir.join("wire").join(&safe_alias).join(&session_id);
        fs::create_dir_all(&dir)?;

        let meta = json!({
            "schema": 2,
            "session_id": session_id,
            "alias": alias,
            "channel": channel,
            "purpose": purpose,
            "started_at": Utc::now().to_rfc3339(),
            "edition": "debug",
            "doc": "docs/tech/WIRE_AND_DEBUG_PROBES.md · crates/rustmadoka-core/src/wire.rs",
            "note": "full wire + probes under data/wire; debug build only; all game/sdk HTTP when active",
        });
        fs::write(dir.join("meta.json"), serde_json::to_string_pretty(&meta)?)?;

        let events = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("events.jsonl"))?;

        let mut guard = slot()
            .lock()
            .map_err(|e| CoreError::Other(format!("wire lock: {e}")))?;
        if guard.is_some() {
            return Err(CoreError::Other(
                "wire session already active in this process".into(),
            ));
        }
        *guard = Some(WireInner {
            dir: dir.clone(),
            events,
            seq: 0,
            module_key: None,
            alias: alias.to_string(),
            channel: channel.to_string(),
            purpose: purpose.to_string(),
        });
        tracing::info!(dir = %dir.display(), alias, channel, purpose, "wire: session started");
        Ok(dir)
    }

    /// Last session dir (readable after stop for CLI wire_dir).
    static LAST_DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

    fn last_dir_slot() -> &'static Mutex<Option<PathBuf>> {
        LAST_DIR.get_or_init(|| Mutex::new(None))
    }

    pub fn stop() {
        let mut guard = match slot().lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        if let Some(inner) = guard.take() {
            let finished = json!({
                "finished_at": Utc::now().to_rfc3339(),
                "events": inner.seq,
                "alias": inner.alias,
                "channel": inner.channel,
                "purpose": inner.purpose,
                "dir": inner.dir.display().to_string(),
            });
            let _ = fs::write(
                inner.dir.join("session_end.json"),
                serde_json::to_string_pretty(&finished).unwrap_or_default(),
            );
            if let Ok(mut last) = last_dir_slot().lock() {
                *last = Some(inner.dir.clone());
            }
            tracing::info!(
                dir = %inner.dir.display(),
                events = inner.seq,
                "wire: session stopped"
            );
        }
    }

    pub fn set_module_key(key: Option<&str>) {
        if let Ok(mut guard) = slot().lock() {
            if let Some(inner) = guard.as_mut() {
                inner.module_key = key.map(|s| s.to_string());
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_game_api(
        method: &str,
        full_url: &str,
        path: &str,
        request_payload: &Value,
        request_envelope: &Value,
        request_ciphertext: &[u8],
        http_status: u16,
        response_plain: Option<&Value>,
        response_ciphertext: Option<&[u8]>,
        error: Option<&str>,
    ) {
        record_game_api_timed(
            method,
            full_url,
            path,
            request_payload,
            request_envelope,
            request_ciphertext,
            http_status,
            response_plain,
            response_ciphertext,
            error,
            None,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_game_api_timed(
        method: &str,
        full_url: &str,
        path: &str,
        request_payload: &Value,
        request_envelope: &Value,
        request_ciphertext: &[u8],
        http_status: u16,
        response_plain: Option<&Value>,
        response_ciphertext: Option<&[u8]>,
        error: Option<&str>,
        duration_ms: Option<u64>,
    ) {
        let rec = json!({
            "kind": "game_api",
            "method": method,
            "url": full_url,
            "path": path,
            "http_status": http_status,
            "request_payload": request_payload,
            "request_envelope": request_envelope,
            "request_ciphertext_b64": B64.encode(request_ciphertext),
            "request_ciphertext_len": request_ciphertext.len(),
            "response": response_plain,
            "response_ciphertext_b64": response_ciphertext.map(|b| B64.encode(b)),
            "response_ciphertext_len": response_ciphertext.map(|b| b.len()),
            "error": error,
            "duration_ms": duration_ms,
        });
        append_event(rec);
    }

    pub fn record_sdk_http(
        method: &str,
        url: &str,
        route: &str,
        request_body: Option<&str>,
        http_status: u16,
        response_json: &Value,
        error: Option<&str>,
    ) {
        let rec = json!({
            "kind": "sdk_http",
            "method": method,
            "url": url,
            "route": route,
            "http_status": http_status,
            "request_body": request_body,
            "response": response_json,
            "error": error,
        });
        append_event(rec);
    }

    pub fn record_note(event: &str, extra: Value) {
        let mut rec = extra;
        if let Some(obj) = rec.as_object_mut() {
            obj.insert("kind".into(), json!("note"));
            obj.insert("event".into(), json!(event));
        }
        append_event(rec);
    }

    pub fn current_dir() -> Option<PathBuf> {
        if let Ok(g) = slot().lock() {
            if let Some(inner) = g.as_ref() {
                return Some(inner.dir.clone());
            }
        }
        last_dir_slot()
            .lock()
            .ok()
            .and_then(|g| g.clone())
    }

    fn append_event(mut rec: Value) {
        let mut guard = match slot().lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let Some(inner) = guard.as_mut() else {
            return;
        };
        inner.seq += 1;
        if let Some(obj) = rec.as_object_mut() {
            obj.insert("seq".into(), json!(inner.seq));
            obj.insert("ts".into(), json!(Utc::now().to_rfc3339()));
            obj.insert("alias".into(), json!(&inner.alias));
            obj.insert("channel".into(), json!(&inner.channel));
            if let Some(m) = &inner.module_key {
                obj.insert("module_key".into(), json!(m));
            } else {
                obj.insert("module_key".into(), Value::Null);
            }
        }
        if let Ok(line) = serde_json::to_string(&rec) {
            let _ = writeln!(inner.events, "{line}");
            let _ = inner.events.flush();
        }
    }

    fn sanitize_name(s: &str) -> String {
        let t: String = s
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        if t.is_empty() {
            "account".into()
        } else {
            t
        }
    }
}
