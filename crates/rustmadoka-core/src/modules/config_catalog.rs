//! 模块配置项目录 — 对照 Python `@inttype/@texttype/@booltype` + shop_priority
//!
//! 存储模型（与原版一致，扁平字典）：
//! - 开关：`config[module_key] = true/false`（如 `loginbonus`）
//! - 设置：`config[setting_key] = value`（如 `stamina_buy_count`、`start_raid_party`）
//!
//! 文档: docs/tech/PHASE_R2_MODULE_PARITY.md · docs/MODULES.md
//! 对照: archive/.../module/config.py · modules/*.py

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// 单个配置字段（发给前端渲染表单）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    pub key: String,
    pub desc: String,
    /// `bool` | `int` | `text` | `single`

    pub config_type: String,
    pub default: Value,
    /// int/single 候选；bool 固定 [true,false]

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<Value>,
}

fn bool_f(key: &str, desc: &str, default: bool) -> ConfigField {
    ConfigField {
        key: key.into(),
        desc: desc.into(),
        config_type: "bool".into(),
        default: json!(default),
        candidates: vec![json!(true), json!(false)],
    }
}

fn int_f(key: &str, desc: &str, default: i64, candidates: Vec<i64>) -> ConfigField {
    ConfigField {
        key: key.into(),
        desc: desc.into(),
        config_type: "int".into(),
        default: json!(default),
        candidates: candidates.into_iter().map(|c| json!(c)).collect(),
    }
}

fn text_f(key: &str, desc: &str, default: &str) -> ConfigField {
    ConfigField {
        key: key.into(),
        desc: desc.into(),
        config_type: "text".into(),
        default: json!(default),
        candidates: vec![],
    }
}

/// 队伍类字段：前端圆点「列表选择 / 自行输入」（PLAN_PARTY_SELECT_UX · MULTI_GROUP §8）。
/// 配置值仍是名称 / partyIndex / partyDataId 字符串；列表模式推荐写入 partyDataId。
/// Docs: docs/tech/PARTY_TEAM_RESOLVE.md · docs/PLAN_PARTY_SELECT_UX.md
fn party_f(key: &str, desc: &str, default: &str) -> ConfigField {
    ConfigField {
        key: key.into(),
        desc: desc.into(),
        config_type: "party".into(),
        default: json!(default),
        candidates: vec![],
    }
}

/// 商店优先级类别（顺序与 Python shop.item_category 一致）
/// 对照: archive/.../modules/shop.py · 短码用下标压缩
/// 文档: docs/tech/LOG_ZH_MAP.md
pub fn shop_item_categories() -> &'static [(&'static str, &'static str)] {
    &[
        ("白送的东西", "白送的东西"),
        ("肖像", "肖像"),
        ("钥匙（碎片）", "钥匙（碎片）"),
        ("10抽钥匙", "10抽钥匙"),
        ("5x交换币", "5x交换币"),
        ("4x交换币", "4x交换币"),
        ("钻石", "钻石"),
        ("玩家经验", "玩家经验"),
        ("称号", "称号"),
        ("玩偶屋", "玩偶屋"),
        ("光之间内容", "光之间内容"),
        ("记忆切符", "记忆切符"),
        ("彩球", "彩球"),
        ("开孔材料", "开孔材料"),
        ("永久锁", "永久锁"),
        ("技能书", "技能书"),
        ("新属性球", "新属性球"),
        ("属性球", "属性球"),
        ("LP体力石", "LP体力石"),
        ("画板", "画板"),
        ("体力石", "体力石"),
        ("心砂", "心砂"),
        ("泪滴", "泪滴"),
        ("经验", "经验"),
        ("晶花抽取EX", "晶花抽取EX"),
        ("晶花抽取", "晶花抽取"),
        ("临时锁", "临时锁"),
        ("小石头", "小石头"),
        ("大石头", "大石头"),
        ("金币", "金币"),
        ("泪滴（无限池）", "泪滴（无限池）"),
        ("小石头（无限池）", "小石头（无限池）"),
        ("经验（无限池）", "经验（无限池）"),
        ("金币（无限池）", "金币（无限池）"),
    ]
}

/// shop_priority：`{prefix}_shop_priority_{类别}`。
/// **产品默认全部 0（不兑换）** — 主人 2026-08-06 口令；原版 Python 从 100 递减仅作对照。
/// 用户若要兑换，须在设置里把对应类别调到 >0。
fn shop_priority_fields(prefix: &str) -> Vec<ConfigField> {
    let mut out = Vec::new();
    let candidates: Vec<i64> = (0..=100).collect();
    for (cat, _) in shop_item_categories() {
        let key = format!("{prefix}_shop_priority_{cat}");
        let desc = format!("{cat}优先级，越高越优先，0为不购买（默认0）");
        out.push(int_f(&key, &desc, 0, candidates.clone()));
    }
    out
}

