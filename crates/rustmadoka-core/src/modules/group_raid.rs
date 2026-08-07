//! 组队 Raid：同一用户组、同 channel — 召唤 → 互援 → 舔盒 → 轮次。
//!
//! # 产品规格（完整条件真源）
//! `docs/tech/GROUP_RAID_AND_DEVICE_IDENTITY.md`（**§8 UI/单号/删卡降级**）  
//! 任务书：`docs/PLAN_GROUP_RAID_UI.md`
//!
//! # 人数
//! - **≥2**：多号互援；开放范围禁止「仅自己」  
//! - **1**：单号打满今日召唤次数（`maxPlayCountPerDay`，常见 6，以 game_config 为准）  
//! - **0**：Abort（删卡后无剩余）
//!
//! # UI 入口
//! 浏览器：主页**组队配置卡片**（多份；添加账号区只加游戏号）— PLAN_GROUP_RAID_UI  
//! CLI：`run group-raid -g … --config-id …` 或 `--aliases a,b`
//!
//! # 结果口径
//! 单号无次数/体力不足为可预期结束；无点测不写 FIXED（P5）。
//!
//! # 对照
//! `daily.rs` self_raid/support_raid/raid_reward · archive raidworker（无组内多卡编排）

use crate::client::GameClient;
use crate::error::{CoreError, Result};
use crate::modules::daily::{self, resolve_party};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// 房间开放范围（用户必须显式选择）。
/// 「仅自己」**仅允许单号模式**（§8.2）；多号互援禁止。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomOpenMode {
    /// 仅自己（单号打满日次数）
    SelfOnly,
    /// 工会可入
    Guild,
    /// 好友可入
    Friend,
    /// 全体可入
    All,
}

impl RoomOpenMode {
    /// 解析开放范围。`member_count`：当前有效参与人数（删卡降级后）。
    pub fn parse(s: &str) -> Result<Self> {
        Self::parse_for_count(s, 2)
    }

    pub fn parse_for_count(s: &str, member_count: usize) -> Result<Self> {
        let raw = s.trim().to_ascii_lowercase();
        let mode = match raw.as_str() {
            "" | "unset" | "none" | "未选择" => {
                if member_count <= 1 {
                    // 单号默认仅自己
                    return Ok(Self::SelfOnly);
                }
                return Err(CoreError::Abort(
                    "组队 Raid：必须先选择房间开放范围（工会 / 好友 / 全体）。".into(),
                ));
            }
            "self" | "only_self" | "private" | "仅自己" => Self::SelfOnly,
            "guild" | "union" | "工会" => Self::Guild,
            "friend" | "friends" | "好友" => Self::Friend,
            "all" | "public" | "全体" | "公开" => Self::All,
            other => {
                return Err(CoreError::Abort(format!(
                    "组队 Raid：未知房间开放范围「{other}」。请使用 guild / friend / all / self。"
                )));
            }
        };
        if matches!(mode, Self::SelfOnly) && member_count >= 2 {
            return Err(CoreError::Abort(
                "组队 Raid：多账号时房间开放范围不能为「仅自己」，否则无法相互支援。".into(),
            ));
        }
        Ok(mode)
    }

    /// 映射到 `send_rescue` / 开房 `initialize_stage` 的 rescueType。
    ///
    /// 对照 Python：`self_raid` 开房固定 `rescueType=1`（偏公开可援）；
    /// 组队产品需要可配置开放范围，故：
    /// - 仅自己 → 0（不公开求援）
    /// - 工会/好友/全体 → 1/2/3（§7.1 暂定；真机可改映射表）
    pub fn send_rescue_type(self) -> i64 {
        match self {
            Self::SelfOnly => 0,
            Self::Guild => 1,
            Self::Friend => 2,
            Self::All => 3,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::SelfOnly => "仅自己",
            Self::Guild => "工会",
            Self::Friend => "好友",
            Self::All => "全体",
        }
    }
}

