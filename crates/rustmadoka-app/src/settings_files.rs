//! 用户组下「可复制 JSON」设置旁路文件（layout schema 2）
//!
//! # 产品意图（主人 2026-08-08）
//! - **纯设置**与**按卡片/别名的设置**分开，方便直接复制 json 做配置同步。
//! - 引继/密码**不**写入这些文件（只在 identity / users 信封）。
//!
//! # 权威读写
//! 当前 Store 权威仍为 `users/{组}.json` 内 `GameAccount.config`。
//! 本模块在保存 config 时**双写**旁路文件；导入时可优先读旁路（可选）。
//!
//! # 路径
//! ```text
//! groups/{group}/settings/shared.json     # 组级纯设置（无别名键时的汇总；见 classify）
//! groups/{group}/cards/{alias}/settings.json  # 该别名完整模块 config（无凭证）
//! groups/{group}/cards/{alias}/link.json     # 可选：card_id 占位（身份独立后填）
//! ```
//!
//! # 文档
//! `docs/tech/DATA_FOLDER_LAYOUT.md` §1.1 · §0.1

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 路径段安全名（避免 `..` 与路径分隔符）
pub fn safe_segment(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    if s.is_empty() {
        s = "_".into();
    }
    // Windows 保留名粗处理
    let upper = s.to_ascii_uppercase();
    if matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "LPT1"
    ) {
        s = format!("_{s}");
    }
    s
}

pub fn group_dir(data_dir: &Path, group: &str) -> PathBuf {
    data_dir.join("groups").join(safe_segment(group))
}

pub fn card_settings_path(data_dir: &Path, group: &str, alias: &str) -> PathBuf {
    group_dir(data_dir, group)
        .join("cards")
        .join(safe_segment(alias))
        .join("settings.json")
}

pub fn shared_settings_path(data_dir: &Path, group: &str) -> PathBuf {
    group_dir(data_dir, group)
        .join("settings")
        .join("shared.json")
}

pub fn card_link_path(data_dir: &Path, group: &str, alias: &str) -> PathBuf {
    group_dir(data_dir, group)
        .join("cards")
        .join(safe_segment(alias))
        .join("link.json")
}

/// 判定「更偏纯设置」的键：不含队伍名/关卡 id 等常因号而异的字段时仍写入 shared 候选。
/// shared 文件保存**显式共享键**；完整卡片设置始终在 cards/{alias}/settings.json。
fn is_shared_key(key: &str) -> bool {
    // 模块开关与确认、商店优先、体力阈值等——跨卡同步有意义
    if key.starts_with("confirm_") {
        return true;
    }
    if key.ends_with("_shop") || key.contains("shop_priority") || key.contains("priority_") {
        return true;
    }
    matches!(
        key,
        "loginbonus"
            | "stamina_buy"
            | "super_sweep"
            | "raid_reward"
            | "self_raid"
            | "support_raid"
            | "like_raid"
            | "solo_raid"
            | "high_score"
            | "arena"
            | "basic"
            | "event"
            | "archive"
            | "event_shop"
            | "raid_shop"
            | "arena_shop"
            | "tower"
            | "heart"
            | "gather"
            | "freegacha"
            | "eventscenario"
            | "collection"
            | "battle_mission"
            | "mission"
            | "present"
            | "info"
            | "stamina_buy_count"
            | "basic_stamina_5star"
            | "basic_stamina_4star"
            | "basic_stamina_3star"
            | "log_auto_clean"
            | "log_keep_one_click"
            | "confirm_one_click_settings"
            | "confirm_one_click_home"
            | "confirm_one_click_daily"
    )
}

/// 从完整 config 抽出可跨卡共享的键。
pub fn extract_shared(config: &HashMap<String, Value>) -> HashMap<String, Value> {
    config
        .iter()
        .filter(|(k, _)| is_shared_key(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(value)?)?;
    std::fs::rename(&tmp, path).or_else(|_| {
        std::fs::copy(&tmp, path)?;
        std::fs::remove_file(&tmp)
    })?;
    Ok(())
}

/// 保存账号 config 后调用：双写卡片 settings + 合并更新 shared 候选。
pub fn mirror_account_settings(
    data_dir: &Path,
    group: &str,
    alias: &str,
    config: &HashMap<String, Value>,
) -> Result<()> {
    let card_path = card_settings_path(data_dir, group, alias);
    let card_doc = json!({
        "schema": 1,
        "kind": "card_settings",
        "group": group,
        "alias": alias,
        "note": "无引继/密码；可复制到另一别名目录实现配置同步",
        "config": config,
    });
    write_json(&card_path, &card_doc)?;

    // link 占位：card_id 待身份独立后填；先写别名便于人工认目录
    let link_path = card_link_path(data_dir, group, alias);
    if !link_path.is_file() {
        let link = json!({
            "schema": 1,
            "alias": alias,
            "card_id": null,
            "note": "card_id 在 accounts/ 独立身份落地后填写；device_id 仍按引继在 cache/device_by_account",
        });
        write_json(&link_path, &link)?;
    }

    // shared：与同组已有 shared 合并本卡的 shared 键（后写覆盖）
    let shared_path = shared_settings_path(data_dir, group);
    let mut shared_map = HashMap::new();
    if shared_path.is_file() {
        if let Ok(t) = std::fs::read_to_string(&shared_path) {
            if let Ok(v) = serde_json::from_str::<Value>(&t) {
                if let Some(obj) = v.get("config").and_then(|c| c.as_object()) {
                    for (k, val) in obj {
                        shared_map.insert(k.clone(), val.clone());
                    }
                }
            }
        }
    }
    for (k, v) in extract_shared(config) {
        shared_map.insert(k, v);
    }
    let shared_doc = json!({
        "schema": 1,
        "kind": "shared_settings",
        "group": group,
        "note": "纯设置（无别名/无引继）。可整文件复制到另一 groups/某组/settings/shared.json 做默认同步。队伍名/关卡ID等在 cards/*/settings.json",
        "config": shared_map,
    });
    write_json(&shared_path, &shared_doc)?;
    Ok(())
}

/// 读卡片旁路 settings（若无则 None）
pub fn load_card_settings(
    data_dir: &Path,
    group: &str,
    alias: &str,
) -> Result<Option<HashMap<String, Value>>> {
    let p = card_settings_path(data_dir, group, alias);
    if !p.is_file() {
        return Ok(None);
    }
    let t = std::fs::read_to_string(&p)?;
    let v: Value = serde_json::from_str(&t)?;
    let cfg = v
        .get("config")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));
    let map: HashMap<String, Value> = serde_json::from_value(cfg)?;
    Ok(Some(map))
}