/// 原版 `shop.py` `@shop_priority` 默认：类别按 `item_category` 插入序，优先级从 100 起每次 −3。
///
/// `top_n`：只取最高优先的前 n 类（n=0 表示空表）。其余类别不写入（调用方保持 0=不买）。
/// 文档：`archive/.../modules/shop.py` · 主人 2026-08-07 测店 top5
pub fn upstream_shop_priority_patch(prefix: &str, top_n: usize) -> HashMap<String, Value> {
    let mut m = HashMap::new();
    let mut priority: i64 = 100;
    for (i, (cat, _)) in shop_item_categories().iter().enumerate() {
        if i >= top_n {
            break;
        }
        m.insert(
            format!("{prefix}_shop_priority_{cat}"),
            json!(priority),
        );
        priority = priority.saturating_sub(3);
    }
    m
}

#[cfg(test)]
mod shop_prio_tests {
    use super::upstream_shop_priority_patch;

    #[test]
    fn top5_matches_python_defaults() {
        let m = upstream_shop_priority_patch("event", 5);
        assert_eq!(m.len(), 5);
        assert_eq!(m["event_shop_priority_白送的东西"], serde_json::json!(100));
        assert_eq!(m["event_shop_priority_肖像"], serde_json::json!(97));
        assert_eq!(m["event_shop_priority_钥匙（碎片）"], serde_json::json!(94));
        assert_eq!(m["event_shop_priority_10抽钥匙"], serde_json::json!(91));
        assert_eq!(m["event_shop_priority_5x交换币"], serde_json::json!(88));
    }
}

/// 某日常模块的全部设置项（不含模块开关本身）
pub fn module_config_fields(module_key: &str) -> Vec<ConfigField> {
    match module_key {
        // stamina.py
        "stamina_buy" => vec![
            int_f(
                "stamina_buy_count",
                "购买次数",
                1,
                (1..=8).collect(),
            ),
            text_f("stamina_retain_count", "保留体力石", "120"),
        ],
        "basic" => vec![
            int_f(
                "basic_stamina_5star",
                "5星角色魔力突破目标",
                120,
                (1..=130).collect(),
            ),
            int_f(
                "basic_stamina_4star",
                "4星角色魔力突破目标",
                110,
                (1..=130).collect(),
            ),
            int_f(
                "basic_stamina_3star",
                "3星角色魔力突破目标",
                59,
                (1..=130).collect(),
            ),
        ],
        // tool.py super_sweep（挂日常）
        "super_sweep" => vec![
            int_f(
                "force_battle_repeat_times",
                "重复次数",
                1,
                (1..1000).collect(),
            ),
            text_f(
                "force_battle_quest_id",
                "关卡ID（可用「查名称」/ CLI mst quest-stages 查 ID↔名称；如 401101=キオクR1）",
                "411105",
            ),
            // 默认空：须手填；勿默认 "20"（易被当成编成序号，实为 partyDataId，母项目遗留坑）
            party_f(
                "force_battle_team",
                "队伍：名称 / 编成序号(partyIndex) / 服务器id(partyDataId)",
                "",
            ),
            int_f("force_battle_auto_mode", "自动模式", 0, vec![0, 1, 2]),
        ],
        // raid.py
        "raid_reward" => vec![bool_f(
            "raid_reward_self_only",
            "仅收取本人发车的战斗",
            false,
        )],
        // 队伍默认空：启用模块前必须手填，避免误用默认 party id 发车/援助
        "self_raid" => vec![
            text_f("start_raid_damage_min", "伤害下限", "900000"),
            text_f("start_raid_damage_max", "伤害上限", "1100000"),
            party_f("start_raid_party", "队伍名/id（须手填，默认空）", ""),
            text_f(
                "start_raid_result",
                "战斗结果(1:win, 2:lose, 3:timeout)",
                "3",
            ),
            bool_f(
                "start_raid_receive",
                "如果上一奖励未领取导致无法发车，则自动领取",
                true,
            ),
            bool_f(
                "start_raid_queue",
                "将召唤后的战斗放入待秒列表中",
                true,
            ),
            int_f("raid_recovery_count", "Raid氪体数", 0, vec![0, 1, 2, 3]),
        ],
        "support_raid" => vec![
            text_f("support_raid_damage_min", "伤害下限", "900000"),
            text_f("support_raid_damage_max", "伤害上限", "1100000"),
            text_f("support_raid_id", "关卡id（逗号分隔）", "120"),
            party_f("support_raid_party", "队伍名/id（须手填，默认空）", ""),
            text_f(
                "support_raid_result",
                "战斗结果(1:win, 2:lose, 3:timeout)",
                "3",
            ),
            text_f("support_raid_max", "不超过多少人时进入战斗", "2"),
            text_f("support_raid_time_max", "剩余多少分钟内进入战斗", "10"),
            bool_f("support_guild", "同时支援公会内的团战", true),
            int_f(
                "support_search_times",
                "搜索列表内的团战次数",
                0,
                vec![0, 1, 2, 3],
            ),
            bool_f(
                "support_queue",
                "将支援后的战斗放入待秒列表中",
                true,
            ),
            int_f("raid_recovery_count", "Raid氪体数", 0, vec![0, 1, 2, 3]),
        ],
        "like_raid" => vec![int_f(
            "search_times",
            "搜索列表内的团战次数",
            10,
            (1..=10).collect(),
        )],
        // sweep.py heart
        "heart" => vec![
            bool_f(
                "heart_force_sweep",
                "强制扫荡未解锁的最高心之器",
                false,
            ),
            party_f(
                "heart_team",
                "队伍：名称 / 编成序号 / 服务器id（默认名「心之器」）",
                "心之器",
            ),
            // Python heart 也读 force_battle_auto_mode（与刷图共用键）
            int_f("force_battle_auto_mode", "自动模式(重打时)", 0, vec![0, 1, 2]),
        ],
        // shop.py
        "event_shop" => shop_priority_fields("event"),
        "raid_shop" => shop_priority_fields("raid"),
        "arena_shop" => shop_priority_fields("arena"),
        // wash（工具，供 UI 复用）
        "super_wash" => vec![
            int_f(
                "filter_sub_selection_times",
                "重复次数",
                10,
                (1..1000).collect(),
            ),
            text_f("filter_style", "目标角色 style_id", "0"),
            text_f(
                "filter_style_selection_index",
                "目标技能石序列（1代表1号槽）",
                "1",
            ),
            text_f("filter_sub_selection_key_1", "目标词条1 id", "0"),
            text_f("filter_sub_selection_key_2", "目标词条2 id", "0"),
            text_f("filter_sub_selection_key_3", "目标词条3 id", "0"),
            bool_f(
                "filter_style_intersection_logic",
                "是否启用【或/OR】逻辑",
                false,
            ),
        ],
        _ => vec![],
    }
}