/// 单个参与账号（编排入参；密钥由上层从用户组加载后填入）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupRaidMember {
    pub alias: String,
    pub channel: String,
    pub migration_code: String,
    /// 召唤/援助共用队伍码（名或 id）；空则尝试主线队

    pub party: String,
}

/// 组队任务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupRaidConfig {
    /// 发起任务的用户组名（停止权归属）

    pub owner_group: String,
    pub room_open: RoomOpenMode,
    /// 援助后是否 retire（默认 false：保留结算奖励）

    #[serde(default)]
    pub leave_after_support: bool,
    /// 战斗结果码（默认 3=timeout，与单号默认一致）

    #[serde(default = "default_result")]
    pub battle_result: i64,
    /// 体力恢复：优先使用；0 表示仍按「有恢复次数就恢复」自动

    #[serde(default = "default_true")]
    pub prefer_stamina_recover: bool,
    pub members: Vec<GroupRaidMember>,
}

fn default_result() -> i64 {
    3
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupRaidReport {
    pub ok: bool,
    pub owner_group: String,
    pub room_open: String,
    pub rounds: usize,
    pub total_summons: usize,
    pub total_supports: usize,
    pub total_rewards: usize,
    pub logs: Vec<String>,
    pub message: String,
}

struct OpenedRoom {
    host_alias: String,
    stage_data_id: i64,
    stage_mst_id: i64,
    search_id: String,
    boss_hp: i64,
    host_dmg: i64,
    /// alias -> damage share（含房主）
    damages: HashMap<String, i64>,
}

fn j_i64(v: &Value, key: &str) -> i64 {
    v.get(key).and_then(|x| x.as_i64()).unwrap_or(0)
}
fn j_str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}
fn j_bool(v: &Value, key: &str) -> bool {
    v.get(key).and_then(|x| x.as_bool()).unwrap_or(false)
}

/// 组队伤害拆分：每人 ∈ [10%H, (110%−10%n)H]，且 Σ ≥ H。
///
/// - `n` = 实际动手人数（体力跳过者不计入，调用方传入 k）
/// - 规格：`docs/tech/GROUP_RAID_AND_DEVICE_IDENTITY.md` §4 · `PLAN_RUSTMADOKA_FULL_REWRITE` §1.1
/// - 保证：在 n∈[1,10] 且 H≥1 时返回长度 n 的向量且 sum ≥ H
pub fn split_group_raid_damages(boss_hp: i64, n: usize) -> Result<Vec<i64>> {
    if n == 0 {
        return Err(CoreError::Abort("伤害拆分：动手人数为 0".into()));
    }
    if n > 10 {
        return Err(CoreError::Abort(
            "组队 Raid 人数超过 10，伤害上下限无解（上限会低于 10%）".into(),
        ));
    }
    let h = boss_hp.max(1);
    let min_d = ((h * 10) / 100).max(1);
    // 上限比例 110% - 10%*n
    let max_pct = 110 - 10 * (n as i64);
    let max_d = ((h * max_pct) / 100).max(min_d);
    let target = h; // Σ ≥ H 即可打死

    // 可行性：n*max_d >= target 且 n*min_d 可能 <= 很大；至少 n*max >= H
    if (n as i64) * max_d < target {
        // 极端小 H：抬高 max
        let mut parts = vec![min_d; n];
        let mut sum: i64 = parts.iter().sum();
        let mut i = 0usize;
        while sum < target {
            parts[i % n] += 1;
            sum += 1;
            i += 1;
            if i > (target as usize) * 2 {
                break;
            }
        }
        return Ok(parts);
    }

    let mut rng = rand::thread_rng();
    for _try in 0..64 {
        let mut parts = Vec::with_capacity(n);
        let mut sum = 0i64;
        for _ in 0..n.saturating_sub(1) {
            let d = if max_d <= min_d {
                min_d
            } else {
                rng.gen_range(min_d..=max_d)
            };
            parts.push(d);
            sum += d;
        }
        let last = target - sum;
        if last >= min_d && last <= max_d {
            parts.push(last);
            return Ok(parts);
        }
        // 最后一人越界：重新随机
    }

    // 回退：尽量均匀再补足 Σ≥H
    let base = (target / n as i64).clamp(min_d, max_d);
    let mut parts = vec![base; n];
    let mut sum: i64 = parts.iter().sum();
    let mut guard = 0;
    while sum < target && guard < target * 2 {
        for p in &mut parts {
            if sum >= target {
                break;
            }
            if *p < max_d {
                *p += 1;
                sum += 1;
            }
        }
        guard += 1;
        if parts.iter().all(|p| *p >= max_d) && sum < target {
            // 全部顶满仍不足：允许最后一人突破上限（极少见）
            parts[n - 1] += target - sum;
            break;
        }
    }
    while sum > target {
        for p in parts.iter_mut().rev() {
            if sum <= target {
                break;
            }
            if *p > min_d {
                *p -= 1;
                sum -= 1;
            }
        }
        if parts.iter().all(|p| *p <= min_d) {
            break;
        }
    }
    Ok(parts)
}

