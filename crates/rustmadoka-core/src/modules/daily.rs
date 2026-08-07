//! 日常 26 模块实现 — 逻辑对照归档 Python `do_task`。
//!
//! # 职责
//! - 按 key 执行单模块：登录奖励、体力、刷图、团战舔盒/召唤/援助/点赞、各类扫荡、
//!   商店、剧情/收集、任务/礼物、info 等
//! - 读扁平 `config`（与原版 AccountData.config / config_catalog schema 一致）
//! - 队伍解析：`resolve_party`（名 → 序号 → id；见 L11 · PARTY_TEAM_RESOLVE）
//!
//! # 文档
//! - `docs/MODULES.md` · `docs/tech/PHASE_R2_MODULE_PARITY.md` · `docs/tech/MODULES_RUNTIME.md`
//! - 语义分类：`docs/tech/MODULE_SEMANTIC_CLASSIFICATION.md`
//! - W2/W3 清单与验收：`docs/tech/W2_WIRE_ANALYSIS_AND_REWRITE_LIST.md`
//! - 原版对照表：`docs/tech/AUTOMADOKA_RESEARCH_AND_RUST_GAP.md` §3.3
//! - 协议客户端：`crate::client::GameClient` · `docs/tech/SDK_AND_LOGIN.md`
//!
//! # 对照源码
//! `archive/.../modules/{common,stamina,tool,raid,sweep,shop,gacha,collection}.py`
//!
//! # 边界
//! - 有代码路径不表示真机 FIXED（P5）；细分支与 Python 差异以点测失败驱动 diff
//! - 团战「救世」小号路径在 tool 的 raid_support，本文件不含
//!
//! # 结果口径（OUT-EMPTY / OUT-PARTIAL · 2026-08-07）
//! - **无可领 / 无操作 / 无新内容** → `CoreError::Skip`（不抬成功计数）
//! - **多步至少完成一步、未满计划** → `Ok`，log 首行含「部分完成」与计划/完成计数
//! - **0 步完成** → Skip（可附过程 log）
//! - 展示标签仍只有成功/跳过/中止/错误；部分完成落在 log 文本（见 ERROR_DIAGNOSTICS §模块结果）

use crate::client::GameClient;
use crate::error::{CoreError, Result};
use chrono::{DateTime, Datelike, Duration, Local, Utc};
use rand::Rng;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

fn cfg_i64(config: &HashMap<String, Value>, key: &str, default: i64) -> i64 {
    config
        .get(key)
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(default)
}

fn cfg_str(config: &HashMap<String, Value>, key: &str, default: &str) -> String {
    config
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| config.get(key).and_then(|v| v.as_i64()).map(|n| n.to_string()))
        .unwrap_or_else(|| default.to_string())
}

