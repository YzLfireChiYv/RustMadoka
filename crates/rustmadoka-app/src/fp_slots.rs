//! Fingerprint slots: embedded (compile-time) / pulled (rules raw) / custom.
//!
//! # Product intent (主人 2026-08-07 再确认)
//! - **内嵌**：编译期 `EMBEDDED_COMBINED_JSON`，exe 内不改写。
//! - **刷新**：从 rules 仓拉 JSON → 写入数据目录槽 `default_pulled`，并**自动启用**该槽，
//!   同时旁路缓存 `cache/automadoka.json`，使登录立刻用上远端版本。
//! - **UI**：`/api/fp/slots` 返回日服/国际服版本、上次刷新、启用标签、槽可启用标志。
//!
//! Docs: `docs/tech/VERSION_FINGERPRINT.md` · NORMS P15 · HANDOFF

use anyhow::{bail, Result};
use rustmadoka_core::fingerprint::{
    channel_versions_from_text, fetch_fingerprint_text, fingerprint_from_combined_text,
    parse_fingerprint_file_text, Fingerprint, EMBEDDED_COMBINED_JSON,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

pub const SLOT_DEFAULT_EMBEDDED: &str = "default_embedded";
pub const SLOT_DEFAULT_PULLED: &str = "default_pulled";
pub const SLOT_CUSTOM_0: &str = "custom_0";
pub const SLOT_CUSTOM_1: &str = "custom_1";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LastDefaultRefresh {
    /// ISO-8601
    #[serde(default)]
    pub at: String,
    /// ok | unreachable | error
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub message_zh: String,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub bytes: usize,
    #[serde(default)]
    pub jp_version: Option<String>,
    #[serde(default)]
    pub en_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FpSlot {
    pub id: String,
    pub label: String,
    pub kind: String,
    #[serde(default)]
    pub filled: bool,
    #[serde(default)]
    pub combined_json: Option<String>,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub jp_version: Option<String>,
    #[serde(default)]
    pub en_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FpSlotStore {
    pub active_slot_id: String,
    pub slots: Vec<FpSlot>,
    #[serde(default)]
    pub last_default_refresh: Option<LastDefaultRefresh>,
}

impl Default for FpSlotStore {
    fn default() -> Self {
        let mut s = Self {
            active_slot_id: SLOT_DEFAULT_EMBEDDED.into(),
            slots: vec![
                slot_embedded(),
                empty_slot(SLOT_DEFAULT_PULLED, "default_pulled", "已拉取（默认源）"),
                empty_slot(SLOT_CUSTOM_0, "custom", "自定义槽 1"),
                empty_slot(SLOT_CUSTOM_1, "custom", "自定义槽 2"),
            ],
            last_default_refresh: None,
        };
        s.reenrich_all();
        s
    }
}

fn empty_slot(id: &str, kind: &str, label: &str) -> FpSlot {
    FpSlot {
        id: id.into(),
        label: label.into(),
        kind: kind.into(),
        filled: false,
        combined_json: None,
        note: String::new(),
        jp_version: None,
        en_version: None,
    }
}

fn slot_embedded() -> FpSlot {
    let mut s = FpSlot {
        id: SLOT_DEFAULT_EMBEDDED.into(),
        label: "程序内置".into(),
        kind: "embedded".into(),
        filled: true,
        combined_json: Some(EMBEDDED_COMBINED_JSON.to_string()),
        note: "编译期嵌入；不随刷新改写 exe".into(),
        jp_version: None,
        en_version: None,
    };
    enrich_slot_versions(&mut s);
    s
}

fn enrich_slot_versions(slot: &mut FpSlot) {
    if let Some(text) = slot.combined_json.as_deref() {
        let (jp, en) = channel_versions_from_text(text);
        slot.jp_version = jp;
        slot.en_version = en;
        if slot.filled && slot.jp_version.is_none() && slot.en_version.is_none() {
            // still filled but unparseable versions — keep filled
        }
    }
}

fn label_for_id(id: &str) -> &'static str {
    match id {
        SLOT_DEFAULT_EMBEDDED => "程序内置",
        SLOT_DEFAULT_PULLED => "已拉取（默认源）",
        SLOT_CUSTOM_0 => "自定义槽 1",
        SLOT_CUSTOM_1 => "自定义槽 2",
        _ => "指纹槽",
    }
}

impl FpSlotStore {
    fn path(data_dir: &Path) -> std::path::PathBuf {
        data_dir.join("fp_slots.json")
    }

    pub fn load(data_dir: &Path) -> Self {
        let p = Self::path(data_dir);
        if !p.is_file() {
            return Self::default();
        }
        match std::fs::read_to_string(&p) {
            Ok(t) => {
                let mut s: Self = serde_json::from_str(&t).unwrap_or_default();
                s.ensure_shape();
                s.reenrich_all();
                s
            }
            Err(_) => Self::default(),
        }
    }

    /// Ensure required slots exist; refresh embedded content from current binary.
    fn ensure_shape(&mut self) {
        let need = [
            (SLOT_DEFAULT_EMBEDDED, "embedded", "程序内置"),
            (SLOT_DEFAULT_PULLED, "default_pulled", "已拉取（默认源）"),
            (SLOT_CUSTOM_0, "custom", "自定义槽 1"),
            (SLOT_CUSTOM_1, "custom", "自定义槽 2"),
        ];
        for (id, kind, label) in need {
            if self.get(id).is_none() {
                self.slots.push(if id == SLOT_DEFAULT_EMBEDDED {
                    slot_embedded()
                } else {
                    empty_slot(id, kind, label)
                });
            }
        }
        // Always refresh embedded slot text from this binary (rebuild updates embed).
        if let Some(s) = self.get_mut(SLOT_DEFAULT_EMBEDDED) {
            s.combined_json = Some(EMBEDDED_COMBINED_JSON.to_string());
            s.filled = true;
            s.label = "程序内置".into();
            s.kind = "embedded".into();
            s.note = "编译期嵌入；不随刷新改写 exe".into();
            enrich_slot_versions(s);
        }
        if self.get(&self.active_slot_id).is_none()
            || !self
                .get(&self.active_slot_id)
                .map(|s| s.filled)
                .unwrap_or(false)
        {
            self.active_slot_id = SLOT_DEFAULT_EMBEDDED.into();
        }
    }

    fn reenrich_all(&mut self) {
        for s in &mut self.slots {
            if s.filled {
                enrich_slot_versions(s);
            }
            // Prefer Chinese product labels
            if s.id == SLOT_DEFAULT_EMBEDDED {
                s.label = "程序内置".into();
            } else if s.id == SLOT_DEFAULT_PULLED && (s.label == "Pulled default" || s.label.is_empty())
            {
                s.label = "已拉取（默认源）".into();
            } else if s.id == SLOT_CUSTOM_0 && s.label.starts_with("Custom") {
                s.label = "自定义槽 1".into();
            } else if s.id == SLOT_CUSTOM_1 && s.label.starts_with("Custom") {
                s.label = "自定义槽 2".into();
            }
        }
    }

    pub fn save(&self, data_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(data_dir)?;
        std::fs::write(Self::path(data_dir), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    fn get_mut(&mut self, id: &str) -> Option<&mut FpSlot> {
        self.slots.iter_mut().find(|s| s.id == id)
    }

    fn get(&self, id: &str) -> Option<&FpSlot> {
        self.slots.iter().find(|s| s.id == id)
    }

    pub fn activate(&mut self, id: &str) -> Result<()> {
        let s = self
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("找不到指纹槽"))?;
        if !s.filled {
            bail!("空槽不能启用");
        }
        if s.combined_json.is_none() {
            bail!("槽内无指纹数据");
        }
        let _ = parse_fingerprint_file_text(s.combined_json.as_ref().unwrap())
            .map_err(|e| anyhow::anyhow!("槽数据无效: {e}"))?;
        self.active_slot_id = id.into();
        Ok(())
    }

    pub fn reset_to_default_embedded(&mut self) -> Result<()> {
        self.activate(SLOT_DEFAULT_EMBEDDED)
    }

    pub fn fill_custom(&mut self, slot_id: &str, text: &str, note: &str) -> Result<()> {
        if slot_id != SLOT_CUSTOM_0 && slot_id != SLOT_CUSTOM_1 {
            bail!("只能写入自定义槽");
        }
        let _ = parse_fingerprint_file_text(text)
            .map_err(|e| anyhow::anyhow!("指纹 JSON 无效: {e}"))?;
        if let Some(s) = self.get_mut(slot_id) {
            s.filled = true;
            s.combined_json = Some(text.to_string());
            s.note = if note.is_empty() {
                "自定义写入".into()
            } else {
                note.to_string()
            };
            enrich_slot_versions(s);
        }
        Ok(())
    }

    pub fn clear_custom(&mut self, slot_id: &str) -> Result<()> {
        if slot_id != SLOT_CUSTOM_0 && slot_id != SLOT_CUSTOM_1 {
            bail!("只能清空自定义槽");
        }
        if self.active_slot_id == slot_id {
            self.active_slot_id = SLOT_DEFAULT_EMBEDDED.into();
        }
        let label = label_for_id(slot_id);
        if let Some(s) = self.get_mut(slot_id) {
            *s = empty_slot(slot_id, "custom", label);
        }
        Ok(())
    }

    pub fn apply_default_pull(&mut self, text: &str) -> Result<(Option<String>, Option<String>)> {
        let _ = parse_fingerprint_file_text(text)
            .map_err(|e| anyhow::anyhow!("拉取内容无效: {e}"))?;
        let (jp, en) = channel_versions_from_text(text);
        if let Some(s) = self.get_mut(SLOT_DEFAULT_PULLED) {
            s.filled = true;
            s.combined_json = Some(text.to_string());
            s.note = "来自 rules 默认源".into();
            s.label = "已拉取（默认源）".into();
            s.jp_version = jp.clone();
            s.en_version = en.clone();
        }
        Ok((jp, en))
    }

    pub fn active_combined_text(&self) -> Result<&str> {
        let s = self
            .get(&self.active_slot_id)
            .ok_or_else(|| anyhow::anyhow!("当前启用槽不存在"))?;
        s.combined_json
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("当前启用槽为空"))
    }

    /// API payload matching browser frontend expectations.
    pub fn to_public_json(&self) -> Value {
        let active = self.get(&self.active_slot_id);
        let (jp, en) = active
            .and_then(|s| s.combined_json.as_deref())
            .map(channel_versions_from_text)
            .unwrap_or((None, None));
        // Prefer active slot versions; fall back to embedded for display
        let (jp, en) = if jp.is_some() || en.is_some() {
            (jp, en)
        } else {
            channel_versions_from_text(EMBEDDED_COMBINED_JSON)
        };

        let active_label = active
            .map(|s| s.label.clone())
            .unwrap_or_else(|| "—".into());

        let slots: Vec<Value> = self
            .slots
            .iter()
            .map(|s| {
                let can_enable = s.filled && s.combined_json.is_some();
                let can_delete = s.kind == "custom";
                json!({
                    "id": s.id,
                    "label": s.label,
                    "kind": s.kind,
                    "filled": s.filled,
                    "note": s.note,
                    "jp_version": s.jp_version,
                    "en_version": s.en_version,
                    "active": s.id == self.active_slot_id,
                    "can_enable": can_enable,
                    "can_delete": can_delete,
                })
            })
            .collect();

        let last = self.last_default_refresh.clone().unwrap_or_default();

        json!({
            "active_slot_id": self.active_slot_id,
            "active_label": active_label,
            "jp_version": jp,
            "en_version": en,
            "last_default_refresh": {
                "at": last.at,
                "status": last.status,
                "message_zh": last.message_zh,
                "source_url": last.source_url,
                "bytes": last.bytes,
                "jp_version": last.jp_version,
                "en_version": last.en_version,
            },
            "slots": slots,
            "hint": "内嵌=出厂保底（不改写 exe）。刷新=从 rules 拉取到「已拉取」槽并自动启用。自定义槽写入后需手动启用。",
        })
    }
}

/// Load fingerprint from active slot for channel.
pub fn load_fp_from_slots(data_dir: &Path, channel: &str) -> Result<Fingerprint> {
    let store = FpSlotStore::load(data_dir);
    let text = store.active_combined_text()?;
    fingerprint_from_combined_text(text, channel).map_err(|e| anyhow::anyhow!("{e}"))
}

fn write_cache_combined(data_dir: &Path, text: &str) {
    let cache = data_dir.join("cache");
    let _ = std::fs::create_dir_all(&cache);
    let _ = std::fs::write(cache.join("automadoka.json"), text);
    let _ = std::fs::write(cache.join("automadoka.combined.json"), text);
}

/// Pull default fingerprint source into default_pulled, auto-activate, cache on disk.
pub async fn refresh_default_source(data_dir: &Path, _force_daily_gate: bool) -> Result<Value> {
    let url = crate::app_settings::DEFAULT_FP_SOURCE_URL;
    let at = chrono::Utc::now().to_rfc3339();

    match fetch_fingerprint_text(url).await {
        Ok(text) => {
            let mut store = FpSlotStore::load(data_dir);
            let (jp, en) = match store.apply_default_pull(&text) {
                Ok(v) => v,
                Err(e) => {
                    store.last_default_refresh = Some(LastDefaultRefresh {
                        at: at.clone(),
                        status: "error".into(),
                        message_zh: format!("拉取内容无效：{e}"),
                        source_url: url.into(),
                        bytes: text.len(),
                        jp_version: None,
                        en_version: None,
                    });
                    let _ = store.save(data_dir);
                    bail!("拉取内容无效：{e}");
                }
            };
            // Product: after successful pull, use it immediately (no silent stick on embed).
            if let Err(e) = store.activate(SLOT_DEFAULT_PULLED) {
                store.last_default_refresh = Some(LastDefaultRefresh {
                    at: at.clone(),
                    status: "error".into(),
                    message_zh: format!("已下载但启用失败：{e}"),
                    source_url: url.into(),
                    bytes: text.len(),
                    jp_version: jp.clone(),
                    en_version: en.clone(),
                });
                let _ = store.save(data_dir);
                bail!("已下载但启用失败：{e}");
            }
            write_cache_combined(data_dir, &text);
            let msg = format!(
                "已从默认源更新并启用。日服 {} · 国际服 {}",
                jp.as_deref().unwrap_or("—"),
                en.as_deref().unwrap_or("—")
            );
            store.last_default_refresh = Some(LastDefaultRefresh {
                at: at.clone(),
                status: "ok".into(),
                message_zh: msg.clone(),
                source_url: url.into(),
                bytes: text.len(),
                jp_version: jp.clone(),
                en_version: en.clone(),
            });
            store.save(data_dir)?;
            Ok(json!({
                "ok": true,
                "source": url,
                "bytes": text.len(),
                "activated": SLOT_DEFAULT_PULLED,
                "jp_version": jp,
                "en_version": en,
                "result": {
                    "status": "ok",
                    "message_zh": msg,
                    "at": at,
                },
                "store": store.to_public_json(),
            }))
        }
        Err(e) => {
            let mut store = FpSlotStore::load(data_dir);
            let msg = format!("无法连接默认源：{e}");
            store.last_default_refresh = Some(LastDefaultRefresh {
                at: at.clone(),
                status: "unreachable".into(),
                message_zh: msg.clone(),
                source_url: url.into(),
                bytes: 0,
                jp_version: None,
                en_version: None,
            });
            let _ = store.save(data_dir);
            Ok(json!({
                "ok": false,
                "source": url,
                "result": {
                    "status": "unreachable",
                    "message_zh": msg,
                    "at": at,
                },
                "store": store.to_public_json(),
                "error": format!("{e}"),
            }))
        }
    }
}