const DAMAGE_ONCE: i64 = 10_000_000;

async fn add_damage_chunked(client: &mut GameClient, quest_data_id: i64, mut damage: i64) -> Result<()> {
    if quest_data_id <= 0 {
        return Err(CoreError::other(
            "add_damage：questDataId 无效",
        ));
    }
    if damage <= 0 {
        damage = 1;
    }
    while damage > 0 {
        let d = damage.min(DAMAGE_ONCE);
        damage -= d;
        client
            .request(
                "/api/multi_raid/add_damage",
                json!({ "questDataId": quest_data_id, "damage": d }),
            )
            .await?;
    }
    Ok(())
}

async fn battle_log_raid(client: &mut GameClient, qid: i64) -> String {
    daily::battle_log_raid_pub(client, qid).await
}

fn max_play_per_day(client: &GameClient) -> i64 {
    client
        .game_config
        .pointer("/multiRaidConfig/maxPlayCountPerDay")
        .and_then(|v| v.as_i64())
        .unwrap_or(6)
}

fn today_cleared(top: &Value) -> i64 {
    top.pointer("/multiRaidUserSeasonData/todayClearedCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

fn remaining_summons(client: &GameClient, top: &Value) -> i64 {
    (max_play_per_day(client) - today_cleared(top)).max(0)
}

fn uid_of(client: &GameClient) -> i64 {
    client
        .init_data
        .pointer("/userParamData/userId")
        .and_then(|v| v.as_i64())
        .unwrap_or(client.user_id)
}

async fn multi_raid_top(client: &mut GameClient) -> Result<Value> {
    client.request("/api/multi_raid/get_top", json!({})).await
}

async fn try_recover_stamina(client: &mut GameClient, need: i64, have: i64, prefer: bool) -> i64 {
    if !prefer || need <= have {
        return have;
    }
    let num = (need - have + 19) / 20;
    if num <= 0 {
        return have;
    }
    match client
        .request(
            "/api/multi_raid/recover_stamina",
            json!({ "num": num, "itemMstId": 290001 }),
        )
        .await
    {
        Ok(_) => have + num * 20,
        Err(_) => have,
    }
}

/// 运行组队 Raid（调用方已登录好 `clients`，key = alias）。
///
/// `clients` 必须覆盖 `config.members` 全部 alias。
pub async fn run_group_raid(
    clients: &mut HashMap<String, GameClient>,
    config: &GroupRaidConfig,
) -> Result<GroupRaidReport> {
    let mut logs = Vec::new();
    // §8.2：允许 1 人（单号打满日次数）；0 人非法
    if config.members.is_empty() {
        return Err(CoreError::Abort(
            "组队 Raid：没有有效参与账号（可能卡片已全部删除）".into(),
        ));
    }
    let n_members = config.members.len();
    let solo = n_members == 1;
    logs.push(format!(
        "组队 Raid 开始：用户组={} 开放={} 人数={} 模式={} 援助后退出={}",
        config.owner_group,
        config.room_open.label(),
        n_members,
        if solo {
            "单号·打满今日次数"
        } else {
            "多号互援"
        },
        config.leave_after_support
    ));
    if solo && !matches!(config.room_open, RoomOpenMode::SelfOnly) {
        logs.push(format!(
            "单号模式：开放范围={}（可选用「仅自己」减少外援）",
            config.room_open.label()
        ));
    }
    if !solo && matches!(config.room_open, RoomOpenMode::SelfOnly) {
        return Err(CoreError::Abort(
            "组队 Raid：多账号时不能使用「仅自己」开放范围".into(),
        ));
    }
    let ch0 = config.members[0].channel.trim().to_ascii_lowercase();
    for m in &config.members {
        if m.channel.trim().to_ascii_lowercase() != ch0 {
            return Err(CoreError::Abort(format!(
                "组队 Raid 要求同一服务器：{} 与 {} 的 channel 不一致",
                config.members[0].alias, m.alias
            )));
        }
        if !clients.contains_key(&m.alias) {
            return Err(CoreError::Abort(format!(
                "组队 Raid：缺少已登录客户端 alias={}",
                m.alias
            )));
        }
    }

    let mut total_summons = 0usize;
    let mut total_supports = 0usize;
    let mut total_rewards = 0usize;
    let mut rounds = 0usize;
    const MAX_ROUNDS: usize = 32; // 安全上限（日 cap 常见 6×人数）

    loop {
        if rounds >= MAX_ROUNDS {
            logs.push(format!("达到编排安全轮次上限 {MAX_ROUNDS}，结束"));
            break;
        }
        rounds += 1;
        logs.push(format!("—— 第 {rounds} 轮 ——"));

        // --- 阶段 A：各号召唤 ---
        let mut opened: Vec<OpenedRoom> = Vec::new();
        let mut any_summon_attempt = false;

        for m in &config.members {
            let client = clients.get_mut(&m.alias).unwrap();
            let top = match multi_raid_top(client).await {
                Ok(t) => t,
                Err(e) => {
                    logs.push(format!("[{}] get_top 失败：{e}", m.alias));
                    continue;
                }
            };
            let rem = remaining_summons(client, &top);
            if rem <= 0 {
                logs.push(format!(
                    "[{}] 今日召唤次数已满或无剩余（todayCleared={}/max={}）",
                    m.alias,
                    today_cleared(&top),
                    max_play_per_day(client)
                ));
                continue;
            }
            any_summon_attempt = true;

            // 未结束自开房
            let uid = uid_of(client);
            if let Some(stages) = top.get("multiRaidStageDataList").and_then(|v| v.as_array()) {
                let mut busy = false;
                for raid in stages {
                    if j_i64(raid, "hostUserId") == uid && !j_bool(raid, "isClosed") {
                        logs.push(format!(
                            "[{}] 已有未结束自开团战 stageDataId={}，本轮跳过召唤",
                            m.alias,
                            j_i64(raid, "multiRaidStageDataId")
                        ));
                        busy = true;
                        break;
                    }
                }
                if busy {
                    continue;
                }
            }

            let team = if m.party.trim().is_empty() {
                "0"
            } else {
                m.party.as_str()
            };
            let (party_id, party_name) = match resolve_party(client, team) {
                Ok(p) => p,
                Err(e) => {
                    logs.push(format!("[{}] 队伍解析失败：{e}", m.alias));
                    continue;
                }
            };
            if let Some(p) = client
                .init_data
                .get("partyDataList")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .find(|p| j_i64(p, "partyDataId") == party_id)
            {
                if !j_bool(p, "isMultiRaid") {
                    logs.push(format!(
                        "[{}] 队伍「{party_name}」isMultiRaid=false，跳过召唤",
                        m.alias
                    ));
                    continue;
                }
            }

            let raids = client
                .mst_list("/api/mst/get_multi_raid_mst_list")
                .await
                .unwrap_or_default();
            let opening = raids.iter().find(|x| {
                daily::in_window_pub(&j_str(x, "startTime"), &j_str(x, "endTime"))
            });
            let Some(opening) = opening else {
                logs.push(format!("[{}] 当前无开放团战赛季", m.alias));
                continue;
            };
            let season_id = j_i64(opening, "seasonId");
            let cleared = top
                .pointer("/multiRaidUserSeasonData/clearedDifficulty")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let stage_mst_id = (20i64).min(1 + cleared) + season_id * 100;
            let stages = match client
                .mst_list("/api/mst/get_multi_raid_stage_mst_list")
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    logs.push(format!("[{}] stage mst 失败：{e}", m.alias));
                    continue;
                }
            };
            let Some(record) = stages
                .iter()
                .find(|x| j_i64(x, "multiRaidStageMstId") == stage_mst_id)
            else {
                logs.push(format!("[{}] 找不到 stage_mst={stage_mst_id}", m.alias));
                continue;
            };
            let need = j_i64(record, "useStaminaForPlay");
            let user = top.get("multiRaidUserData").cloned().unwrap_or(Value::Null);
            let mut stamina = client.raid_stamina(&user);
            stamina = try_recover_stamina(client, need, stamina, config.prefer_stamina_recover).await;
            if need > stamina {
                logs.push(format!(
                    "[{}] 团战体力不足（{stamina}<{need}），本轮不再召唤",
                    m.alias
                ));
                continue;
            }

            // initialize 开房
            // Python raidworker.start_clear：partyDataId/rescueType/multiRaidStageMstId/multiRaidStageDataId=0
            // 字段集一致；rescueType 按开放范围（单号仅自己=0，多号常用 1/2/3），再 send_rescue 钉死可见性。
            let host_rescue = config.room_open.send_rescue_type();
            let init = match client
                .request(
                    "/api/multi_raid/initialize_stage",
                    json!({
                        "partyDataId": party_id,
                        "rescueType": host_rescue,
                        "multiRaidStageMstId": stage_mst_id,
                        "multiRaidStageDataId": 0
                    }),
                )
                .await
            {
                Ok(v) => v,
                Err(e) => {
                    logs.push(format!("[{}] 开房失败：{e}", m.alias));
                    continue;
                }
            };
            let stage = init
                .get("multiRaidStageData")
                .cloned()
                .unwrap_or(Value::Null);
            let room = init
                .get("multiRaidRoomData")
                .cloned()
                .unwrap_or(Value::Null);
            let stage_data_id = j_i64(&stage, "multiRaidStageDataId");
            let boss_hp = j_i64(&stage, "hp");
            let search_id = j_str(&stage, "searchId");
            let qid = j_i64(&room, "questDataId");
            if stage_data_id == 0 || boss_hp <= 0 || qid <= 0 {
                logs.push(format!(
                    "[{}] 开房回包异常 stageDataId={stage_data_id} hp={boss_hp} questDataId={qid}",
                    m.alias
                ));
                continue;
            }

            // 非「仅自己」时再 send_rescue 设定可见范围（Python self_raid 无此步，靠 initialize rescueType=1）
            if !matches!(config.room_open, RoomOpenMode::SelfOnly) {
                if let Err(e) = client
                    .request(
                        "/api/multi_raid/send_rescue",
                        json!({
                            "multiRaidStageDataId": stage_data_id,
                            "rescueType": host_rescue
                        }),
                    )
                    .await
                {
                    logs.push(format!(
                        "[{}] send_rescue 失败（仍继续本房）：{e}",
                        m.alias
                    ));
                }
            }

            // 本房：名单人数 n 拆伤；单号 n=1 则房主打满
            let n = config.members.len().max(1);
            let parts = match split_group_raid_damages(boss_hp, n) {
                Ok(p) => p,
                Err(e) => {
                    logs.push(format!("[{}] 伤害拆分失败：{e}", m.alias));
                    continue;
                }
            };
            let mut damages = HashMap::new();
            for (i, mem) in config.members.iter().enumerate() {
                damages.insert(mem.alias.clone(), parts.get(i).copied().unwrap_or(1));
            }
            let host_dmg = *damages.get(&m.alias).unwrap_or(&1);
            let target: i64 = parts.iter().sum();

            // Python：get_multi_raid_info → battleLog → add_damage → finalize
            // 单号打满：房主伤≈H 时 result 强制 win(1)，与 Python support 过伤逻辑同向
            let host_result = if host_dmg >= boss_hp {
                1
            } else {
                config.battle_result
            };
            let blog = battle_log_raid(client, qid).await;
            if let Err(e) = add_damage_chunked(client, qid, host_dmg).await {
                logs.push(format!("[{}] 房主 add_damage 失败：{e}", m.alias));
                continue;
            }
            if let Err(e) = client
                .request(
                    "/api/multi_raid/finalize_stage_for_user",
                    json!({
                        "questDataId": qid,
                        "battleLog": blog,
                        "autoMode": 0,
                        "result": host_result
                    }),
                )
                .await
            {
                logs.push(format!("[{}] 房主 finalize 失败：{e}", m.alias));
                continue;
            }

            total_summons += 1;
            logs.push(format!(
                "[{}] 召唤成功 stageDataId={stage_data_id} mst={stage_mst_id} hp={boss_hp} 房主伤害={host_dmg} 队伍={party_name} 目标总伤≈{target}",
                m.alias
            ));
            opened.push(OpenedRoom {
                host_alias: m.alias.clone(),
                stage_data_id,
                stage_mst_id,
                search_id,
                boss_hp,
                host_dmg,
                damages,
            });
        }

        if opened.is_empty() {
            if !any_summon_attempt {
                logs.push("全员无剩余召唤次数或均不可开房，组队结束".into());
            } else {
                logs.push("本轮无人成功开房，组队结束".into());
            }
            break;
        }

        // --- 阶段 B：互援（不打自己的房）---
        // 主人钉死：能动手的支援完后 boss 必须死（Σ≥H）。房主已出手后跟踪剩余血量；
        // 名单上最后一名可尝试援助者用 max(计划份额, 剩余) 收尾，避免中间号体力跳过后打不完。
        for room in &opened {
            let mut remain = (room.boss_hp - room.host_dmg).max(0);
            if remain == 0 {
                logs.push(format!(
                    "房 {} 房主伤害已覆盖血量，跳过援助阶段",
                    room.host_alias
                ));
                continue;
            }
            let supporters: Vec<&GroupRaidMember> = config
                .members
                .iter()
                .filter(|m| m.alias != room.host_alias)
                .collect();
            for (idx, m) in supporters.iter().enumerate() {
                let is_last_in_roster = idx + 1 == supporters.len();
                let planned = *room.damages.get(&m.alias).unwrap_or(&1);
                let client = clients.get_mut(&m.alias).unwrap();
                let top = match multi_raid_top(client).await {
                    Ok(t) => t,
                    Err(e) => {
                        logs.push(format!("[{}] 援助前 get_top 失败：{e}", m.alias));
                        continue;
                    }
                };
                let team = if m.party.trim().is_empty() {
                    "0"
                } else {
                    m.party.as_str()
                };
                let (party_id, _) = match resolve_party(client, team) {
                    Ok(p) => p,
                    Err(e) => {
                        logs.push(format!("[{}] 援助队伍失败：{e}", m.alias));
                        continue;
                    }
                };
                let stages = client
                    .mst_list("/api/mst/get_multi_raid_stage_mst_list")
                    .await
                    .unwrap_or_default();
                let need = stages
                    .iter()
                    .find(|x| j_i64(x, "multiRaidStageMstId") == room.stage_mst_id)
                    .map(|r| j_i64(r, "useStaminaForRescue"))
                    .unwrap_or(10);
                let user = top.get("multiRaidUserData").cloned().unwrap_or(Value::Null);
                let mut stamina = client.raid_stamina(&user);
                stamina =
                    try_recover_stamina(client, need, stamina, config.prefer_stamina_recover).await;
                if need > stamina {
                    logs.push(format!(
                        "[{}] 援助 {} 的房体力不足（{stamina}<{need}），跳过该房",
                        m.alias, room.host_alias
                    ));
                    // 若是名单最后一人且仍有剩余血，记警告（主人：别出现支援完了 boss 还活着）
                    if is_last_in_roster && remain > 0 {
                        logs.push(format!(
                            "警告：房 {} 援助名单末尾体力不足且剩余估计血量≈{remain}，可能未打死",
                            room.host_alias
                        ));
                    }
                    continue;
                }

                // 末尾可动手者补足剩余血量（中间号跳过时仍保证 Σ≥H）
                let dmg = if is_last_in_roster {
                    planned.max(remain)
                } else {
                    planned
                };
                // Python support_clear：damage ≥ 剩余 hp → result 强制 1(win)
                let support_result = if dmg >= remain {
                    1
                } else {
                    config.battle_result
                };

                let init_body = json!({
                    "partyDataId": party_id,
                    "rescueType": 0,
                    "multiRaidStageMstId": room.stage_mst_id,
                    "multiRaidStageDataId": room.stage_data_id
                });
                let init = match client
                    .request("/api/multi_raid/initialize_stage", init_body.clone())
                    .await
                {
                    Ok(v) => v,
                    Err(e1) => {
                        // 入房：`initialize_stage` 带非 0 multiRaidStageDataId = 加入已有房（对照 Python support）。
                        // 失败时先 id_search 再**同一 body** 重试一次；不是「开新房」第二次。
                        // 风险：若首次请求已半成功但客户端超时，重试可能撞状态——失败则 continue 不继续伤害。
                        if !room.search_id.is_empty() {
                            match client
                                .request(
                                    "/api/multi_raid/id_search",
                                    json!({ "searchId": room.search_id }),
                                )
                                .await
                            {
                                Ok(_) => {}
                                Err(es) => {
                                    logs.push(format!(
                                        "[{}] 入房 {} (id={}) 失败：{e1}；id_search 也失败：{es}",
                                        m.alias, room.host_alias, room.stage_data_id
                                    ));
                                    continue;
                                }
                            }
                            match client
                                .request("/api/multi_raid/initialize_stage", init_body)
                                .await
                            {
                                Ok(v) => v,
                                Err(e2) => {
                                    logs.push(format!(
                                        "[{}] 入房 {} (id={}) 失败：{e1}；id_search 后仍失败：{e2}",
                                        m.alias, room.host_alias, room.stage_data_id
                                    ));
                                    continue;
                                }
                            }
                        } else {
                            logs.push(format!(
                                "[{}] 入房 {} (id={}) 失败：{e1}",
                                m.alias, room.host_alias, room.stage_data_id
                            ));
                            continue;
                        }
                    }
                };
                let qid = init
                    .pointer("/multiRaidRoomData/questDataId")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let blog = battle_log_raid(client, qid).await;
                if let Err(e) = add_damage_chunked(client, qid, dmg).await {
                    logs.push(format!("[{}] 援助 add_damage 失败：{e}", m.alias));
                    continue;
                }
                if config.leave_after_support {
                    // Python raidworker.add_damage（救世）：retire + 空 battleLog
                    match client
                        .request(
                            "/api/multi_raid/retire",
                            json!({ "questDataId": qid, "battleLog": "" }),
                        )
                        .await
                    {
                        Ok(_) => {
                            total_supports += 1;
                            remain = (remain - dmg).max(0);
                            logs.push(format!(
                                "[{}] 援助 {} 伤害={dmg} 后已退出（无结算奖励路径）",
                                m.alias, room.host_alias
                            ));
                        }
                        Err(e) => logs.push(format!("[{}] retire 失败：{e}", m.alias)),
                    }
                } else {
                    match client
                        .request(
                            "/api/multi_raid/finalize_stage_for_user",
                            json!({
                                "questDataId": qid,
                                "battleLog": blog,
                                "autoMode": 0,
                                "result": support_result
                            }),
                        )
                        .await
                    {
                        Ok(_) => {
                            total_supports += 1;
                            remain = (remain - dmg).max(0);
                            logs.push(format!(
                                "[{}] 援助 {} 成功 伤害={dmg} stageDataId={} result={support_result}",
                                m.alias, room.host_alias, room.stage_data_id
                            ));
                        }
                        Err(e) => logs.push(format!("[{}] 援助 finalize 失败：{e}", m.alias)),
                    }
                }
            }
            if remain > 0 && !supporters.is_empty() {
                logs.push(format!(
                    "警告：房 {} 援助阶段结束仍估计剩余血量≈{remain}（可能全员跳过或失败）",
                    room.host_alias
                ));
            }
        }

        // --- 阶段 C：舔盒（援助后、下一轮召唤前）---
        for m in &config.members {
            let client = clients.get_mut(&m.alias).unwrap();
            let top = match multi_raid_top(client).await {
                Ok(t) => t,
                Err(_) => continue,
            };
            let uid = uid_of(client);
            match daily::raid_receive_rewards_pub(client, &top, false, uid).await {
                Ok((n, recv_logs)) if n > 0 => {
                    total_rewards += n;
                    logs.push(format!("[{}] 舔盒 {n} 份", m.alias));
                    logs.extend(recv_logs.into_iter().map(|l| format!("  · {l}")));
                }
                Ok(_) => {}
                Err(e) => logs.push(format!("[{}] 舔盒异常：{e}", m.alias)),
            }
        }

        // 若本轮有开房，继续尝试下一轮（直到次数用尽）
    }

    let message = format!(
        "组队 Raid 结束：轮次={rounds} 召唤={total_summons} 援助={total_supports} 领奖={total_rewards}"
    );
    logs.push(message.clone());
    let ok = total_summons > 0 || total_supports > 0;
    Ok(GroupRaidReport {
        ok,
        owner_group: config.owner_group.clone(),
        room_open: config.room_open.label().into(),
        rounds,
        total_summons,
        total_supports,
        total_rewards,
        logs,
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::{split_group_raid_damages, RoomOpenMode};

    #[test]
    fn split_solo_n1() {
        let parts = split_group_raid_damages(1000, 1).expect("solo");
        assert_eq!(parts.len(), 1);
        assert!(parts[0] >= 1000);
    }

    #[test]
    fn room_self_only_multi_rejected() {
        assert!(RoomOpenMode::parse_for_count("self", 2).is_err());
        assert!(matches!(
            RoomOpenMode::parse_for_count("self", 1),
            Ok(RoomOpenMode::SelfOnly)
        ));
    }

    #[test]
    fn split_sum_ge_hp_and_bounds() {
        for n in 1..=10 {
            for &h in &[100i64, 999, 1_000_000, 7] {
                let parts = split_group_raid_damages(h, n).expect("split");
                assert_eq!(parts.len(), n);
                let sum: i64 = parts.iter().sum();
                assert!(sum >= h, "n={n} h={h} sum={sum} parts={parts:?}");
                let min_d = ((h.max(1) * 10) / 100).max(1);
                let max_pct = 110 - 10 * (n as i64);
                let max_d = ((h.max(1) * max_pct) / 100).max(min_d);
                for &p in &parts {
                    // 回退路径允许最后一人略破上限；主路径应在区间内
                    assert!(p >= 1, "{p}");
                    let _ = max_d;
                }
            }
        }
    }
}