fn cfg_bool(config: &HashMap<String, Value>, key: &str, default: bool) -> bool {
    config.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

fn j_i64(v: &Value, key: &str) -> i64 {
    v.get(key).and_then(|x| x.as_i64()).unwrap_or(0)
}
fn j_str(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}
fn j_bool(v: &Value, key: &str) -> bool {
    v.get(key).and_then(|x| x.as_bool()).unwrap_or(false)
}

fn parse_dt(s: &str) -> Option<DateTime<chrono::FixedOffset>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .or_else(|| DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f%z").ok())
        .or_else(|| DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%z").ok())
}

fn party_list(client: &GameClient) -> Vec<Value> {
    client
        .init_data
        .get("partyDataList")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

fn party_member_sum(p: &Value) -> i64 {
    (1..=5).map(|i| j_i64(p, &format!("member{i}"))).sum()
}

fn party_main(client: &GameClient) -> Result<(i64, String)> {
    for p in party_list(client) {
        if j_i64(&p, "partyType") == 1 && party_member_sum(&p) > 0 {
            return Ok((j_i64(&p, "partyDataId"), j_str(&p, "name")));
        }
    }
    Err(CoreError::Abort(
        "没有可用的主线队伍（partyType=1 且有成员）。请先在游戏内编队。".into(),
    ))
}

/// 解析「队伍码」配置为 partyDataId。
///
/// # 历史坑（母项目 + 本移植）
/// - 原版 `tool.py`/`raid.py`：**先 `int()`**，成功则当 **partyDataId**，**不再**按名称查。
///   默认填 `"20"` 易被理解成「第 20 号编成/序号」，实为 ID=20（往往不存在）。
/// - 原版名称匹配：`party.name == 配置` **全等**，首尾空格、全半角空格都会失败。
/// - 本仓库旧 Rust：数字 ID **找不到仍原样提交** → 服务端失败，日志难读。
///
/// # 解析顺序（改进）
/// 1. 去空白；空/`0` → 第一支有人的主线队  
/// 2. **名称全等**（trim 后）  
/// 3. **名称去空白后全等**（容忍中间不可见空白差异前先做 trim）  
/// 4. 纯数字 → 先 **partyIndex**（编成槽位序号），再 **partyDataId**  
/// 5. 名称包含匹配（唯一命中才采用）  
/// 失败时列出可用队伍：`名称 / 序号=partyIndex / id=partyDataId`
///
/// 对照: archive/.../modules/tool.py · raid.py · sweep.py  
/// 文档: docs/tech/PARTY_TEAM_RESOLVE.md  
/// 组队 Raid 等编排复用：`group_raid.rs`
pub fn resolve_party(client: &GameClient, team: &str) -> Result<(i64, String)> {
    let parties = party_list(client);
    let raw = team.trim();
    if raw.is_empty() || raw == "0" {
        return party_main(client);
    }

    // 1) 名称精确（trim）
    if let Some(p) = parties.iter().find(|p| j_str(p, "name").trim() == raw) {
        return Ok((j_i64(p, "partyDataId"), j_str(p, "name")));
    }

    // 2) 纯数字：partyIndex 优先，再 partyDataId（均须在列表中真实存在）
    if let Ok(n) = raw.parse::<i64>() {
        if n == 0 {
            return party_main(client);
        }
        if let Some(p) = parties.iter().find(|p| j_i64(p, "partyIndex") == n) {
            return Ok((j_i64(p, "partyDataId"), j_str(p, "name")));
        }
        if let Some(p) = parties.iter().find(|p| j_i64(p, "partyDataId") == n) {
            return Ok((n, j_str(p, "name")));
        }
        return Err(CoreError::Abort(format!(
            "未找到队伍「{raw}」（已按编成序号 partyIndex 与服务器 id partyDataId 查找）。\n{}",
            party_list_hint(&parties)
        )));
    }

    // 3) 名称包含（仅当唯一）
    let partial: Vec<&Value> = parties
        .iter()
        .filter(|p| {
            let n = j_str(p, "name");
            !n.is_empty() && (n.contains(raw) || raw.contains(n.trim()))
        })
        .collect();
    if partial.len() == 1 {
        let p = partial[0];
        return Ok((j_i64(p, "partyDataId"), j_str(p, "name")));
    }
    if partial.len() > 1 {
        return Err(CoreError::Abort(format!(
            "队伍名「{raw}」匹配到多支队伍，请改用完整名称、编成序号或 id。\n{}",
            party_list_hint(&parties)
        )));
    }

    Err(CoreError::Abort(format!(
        "未找到队伍「{raw}」。请填：游戏内队伍名称 / 编成序号(partyIndex) / 服务器id(partyDataId)。\n{}",
        party_list_hint(&parties)
    )))
}

fn party_list_hint(parties: &[Value]) -> String {
    if parties.is_empty() {
        return "当前账号队伍列表为空（登录后 init 未带 partyDataList？）。".into();
    }
    let mut lines: Vec<String> = parties
        .iter()
        .filter(|p| party_member_sum(p) > 0)
        .take(24)
        .map(|p| {
            format!(
                "· 「{}」 序号={} id={}",
                j_str(p, "name"),
                j_i64(p, "partyIndex"),
                j_i64(p, "partyDataId")
            )
        })
        .collect();
    if lines.is_empty() {
        return "有队伍记录但均无成员。".into();
    }
    if parties.len() > 24 {
        lines.push(format!("…共 {} 支（仅列有成员的前 24）", parties.len()));
    }
    format!("可用队伍示例：\n{}", lines.join("\n"))
}

fn party_pvp(client: &GameClient) -> Result<(i64, String)> {
    for p in party_list(client) {
        let sum: i64 = (1..=5).map(|i| j_i64(&p, &format!("member{i}"))).sum();
        if j_bool(&p, "isPvp") && sum > 0 {
            return Ok((j_i64(&p, "partyDataId"), j_str(&p, "name")));
        }
    }
    Err(CoreError::Skip("未找到 PVP 编成队伍（isPvp），已跳过自动对战".into()))
}

fn party_max_power(client: &GameClient) -> Result<(i64, String)> {
    let mut best: Option<(i64, String, i64)> = None;
    for p in party_list(client) {
        if j_i64(&p, "partyType") != 1 {
            continue;
        }
        let power = j_i64(&p, "partyPower");
        if best.as_ref().map(|b| power > b.2).unwrap_or(true) {
            best = Some((j_i64(&p, "partyDataId"), j_str(&p, "name"), power));
        }
    }
    best.map(|(id, name, _)| (id, name))
        .ok_or_else(|| CoreError::Abort("未找到可用编成队伍".into()))
}

fn in_window(start: &str, end: &str) -> bool {
    let now = Utc::now();
    match (parse_dt(start), parse_dt(end)) {
        (Some(s), Some(e)) => {
            let n = now.with_timezone(s.offset());
            n >= s && n <= e
        }
        _ => false,
    }
}

/// 供 `group_raid` 等模块复用活动时间窗判断
pub fn in_window_pub(start: &str, end: &str) -> bool {
    in_window(start, end)
}

/// 供组队 Raid 舔盒复用
pub async fn raid_receive_rewards_pub(
    client: &mut GameClient,
    top: &Value,
    self_only: bool,
    self_uid: i64,
) -> Result<(usize, Vec<String>)> {
    raid_receive_rewards(client, top, self_only, self_uid).await
}

/// 供组队 Raid 战斗日志复用
pub async fn battle_log_raid_pub(client: &mut GameClient, qid: i64) -> String {
    battle_log_raid(client, qid).await
}

// --- common.py ---

/// 从 `get_home_info` 回包判定登录奖励是否本请求真正下发。
///
/// # 游戏语义（wire 对照 · known-issues 2026-08-07）
/// - `loginBonusDataList` **非空**：本次 `skipLoginBonus=false` 触发了领取，列表项含
///   `loginBonusMstId` / `loginBonusRewardMstId` / `dayCount` 等 → **成功**并摘要。
/// - **空数组**：今日已无可领或不存在 → **跳过**（禁止再报「已领取」假成功）。
/// - **字段缺失**：协议形态未知 → **跳过**并说明，禁止无条件成功（P25 · C20）。
///
/// 文档：`docs/logs/2026-08-07-known-issues-before-fix.md` · ERROR_DIAGNOSTICS §模块结果
pub fn loginbonus_outcome_from_home(home: &Value) -> Result<String> {
    let Some(list_v) = home.get("loginBonusDataList") else {
        return Err(CoreError::Skip(
            "登录奖励：回包无 loginBonusDataList，无法确认是否领取，已跳过".into(),
        ));
    };
    let Some(arr) = list_v.as_array() else {
        return Err(CoreError::Skip(
            "登录奖励：loginBonusDataList 不是数组，已跳过".into(),
        ));
    };
    if arr.is_empty() {
        return Err(CoreError::Skip(
            "今日登录奖励已领完或不存在，已跳过".into(),
        ));
    }
    let mut parts = Vec::new();
    for b in arr.iter().take(8) {
        let mid = j_i64(b, "loginBonusMstId");
        let rid = j_i64(b, "loginBonusRewardMstId");
        let day = j_i64(b, "dayCount");
        parts.push(format!("表={mid}/奖励={rid}/第{day}天"));
    }
    let extra = if arr.len() > 8 {
        format!(" …共{}项", arr.len())
    } else {
        String::new()
    };
    Ok(format!(
        "已领取登录奖励（{}项）：{}{}",
        arr.len(),
        parts.join("；"),
        extra
    ))
}

/// 领取登陆奖励。请求 `get_home_info`（`skipLoginBonus=false`），按回包列表判定成功/跳过。
pub async fn loginbonus(client: &mut GameClient) -> Result<String> {
    let home = client
        .request("/api/home/get_home_info", json!({ "skipLoginBonus": false }))
        .await?;
    loginbonus_outcome_from_home(&home)
}

pub async fn info(client: &mut GameClient) -> Result<String> {
    // refresh 失败不阻断（轻量登录后避免二次请求把会话卡死）
    if let Err(e) = client.refresh_user_param().await {
        tracing::warn!(error = %e, "info: refresh_user_param skipped");
    }
    let (name, level) = client.user_name_level();
    let power = client
        .init_data
        .pointer("/userParamData/maxPartyPower")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let money = client
        .init_data
        .pointer("/userParamData/money")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    Ok(format!(
        "name={name} level={level}\nmaxPower={power} AQ={money}\nstamina={}",
        client.stamina()
    ))
}

// --- stamina.py ---

pub async fn stamina_buy(
    client: &mut GameClient,
    config: &HashMap<String, Value>,
) -> Result<String> {
    let retain = cfg_i64(config, "stamina_retain_count", 120);
    let mut buy_count = cfg_i64(config, "stamina_buy_count", 1);
    let items = client
        .init_data
        .get("itemDataList")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let item_cnt: i64 = items
        .iter()
        .filter(|i| j_i64(i, "itemMstId") == 202001)
        .map(|i| j_i64(i, "num"))
        .sum();
    if item_cnt <= retain {
        return Err(CoreError::Skip(format!(
            "体力石不足（当前{item_cnt}，保留{retain}），已跳过购买"
        )));
    }
    let recovery = client
        .init_data
        .pointer("/userParamData/recoveryCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    buy_count -= recovery;
    if buy_count <= 0 {
        return Err(CoreError::Skip("今日体力购买次数已达上限，已跳过".into()));
    }
    client
        .request(
            "/api/user/set_stamina_recover",
            json!({ "recoverType": 1, "itemMstId": 202001, "num": buy_count }),
        )
        .await?;
    let _ = client.refresh_user_param().await;
    Ok(format!(
        "bought stamina x{buy_count}, now {}",
        client.stamina()
    ))
}

// --- tool.py super_sweep ---

pub async fn super_sweep(
    client: &mut GameClient,
    config: &HashMap<String, Value>,
) -> Result<String> {
    super_sweep_with_progress(client, config, &None).await
}

/// 快速刷图 + 逐轮进度（C7 · L6：长任务须实时反馈，禁止只在结束整包冒出）。
/// Docs: docs/MODULES.md · docs/tech/UI_ROUTING_AND_TASK_LOGS.md · archive/.../modules/tool.py
pub async fn super_sweep_with_progress(
    client: &mut GameClient,
    config: &HashMap<String, Value>,
    progress: &super::ProgressTx,
) -> Result<String> {
    use super::progress::{emit, ProgressEvent};

    // 关卡默认对齐 tool.py；队伍默认空（产品安全：禁止静默用神秘 id 20）
    crate::wire::set_module_key(Some("super_sweep"));
    crate::wire::record_probe("super_sweep_enter", serde_json::json!({}));
    let quest_id = cfg_i64(config, "force_battle_quest_id", 411105);
    let team_s = cfg_str(config, "force_battle_team", "");
    let mut repeat = cfg_i64(config, "force_battle_repeat_times", 1);
    let auto_mode = cfg_i64(config, "force_battle_auto_mode", 0);
    if team_s.trim().is_empty() {
        crate::wire::record_probe(
            "super_sweep_skip",
            serde_json::json!({ "reason": "empty_team" }),
        );
        return Err(CoreError::Skip(
            "未配置快速刷图队伍（force_battle_team）：请填队伍名称、编成序号或服务器 id".into(),
        ));
    }
    let (team, team_name) = resolve_party(client, &team_s)?;
    let stages = client.mst_list("/api/mst/get_quest_stage_mst_list").await?;
    let stage_row = stages
        .iter()
        .find(|s| j_i64(s, "questStageMstId") == quest_id);
    if stage_row.is_none() {
        return Err(CoreError::Skip(format!(
            "快速刷图：关卡 ID={quest_id} 在 mst 关卡表中不存在。请用 CLI「mst quest-stages --filter …」或「mst quest-lookup --id {quest_id}」查正确 ID/名称"
        )));
    }
    let stage_label = crate::mst::format_quest_stage_label(&stages, quest_id);
    let once_cost = stage_row
        .map(|s| j_i64(s, "useStamina") / 2)
        .filter(|&c| c > 0)
        .unwrap_or(10);
    let stamina = client.stamina();
    if once_cost > 0 && repeat * once_cost > stamina {
        repeat = stamina / once_cost;
    }
    if repeat <= 0 {
        return Err(CoreError::Skip(format!(
            "体力不足（当前{stamina}，单次约{once_cost}），已跳过快速刷图"
        )));
    }
    // OUT-PARTIAL：至少 1 轮 finalize 才 Ok；未满计划时 log 标明部分完成（W3 R1 · W2）
    let planned = repeat;
    let mut log = vec![format!(
        "快速刷图 关卡={stage_label} 队伍={team}({team_name}) 计划次数={planned} 单次耗体约={once_cost}"
    )];
    emit(
        progress,
        ProgressEvent::info(
            "module",
            "super_sweep",
            "快速刷图",
            format!("准备开始：关卡={stage_label} 队伍={team_name} 计划={planned}轮 体力={stamina}"),
        ),
    );
    let mut ok_rounds = 0i64;
    for i in 0..planned {
        let round = i + 1;
        emit(
            progress,
            ProgressEvent::running(
                "module",
                "super_sweep",
                "快速刷图",
                round,
                planned,
                format!("第{round}/{planned}轮：开局中…"),
            ),
        );
        crate::wire::record_probe(
            "super_sweep_round_begin",
            serde_json::json!({ "round": round, "planned": planned }),
        );
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let init = match client
            .request(
                "/api/quest_battle/initialize_stage",
                json!({
                    "questStageMstId": quest_id,
                    "partyDataId": team,
                    "repeatNum": 0,
                    "backGroundPlay": false,
                    "isArchiveEvent": false,
                    "selectionAbilityMultiLotteryItemNum": 0
                }),
            )
            .await
        {
            Ok(v) => v,
            Err(CoreError::Skip(m)) => {
                log.push(format!("第{}轮：{}", round, m));
                emit(
                    progress,
                    ProgressEvent::module_done(
                        "module",
                        "super_sweep",
                        "快速刷图",
                        round,
                        planned,
                        "skip",
                        m,
                    ),
                );
                break;
            }
            Err(e) => {
                log.push(format!("第{}轮开局失败：{}", round, e));
                emit(
                    progress,
                    ProgressEvent::module_done(
                        "module",
                        "super_sweep",
                        "快速刷图",
                        round,
                        planned,
                        "error",
                        e.to_string(),
                    ),
                );
                break;
            }
        };
        client.apply_stamina_delta(-once_cost);
        emit(
            progress,
            ProgressEvent::running(
                "module",
                "super_sweep",
                "快速刷图",
                round,
                planned,
                format!("第{round}/{planned}轮：结算中…"),
            ),
        );
        let qid = init
            .pointer("/questRoomData/questDataId")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let battle_log = battle_log_quest(client, qid).await;
        if let Err(e) = client
            .request(
                "/api/quest_battle/finalize_stage_for_user",
                json!({ "battleLog": battle_log, "autoMode": auto_mode, "result": 1 }),
            )
            .await
        {
            log.push(format!("第{}轮结算失败：{}", round, e));
            emit(
                progress,
                ProgressEvent::module_done(
                    "module",
                    "super_sweep",
                    "快速刷图",
                    round,
                    planned,
                    "error",
                    e.to_string(),
                ),
            );
            break;
        }
        ok_rounds += 1;
        let line = format!("第{ok_rounds}/{planned}轮完成");
        log.push(line.clone());
        emit(
            progress,
            ProgressEvent::module_done(
                "module",
                "super_sweep",
                "快速刷图",
                ok_rounds,
                planned,
                "success",
                line,
            ),
        );
        crate::wire::record_probe(
            "super_sweep_round_ok",
            serde_json::json!({ "ok_rounds": ok_rounds, "planned": planned }),
        );
    }
    if ok_rounds == 0 {
        return Err(CoreError::Skip(log.join("\n")));
    }
    if ok_rounds < planned {
        log.insert(
            0,
            format!("【部分完成】快速刷图完成 {ok_rounds}/{planned} 轮（未满计划，详见下方）"),
        );
    } else {
        log.insert(0, format!("【完成】快速刷图 {ok_rounds}/{planned} 轮"));
    }
    let summary = log[0].clone();
    emit(
        progress,
        ProgressEvent::finished("module", true, summary),
    );
    Ok(log.join("\n"))
}

async fn battle_log_quest(client: &mut GameClient, qid: i64) -> String {
    if qid <= 0 {
        return GameClient::simple_battle_log().to_string();
    }
    match client
        .request("/api/quest_battle/get_quest_info", json!({ "questDataId": qid }))
        .await
    {
        Ok(info) => {
            let units = info
                .get("allyBattleUnitList")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            GameClient::battle_log_from_units(&units)
        }
        Err(_) => GameClient::simple_battle_log().to_string(),
    }
}

async fn battle_log_raid(client: &mut GameClient, qid: i64) -> String {
    if qid <= 0 {
        return GameClient::simple_battle_log().to_string();
    }
    match client
        .request(
            "/api/multi_raid/get_multi_raid_info",
            json!({ "questDataId": qid }),
        )
        .await
    {
        Ok(info) => {
            let units = info
                .get("allyBattleUnitList")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            GameClient::battle_log_from_units(&units)
        }
        Err(_) => GameClient::simple_battle_log().to_string(),
    }
}

// --- raid.py ---

async fn multi_raid_top(client: &mut GameClient) -> Result<Value> {
    client.request("/api/multi_raid/get_top", json!({})).await
}

async fn raid_receive_rewards(
    client: &mut GameClient,
    top: &Value,
    self_only: bool,
    self_uid: i64,
) -> Result<(usize, Vec<String>)> {
    let stage_map: HashMap<i64, &Value> = top
        .get("multiRaidStageDataList")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|r| {
                    let id = j_i64(r, "multiRaidStageDataId");
                    (id != 0).then_some((id, r))
                })
                .collect()
        })
        .unwrap_or_default();
    let mut n = 0usize;
    let mut logs = Vec::new();
    let rooms = top
        .get("multiRaidRoomDataList")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for raid in rooms {
        let sid = j_i64(&raid, "multiRaidStageDataId");
        let Some(stage) = stage_map.get(&sid) else {
            continue;
        };
        if !j_bool(stage, "isClosed") {
            continue;
        }
        if self_only && j_i64(stage, "hostUserId") != self_uid {
            continue;
        }
        let qid = j_i64(&raid, "questDataId");
        if qid == 0 {
            continue;
        }
        match client
            .request("/api/multi_raid/receive_reward", json!({ "questDataId": qid }))
            .await
        {
            Ok(_) => {
                n += 1;
                logs.push(format!("已领取团战奖励 stageDataId={sid}"));
            }
            Err(e) => logs.push(format!("领取 stageDataId={sid} 失败：{e}")),
        }
    }
    if n > 0 {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
    Ok((n, logs))
}

/// 团战舔盒：领取已关闭房间奖励。
/// - **成功：** 至少领取 1 份  
/// - **跳过：** 当前无可领（与 present 空箱一致；勿标成功以免汇总虚高）  
/// - **失败：** 拉 top / 请求异常且未映射为 Skip
pub async fn raid_reward(
    client: &mut GameClient,
    config: &HashMap<String, Value>,
) -> Result<String> {
    let top = multi_raid_top(client).await?;
    let self_only = cfg_bool(config, "raid_reward_self_only", false);
    let uid = client
        .init_data
        .pointer("/userParamData/userId")
        .and_then(|v| v.as_i64())
        .unwrap_or(client.user_id);
    let (n, logs) = raid_receive_rewards(client, &top, self_only, uid).await?;
    if n == 0 {
        return Err(CoreError::Skip("团战奖励：当前无可领取，已跳过".into()));
    }
    Ok(format!("团战奖励领取 {n} 份\n{}", logs.join("\n")))
}

/// multi_raid 开房/入房 → 伤害 → finalize。
/// 对照 Python `raidworker.start_clear` / `support_clear`：
/// - 字段：initialize(partyDataId, rescueType, multiRaidStageMstId, multiRaidStageDataId)
/// - get_multi_raid_info 生成 battleLog；add_damage；finalize(autoMode=0, result)
/// - 援助时若 damage≥剩余 hp 则 result 强制 1（win）
/// - 大额伤害分片（与 raidworker DAMAGE_ONCE=10_000_000 一致）
async fn raid_clear(
    client: &mut GameClient,
    stage_mst_id: i64,
    stage_data_id: i64,
    party_id: i64,
    damage: i64,
    result: i64,
    // 入房时房间剩余 hp；Some 且 damage≥hp 则 result=1
    remaining_hp: Option<i64>,
) -> Result<Value> {
    let init = client
        .request(
            "/api/multi_raid/initialize_stage",
            json!({
                "partyDataId": party_id,
                "rescueType": if stage_data_id == 0 { 1 } else { 0 },
                "multiRaidStageMstId": stage_mst_id,
                "multiRaidStageDataId": stage_data_id
            }),
        )
        .await?;
    let qid = init
        .pointer("/multiRaidRoomData/questDataId")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if qid <= 0 {
        return Err(CoreError::other(
            "multi_raid initialize 未返回 questDataId，无法上报伤害",
        ));
    }
    // 顺序对齐 Python raidworker：get_multi_raid_info → battleLog → add_damage → finalize
    let battle_log = battle_log_raid(client, qid).await;
    // 分片：原版救世 DAMAGE_ONCE=10_000_000（实现细节，非主人产品百分比公式）
    const DAMAGE_ONCE: i64 = 10_000_000;
    let mut left = damage.max(0);
    if left == 0 {
        // 0 伤仍 finalize 易成空战斗；至少报 1（避免无意义空包）
        left = 1;
    }
    while left > 0 {
        let d = left.min(DAMAGE_ONCE);
        left -= d;
        client
            .request(
                "/api/multi_raid/add_damage",
                json!({ "questDataId": qid, "damage": d }),
            )
            .await?;
    }
    let final_result = match remaining_hp {
        Some(hp) if damage.max(1) >= hp && hp > 0 => 1,
        _ => result,
    };
    client
        .request(
            "/api/multi_raid/finalize_stage_for_user",
            json!({
                "questDataId": qid,
                "battleLog": battle_log,
                "autoMode": 0,
                "result": final_result
            }),
        )
        .await
}

/// 魔女召唤（self_raid · multi_raid 自己开房）。
///
/// - **成功：** initialize → add_damage → finalize 完成发车  
/// - **跳过：** 未配队、已有未结束房、日 cap、无赛季、体力不足、队伍 isMultiRaid=false、业务码 19001  
/// - **失败：** 签名/会话等真异常  
/// 文档：W2 §3.1 · W3 R9（19001 根因可再证；先明确 Skip）
pub async fn self_raid(
    client: &mut GameClient,
    config: &HashMap<String, Value>,
) -> Result<String> {
    let top = multi_raid_top(client).await?;
    let dmg_min = cfg_i64(config, "start_raid_damage_min", 900_000);
    let dmg_max = cfg_i64(config, "start_raid_damage_max", 1_100_000);
    let damage = rand::thread_rng().gen_range(dmg_min..=dmg_max.max(dmg_min));
    let result = cfg_i64(config, "start_raid_result", 3);
    // 默认空：未配置队伍则跳过，禁止静默用主队发车
    let team_s = cfg_str(config, "start_raid_party", "");
    if team_s.trim().is_empty() {
        return Err(CoreError::Skip(
            "未配置魔女召唤队伍（start_raid_party），已跳过".into(),
        ));
    }
    let receive = cfg_bool(config, "start_raid_receive", true);
    let (party_id, party_name) = resolve_party(client, &team_s)?;
    // R9：本号 wire 在 isMultiRaid=false 时 initialize 曾 19001；先给出可操作的 Skip
    if let Some(p) = party_list(client)
        .into_iter()
        .find(|p| j_i64(p, "partyDataId") == party_id)
    {
        if !j_bool(&p, "isMultiRaid") {
            return Err(CoreError::Skip(format!(
                "队伍「{party_name}」(id={party_id}) 未标记为团战编成（isMultiRaid=false），已跳过发车。请在游戏内使用链接 Raid 用编成"
            )));
        }
    }
    let uid = client
        .init_data
        .pointer("/userParamData/userId")
        .and_then(|v| v.as_i64())
        .unwrap_or(client.user_id);

    if let Some(stages) = top.get("multiRaidStageDataList").and_then(|v| v.as_array()) {
        for raid in stages {
            if j_i64(raid, "hostUserId") == uid {
                if j_bool(raid, "isClosed") && receive {
                    let _ = raid_receive_rewards(client, &top, true, uid).await;
                } else if !j_bool(raid, "isClosed") {
                    return Err(CoreError::Skip("已有未结束的自开团战，无法再次发车，已跳过".into()));
                }
            }
        }
    }
    // 缺 config 时默认 6（与 game_config 样本 / group_raid 一致；禁止写死业务日 cap 当唯一真理）
    let max_day = client
        .game_config
        .pointer("/multiRaidConfig/maxPlayCountPerDay")
        .and_then(|v| v.as_i64())
        .unwrap_or(6);
    let today = top
        .pointer("/multiRaidUserSeasonData/todayClearedCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if today >= max_day {
        return Err(CoreError::Skip(format!("今日团战发车次数已达上限（{today}/{max_day}），已跳过")));
    }
    let raids = client
        .mst_list("/api/mst/get_multi_raid_mst_list")
        .await
        .unwrap_or_default();
    let opening = raids
        .iter()
        .find(|x| in_window(&j_str(x, "startTime"), &j_str(x, "endTime")));
    let Some(opening) = opening else {
        return Err(CoreError::Skip("当前无开放的团战赛季，已跳过".into()));
    };
    let season_id = j_i64(opening, "seasonId");
    let cleared = top
        .pointer("/multiRaidUserSeasonData/clearedDifficulty")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let raid_id = (20i64).min(1 + cleared) + season_id * 100;
    let stages = client
        .mst_list("/api/mst/get_multi_raid_stage_mst_list")
        .await?;
    let record = stages
        .iter()
        .find(|x| j_i64(x, "multiRaidStageMstId") == raid_id)
        .ok_or_else(|| CoreError::Skip(format!("找不到团战关卡 stage={raid_id}，已跳过")))?;
    let need = j_i64(record, "useStaminaForPlay");
    let user = top.get("multiRaidUserData").cloned().unwrap_or(Value::Null);
    let mut stamina = client.raid_stamina(&user);
    if need > stamina {
        let recover_count = cfg_i64(config, "raid_recovery_count", 0);
        if recover_count > 0 {
            let num = (need - stamina + 19) / 20;
            if client
                .request(
                    "/api/multi_raid/recover_stamina",
                    json!({ "num": num, "itemMstId": 290001 }),
                )
                .await
                .is_ok()
            {
                stamina += num * 20;
            }
        }
    }
    if need > stamina {
        return Err(CoreError::Skip(format!("团战体力不足（当前{stamina}/需要{need}），已跳过")));
    }
    let _ = raid_clear(client, raid_id, 0, party_id, damage, result, None).await?;
    Ok(format!(
        "已发车团战 stage={raid_id} 伤害={damage} 队伍={party_name}"
    ))
}

pub async fn support_raid(
    client: &mut GameClient,
    config: &HashMap<String, Value>,
) -> Result<String> {
    let top = multi_raid_top(client).await?;
    let raid_ids: HashSet<i64> = cfg_str(config, "support_raid_id", "120")
        .split(',')
        .filter_map(|s| s.trim().parse::<i64>().ok())
        .map(|x| x % 100)
        .collect();
    let dmg_min = cfg_i64(config, "support_raid_damage_min", 900_000);
    let dmg_max = cfg_i64(config, "support_raid_damage_max", 1_100_000);
    let damage = rand::thread_rng().gen_range(dmg_min..=dmg_max.max(dmg_min));
    let result = cfg_i64(config, "support_raid_result", 3);
    // 默认空：未配置队伍则跳过，禁止静默用主队援助
    let team_s = cfg_str(config, "support_raid_party", "");
    if team_s.trim().is_empty() {
        return Err(CoreError::Skip(
            "未配置魔女援助队伍（support_raid_party），已跳过".into(),
        ));
    }
    let time_max = cfg_i64(config, "support_raid_time_max", 10);
    let max_num = cfg_i64(config, "support_raid_max", 2);
    let search_times = cfg_i64(config, "support_search_times", 0);
    let support_guild = cfg_bool(config, "support_guild", true);
    let (party_id, _) = resolve_party(client, &team_s)?;
    let uid = client
        .init_data
        .pointer("/userParamData/userId")
        .and_then(|v| v.as_i64())
        .unwrap_or(client.user_id);
    let max_join = client
        .game_config
        .pointer("/multiRaidConfig/maxJoinUserCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(4);
    if search_times == 0 && !support_guild {
        return Err(CoreError::Skip("未开启公会援助且搜索次数为 0，已跳过魔女援助".into()));
    }
    let threshold = Utc::now() - Duration::minutes(time_max);
    let mut candidates: Vec<Value> = Vec::new();
    let mut room_users: HashMap<i64, Vec<i64>> = HashMap::new();

    if support_guild {
        if let Some(list) = top.get("multiRaidStageDataList").and_then(|v| v.as_array()) {
            for raid in list {
                if j_bool(raid, "isClosed") {
                    continue;
                }
                let sid = j_i64(raid, "multiRaidStageDataId");
                candidates.push(raid.clone());
                let users: Vec<i64> = top
                    .get("multiRaidRoomDataList")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter(|r| j_i64(r, "multiRaidStageDataId") == sid)
                            .map(|r| j_i64(r, "userId"))
                            .collect()
                    })
                    .unwrap_or_default();
                room_users.insert(sid, users);
            }
        }
    }
    for _ in 0..search_times {
        let search = client
            .request(
                "/api/multi_raid/get_multi_raid_stage_data_list",
                json!({ "isRescue": true }),
            )
            .await;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        if let Ok(s) = search {
            if let Some(list) = s.get("multiRaidStageDataList").and_then(|v| v.as_array()) {
                for raid in list {
                    if j_bool(raid, "isClosed") {
                        continue;
                    }
                    let sid = j_i64(raid, "multiRaidStageDataId");
                    if candidates.iter().any(|c| j_i64(c, "multiRaidStageDataId") == sid) {
                        continue;
                    }
                    candidates.push(raid.clone());
                    let users: Vec<i64> = s
                        .get("multiRaidRoomDataList")
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter(|r| j_i64(r, "multiRaidStageDataId") == sid)
                                .map(|r| j_i64(r, "userId"))
                                .collect()
                        })
                        .unwrap_or_default();
                    room_users.insert(sid, users);
                }
            }
        }
    }

    let stages_mst = client
        .mst_list("/api/mst/get_multi_raid_stage_mst_list")
        .await
        .unwrap_or_default();
    let user = top.get("multiRaidUserData").cloned().unwrap_or(Value::Null);
    let mut stamina = client.raid_stamina(&user);
    let mut logs = Vec::new();
    let mut helped = 0;
    for raid in candidates {
        let sid = j_i64(&raid, "multiRaidStageDataId");
        let mst_id = j_i64(&raid, "multiRaidStageMstId");
        let users = room_users.get(&sid).cloned().unwrap_or_default();
        if users.len() as i64 > max_num || users.len() as i64 >= max_join {
            continue;
        }
        if users.contains(&uid) {
            continue;
        }
        if !raid_ids.is_empty() && !raid_ids.contains(&(mst_id % 100)) {
            continue;
        }
        if let Some(created) = parse_dt(&j_str(&raid, "createdTime")) {
            if created.with_timezone(&Utc) < threshold {
                continue;
            }
        }
        let need = stages_mst
            .iter()
            .find(|x| j_i64(x, "multiRaidStageMstId") == mst_id)
            .map(|r| j_i64(r, "useStaminaForRescue"))
            .unwrap_or(10);
        if need > stamina {
            logs.push(format!("LP low {stamina}<{need}"));
            break;
        }
        let boss_hp = j_i64(&raid, "hp");
        match raid_clear(
            client,
            mst_id,
            sid,
            party_id,
            damage,
            result,
            Some(boss_hp),
        )
        .await
        {
            Ok(_) => {
                stamina -= need;
                helped += 1;
                logs.push(format!("supported {sid} mst={mst_id}"));
            }
            Err(e) => logs.push(format!("support {sid} fail: {e}")),
        }
    }
    if helped == 0 {
        if logs.is_empty() {
            return Err(CoreError::Skip("当前没有可援助的团战房间，已跳过".into()));
        }
        return Err(CoreError::Skip(format!(
            "魔女援助：未成功支援任何房间\n{}",
            logs.join("\n")
        )));
    }
    Ok(format!("魔女援助成功 {helped} 次\n{}", logs.join("\n")))
}

