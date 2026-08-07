//! Master 表 revision 缓存与登录预拉取。
//!
//! # 职责
//! - 登录后拉 resource master revision，再预取 style / selection_ability / character / figure
//! - 按 URL 末段 camelCase 名做 cache key；revision 未变则命中缓存
//! - 洗词条 UI 角色列表依赖上述全表（**不是**账号持有列表 · L3）
//!
//! # 文档
//! - `docs/tech/DATA_AND_MST.md` · `docs/tech/WASH_CHARACTER_LIST.md`
//! - `docs/tech/AUTOMADOKA_RESEARCH_AND_RUST_GAP.md` §2.4
//!
//! # 对照
//! `archive/pre-rust-2026-08/autopcr/db/database.py`

use crate::client::GameClient;
use crate::error::Result;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct MstCache {
    pub revision: HashMap<String, i64>,
    pub style_list: Vec<Value>,
    pub selection_ability_list: Vec<Value>,
    pub character_list: Vec<Value>,
    pub figure_list: Vec<Value>,
    cache: HashMap<String, Vec<Value>>,
}

impl MstCache {
    /// 按需缓存表（E1 导出用）

    pub fn on_demand_cache_json(&self) -> Value {
        let mut m = serde_json::Map::new();
        for (k, v) in &self.cache {
            m.insert(k.clone(), Value::Array(v.clone()));
        }
        Value::Object(m)
    }

    /// 取已缓存的 mst 表（key = URL 末段，如 `get_quest_stage_mst_list`）。
    /// 供 app 层落盘「关卡 ID↔名称」产品缓存，避免重复对服。
    pub fn cached_list(&self, url_or_key: &str) -> Option<&Vec<Value>> {
        let key = url_or_key.rsplit('/').next().unwrap_or(url_or_key);
        self.cache.get(key)
    }
}

impl GameClient {
    /// 登录后强制拉取四类 mst（洗词条 UI 全表）

    pub async fn bootstrap_mst(&mut self) -> Result<()> {
        // Python: GetResourceMasterDataMstListRequest → /api/mst/get_resource_master_data_mst_list
        if let Ok(rev) = self
            .request(
                "/api/mst/get_resource_master_data_mst_list",
                serde_json::json!({}),
            )
            .await
        {
            if let Some(list) = rev.get("mstList").and_then(|v| v.as_array()) {
                for x in list {
                    if let (Some(name), Some(r)) = (
                        x.get("name").and_then(|n| n.as_str()),
                        x.get("revision").and_then(|r| r.as_i64()),
                    ) {
                        self.mst.revision.insert(name.to_string(), r);
                    }
                }
            }
        }

        self.mst.style_list = self.fetch_mst_list("/api/mst/get_style_mst_list").await?;
        self.mst.selection_ability_list = self
            .fetch_mst_list("/api/mst/get_selection_ability_mst_list")
            .await?;
        self.mst.character_list = self
            .fetch_mst_list("/api/mst/get_character_mst_list")
            .await?;
        self.mst.figure_list = self
            .fetch_mst_list("/api/mst/get_style_figure_mst_list")
            .await?;

        tracing::info!(
            style = self.mst.style_list.len(),
            selection = self.mst.selection_ability_list.len(),
            character = self.mst.character_list.len(),
            figure = self.mst.figure_list.len(),
            "mst bootstrap done"
        );
        Ok(())
    }

    async fn fetch_mst_list(&mut self, url: &str) -> Result<Vec<Value>> {
        let p = self.request(url, serde_json::json!({})).await?;
        Ok(p.get("mstList")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default())
    }

    pub async fn mst_list(&mut self, url: &str) -> Result<Vec<Value>> {
        let key = url.rsplit('/').next().unwrap_or(url).to_string();
        if let Some(v) = self.mst.cache.get(&key) {
            return Ok(v.clone());
        }
        let list = self.fetch_mst_list(url).await?;
        self.mst.cache.insert(key, list.clone());
        Ok(list)
    }
}

// --- 关卡 ID ↔ 名称（产品化 · 对照原版 mst）---------------------------------
//
// 原版：`db.mst(MstApiGetQuestStageMstListRequest())` → QuestOutGameQuestStageMstRecord
// 字段真源：wire `get_quest_stage_mst_list` payload.mstList（见 BASIC_SUPER_SWEEP 文档）。
// Docs: docs/tech/BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS.md · DATA_AND_MST.md
// Outbound app: `run_ops::quest_stage_*` · CLI `mst quest-stages`