/// 模块中文描述（@description）
pub fn module_description(module_key: &str) -> &'static str {
    match module_key {
        "loginbonus" => "领取登陆奖励",
        "stamina_buy" => "消耗体力石购买体力",
        "super_sweep" => "按关卡ID重复战斗（耗体力）",
        "raid_reward" => "自动收取团战结算奖励",
        "self_raid" => "使用给定伤害记录发车",
        "support_raid" => "查询团战池内的团战并进行支援",
        "like_raid" => "团战列表点赞赚好友勋章",
        "solo_raid" => "扫荡最高已通关难度的总力战",
        "high_score" => "扫荡打分",
        "arena" => "PVP自动全输掉",
        "basic" => "根据角色缺口扫荡最高等级素材本",
        "event" => "自动扫荡当前已通关活动",
        "archive" => "自动使用扫荡最高加成档案活动",
        "event_shop" => "按顺序兑换活动商店物品",
        "raid_shop" => "按顺序兑换raid商店物品",
        "arena_shop" => "按顺序兑换jjc商店物品",
        "tower" => "扫荡最高经验获得的露娜塔层",
        "heart" => "扫荡最高好感的心之器",
        "gather" => "收集首页宝箱",
        "freegacha" => "自动抽取免费扭蛋",
        "eventscenario" => "阅读全部活动剧情",
        "collection" => "去除所有光之间红点",
        "battle_mission" => "自动完成已通关的主线打一遍任务",
        "mission" => "领取已经完成的任务奖励",
        "present" => "领取礼物的所有礼物",
        "info" => "显示玩家信息并回写名称等级",
        "super_wash" => "快速洗词条",
        _ => "",
    }
}

/// 全部设置项的默认值扁平 map（不含模块开关）
pub fn all_setting_defaults() -> HashMap<String, Value> {
    let mut m = HashMap::new();
    for key in [
        "stamina_buy",
        "basic",
        "super_sweep",
        "raid_reward",
        "self_raid",
        "support_raid",
        "like_raid",
        "heart",
        "event_shop",
        "raid_shop",
        "arena_shop",
        "super_wash",
    ] {
        for f in module_config_fields(key) {
            m.entry(f.key).or_insert(f.default);
        }
    }
    m
}

/// 合并配置：目录默认 < 账号已存 < 请求体覆盖
pub fn merge_run_config(
    account_config: &HashMap<String, Value>,
    request_config: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut out = all_setting_defaults();
    for (k, v) in account_config {
        out.insert(k.clone(), v.clone());
    }
    for (k, v) in request_config {
        out.insert(k.clone(), v.clone());
    }
    out
}

/// 解析启用表：请求 enabled > 账号 config[module_key] > 目录默认
pub fn resolve_enabled_from_store(
    catalog_defaults: &[(String, bool)],
    account_config: &HashMap<String, Value>,
    request_enabled: &HashMap<String, bool>,
) -> HashMap<String, bool> {
    let mut out = HashMap::new();
    for (key, default_on) in catalog_defaults {
        let from_acc = account_config
            .get(key)
            .and_then(|v| v.as_bool());
        let on = request_enabled
            .get(key)
            .copied()
            .or(from_acc)
            .unwrap_or(*default_on);
        out.insert(key.clone(), on);
    }
    out
}

/// 把 enabled + settings 合并成可 PUT 回账号的扁平 config
pub fn flatten_for_save(
    enabled: &HashMap<String, bool>,
    settings: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut out = HashMap::new();
    for (k, v) in settings {
        out.insert(k.clone(), v.clone());
    }
    for (k, on) in enabled {
        out.insert(k.clone(), json!(*on));
    }
    out
}