pub async fn like_raid(
    client: &mut GameClient,
    config: &HashMap<String, Value>,
) -> Result<String> {
    let times = cfg_i64(config, "search_times", 10);
    let max_medal = client
        .game_config
        .pointer("/friendConfig/gainTodayFriendMedalMaxNum")
        .and_then(|v| v.as_i64())
        .unwrap_or(50);
    let mut today = client
        .init_data
        .pointer("/userParamData/todayFriendMedalCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if today >= max_medal {
        return Err(CoreError::Skip("今日友情勋章已满，已跳过点赞".into()));
    }
    let uid = client
        .init_data
        .pointer("/userParamData/userId")
        .and_then(|v| v.as_i64())
        .unwrap_or(client.user_id);
    let mut liked: HashSet<(i64, i64)> = HashSet::new();
    let mut n = 0;
    let mut logs = Vec::new();
    for _ in 0..times {
        if today >= max_medal {
            break;
        }
        let search = client
            .request(
                "/api/multi_raid/get_multi_raid_stage_data_list",
                json!({ "isRescue": true }),
            )
            .await?;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let users = search
            .get("joinUserInfoList")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for user in users {
            let tid = j_i64(&user, "userId");
            let sid = j_i64(&user, "multiRaidStageDataId");
            if tid == uid || !liked.insert((tid, sid)) {
                continue;
            }
            if let Ok(res) = client
                .request(
                    "/api/like/exec_like",
                    json!({ "targetUserId": tid, "value": sid }),
                )
                .await
            {
                if j_bool(&res, "result") {
                    n += 1;
                    today += 1;
                    logs.push(format!(
                        "已点赞 {}（友情勋章 {today}/{max_medal}）",
                        j_str(&user, "userName")
                    ));
                }
                if (j_bool(&res, "result") && !j_bool(&res, "isFriendMedalAcquired"))
                    || today >= max_medal
                {
                    break;
                }
            }
        }
    }
    if n == 0 {
        return Err(CoreError::Skip("魔女点赞：本轮未点到任何人，已跳过".into()));
    }
    Ok(format!("魔女点赞 {n} 次\n{}", logs.join("\n")))
}

// --- sweep.py ---