/// 从一条 mst 行取 i64 字段（缺省 0）。
fn row_i64(v: &Value, key: &str) -> i64 {
    v.get(key)
        .and_then(|x| x.as_i64().or_else(|| x.as_f64().map(|f| f as i64)))
        .unwrap_or(0)
}

fn row_str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

/// 关卡表公开摘要行（给人读/脚本用；去掉战斗预制体等噪音字段）。
///
/// # 字段
/// - `questStageMstId` · `name`：产品对照表主键
/// - `questGroupMstId` · `useStamina` · `difficulty`：扫荡/刷图选关
/// - `prevQuestStageMstId` · `recommendationPartyPower` · `rewardGroupId`：辅助
pub fn summarize_quest_stage(row: &Value) -> Value {
    serde_json::json!({
        "questStageMstId": row_i64(row, "questStageMstId"),
        "questGroupMstId": row_i64(row, "questGroupMstId"),
        "name": row_str(row, "name"),
        "useStamina": row_i64(row, "useStamina"),
        "difficulty": row_i64(row, "difficulty"),
        "prevQuestStageMstId": row_i64(row, "prevQuestStageMstId"),
        "recommendationPartyPower": row_i64(row, "recommendationPartyPower"),
        "rewardGroupId": row_i64(row, "rewardGroupId"),
    })
}

/// 按关卡 ID 查名称；找不到返回 None。
pub fn quest_stage_name(stages: &[Value], quest_stage_mst_id: i64) -> Option<String> {
    stages.iter().find_map(|r| {
        if row_i64(r, "questStageMstId") == quest_stage_mst_id {
            let n = row_str(r, "name");
            if n.is_empty() {
                None
            } else {
                Some(n)
            }
        } else {
            None
        }
    })
}

/// 格式化「ID + 名称」供模块日志：有名则 `411102（能力晶花…）`，无名则只 ID。
pub fn format_quest_stage_label(stages: &[Value], quest_stage_mst_id: i64) -> String {
    match quest_stage_name(stages, quest_stage_mst_id) {
        Some(n) => format!("{quest_stage_mst_id}（{n}）"),
        None => quest_stage_mst_id.to_string(),
    }
}

/// 过滤关卡摘要列表。
///
/// - `id`：精确关卡 ID
/// - `group_id`：精确组 ID（如 101=キオク）
/// - `name_contains`：名称子串（大小写不敏感）
/// - `limit`：0 表示不截断
pub fn filter_quest_stages(
    stages: &[Value],
    id: Option<i64>,
    group_id: Option<i64>,
    name_contains: Option<&str>,
    limit: usize,
) -> Vec<Value> {
    let needle = name_contains
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());
    let mut out: Vec<Value> = stages
        .iter()
        .filter(|r| {
            if let Some(want) = id {
                if row_i64(r, "questStageMstId") != want {
                    return false;
                }
            }
            if let Some(gid) = group_id {
                if row_i64(r, "questGroupMstId") != gid {
                    return false;
                }
            }
            if let Some(ref n) = needle {
                let name = row_str(r, "name").to_lowercase();
                if !name.contains(n.as_str()) {
                    return false;
                }
            }
            true
        })
        .map(summarize_quest_stage)
        .collect();
    out.sort_by_key(|r| row_i64(r, "questStageMstId"));
    if limit > 0 && out.len() > limit {
        out.truncate(limit);
    }
    out
}

#[cfg(test)]
mod quest_stage_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn format_and_filter_quest_stages() {
        let stages = vec![
            json!({
                "questStageMstId": 411102,
                "questGroupMstId": 403,
                "name": "能力晶花クエスト[木]RankNormal",
                "useStamina": 15,
                "difficulty": 2,
            }),
            json!({
                "questStageMstId": 401101,
                "questGroupMstId": 101,
                "name": "キオク強化素材獲得クエストRank1",
                "useStamina": 10,
                "difficulty": 1,
            }),
        ];
        assert_eq!(
            quest_stage_name(&stages, 411102).as_deref(),
            Some("能力晶花クエスト[木]RankNormal")
        );
        assert!(format_quest_stage_label(&stages, 411102).contains("411102"));
        assert!(format_quest_stage_label(&stages, 411102).contains("能力晶花"));
        let kioku = filter_quest_stages(&stages, None, Some(101), None, 0);
        assert_eq!(kioku.len(), 1);
        assert_eq!(kioku[0]["questStageMstId"], 401101);
        let by_name = filter_quest_stages(&stages, None, None, Some("晶花"), 10);
        assert_eq!(by_name.len(), 1);
        let by_id = filter_quest_stages(&stages, Some(401101), None, None, 0);
        assert_eq!(by_id.len(), 1);
    }
}