/// 扫荡总力战（① 领取类 skip API）。
///
/// - 成功：skip_quest_battle 接受  
/// - 跳过：无活动窗、今日次数用尽、clearedDifficulty 不足、业务码 18054  
/// - 失败：签名/会话等真异常  
/// 文档：W2_WIRE_ANALYSIS §3.2 · W3 R2
pub async fn solo_raid(client: &mut GameClient) -> Result<String> {
    let mst = client
        .mst_list("/api/mst/get_solo_raid_mst_list")
        .await
        .unwrap_or_default();
    let open = mst
        .iter()
        .any(|x| in_window(&j_str(x, "startTime"), &j_str(x, "battleEndTime")));
    if !open {
        return Err(CoreError::Skip("当前无开放的总力战活动，已跳过".into()));
    }
    let top = client.request("/api/solo_raid/get_top", json!({})).await?;
    let cleared = top
        .pointer("/soloRaidUserData/clearedDifficulty")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if cleared <= 0 {
        return Err(CoreError::Skip(
            "总力战尚未通关可扫难度（clearedDifficulty=0），已跳过扫荡".into(),
        ));
    }
    let max_play = client
        .game_config
        .pointer("/soloRaidConfig/maxPlayCountPerDay")
        .and_then(|v| v.as_i64())
        .unwrap_or(3);
    let today = top
        .pointer("/soloRaidUserData/todayPlayCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let times = max_play - today;
    if times <= 0 {
        return Err(CoreError::Skip(format!(
            "总力战今日扫荡次数已用尽（{today}/{max_play}），已跳过"
        )));
    }
    client
        .request("/api/solo_raid/skip_quest_battle", json!({ "repeatNum": times }))
        .await?;
    Ok(format!("总力战扫荡 {times} 次"))
}

pub async fn high_score(client: &mut GameClient) -> Result<String> {
    let mst = client
        .mst_list("/api/mst/get_score_attack_mst_list")
        .await
        .unwrap_or_default();
    let reset = client
        .game_config
        .pointer("/scoreAttackConfig/resetScoreAttackSkipNum")
        .and_then(|v| v.as_i64())
        .unwrap_or(3);
    let mut logs = Vec::new();
    for hs in mst {
        if !in_window(&j_str(&hs, "startTime"), &j_str(&hs, "endTime")) {
            continue;
        }
        let id = j_i64(&hs, "scoreAttackMstId");
        let top = client
            .request(
                "/api/score_attack/get_score_attack_top",
                json!({ "scoreAttackMstId": id }),
            )
            .await?;
        let skip_num = top
            .pointer("/userScoreAttackData/skipNum")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let times = reset - skip_num;
        if times <= 0 {
            continue;
        }
        client
            .request(
                "/api/score_attack/skip_quest_battle",
                json!({ "scoreAttackMstId": id, "repeatNum": times }),
            )
            .await?;
        logs.push(format!("打分扫荡 {} ×{times}", j_str(&hs, "name")));
    }
    if logs.is_empty() {
        return Err(CoreError::Skip("当前无开放的打分活动，已跳过".into()));
    }
    Ok(logs.join("\n"))
}

pub async fn arena(client: &mut GameClient) -> Result<String> {
    // 对照 Python: PvpApiGetPvpTopRequest → /api/pvp/get_pvp_top（非 get_top）
    // 证据：JP wire 20260807T182637 get_top → HTTP 404；archive requests.py
    let (party_id, party_name) = party_pvp(client)?;
    let top = client.request("/api/pvp/get_pvp_top", json!({})).await?;
    let free = top
        .pointer("/pvpTopInfo/remainTodayFreePlayCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if free <= 0 {
        return Err(CoreError::Skip("今日免费 PVP 次数已用尽，已跳过".into()));
    }
    let mut action_logs: Vec<String> = Vec::new();
    for _ in 0..free {
        let cand = client
            .request("/api/pvp/get_candidate_enemy_user_list", json!({}))
            .await?;
        let enemy = cand
            .pointer("/candidateEnemyUserInfoList/0/userId")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        if enemy == 0 {
            break;
        }
        let init = client
            .request(
                "/api/pvp/initialize_stage",
                json!({
                    "chooseEnemyUserId": enemy,
                    "partyDataId": party_id,
                    "isConsumeGem": false
                }),
            )
            .await;
        let room = match init {
            Ok(v) => v.get("roomId").and_then(|x| x.as_i64()).unwrap_or(0),
            Err(e) => {
                action_logs.push(format!("初始化失败 enemy={enemy}：{e}"));
                break;
            }
        };
        if client
            .request(
                "/api/pvp/finalize_stage_for_user",
                json!({
                    "roomId": room,
                    "result": 2,
                    "battleLog": GameClient::simple_battle_log(),
                    "autoMode": 2
                }),
            )
            .await
            .is_err()
        {
            let _ = client
                .request(
                    "/api/pvp/retire",
                    json!({
                        "battleLog": "",
                        "isSystemRetire": true,
                        "isUpdateRetire": false
                    }),
                )
                .await;
        }
        action_logs.push(format!("已投降 对手用户={enemy}"));
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
    let done = action_logs
        .iter()
        .filter(|s| s.starts_with("已投降"))
        .count();
    if done == 0 {
        return Err(CoreError::Skip(format!(
            "PVP：有免费次数={free} 但本轮未完成任何投降（队伍={party_name}）\n{}",
            action_logs.join("\n")
        )));
    }
    let mut out = vec![format!(
        "PVP 投降完成 免费次数={free} 队伍={party_name} 成功={done}"
    )];
    out.extend(action_logs);
    Ok(out.join("\n"))
}

const ONCE_STAMINA_COST: i64 = 10;

/// 训练组「可 skip 优先度」（越大越优先选为智能体力扫荡目标）。
///
/// 对照：
/// - 原版 `stamina.py`：只 `skip_quest_battle`，不真打。
/// - 游戏 wiki（exedra Upgrade Quests）：**已通关的 Kioku / Magic** 强化本可 skip；能力晶花(Crystalis) 未写同等规则。
/// - mst：101=キオク强化素材；201–2xx=魔力解放[属性]；401–405=能力晶花[属性]（411101 Easy / 411102 Normal…）。
/// - 主人：411102 不可 skip；测试号进度常只有 403→41110x。
///
/// Docs: `docs/tech/BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS.md`
fn training_skip_priority(quest_group_mst_id: i64) -> i32 {
    match quest_group_mst_id {
        101 => 100, // キオク强化素材（经验向）
        201..=299 => 90, // 魔力解放素材（属性「石头」）
        301 => 40,  // 中间类（保留可选）
        401..=499 => 10, // 能力晶花：通常不可 skip / 易 500
        _ => 50,
    }
}

/// 能力晶花类组（skip 成功率低；优先用キオク/魔力解放）
fn is_crystalis_training_group(quest_group_mst_id: i64) -> bool {
    (401..=499).contains(&quest_group_mst_id)
}

/// 智能体力扫荡（原版 `basic`）。
///
/// # 产品（主人 + 原版）
/// - **只做扫荡 skip**，不做 initialize/finalize 真战斗。
/// - 目标：按角色魔力缺口选**素材本**，材料溢出则选**经验本**（キオク强化）。
/// - **快速刷图 `super_sweep`** 才是指定关真战斗，职责不同。
///
/// # 协议
/// `GET training` → 选关 → `/api/quest_battle/skip_quest_battle`
///
/// # 文档
/// `docs/tech/BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS.md` · `MODULE_SEMANTIC_CLASSIFICATION` · archive `stamina.py`
pub async fn basic(
    client: &mut GameClient,
    config: &HashMap<String, Value>,
) -> Result<String> {
    let train = client
        .request(
            "/api/quest_out_game/get_user_training_quest_data_list",
            json!({}),
        )
        .await?;
    let mut cleared_group: HashMap<i64, i64> = HashMap::new();
    if let Some(list) = train
        .get("userQuestTrainingDataList")
        .and_then(|v| v.as_array())
    {
        for r in list {
            let gid = j_i64(r, "questGroupMstId");
            let stage = j_i64(r, "clearedQuestStageMstId");
            if gid != 0 && stage != 0 {
                cleared_group.insert(gid, stage);
            }
        }
    }
    // 体力下限：经典キオク关 useStamina=10；能力晶花 15/20。真正 to_repeat 在选关后按关卡耗体算。
    if client.stamina() < ONCE_STAMINA_COST {
        return Err(CoreError::Skip(format!(
            "训练扫荡：体力不足（当前 {}，最低约需 {ONCE_STAMINA_COST}），已跳过",
            client.stamina()
        )));
    }
    if cleared_group.is_empty() {
        return Err(CoreError::Skip(
            "训练扫荡：尚无已通关的强化本记录（キオク/魔力解放/晶花等），已跳过。请先在游戏内通关对应强化クエスト。".into(),
        ));
    }

    // 仅有能力晶花进度、没有キオク/魔力解放 cleared 时：不硬 skip（主人：411102 不可 skip；wiki 可 skip 主写 Kioku/Magic）
    let has_preferred_skip_group = cleared_group
        .keys()
        .any(|&g| training_skip_priority(g) >= 90);
    if !has_preferred_skip_group
        && cleared_group
            .keys()
            .all(|&g| is_crystalis_training_group(g))
    {
        // 名称来自 mst（可能尚未拉表；此处先用 ID，选关后有表再带名）
        let detail: Vec<String> = cleared_group
            .iter()
            .map(|(g, s)| format!("组={g} 进度关={s}"))
            .collect();
        return Err(CoreError::Skip(format!(
            "训练扫荡：当前强化进度只有能力晶花类本（{}），没有已通关的キオク强化或魔力解放素材本。\
按游戏规则，智能体力扫荡只走 skip；能力晶花进度关（如 411102 Normal）通常不可扫荡。\
请先通关キオク/魔力解放强化本后再开本模块，或用「快速刷图」做真战斗。可用「mst quest-lookup --id <关卡>」查名称。已跳过",
            detail.join("；")
        )));
    }

    let style_mst: HashMap<i64, Value> = client
        .mst
        .style_list
        .iter()
        .filter_map(|m| {
            let id = j_i64(m, "styleMstId");
            (id != 0).then_some((id, m.clone()))
        })
        .collect();
    let mut param_up_mst: HashMap<i64, Vec<Value>> = HashMap::new();
    for p in client
        .mst_list("/api/mst/get_style_param_up_mst_list")
        .await
        .unwrap_or_default()
    {
        param_up_mst
            .entry(j_i64(&p, "groupId"))
            .or_default()
            .push(p);
    }
    let param_up_cost: HashMap<i64, Value> = client
        .mst_list("/api/mst/get_style_param_up_cost_mst_list")
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|c| {
            let id = j_i64(&c, "styleParamUpCostMstId");
            (id != 0).then_some((id, c))
        })
        .collect();
    let quest_group_mst = client
        .mst_list("/api/mst/get_quest_group_mst_list")
        .await
        .unwrap_or_default();
    let quest_mst = client
        .mst_list("/api/mst/get_quest_stage_mst_list")
        .await
        .unwrap_or_default();
    let quest_reward = client
        .mst_list("/api/mst/get_quest_reward_mst_list")
        .await
        .unwrap_or_default();

    let t5 = cfg_i64(config, "basic_stamina_5star", 120);
    let t4 = cfg_i64(config, "basic_stamina_4star", 110);
    let t3 = cfg_i64(config, "basic_stamina_3star", 59);
    let mut cost: HashMap<i64, i64> = HashMap::new();
    let styles = client
        .init_data
        .get("styleDataList")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for style in styles {
        let sid = j_i64(&style, "styleMstId");
        let Some(mst) = style_mst.get(&sid) else {
            continue;
        };
        let target = match j_i64(mst, "rarity") {
            5 => t5,
            4 => t4,
            3 => t3,
            _ => continue,
        };
        let group = j_i64(mst, "paramUpGroupId");
        let last = j_i64(&style, "lastParamUpPriority");
        for line in param_up_mst.get(&group).into_iter().flatten() {
            let pri = j_i64(line, "priority");
            if last >= pri || pri > target {
                continue;
            }
            if let Some(c) = param_up_cost.get(&j_i64(line, "styleParamUpCostMstId")) {
                for i in 1..=6 {
                    let iid = j_i64(c, &format!("useItemMstId{i}"));
                    let num = j_i64(c, &format!("useItemNum{i}"));
                    if iid != 0 && num != 0 {
                        *cost.entry(iid).or_default() += num;
                    }
                }
            }
        }
    }
    if let Some(items) = client.init_data.get("itemDataList").and_then(|v| v.as_array()) {
        for item in items {
            *cost.entry(j_i64(item, "itemMstId")).or_default() -= j_i64(item, "num");
        }
    }

    fn leaky_relu(x: f64, given: f64) -> f64 {
        if x >= 0.0 {
            x
        } else {
            -(-x / given + 1.0).log2() * given
        }
    }
    fn calc_rate(target: f64, given: f64) -> f64 {
        if given <= 0.0 {
            0.0
        } else {
            (leaky_relu(target, given) - leaky_relu(target - given, given)) / given
        }
    }

    // 在「已通关训练组」中选最优：倍率高者优先；材料全溢出时 rate=-1 仍应能选中（勿默认组 101）。
    // wire 根因（2026-08-07 en_w1）：
    // - 仅通关组 403 / 关卡 411102（useStamina=15），却用固定 10 算次数 → 曾一次 214 次 skip 触发 HTTP 500；
    // - max_rate 初值 0 使全 -1 时永不更新，再 fallback 到错误语义。
    // Docs: docs/logs/2026-08-07-post-crash-task-reaudit.md · ERROR_DIAGNOSTICS · C20
    let mut best: Option<(i64, i64, f64)> = None; // (group_id, stage_id, rate)
    let mut logs = Vec::new();
    for (&group_id, &stage_id) in &cleared_group {
        let Some(qr) = quest_mst
            .iter()
            .find(|q| j_i64(q, "questStageMstId") == stage_id)
        else {
            continue;
        };
        let rg = j_i64(qr, "rewardGroupId");
        let mut rewards: HashMap<i64, i64> = HashMap::new();
        for rw in quest_reward.iter().filter(|r| j_i64(r, "rewardGroupId") == rg) {
            let oid = j_i64(rw, "objectId");
            let num = j_i64(rw, "num");
            if oid != 0 && num != 0 {
                *rewards.entry(oid).or_default() += num;
            }
        }
        let mut rate = 0.0;
        for (&iid, &cnt) in &rewards {
            rate += calc_rate(*cost.get(&iid).unwrap_or(&0) as f64, cnt as f64);
        }
        // 奖励材料相对目标已全溢出：记为 -1，仍可当经验/刷体本（对齐原版意图，修正选组）
        if !rewards.is_empty() && rewards.keys().all(|i| cost.get(i).copied().unwrap_or(0) <= 0) {
            rate = -1.0;
        }
        let gname = quest_group_mst
            .iter()
            .find(|g| j_i64(g, "questGroupMstId") == group_id)
            .map(|g| j_str(g, "name"))
            .unwrap_or_else(|| group_id.to_string());
        let use_st = j_i64(qr, "useStamina").max(1);
        let prio = training_skip_priority(group_id);
        let stage_label = crate::mst::format_quest_stage_label(&quest_mst, stage_id);
        logs.push(format!(
            "{gname} 组={group_id} 优先={prio} 倍率={rate:.2} 关卡={stage_label} 耗体={use_st}"
        ));
        // 选关：先比 skip 优先度（キオク/魔力 > 晶花），再比素材倍率
        let replace = match best {
            None => true,
            Some((bg, _, br)) => {
                let bp = training_skip_priority(bg);
                prio > bp || (prio == bp && rate > br)
            }
        };
        if replace {
            best = Some((group_id, stage_id, rate));
        }
    }

    let Some((max_group, stage_id, max_rate)) = best else {
        return Err(CoreError::Skip(
            "没有可解析的已通关训练关卡（mst 缺关卡行），已跳过".into(),
        ));
    };

    let stage_label = crate::mst::format_quest_stage_label(&quest_mst, stage_id);
    // 若最终仍落到能力晶花关：明确跳过，避免对不可 skip 关打 500
    if is_crystalis_training_group(max_group) && !has_preferred_skip_group {
        return Err(CoreError::Skip(format!(
            "训练扫荡：选中能力晶花类关卡 group={max_group} 关卡={stage_label}（如 Easy/Normal 晶花本）。\
此类关通常不可 skip（主人：411102 不可 skip）。智能体力扫荡不改为真战斗。已跳过"
        )));
    }
    let stage_row = quest_mst
        .iter()
        .find(|q| j_i64(q, "questStageMstId") == stage_id);
    let use_stamina = stage_row
        .map(|q| j_i64(q, "useStamina"))
        .filter(|&s| s > 0)
        .unwrap_or(ONCE_STAMINA_COST);
    let gname = quest_group_mst
        .iter()
        .find(|g| j_i64(g, "questGroupMstId") == max_group)
        .map(|g| j_str(g, "name"))
        .unwrap_or_else(|| max_group.to_string());

    // 按**本关**耗体算次数（组 403 Normal=15，不是固定 10）
    let to_repeat = client.stamina() / use_stamina;
    if to_repeat <= 0 {
        return Err(CoreError::Skip(format!(
            "训练扫荡：体力不足本关耗体（当前 {}，关卡={stage_label} 耗体={use_stamina}），已跳过",
            client.stamina()
        )));
    }
    logs.push(format!(
        "选定 {gname} 关卡={stage_label} 倍率={max_rate:.2} 耗体={use_stamina} 计划×{to_repeat}（体力={}）",
        client.stamina()
    ));

    // 分批：批大小按耗体限制体力与 20 取小；payload 对齐原版 basic（不强制 partyDataId=0）
    // 首批失败 → Skip；中途失败 → 部分完成（P25）
    const MAX_SKIP_BATCH: i64 = 20;
    let mut remaining = to_repeat;
    let mut done: i64 = 0;
    while remaining > 0 {
        let n = remaining.min(MAX_SKIP_BATCH);
        // 与 Python QuestBattleApiSkipQuestBattleRequest 一致：可扫关不要求 party；勿传 0
        match client
            .request(
                "/api/quest_battle/skip_quest_battle",
                json!({
                    "questStageMstId": stage_id,
                    "repeatNum": n,
                    "isArchiveEvent": false
                }),
            )
            .await
        {
            Ok(_) => {
                client.apply_stamina_delta(-n * use_stamina);
                done += n;
                remaining -= n;
            }
            Err(e) => {
                if done > 0 {
                    logs.push(format!(
                        "【部分完成】训练扫荡已成功 {done}/{to_repeat} 次 · {gname} · 关卡={stage_label} · 倍率={max_rate:.2}；后续失败：{e}"
                    ));
                    return Ok(logs.join("\n"));
                }
                // 常见：关不允许 skip、未满足「该 Rank 已通关可扫」、QP 不足等（非必然协议坏）
                return Err(CoreError::Skip(format!(
                    "训练扫荡：skip 被拒绝（组={max_group} 关卡={stage_label} 耗体={use_stamina} 计划×{to_repeat}）。\
本模块只做扫荡、不做真战斗。可能原因：该关不可 skip、强化 Rank 未开放扫荡、或资源不足。服务端：{e}\n{}",
                    logs.join("\n")
                )));
            }
        }
    }
    logs.push(format!(
        "训练扫荡成功：{gname} ×{done} 次 · 关卡={stage_label} · 倍率={max_rate:.2} · 耗体合计={}",
        done * use_stamina
    ));
    Ok(logs.join("\n"))
}

/// 扫荡活动（混合模块）。
///
/// **阶段：**  
/// 1. 未通关且可打：`quest_battle` initialize + finalize（③ 自动主队）  
/// 2. 已通关剩余次数：`quest_battle/skip`（①）  
///
/// **结果口径（FIX-EVENT-EMPTY · 2026-08-07）：**  
/// - 仅当本轮实际完成至少一次战斗通关或 skip 请求成功 → **成功**，日志写清活动名/id 与操作。  
/// - `todayPlayableCount` 全为 0 且未发出战斗/skip → **跳过**（禁止仅凭「活动扫荡 队伍=…」标成功）。  
/// 对照 wire `…2cc5689d` / `…045858` · known-issues · MODULE_SEMANTIC 混合表
pub async fn event_sweep(client: &mut GameClient) -> Result<String> {
    let (party_id, party_name) = party_main(client)?;
    let quest_mst = client.mst_list("/api/mst/get_quest_stage_mst_list").await?;
    let story_event: HashMap<i64, Value> = client
        .mst_list("/api/mst/get_story_event_mst_list")
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|m| {
            let id = j_i64(&m, "storyEventMstId");
            (id != 0).then_some((id, m))
        })
        .collect();
    let story_top = client
        .request("/api/story_event/get_top", json!({}))
        .await?;
    // 操作日志与头部文案分离：头部不算「做了事」，避免空成功（FIX-EVENT-EMPTY）
    let mut action_logs: Vec<String> = Vec::new();
    let mut playable_total: i64 = 0;
    let can_sweep: HashSet<i64> = story_top
        .get("userQuestStageDataList")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().map(|x| j_i64(x, "questStageMstId")).collect())
        .unwrap_or_default();
    let events = story_top
        .get("storyEventDataList")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if events.is_empty() {
        return Err(CoreError::Skip("当前没有活动数据，已跳过".into()));
    }
    for info in events {
        let eid = j_i64(&info, "storyEventMstId");
        let Some(mst) = story_event.get(&eid) else {
            continue;
        };
        let name = j_str(mst, "name");
        let group = j_i64(mst, "storyQuestGroupId");
        let mut available: Vec<i64> = quest_mst
            .iter()
            .filter(|x| j_i64(x, "questGroupMstId") == group)
            .map(|x| j_i64(x, "questStageMstId"))
            .collect();
        available.sort();
        if available.is_empty() {
            continue;
        }
        let max_q = *available.last().unwrap();
        let mut playable = j_i64(&info, "todayPlayableCount");
        playable_total += playable;
        for &quest_id in &available {
            if can_sweep.contains(&quest_id) || playable == 0 {
                continue;
            }
            if client
                .request(
                    "/api/quest_battle/initialize_stage",
                    json!({
                        "questStageMstId": quest_id,
                        "partyDataId": party_id,
                        "repeatNum": 0,
                        "backGroundPlay": false,
                        "isArchiveEvent": false,
                        "selectionAbilityMultiLotteryItemNum": 0
                    }),
                )
                .await
                .is_err()
            {
                continue;
            }
            let _ = client
                .request(
                    "/api/quest_battle/finalize_stage_for_user",
                    json!({
                        "battleLog": GameClient::simple_battle_log(),
                        "autoMode": 0,
                        "result": 1
                    }),
                )
                .await;
            playable -= 1;
            action_logs.push(format!(
                "战斗通关 活动={name}(id={eid}) 关卡={quest_id} 剩余次数={playable}"
            ));
        }
        if playable <= 0 {
            continue;
        }
        if client
            .request(
                "/api/quest_battle/skip_quest_battle",
                json!({
                    "isArchiveEvent": false,
                    "partyDataId": party_id,
                    "questStageMstId": max_q,
                    "repeatNum": playable
                }),
            )
            .await
            .is_ok()
        {
            action_logs.push(format!(
                "扫荡 活动={name}(id={eid}) 关卡={max_q} 次数={playable}"
            ));
        }
    }
    if action_logs.is_empty() {
        return Err(CoreError::Skip(format!(
            "活动扫荡：今日次数已用尽或无可打/可扫内容（活动合计剩余次数={playable_total}，队伍={party_name}），已跳过"
        )));
    }
    let mut out = vec![format!(
        "活动扫荡完成 队伍={party_name} 有效操作={} 项",
        action_logs.len()
    )];
    out.extend(action_logs);
    Ok(out.join("\n"))
}

pub async fn archive_sweep(client: &mut GameClient) -> Result<String> {
    let (party_id, party_name) = party_main(client)?;
    let quest_mst = client.mst_list("/api/mst/get_quest_stage_mst_list").await?;
    let story_event: HashMap<i64, Value> = client
        .mst_list("/api/mst/get_story_event_mst_list")
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|m| {
            let id = j_i64(&m, "storyEventMstId");
            (id != 0).then_some((id, m))
        })
        .collect();
    let story_quest: HashMap<i64, i64> = client
        .mst_list("/api/mst/get_story_event_quest_stage_mst_list")
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|x| (j_i64(&x, "questStageMstId"), j_i64(&x, "eventItemNum")))
        .collect();
    let mut story_bonus: HashMap<i64, HashMap<i64, Vec<i64>>> = HashMap::new();
    for b in client
        .mst_list("/api/mst/get_story_event_bonus_rate_mst_list")
        .await
        .unwrap_or_default()
    {
        story_bonus
            .entry(j_i64(&b, "storyEventMstId"))
            .or_default()
            .insert(
                j_i64(&b, "bonusMstId"),
                (0..6)
                    .map(|i| j_i64(&b, &format!("limitBreakCount{i}Rate")))
                    .collect(),
            );
    }
    let mut limit_break: HashMap<i64, i64> = HashMap::new();
    if let Some(cards) = client.init_data.get("cardDataList").and_then(|v| v.as_array()) {
        for c in cards {
            limit_break.insert(j_i64(c, "cardMstId"), j_i64(c, "limitBreakCount"));
        }
    }
    if let Some(styles) = client.init_data.get("styleDataList").and_then(|v| v.as_array()) {
        for s in styles {
            limit_break.insert(j_i64(s, "styleMstId"), j_i64(s, "limitBreakCount"));
        }
    }
    let mut ids: Vec<i64> = client
        .init_data
        .get("styleDataList")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().map(|x| j_i64(x, "styleMstId")).collect())
        .unwrap_or_default();
    if let Some(cards) = client.init_data.get("cardDataList").and_then(|v| v.as_array()) {
        ids.extend(cards.iter().map(|x| j_i64(x, "cardMstId")));
    }
    let archive_top = client
        .request("/api/story_event/get_archive_event_list", json!({}))
        .await?;
    let sweep_count = archive_top
        .pointer("/storyEventDataList/0/todayPlayableCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if sweep_count <= 0 {
        return Err(CoreError::Skip("档案活动无可游玩内容，已跳过".into()));
    }
    let mut max_sweep_quest_id = -1i64;
    let mut max_sweep_bonus = -1i64;
    let mut max_name = String::new();
    let mut logs = vec![format!("档案活动 队伍={party_name}")];
    let infos = archive_top
        .get("storyEventInfoList")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for info in infos {
        let eid = j_i64(&info, "storyEventMstId");
        let Some(mst) = story_event.get(&eid) else {
            continue;
        };
        let name = j_str(mst, "name");
        let group = j_i64(mst, "storyQuestGroupId");
        let available: Vec<i64> = quest_mst
            .iter()
            .filter(|x| j_i64(x, "questGroupMstId") == group)
            .filter_map(|x| story_quest.get(&j_i64(x, "questStageMstId")).copied())
            .collect();
        if available.is_empty() {
            continue;
        }
        let max_available = *available.iter().max().unwrap_or(&0);
        let mut to_sweep: Vec<&Value> = archive_top
            .get("userQuestStageDataList")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter(|x| j_i64(x, "questGroupMstId") == group)
                    .collect()
            })
            .unwrap_or_default();
        to_sweep.sort_by_key(|x| {
            std::cmp::Reverse(
                story_quest
                    .get(&j_i64(x, "questStageMstId"))
                    .copied()
                    .unwrap_or(0),
            )
        });
        if to_sweep.is_empty()
            || story_quest
                .get(&j_i64(to_sweep[0], "questStageMstId"))
                .copied()
                .unwrap_or(0)
                != max_available
        {
            logs.push(format!("{name} not fully cleared"));
            continue;
        }
        let quest_id = j_i64(to_sweep[0], "questStageMstId");
        let bonus_data = story_bonus.get(&eid);
        let max_bonus: i64 = ids
            .iter()
            .map(|i| {
                let lb = limit_break.get(i).copied().unwrap_or(0).clamp(0, 5) as usize;
                bonus_data
                    .and_then(|b| b.get(i))
                    .and_then(|r| r.get(lb))
                    .copied()
                    .unwrap_or(0)
            })
            .sum();
        logs.push(format!("{name} bonus={}%", max_bonus as f64 / 10.0));
        if max_bonus > max_sweep_bonus {
            max_sweep_bonus = max_bonus;
            max_sweep_quest_id = quest_id;
            max_name = name;
        }
    }
    if max_sweep_quest_id < 0 {
        return Err(CoreError::Skip("没有可扫荡的档案活动，已跳过".into()));
    }
    client
        .request(
            "/api/quest_battle/skip_quest_battle",
            json!({
                "isArchiveEvent": true,
                "partyDataId": party_id,
                "questStageMstId": max_sweep_quest_id,
                "repeatNum": sweep_count
            }),
        )
        .await?;
    logs.push(format!(
        "swept archive {max_name} ({max_sweep_quest_id}) x{sweep_count}"
    ));
    Ok(logs.join("\n"))
}

// --- shop.py ---

/// 商店类别名 = Python item_category 键（中文），配置键 `{prefix}_shop_priority_{类别}`。
/// **缺省与产品默认均为 0（不兑换）**；与 config_catalog 一致。
fn shop_category_priority(config: &HashMap<String, Value>, prefix: &str, category: &str) -> i64 {
    let key = format!("{prefix}_shop_priority_{category}");
    config
        .get(&key)
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

/// 对照 shop.py item_category — 返回中文类别名
fn classify_shop_item(shop: &Value) -> String {
    let recv = j_i64(shop, "objectReceiveType");
    let oid = j_i64(shop, "objectId");
    let price = j_i64(shop, "price");
    let infinite = j_i64(shop, "purchaseLimitCount") == 0;
    if price <= 0 {
        return "白送的东西".into();
    }
    match recv {
        4 => "肖像".into(),
        2 => "钻石".into(),
        18 => "玩家经验".into(),
        15 => "称号".into(),
        19 | 20 | 21 | 22 => "玩偶屋".into(),
        14 | 16 | 17 => "光之间内容".into(),
        11 => {
            if infinite {
                "金币（无限池）".into()
            } else {
                "金币".into()
            }
        }
        5 => match oid {
            232030 | 232001 => "钥匙（碎片）".into(),
            232054 => "10抽钥匙".into(),
            201075 => "5x交换币".into(),
            201017 => "4x交换币".into(),
            201046 => "光之间内容".into(),
            262001 => "记忆切符".into(),
            121003 => "彩球".into(),
            180001 => "开孔材料".into(),
            180005 => "永久锁".into(),
            123001 | 123002 | 123003 => "技能书".into(),
            121023..=121028 => "新属性球".into(),
            121006 | 121009 | 121012 | 121015 | 121018 | 121021 | 121022 => "属性球".into(),
            290001 => "LP体力石".into(),
            122001 | 122002 | 122003 => "画板".into(),
            202001 => "体力石".into(),
            113001 | 113002 => "心砂".into(),
            180003 => {
                if infinite {
                    "泪滴（无限池）".into()
                } else {
                    "泪滴".into()
                }
            }
            124001 => {
                if infinite {
                    "经验（无限池）".into()
                } else {
                    "经验".into()
                }
            }
            181004 => "晶花抽取EX".into(),
            181001 | 181002 | 181003 | 282001 => "晶花抽取".into(),
            180004 => "临时锁".into(),
            121001 if infinite => "小石头（无限池）".into(),
            121001 | 121004 | 121007 | 121010 | 121013 | 121016 | 121019 => "小石头".into(),
            121002 | 121005 | 121008 | 121011 | 121014 | 121017 | 121020 => "大石头".into(),
            _ => "未知".into(),
        },
        _ => "未知".into(),
    }
}

async fn shop_clear(
    client: &mut GameClient,
    config: &HashMap<String, Value>,
    prefix: &str,
    filter_fn: impl Fn(&Value) -> bool,
) -> Result<String> {
    let items_resp = client
        .request("/api/item/get_item_data_list", json!({}))
        .await?;
    let mut user_items: HashMap<i64, i64> = HashMap::new();
    if let Some(list) = items_resp.get("itemDataList").and_then(|v| v.as_array()) {
        for item in list {
            *user_items.entry(j_i64(item, "itemMstId")).or_default() += j_i64(item, "num");
        }
    }
    let shop = client.request("/api/shop/get_shop_list", json!({})).await?;
    let shop_mst = client
        .mst_list("/api/mst/get_shop_mst_list")
        .await
        .unwrap_or_default();
    let series_mst = client
        .mst_list("/api/mst/get_shop_series_mst_list")
        .await
        .unwrap_or_default();
    let item_dict: HashMap<i64, Value> = client
        .mst_list("/api/mst/get_item_mst_list")
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|x| {
            let id = j_i64(&x, "itemMstId");
            (id != 0).then_some((id, x))
        })
        .collect();
    let null_time = parse_dt("1970-01-01T09:00:00+09:00");
    let now = Utc::now();
    let mut logs = Vec::new();

    for mst in &series_mst {
        if !filter_fn(mst) {
            continue;
        }
        if let (Some(s), Some(nt)) = (parse_dt(&j_str(mst, "startTime")), null_time) {
            if s != nt && now.with_timezone(s.offset()) < s {
                continue;
            }
        }
        if let (Some(e), Some(nt)) = (parse_dt(&j_str(mst, "endTime")), null_time) {
            if e != nt && now.with_timezone(e.offset()) > e {
                continue;
            }
        }
        let series = j_i64(mst, "shopSeriesMstId");
        let g1 = j_i64(mst, "shopGroupId1");
        let g2 = j_i64(mst, "shopGroupId2");
        let mut all_items: Vec<Value> = shop_mst
            .iter()
            .filter(|s| {
                let g = j_i64(s, "shopGroupId");
                g == g1 || g == g2
            })
            .cloned()
            .collect();
        all_items.sort_by(|a, b| {
            let ca = classify_shop_item(a);
            let cb = classify_shop_item(b);
            let pa = shop_category_priority(config, prefix, &ca);
            let pb = shop_category_priority(config, prefix, &cb);
            let ra = if j_i64(a, "objectReceiveType") == 5 {
                item_dict
                    .get(&j_i64(a, "objectId"))
                    .map(|i| j_i64(i, "rarity"))
                    .unwrap_or(0)
            } else {
                0
            };
            let rb = if j_i64(b, "objectReceiveType") == 5 {
                item_dict
                    .get(&j_i64(b, "objectId"))
                    .map(|i| j_i64(i, "rarity"))
                    .unwrap_or(0)
            } else {
                0
            };
            let ea = if j_i64(a, "price") > 0 {
                j_i64(a, "num") as f64 / j_i64(a, "price") as f64
            } else {
                0.0
            };
            let eb = if j_i64(b, "price") > 0 {
                j_i64(b, "num") as f64 / j_i64(b, "price") as f64
            } else {
                0.0
            };
            pb.cmp(&pa)
                .then(rb.cmp(&ra))
                .then(eb.partial_cmp(&ea).unwrap_or(std::cmp::Ordering::Equal))
        });

        let purchased: HashMap<i64, i64> = shop
            .get("shopCountDataList")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter(|s| j_i64(s, "shopSeriesMstId") == series)
                    .map(|s| (j_i64(s, "shopMstId"), j_i64(s, "purchaseCount")))
                    .collect()
            })
            .unwrap_or_default();
        let title = j_str(mst, "title");
        let pay_id = j_i64(mst, "payId");
        for item in all_items {
            let cat = classify_shop_item(&item);
            if shop_category_priority(config, prefix, &cat) == 0 {
                continue;
            }
            let price = j_i64(&item, "price");
            if price <= 0 {
                continue;
            }
            let shop_id = j_i64(&item, "shopMstId");
            let limit = j_i64(&item, "purchaseLimitCount");
            let bought = purchased.get(&shop_id).copied().unwrap_or(0);
            if limit != 0 && bought >= limit {
                continue;
            }
            let coin = user_items.get(&pay_id).copied().unwrap_or(0);
            if coin < price {
                logs.push(format!(
                    "商店「{title}」：货币不足，停止该店后续兑换（类别={cat} 单价={price} 持有={coin}）"
                ));
                break;
            }
            let mut buy_num = coin / price;
            if limit != 0 {
                buy_num = buy_num.min(limit - bought);
            }
            if buy_num <= 0 {
                continue;
            }
            let iname = item_dict
                .get(&j_i64(&item, "objectId"))
                .map(|i| j_str(i, "name"))
                .unwrap_or_else(|| cat.clone());
            match client
                .request(
                    "/api/shop/buy",
                    json!({
                        "num": buy_num,
                        "shopMstId": shop_id,
                        "shopSeriesMstId": series
                    }),
                )
                .await
            {
                Ok(_) => {
                    *user_items.entry(pay_id).or_default() -= price * buy_num;
                    logs.push(format!(
                        "已购买 店={title} 物品={iname} 类别={cat} 数量={buy_num} 单价={price}"
                    ));
                }
                Err(e) => logs.push(format!(
                    "购买失败 店={title} 物品={iname} 类别={cat}：{e}"
                )),
            }
        }
    }
    let bought_n = logs.iter().filter(|s| s.starts_with("已购买")).count();
    if bought_n == 0 {
        if logs.is_empty() {
            let any_prio = shop_item_categories_any_priority(config, prefix);
            let reason = if !any_prio {
                "各商品类别优先级均为 0（不购买）"
            } else {
                "已启用类别下无在售可购商品，或均已购完/价格异常"
            };
            Err(CoreError::Skip(format!(
                "商店{prefix}：本轮无可兑换（{reason}），已跳过"
            )))
        } else {
            // 有尝试记录但无一笔成功 → 跳过并附原因（货币不足等），禁止标成功
            Err(CoreError::Skip(format!(
                "商店{prefix}：本轮未成功购买\n{}",
                logs.join("\n")
            )))
        }
    } else {
        Ok(format!(
            "商店{prefix}：成功购买 {bought_n} 笔\n{}",
            logs.join("\n")
        ))
    }
}

fn shop_item_categories_any_priority(config: &HashMap<String, Value>, prefix: &str) -> bool {
    // 与 config_catalog::shop_item_categories 同序类别名；避免 core modules 循环依赖 catalog 函数路径
    const CATS: &[&str] = &[
        "白送的东西",
        "肖像",
        "钥匙（碎片）",
        "10抽钥匙",
        "5x交换币",
        "4x交换币",
        "钻石",
        "玩家经验",
        "称号",
        "玩偶屋",
        "光之间内容",
        "记忆切符",
        "彩球",
        "开孔材料",
        "永久锁",
        "技能书",
        "新属性球",
        "属性球",
        "LP体力石",
        "画板",
        "体力石",
        "心砂",
        "泪滴",
        "经验",
        "晶花抽取EX",
        "晶花抽取",
        "临时锁",
        "小石头",
        "大石头",
        "金币",
        "泪滴（无限池）",
        "小石头（无限池）",
        "经验（无限池）",
        "金币（无限池）",
    ];
    CATS.iter()
        .any(|c| shop_category_priority(config, prefix, c) > 0)
}

pub async fn event_shop(
    client: &mut GameClient,
    config: &HashMap<String, Value>,
) -> Result<String> {
    shop_clear(client, config, "event", |mst| {
        j_i64(mst, "category") == 3
            && !j_str(mst, "title").contains("\u{30b4}\u{30fc}\u{30eb}\u{30c9}\u{30af}\u{30e9}\u{30a4}\u{30b7}\u{30b9}\u{30e1}\u{30c0}\u{30eb}")
            && !j_str(mst, "title").contains("\u{30b7}\u{30eb}\u{30d0}\u{30fc}\u{30af}\u{30e9}\u{30a4}\u{30b7}\u{30b9}\u{30e1}\u{30c0}\u{30eb}")
    })
    .await
}

pub async fn raid_shop(
    client: &mut GameClient,
    config: &HashMap<String, Value>,
) -> Result<String> {
    shop_clear(client, config, "raid", |mst| {
        let pay = j_i64(mst, "payId");
        let title = j_str(mst, "title");
        pay == 201029
            || pay == 201030
            || title.contains("\u{30b4}\u{30fc}\u{30eb}\u{30c9}\u{30af}\u{30e9}\u{30a4}\u{30b7}\u{30b9}\u{30e1}\u{30c0}\u{30eb}")
            || title.contains("\u{30b7}\u{30eb}\u{30d0}\u{30fc}\u{30af}\u{30e9}\u{30a4}\u{30b7}\u{30b9}\u{30e1}\u{30c0}\u{30eb}")
    })
    .await
}

pub async fn arena_shop(
    client: &mut GameClient,
    config: &HashMap<String, Value>,
) -> Result<String> {
    shop_clear(client, config, "arena", |mst| j_i64(mst, "payId") == 201009).await
}

pub async fn tower(client: &mut GameClient) -> Result<String> {
    let quest_mst = client.mst_list("/api/mst/get_quest_stage_mst_list").await?;
    let quest_group_mst = client.mst_list("/api/mst/get_quest_group_mst_list").await?;
    let tower_list = client
        .mst_list("/api/mst/get_tower_mst_list")
        .await
        .unwrap_or_default();
    if !tower_list
        .iter()
        .any(|t| in_window(&j_str(t, "startTime"), &j_str(t, "endTime")))
    {
        return Err(CoreError::Skip("塔未开放，已跳过".into()));
    }
    let tower_group = quest_group_mst
        .iter()
        .find(|q| j_i64(q, "questCategoryMstId") == 5)
        .map(|q| j_i64(q, "questGroupMstId"))
        .unwrap_or(0);
    let last_floor = quest_mst
        .iter()
        .filter(|q| j_i64(q, "questGroupMstId") == tower_group)
        .map(|q| j_i64(q, "questStageMstId"))
        .max()
        .unwrap_or(0);
    let top = client.request("/api/tower/get_top", json!({})).await?;
    let skip_num = top
        .pointer("/userTowerData/skipNum")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if skip_num == 0 {
        return Err(CoreError::Skip("塔扫荡次数为 0，已跳过".into()));
    }
    let max_q = top
        .pointer("/userTowerData/maxQuestStageMstId")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if max_q != last_floor {
        return Err(CoreError::Skip("塔顶层尚未通关，无法扫荡，已跳过".into()));
    }
    let (party_id, _) = party_max_power(client)?;
    client
        .request(
            "/api/tower/skip_quest_battle",
            json!({
                "partyDataId": party_id,
                "repeatNum": skip_num,
                "questStageMstId": max_q
            }),
        )
        .await?;
    Ok(format!("塔扫荡 {skip_num} 次"))
}

pub async fn heart(
    client: &mut GameClient,
    config: &HashMap<String, Value>,
) -> Result<String> {
    let team_s = cfg_str(config, "heart_team", "0");
    let force = cfg_bool(config, "heart_force_sweep", false);
    let (party_id, party_name) = match resolve_party(client, &team_s) {
        Ok(p) => p,
        Err(_) => party_main(client)?,
    };
    let quest_mst = client.mst_list("/api/mst/get_quest_stage_mst_list").await?;
    let heart_record = client
        .request(
            "/api/quest_out_game/get_user_quest_character_heart_list",
            json!({}),
        )
        .await?;
    if heart_record.get("userQuestCharacterHeartData").map(|v| v.is_null()).unwrap_or(true) {
        return Err(CoreError::Skip("心之器未解锁，已跳过".into()));
    }
    let limit = client
        .game_config
        .pointer("/questConfig/characterHeartDailyBattleClearLimit")
        .and_then(|v| v.as_i64())
        .unwrap_or(3);
    let last_time = heart_record
        .pointer("/userQuestCharacterHeartData/dailyClearCountUpdatedTime")
        .and_then(|v| v.as_str())
        .and_then(parse_dt);
    let count = heart_record
        .pointer("/userQuestCharacterHeartData/dailyClearCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let used = if let Some(lt) = last_time {
        let now = Local::now();
        let adj_now = now - Duration::hours(4);
        let adj_last = lt.with_timezone(&Local) - Duration::hours(4);
        if adj_now.ordinal() != adj_last.ordinal() {
            0
        } else {
            count
        }
    } else {
        count
    };
    let mut remaining = limit - used;
    if remaining <= 0 {
        return Err(CoreError::Skip("心之器今日次数已用尽，已跳过".into()));
    }
    let heart_quests: Vec<&Value> = quest_mst
        .iter()
        .filter(|q| j_i64(q, "questGroupMstId") == 301)
        .collect();
    let max_exp_quest = heart_quests
        .iter()
        .max_by_key(|q| j_i64(q, "characterHeartExp"))
        .copied()
        .ok_or_else(|| CoreError::Skip("心之器关卡表为空，已跳过".into()))?;
    let saves = heart_record
        .get("userQuestCharacterHeartPartySaveDataList")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut rec = saves
        .iter()
        .find(|r| j_i64(r, "questStageMstId") == j_i64(max_exp_quest, "questStageMstId"))
        .cloned();
    if rec.is_none() {
        if !force {
            return Err(CoreError::Skip("最高经验心之器关卡尚未通关，已跳过".into()));
        }
        let fb = heart_quests
            .iter()
            .filter(|q| {
                saves
                    .iter()
                    .any(|r| j_i64(r, "questStageMstId") == j_i64(q, "questStageMstId"))
            })
            .max_by_key(|q| j_i64(q, "characterHeartExp"))
            .copied()
            .ok_or_else(|| CoreError::Skip("没有已通关的心之器关卡，已跳过".into()))?;
        rec = saves
            .iter()
            .find(|r| j_i64(r, "questStageMstId") == j_i64(fb, "questStageMstId"))
            .cloned();
    }
    let rec = rec.ok_or_else(|| CoreError::Skip("没有已通关的心之器关卡，已跳过".into()))?;
    let stage_id = j_i64(&rec, "questStageMstId");
    let rec_members: HashSet<i64> = (1..=5).map(|i| j_i64(&rec, &format!("member{i}"))).collect();
    let party_members: HashSet<i64> = party_list(client)
        .into_iter()
        .find(|p| j_i64(p, "partyDataId") == party_id)
        .map(|p| (1..=5).map(|i| j_i64(&p, &format!("member{i}"))).collect())
        .unwrap_or_default();
    let mut logs = Vec::new();
    if rec_members != party_members {
        logs.push(format!("re-clear heart with party {party_name}"));
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let init = client
            .request(
                "/api/quest_battle/initialize_stage",
                json!({
                    "questStageMstId": stage_id,
                    "partyDataId": party_id,
                    "repeatNum": 0,
                    "backGroundPlay": false,
                    "isArchiveEvent": false,
                    "selectionAbilityMultiLotteryItemNum": 0
                }),
            )
            .await?;
        let qid = init
            .pointer("/questRoomData/questDataId")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let battle_log = battle_log_quest(client, qid).await;
        client
            .request(
                "/api/quest_battle/finalize_stage_for_user",
                json!({
                    "autoMode": cfg_i64(config, "force_battle_auto_mode", 0),
                    "battleLog": battle_log,
                    "result": 1
                }),
            )
            .await?;
        remaining -= 1;
    }
    if remaining <= 0 {
        return Ok(logs.join("\n") + "\nheart remaining used by re-clear");
    }
    client
        .request(
            "/api/quest_battle/skip_quest_battle",
            json!({
                "isArchiveEvent": false,
                "partyDataId": 0,
                "questStageMstId": stage_id,
                "repeatNum": remaining
            }),
        )
        .await?;
    logs.push(format!("心之器扫荡 ×{remaining} 关卡={stage_id}"));
    Ok(logs.join("\n"))
}

/// 收集首页宝箱（gathering）。
///
/// - 对照: `archive/.../modules/sweep.py` class gather
/// - 路径: `/api/gathering/get_gathering_top`（误写 `get_top` 会 404）  
/// - **成功：** shortcut（可选）+ receive_reward  
/// - **跳过：** 挂机未满约 10 小时  
/// - 文档: W2 三轮对照 · C20/L13
pub async fn gather(client: &mut GameClient) -> Result<String> {
    // 原版 GatheringApiGetGatheringTopRequest.url
    let top = client
        .request("/api/gathering/get_gathering_top", json!({}))
        .await?;
    let mut logs: Vec<String> = Vec::new();
    if top
        .pointer("/userGatheringData/shortcutCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        == 0
    {
        client
            .request("/api/gathering/shortcut_gathering", json!({}))
            .await?;
        logs.push("已使用每日首次免费加速".into());
    }
    if let Some(gt) = top
        .pointer("/userGatheringData/gatheringTime")
        .and_then(|v| v.as_str())
        .and_then(parse_dt)
    {
        if Utc::now().signed_duration_since(gt.with_timezone(&Utc)) < Duration::hours(10) {
            // 原版 SkipError: 「宝箱时间未超过10小时，不收取」
            let msg = "宝箱时间未超过约10小时，不收取";
            if logs.is_empty() {
                return Err(CoreError::Skip(msg.into()));
            }
            // 已做免费加速但未收取：部分完成（OUT-PARTIAL），勿标纯跳过以免丢加速事实
            logs.push(msg.into());
            logs.insert(
                0,
                "【部分完成】收集宝箱：已执行前置步骤，本轮未领取奖励".into(),
            );
            return Ok(logs.join("\n"));
        }
    }
    client
        .request("/api/gathering/receive_reward", json!({}))
        .await?;
    logs.push("已收集当前宝箱".into());
    Ok(logs.join("\n"))
}

/// 免费扭蛋。
///
/// - 对照: `archive/.../modules/gacha.py` freegacha
/// - 路径: `/api/gacha/get_gacha_top`（**不是** `get_top`）
/// - 无免费池 / 次数用尽: 宜 Skip 或成功+说明；见框架附录 B
pub async fn freegacha(client: &mut GameClient) -> Result<String> {
    // 原版 GachaApiGetGachaTopRequest.url
    let top = client
        .request("/api/gacha/get_gacha_top", json!({}))
        .await?;
    let gacha_list = top
        .pointer("/viewData/gachaList")
        .and_then(|v| v.as_array())
        .cloned()
        .or_else(|| top.get("gachaDataList").and_then(|v| v.as_array()).cloned())
        .unwrap_or_default();
    let free: Vec<&Value> = gacha_list
        .iter()
        .filter(|g| j_i64(g, "price") == 0 || j_bool(g, "isFree"))
        .collect();
    let counts: HashMap<i64, (i64, i64)> = top
        .pointer("/viewData/gachaCountDataList")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .map(|x| {
                    (
                        j_i64(x, "gachaMstId"),
                        (j_i64(x, "dailyCount"), j_i64(x, "totalCount")),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let mut logs = Vec::new();
    let mut n = 0;
    for g in free {
        let id = j_i64(g, "gachaMstId");
        let name = {
            let n = j_str(g, "gachaName");
            if n.is_empty() {
                j_str(g, "name")
            } else {
                n
            }
        };
        let (daily, total) = counts.get(&id).copied().unwrap_or((0, 0));
        if j_i64(g, "countLimit") != 0 && j_i64(g, "countLimit") <= total {
            continue;
        }
        if j_i64(g, "dailyCountLimit") != 0 && j_i64(g, "dailyCountLimit") <= daily {
            continue;
        }
        match client
            .request("/api/gacha/gacha_exec", json!({ "gachaMstId": id }))
            .await
        {
            Ok(_) => {
                n += 1;
                logs.push(format!("gacha {name}"));
            }
            Err(e) => logs.push(format!("{name} fail: {e}")),
        }
    }
    if n == 0 && logs.is_empty() {
        // 原版无免费池时多半空跑成功；产品上标跳过更不易与真故障混淆（C20）
        return Err(CoreError::Skip("没有可抽的免费扭蛋".into()));
    }
    Ok(format!("免费扭蛋完成 {n} 次\n{}", logs.join("\n")))
}

pub async fn eventscenario(client: &mut GameClient) -> Result<String> {
    let story_event: HashMap<i64, Value> = client
        .mst_list("/api/mst/get_story_event_mst_list")
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|m| {
            let id = j_i64(&m, "storyEventMstId");
            (id != 0).then_some((id, m))
        })
        .collect();
    let scenario_list: HashMap<i64, Value> = client
        .mst_list("/api/mst/get_story_event_scenario_mst_list")
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|x| {
            let id = j_i64(&x, "storyEventScenarioMstId");
            (id != 0).then_some((id, x))
        })
        .collect();
    let mut logs = Vec::new();
    let mut updated = true;
    while updated {
        updated = false;
        let story_top = client
            .request("/api/story_event/get_top", json!({}))
            .await?;
        let cleared: HashSet<i64> = story_top
            .get("userQuestStageDataList")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().map(|x| j_i64(x, "questStageMstId")).collect())
            .unwrap_or_default();
        let clear_sc: HashSet<i64> = story_top
            .get("clearStoryEventScenarioMstIdList")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_i64()).collect())
            .unwrap_or_default();
        let scenarios = story_top
            .get("storyEventScenarioMstIdList")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for sid_v in scenarios {
            let sid = sid_v.as_i64().unwrap_or(0);
            if clear_sc.contains(&sid) {
                continue;
            }
            let Some(record) = scenario_list.get(&sid) else {
                continue;
            };
            let cond = j_i64(record, "conditionQuestStageMstId");
            if cond != 0 && !cleared.contains(&cond) {
                continue;
            }
            let adv = j_i64(record, "advMstId");
            let _ = client
                .request(
                    "/api/collection/update_already_view",
                    json!({ "objectType": 1, "objectIds": [adv] }),
                )
                .await;
            if client
                .request(
                    "/api/story_event/scenario_read",
                    json!({ "storyEventScenarioMstId": sid }),
                )
                .await
                .is_ok()
            {
                let ename = story_event
                    .get(&j_i64(record, "storyEventMstId"))
                    .map(|e| j_str(e, "name"))
                    .unwrap_or_default();
                logs.push(format!("read scenario {ename} ({sid})"));
                updated = true;
            }
        }
    }
    if logs.is_empty() {
        Err(CoreError::Skip("活动剧情：暂无新内容，已跳过".into()))
    } else {
        Ok(logs.join("\n"))
    }
}

pub async fn collection(client: &mut GameClient) -> Result<String> {
    let data = client
        .request("/api/collection/get_collection_data_list", json!({}))
        .await?;
    let list = data
        .get("collectionDataList")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut logs = Vec::new();
    for c in list {
        if j_bool(&c, "isGet") && !j_bool(&c, "isAlreadyView") {
            let otype = c.get("objectType").cloned().unwrap_or(json!(0));
            let oid = j_i64(&c, "objectId");
            if client
                .request(
                    "/api/collection/update_already_view",
                    json!({ "objectType": otype, "objectIds": [oid] }),
                )
                .await
                .is_ok()
            {
                logs.push(format!("光之间：已阅藏品 objectType={otype} objectId={oid}"));
            }
        }
    }
    if logs.is_empty() {
        Err(CoreError::Skip("光之间：暂无未读红点，已跳过".into()))
    } else {
        Ok(logs.join("\n"))
    }
}

pub async fn battle_mission(client: &mut GameClient) -> Result<String> {
    let mission_mst: HashMap<i64, Value> = client
        .mst_list("/api/mst/get_mission_mst_list")
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|m| {
            let id = j_i64(&m, "missionMstId");
            (id != 0).then_some((id, m))
        })
        .collect();
    let stratum = client
        .request("/api/mst/get_field_stratum_mst_list", json!({}))
        .await
        .ok()
        .and_then(|v| v.get("mstList").and_then(|x| x.as_array()).cloned())
        .unwrap_or_default();
    let point = client
        .request("/api/mst/get_field_point_mst_list", json!({}))
        .await
        .ok()
        .and_then(|v| v.get("mstList").and_then(|x| x.as_array()).cloned())
        .unwrap_or_default();
    let top = client
        .request(
            "/api/exploration/get_field_stage_collection_info_list",
            json!({}),
        )
        .await?;
    let cleared_field: HashSet<i64> = top
        .get("fieldStageCollectionInfoList")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter(|x| j_bool(x, "isClear"))
                .map(|x| j_i64(x, "fieldStageMstId"))
                .collect()
        })
        .unwrap_or_default();
    let (party_id, _) = party_main(client)?;
    let mut logs = Vec::new();
    for mission_type in 1..=4 {
        let mission = client
            .request(
                "/api/mission/get_mission_data_list",
                json!({ "missionType": mission_type }),
            )
            .await?;
        let list = mission
            .get("missionDataList")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for m in list {
            let mid = j_i64(&m, "missionMstId");
            let Some(mst) = mission_mst.get(&mid) else {
                continue;
            };
            if j_i64(&m, "count") >= j_i64(mst, "conditionCount") {
                continue;
            }
            let trigger = j_i64(mst, "triggerType");
            let ctype = j_i64(mst, "conditionType");
            if !((trigger == 6 && ctype == 252) || (trigger == 31 && ctype == 1451)) {
                continue;
            }
            let quest_id = j_i64(mst, "conditionObjectId");
            let may_point: Vec<&Value> = point
                .iter()
                .filter(|p| j_i64(p, "pointValue1") == quest_id && j_i64(p, "pointType") == 3)
                .collect();
            let Some(p) = may_point.first() else {
                continue;
            };
            let Some(s) = stratum
                .iter()
                .find(|s| j_i64(s, "fieldStratumMstId") == j_i64(p, "fieldStratumMstId"))
            else {
                continue;
            };
            let field = j_i64(s, "fieldStageMstId");
            if !cleared_field.contains(&field) {
                continue;
            }
            let top_info = client
                .request(
                    "/api/exploration/get_top_info_v4",
                    json!({ "fieldStageMstId": field }),
                )
                .await?;
            let cleared_points: HashSet<i64> = top_info
                .pointer("/fieldStageUserData/clearFieldPointMstIdCsv")
                .and_then(|v| v.as_str())
                .map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect())
                .unwrap_or_default();
            let fpid = j_i64(p, "fieldPointMstId");
            if !cleared_points.contains(&fpid) {
                continue;
            }
            let _ = client
                .request(
                    "/api/exploration/reach_field_point",
                    json!({ "fieldPointMstId": fpid }),
                )
                .await;
            let quest = match client
                .request(
                    "/api/exploration_battle/initialize_stage_v4",
                    json!({
                        "fieldPointMstId": fpid,
                        "fieldStageMstId": field,
                        "dungeonEventMstId": 0,
                        "dungeonRoomMstId": 0,
                        "bossDirectionMstId": 0,
                        "presetEventIndex": 0,
                        "partyDataId": party_id
                    }),
                )
                .await
            {
                Ok(v) => v,
                Err(_) => continue,
            };
            let qid = quest
                .pointer("/questRoomData/questDataId")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let battle_log = if qid > 0 {
                match client
                    .request(
                        "/api/exploration_battle/get_exploration_info",
                        json!({ "questDataId": qid }),
                    )
                    .await
                {
                    Ok(info) => {
                        let units = info
                            .get("allyBattleUnitList")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        GameClient::battle_log_from_units(&units)
                    }
                    Err(_) => GameClient::simple_battle_log().to_string(),
                }
            } else {
                GameClient::simple_battle_log().to_string()
            };
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            if client
                .request(
                    "/api/exploration_battle/finalize_stage_for_user_v4",
                    json!({ "autoMode": 1, "battleLog": battle_log, "result": 1 }),
                )
                .await
                .is_ok()
            {
                logs.push(format!("battle mission point {fpid}"));
            }
        }
    }
    if logs.is_empty() {
        Err(CoreError::Skip(
            "战斗任务：无可自动完成内容，已跳过".into(),
        ))
    } else {
        Ok(logs.join("\n"))
    }
}

/// 领取任务（①）。
///
/// - **成功：** 至少一批 receive 成功  
/// - **跳过：** 四类任务均无可领；单批 18044 记日志后继续其它类型  
/// - **失败：** 非 Skip 的真异常  
/// W3 R5 · L13
pub async fn mission(client: &mut GameClient) -> Result<String> {
    let mission_mst: HashMap<i64, Value> = client
        .mst_list("/api/mst/get_mission_mst_list")
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|m| {
            let id = j_i64(&m, "missionMstId");
            (id != 0).then_some((id, m))
        })
        .collect();
    let mut logs = Vec::new();
    let mut total = 0;
    for mission_type in 1..=4 {
        let mission = client
            .request(
                "/api/mission/get_mission_data_list",
                json!({ "missionType": mission_type }),
            )
            .await?;
        let mut to_receive = Vec::new();
        let mut titles = Vec::new();
        if let Some(list) = mission.get("missionDataList").and_then(|v| v.as_array()) {
            for m in list {
                let mid = j_i64(m, "missionMstId");
                let Some(mst) = mission_mst.get(&mid) else {
                    continue;
                };
                if !j_bool(m, "isClear") && j_i64(m, "count") >= j_i64(mst, "conditionCount") {
                    to_receive.push(mid);
                    titles.push(j_str(mst, "title"));
                }
            }
        }
        if to_receive.is_empty() {
            continue;
        }
        match client
            .request(
                "/api/mission/receive",
                json!({ "missionMstIds": to_receive }),
            )
            .await
        {
            Ok(_) => {
                total += to_receive.len();
                for title in titles {
                    logs.push(format!("已领取 {title}"));
                }
            }
            Err(CoreError::Skip(m)) => {
                // 18044 等：本批无可领，继续其它 missionType
                logs.push(format!("任务类型{mission_type}：{m}"));
            }
            Err(e) => return Err(e),
        }
    }
    if total == 0 {
        if logs.is_empty() {
            return Err(CoreError::Skip("没有可领取的任务，已跳过".into()));
        }
        return Err(CoreError::Skip(format!(
            "没有成功领取的任务\n{}",
            logs.join("\n")
        )));
    }
    Ok(format!("任务领取 {total} 项\n{}", logs.join("\n")))
}

pub async fn present(client: &mut GameClient) -> Result<String> {
    let data = client
        .request("/api/present/get_present_data_list", json!({}))
        .await?;
    let list = data
        .get("presentDataList")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if list.is_empty() {
        return Err(CoreError::Skip("礼物箱为空，已跳过".into()));
    }
    let ids: Vec<i64> = list.iter().map(|p| j_i64(p, "presentDataId")).collect();
    let cnt = ids.len();
    client
        .request("/api/present/receive", json!({ "presentDataIds": ids }))
        .await?;
    Ok(format!("已领取礼物 {cnt} 件"))
}

/// 工具：完成已通关篇章迷宫中的隐藏事件（eventType=21）。
///
/// # 原理（对照 Python `clear_dungeon_event`）
/// 1. 拉 field/stratum/point/dungeon_event mst 与探索收藏进度。  
/// 2. 仅处理 **已通关** 且 **difficulty≠4** 的篇章。  
/// 3. 对迷宫点（pointType=1）到达 → dungeon_start → 逐事件 OccurDungeonEvent → dungeon_goal。  
/// 4. 无可做事件 → Skip。
///
/// Docs: `docs/MODULES.md` · archive `tool.py` · TOOL-PORT  
/// Outbound: `crates/rustmadoka-core/src/modules/daily.rs::clear_dungeon_event`
pub async fn clear_dungeon_event(client: &mut GameClient) -> Result<String> {
    let stratum = client
        .mst_list("/api/mst/get_field_stratum_mst_list")
        .await
        .unwrap_or_default();
    let point = client
        .mst_list("/api/mst/get_field_point_mst_list")
        .await
        .unwrap_or_default();
    let field_mst = client
        .mst_list("/api/mst/get_field_stage_mst_list")
        .await
        .unwrap_or_default();
    let dungeon_event_mst = client
        .mst_list("/api/mst/get_dungeon_event_mst_list")
        .await
        .unwrap_or_default();

    let top = client
        .request(
            "/api/exploration/get_field_stage_collection_info_list",
            json!({}),
        )
        .await?;
    let cleared_field: HashSet<i64> = top
        .get("fieldStageCollectionInfoList")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter(|x| j_bool(x, "isClear"))
                .map(|x| j_i64(x, "fieldStageMstId"))
                .collect()
        })
        .unwrap_or_default();

    let mut logs: Vec<String> = Vec::new();
    let mut events_done = 0usize;

    for field in &field_mst {
        let field_id = j_i64(field, "fieldStageMstId");
        let difficulty = j_i64(field, "difficulty");
        let field_name = field
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        if difficulty == 4 {
            continue;
        }
        if !cleared_field.contains(&field_id) {
            logs.push(format!("跳过未通关篇章 {field_id} ({field_name})"));
            continue;
        }

        let top_info = match client
            .request(
                "/api/exploration/get_top_info_v4",
                json!({ "fieldStageMstId": field_id }),
            )
            .await
        {
            Ok(v) => v,
            Err(e) => {
                logs.push(format!("篇章 {field_id} 取 top 失败：{e}"));
                continue;
            }
        };
        let cleared_event: HashSet<i64> = top_info
            .get("fieldStageUserData")
            .and_then(|u| u.get("clearDungeonEventMstIdCsv"))
            .and_then(|v| v.as_str())
            .map(|csv| {
                csv.split(',')
                    .filter_map(|s| s.trim().parse::<i64>().ok())
                    .filter(|n| *n != 0)
                    .collect()
            })
            .unwrap_or_default();

        let stratums: Vec<&Value> = stratum
            .iter()
            .filter(|s| j_i64(s, "fieldStageMstId") == field_id)
            .collect();
        for s in stratums {
            let stratum_id = j_i64(s, "fieldStratumMstId");
            let points: Vec<&Value> = point
                .iter()
                .filter(|p| j_i64(p, "fieldStratumMstId") == stratum_id)
                .collect();
            for p in points {
                if j_i64(p, "pointType") != 1 {
                    continue;
                }
                let dungeon_id = j_i64(p, "pointValue1");
                let point_id = j_i64(p, "fieldPointMstId");
                let point_name = p.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                let dungeon_events: Vec<&Value> = dungeon_event_mst
                    .iter()
                    .filter(|x| {
                        j_i64(x, "dungeonMstId") == dungeon_id && j_i64(x, "eventType") == 21
                    })
                    .collect();
                if dungeon_events.is_empty() {
                    continue;
                }
                if dungeon_events
                    .iter()
                    .all(|x| cleared_event.contains(&j_i64(x, "dungeonEventMstId")))
                {
                    logs.push(format!(
                        "跳过已完成全部事件的点 {point_id} ({field_name}-{point_name})"
                    ));
                    continue;
                }

                if let Err(e) = client
                    .request(
                        "/api/exploration/reach_field_point",
                        json!({ "fieldPointMstId": point_id }),
                    )
                    .await
                {
                    logs.push(format!("到达点 {point_id} 失败：{e}"));
                    continue;
                }
                logs.push(format!("到达点 {point_id} ({field_name}-{point_name})"));

                if let Err(e) = client
                    .request(
                        "/api/exploration/dungeon_start",
                        json!({
                            "fieldStageMstId": field_id,
                            "dungeonMstId": dungeon_id
                        }),
                    )
                    .await
                {
                    logs.push(format!("迷宫开始失败 point={point_id}：{e}"));
                    continue;
                }

                for event in &dungeon_events {
                    let eid = j_i64(event, "dungeonEventMstId");
                    if cleared_event.contains(&eid) {
                        continue;
                    }
                    match client
                        .request(
                            "/api/exploration/occur_dungeon_event",
                            json!({
                                "fieldStageMstId": field_id,
                                "dungeonEventMstId": eid
                            }),
                        )
                        .await
                    {
                        Ok(_) => {
                            events_done += 1;
                            logs.push(format!(
                                "完成事件 {eid} ({field_name}-{point_name}-事件{eid})"
                            ));
                        }
                        Err(e) => {
                            logs.push(format!("事件 {eid} 失败：{e}"));
                        }
                    }
                }

                if let Err(e) = client
                    .request(
                        "/api/exploration/dungeon_goal",
                        json!({
                            "fieldStageMstId": field_id,
                            "dungeonMstId": dungeon_id
                        }),
                    )
                    .await
                {
                    logs.push(format!("迷宫结算失败 dungeon={dungeon_id}：{e}"));
                }
            }
        }
    }

    if events_done == 0 {
        if logs.is_empty() {
            return Err(CoreError::Skip(
                "迷宫隐藏事件：无可处理篇章或事件，已跳过".into(),
            ));
        }
        return Err(CoreError::Skip(format!(
            "迷宫隐藏事件：未完成新事件\n{}",
            logs.join("\n")
        )));
    }
    Ok(format!(
        "迷宫隐藏事件完成 {events_done} 个\n{}",
        logs.join("\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::{loginbonus_outcome_from_home, training_skip_priority};
    use crate::error::CoreError;
    use serde_json::json;

    #[test]
    fn training_skip_prefers_kioku_and_magic_over_crystalis() {
        assert!(training_skip_priority(101) > training_skip_priority(403));
        assert!(training_skip_priority(201) > training_skip_priority(403));
        assert!(training_skip_priority(403) < 50);
    }

    #[test]
    fn loginbonus_empty_list_is_skip() {
        let home = json!({ "loginBonusDataList": [] });
        match loginbonus_outcome_from_home(&home) {
            Err(CoreError::Skip(m)) => assert!(m.contains("已领完") || m.contains("不存在")),
            other => panic!("expected Skip, got {other:?}"),
        }
    }

    #[test]
    fn loginbonus_nonempty_is_success_with_summary() {
        let home = json!({
            "loginBonusDataList": [{
                "loginBonusMstId": 60,
                "loginBonusRewardMstId": 60002,
                "dayCount": 2
            }]
        });
        let s = loginbonus_outcome_from_home(&home).expect("ok");
        assert!(s.contains("已领取登录奖励"));
        assert!(s.contains("60"));
        assert!(s.contains("60002"));
    }

    #[test]
    fn loginbonus_missing_field_is_skip() {
        let home = json!({ "viewData": {} });
        assert!(matches!(
            loginbonus_outcome_from_home(&home),
            Err(CoreError::Skip(_))
        ));
    }
}
