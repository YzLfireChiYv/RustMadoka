//! Shared run helpers: login, daily, module, group-raid, session export.
//!
//! # 会话
//! 全量/轻量登录经 **进程内** [`crate::session_pool`] 复用：同一游戏身份在 Owner 进程
//! 存活期间不重复 LoginApi（主人 2026-08-07 要求）。
//!
//! # 组队
//! `exec_group_raid`：支持单号打满日次数；删卡取交后自动降级（§8）。
//!
//! Docs: `docs/PLAN_RUSTMADOKA_FULL_REWRITE.md` · `docs/tech/GROUP_RAID_AND_DEVICE_IDENTITY.md`
//! · `docs/tech/CLIENT_SESSION_SIMULATION_FEASIBILITY.md`

use anyhow::{bail, Result};
use rustmadoka_core::account::{Channel, GameAccount, GroupRaidPanelConfig, Store};
use rustmadoka_core::client::GameClient;
use rustmadoka_core::modules::{
    daily_catalog, merge_run_config, resolve_enabled_from_store, run_daily, run_group_raid,
    run_player_info, run_single_module_with_progress, GroupRaidConfig, GroupRaidMember, ProgressTx,
    RoomOpenMode,
};
use rustmadoka_core::{write_session_export, SessionExportMeta, SessionExportOptions};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

use crate::fp_load::load_fp;
use crate::session_pool::{process_pool, should_drop_session};
use crate::task_gate::TaskGate;
use crate::wire_scope::WireScope;

pub fn catalog_default_pairs() -> Vec<(String, bool)> {
    daily_catalog()
        .iter()
        .map(|e| (e.key.to_string(), false))
        .collect()
}

pub fn all_modules_enabled_map() -> HashMap<String, bool> {
    daily_catalog()
        .iter()
        .map(|e| (e.key.to_string(), true))
        .collect()
}

pub fn safe_raid_damage_config() -> HashMap<String, Value> {
    let mut m = HashMap::new();
    m.insert("start_raid_damage_min".into(), json!(1));
    m.insert("start_raid_damage_max".into(), json!(10000));
    m.insert("support_raid_damage_min".into(), json!(1));
    m.insert("support_raid_damage_max".into(), json!(10000));
    m
}

pub fn default_party_config(party: &str) -> HashMap<String, Value> {
    let mut m = HashMap::new();
    m.insert("start_raid_party".into(), json!(party));
    m.insert("support_raid_party".into(), json!(party));
    m.insert("force_battle_team".into(), json!(party));
    m
}

/// 旁路缓存：`RustMadoka_data/cache/parties/{game_id_hash}.json`
pub fn party_cache_path(data_dir: &Path, channel: &str, migration_code: &str) -> std::path::PathBuf {
    let gid = TaskGate::game_id_hash(channel, migration_code);
    data_dir.join("cache").join("parties").join(format!("{gid}.json"))
}

pub fn load_party_cache(data_dir: &Path, channel: &str, migration_code: &str) -> Option<Value> {
    let p = party_cache_path(data_dir, channel, migration_code);
    let raw = std::fs::read_to_string(p).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save_party_cache(
    data_dir: &Path,
    channel: &str,
    migration_code: &str,
    parties: &[Value],
) -> Result<()> {
    let p = party_cache_path(data_dir, channel, migration_code);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let doc = json!({
        "schema": 1,
        "channel": channel,
        "game_id_hash": TaskGate::game_id_hash(channel, migration_code),
        "fetched_at": chrono::Utc::now().to_rfc3339(),
        "parties": parties,
    });
    std::fs::write(p, serde_json::to_string_pretty(&doc)?)?;
    Ok(())
}

// --- 关卡 ID ↔ 名称（mst 产品化）-------------------------------------------
// Docs: docs/tech/BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS.md · DATA_AND_MST.md
// CLI: `mst quest-stages` · HTTP: `/api/accounts/:alias/mst/quest-stages`

/// 旁路缓存：`RustMadoka_data/cache/mst/{channel}/quest_stage.json`
pub fn quest_stage_cache_path(data_dir: &Path, channel: &str) -> std::path::PathBuf {
    let ch = Channel::from_user(channel).as_str().to_string();
    data_dir
        .join("cache")
        .join("mst")
        .join(ch)
        .join("quest_stage.json")
}

/// 读取本地关卡对照缓存（不登录）。无文件返回 None。
pub fn load_quest_stage_cache(data_dir: &Path, channel: &str) -> Option<Value> {
    let p = quest_stage_cache_path(data_dir, channel);
    let raw = std::fs::read_to_string(p).ok()?;
    serde_json::from_str(&raw).ok()
}

/// 将摘要关卡列表写入数据文件夹（渠道级共享，非账号密钥）。
pub fn save_quest_stage_cache(
    data_dir: &Path,
    channel: &str,
    stages_full: &[Value],
) -> Result<Value> {
    let ch = Channel::from_user(channel);
    let summaries: Vec<Value> = stages_full
        .iter()
        .map(rustmadoka_core::summarize_quest_stage)
        .collect();
    let p = quest_stage_cache_path(data_dir, ch.as_str());
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let doc = json!({
        "schema": 1,
        "channel": ch.as_str(),
        "table": "quest_stage",
        "source": "live_mst",
        "fetched_at": chrono::Utc::now().to_rfc3339(),
        "count": summaries.len(),
        "stages": summaries,
    });
    std::fs::write(&p, serde_json::to_string(&doc)?)?;
    Ok(json!({
        "ok": true,
        "path": p.display().to_string(),
        "count": summaries.len(),
        "channel": ch.as_str(),
    }))
}

/// 登录后拉 `get_quest_stage_mst_list`，写入缓存，返回可过滤结果。
///
/// # 参数
/// - `refresh`：true 时强制对服拉取；false 时若缓存已有则只用缓存
/// - `id` / `group_id` / `name_contains` / `limit`：过滤（limit=0 不截断）
pub async fn query_quest_stages(
    data_dir: &Path,
    fp_url: &str,
    acc: &GameAccount,
    refresh: bool,
    id: Option<i64>,
    group_id: Option<i64>,
    name_contains: Option<&str>,
    limit: usize,
) -> Result<Value> {
    let ch = Channel::from_user(&acc.channel);
    let stages: Vec<Value> = if !refresh {
        if let Some(doc) = load_quest_stage_cache(data_dir, ch.as_str()) {
            if let Some(arr) = doc.get("stages").and_then(|v| v.as_array()) {
                if !arr.is_empty() {
                    let filtered = rustmadoka_core::filter_quest_stages(
                        arr,
                        id,
                        group_id,
                        name_contains,
                        limit,
                    );
                    return Ok(json!({
                        "ok": true,
                        "channel": ch.as_str(),
                        "source": "cache",
                        "cache_fetched_at": doc.get("fetched_at"),
                        "total_in_source": arr.len(),
                        "count": filtered.len(),
                        "stages": filtered,
                    }));
                }
            }
        }
        // 无缓存则对服
        fetch_quest_stages_live(data_dir, fp_url, acc).await?
    } else {
        fetch_quest_stages_live(data_dir, fp_url, acc).await?
    };
    let filtered =
        rustmadoka_core::filter_quest_stages(&stages, id, group_id, name_contains, limit);
    Ok(json!({
        "ok": true,
        "channel": ch.as_str(),
        "source": if refresh { "live_refresh" } else { "live" },
        "total_in_source": stages.len(),
        "count": filtered.len(),
        "stages": filtered,
    }))
}

/// 仅读缓存查询（不登录、不需要账号）。渠道 en/jp/tw。
pub fn query_quest_stages_from_cache(
    data_dir: &Path,
    channel: &str,
    id: Option<i64>,
    group_id: Option<i64>,
    name_contains: Option<&str>,
    limit: usize,
) -> Result<Value> {
    let ch = Channel::from_user(channel);
    let Some(doc) = load_quest_stage_cache(data_dir, ch.as_str()) else {
        bail!(
            "无本地关卡对照缓存（{}）。请先用账号执行：mst quest-stages -g <组> -a <别名> --refresh",
            quest_stage_cache_path(data_dir, ch.as_str()).display()
        );
    };
    let arr = doc
        .get("stages")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let filtered =
        rustmadoka_core::filter_quest_stages(&arr, id, group_id, name_contains, limit);
    Ok(json!({
        "ok": true,
        "channel": ch.as_str(),
        "source": "cache_only",
        "cache_fetched_at": doc.get("fetched_at"),
        "total_in_source": arr.len(),
        "count": filtered.len(),
        "stages": filtered,
    }))
}

async fn fetch_quest_stages_live(
    data_dir: &Path,
    fp_url: &str,
    acc: &GameAccount,
) -> Result<Vec<Value>> {
    let ch = Channel::from_user(&acc.channel);
    let fut = async {
        let fp = load_fp(data_dir, fp_url, ch.as_str()).await?;
        let pool = process_pool();
        let (key, mut client, kind) = pool
            .acquire_full(ch.as_str(), &acc.username, &acc.password, fp, data_dir)
            .await?;
        let run = async {
            let list = client
                .mst_list("/api/mst/get_quest_stage_mst_list")
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let _ = save_quest_stage_cache(data_dir, ch.as_str(), &list)?;
            Ok::<Vec<Value>, anyhow::Error>(list)
        }
        .await;
        let drop = run.as_ref().err().map(should_drop_session).unwrap_or(false);
        pool.release(key, kind, client, drop).await;
        run
    };
    match tokio::time::timeout(std::time::Duration::from_secs(180), fut).await {
        Ok(r) => r,
        Err(_) => bail!("quest stage mst timeout (180s)"),
    }
}

pub async fn fetch_account_info(
    data_dir: &Path,
    fp_url: &str,
    acc: &GameAccount,
) -> Result<Value> {
    let fut = async {
        let ch = Channel::from_user(&acc.channel);
        let fp = load_fp(data_dir, fp_url, ch.as_str()).await?;
        let pool = process_pool();
        let (key, mut client, kind) = pool
            .acquire_light(ch.as_str(), &acc.username, &acc.password, fp, data_dir)
            .await?;
        let run = async {
            let _ = run_player_info(&mut client).await;
            let (name, level) = client.user_name_level();
            let max_power = client
                .init_data
                .pointer("/userParamData/maxPartyPower")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let money = client
                .init_data
                .pointer("/userParamData/money")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let stamina = client.stamina();
            let parties = client.party_summaries();
            if !parties.is_empty() {
                let _ = save_party_cache(data_dir, ch.as_str(), &acc.username, &parties);
            }
            if name.is_empty() && level == 0 {
                bail!("empty player name/level after login");
            }
            Ok(json!({
                "name": name,
                "level": level,
                "max_power": max_power,
                "money": money,
                "stamina": stamina,
                "alias": acc.alias,
                "channel": acc.channel,
                "party_count": parties.len(),
            }))
        }
        .await;
        let drop = run.as_ref().err().map(should_drop_session).unwrap_or(false);
        pool.release(key, kind, client, drop).await;
        run
    };
    match tokio::time::timeout(std::time::Duration::from_secs(75), fut).await {
        Ok(r) => r,
        Err(_) => bail!("info timeout (75s)"),
    }
}

pub async fn refresh_party_list(
    data_dir: &Path,
    fp_url: &str,
    acc: &GameAccount,
) -> Result<Value> {
    let fut = async {
        let ch = Channel::from_user(&acc.channel);
        let fp = load_fp(data_dir, fp_url, ch.as_str()).await?;
        let pool = process_pool();
        let (key, client, kind) = pool
            .acquire_full(ch.as_str(), &acc.username, &acc.password, fp, data_dir)
            .await?;
        let parties = client.party_summaries();
        let out = save_party_cache(data_dir, ch.as_str(), &acc.username, &parties).map(|_| {
            json!({
                "ok": true,
                "party_count": parties.len(),
                "parties": parties,
                "fetched_at": chrono::Utc::now().to_rfc3339(),
                "note": if parties.is_empty() {
                    "登录成功但 partyDataList 为空；请确认账号已有编成，或稍后再试"
                } else {
                    "已缓存队伍列表"
                },
            })
        });
        let drop = out.as_ref().err().map(should_drop_session).unwrap_or(false);
        pool.release(key, kind, client, drop).await;
        out
    };
    match tokio::time::timeout(std::time::Duration::from_secs(120), fut).await {
        Ok(r) => r,
        Err(_) => bail!("party refresh timeout (120s)"),
    }
}

pub async fn run_account_daily(
    data_dir: &Path,
    fp_url: &str,
    acc: &GameAccount,
    request_enabled: &HashMap<String, bool>,
    request_config: &HashMap<String, Value>,
) -> Result<rustmadoka_core::DailyReport> {
    let ch = Channel::from_user(&acc.channel);
    let _wire = WireScope::enter(data_dir, &acc.alias, ch.as_str(), "daily");
    rustmadoka_core::wire::record_probe(
        "daily_begin",
        json!({ "alias": acc.alias, "channel": ch.as_str() }),
    );
    let fp = load_fp(data_dir, fp_url, ch.as_str()).await?;
    let pool = process_pool();
    let (key, mut client, kind) = pool
        .acquire_full(ch.as_str(), &acc.username, &acc.password, fp, data_dir)
        .await?;
    let enabled =
        resolve_enabled_from_store(&catalog_default_pairs(), &acc.config, request_enabled);
    let config = merge_run_config(&acc.config, request_config);
    let report = run_daily(&mut client, &enabled, &config).await;
    // 若本趟已拉过关卡 mst，顺带写入产品缓存（CLI/网页查 ID↔名称）
    if let Some(list) = client.mst.cached_list("get_quest_stage_mst_list") {
        let _ = save_quest_stage_cache(data_dir, ch.as_str(), list);
    }
    pool.release(key, kind, client, false).await;
    rustmadoka_core::wire::record_probe(
        "daily_end",
        json!({
            "ok": report.ok,
            "success": report.success,
            "skipped": report.skipped,
            "errors": report.errors,
        }),
    );
    rustmadoka_core::wire::stop();
    Ok(report)
}

pub async fn run_account_module(
    data_dir: &Path,
    fp_url: &str,
    acc: &GameAccount,
    key: &str,
    request_config: &HashMap<String, Value>,
) -> Result<Value> {
    run_account_module_with_progress(
        data_dir,
        fp_url,
        acc,
        key,
        request_config,
        &None,
        std::time::Instant::now(),
    )
    .await
}

/// 单模块 + 进度通道（HTTP 流式 / RunHub 实时用）。
pub async fn run_account_module_with_progress(
    data_dir: &Path,
    fp_url: &str,
    acc: &GameAccount,
    key: &str,
    request_config: &HashMap<String, Value>,
    progress: &ProgressTx,
    t0: std::time::Instant,
) -> Result<Value> {
    let ch = Channel::from_user(&acc.channel);
    let _wire = WireScope::enter(data_dir, &acc.alias, ch.as_str(), key);
    rustmadoka_core::wire::set_module_key(Some(key));
    rustmadoka_core::wire::record_probe(
        "module_begin",
        json!({ "alias": acc.alias, "key": key, "channel": ch.as_str() }),
    );
    let fp = load_fp(data_dir, fp_url, ch.as_str()).await?;
    let config = merge_run_config(&acc.config, request_config);
    let pool = process_pool();
    let (skey, mut client, skind) = pool
        .acquire_full(ch.as_str(), &acc.username, &acc.password, fp, data_dir)
        .await?;
    let out = match run_single_module_with_progress(&mut client, key, &config, progress).await {
        Ok(log) => {
            rustmadoka_core::wire::record_probe(
                "module_end",
                json!({
                    "key": key,
                    "status": "success",
                    "duration_ms": t0.elapsed().as_millis() as u64,
                    "log_len": log.len(),
                }),
            );
            Ok(json!({
                "ok": true,
                "key": key,
                "status": "success",
                "log": log,
                "duration_ms": t0.elapsed().as_millis() as u64,
                "wire_active": rustmadoka_core::wire::is_active(),
                "wire_dir": rustmadoka_core::wire::current_dir().map(|p| p.display().to_string()),
            }))
        }
        Err(e) => {
            let status = match &e {
                rustmadoka_core::CoreError::Skip(_) => "skip",
                rustmadoka_core::CoreError::Abort(_) => "abort",
                _ => "error",
            };
            rustmadoka_core::wire::record_probe(
                "module_end",
                json!({
                    "key": key,
                    "status": status,
                    "duration_ms": t0.elapsed().as_millis() as u64,
                    "error": e.to_string(),
                }),
            );
            let drop = matches!(e, rustmadoka_core::CoreError::Http { status: 401, .. });
            if drop {
                pool.release(skey, skind, client, true).await;
                rustmadoka_core::wire::stop();
                return Ok(json!({
                    "ok": false,
                    "key": key,
                    "status": "error",
                    "log": e.to_string(),
                    "duration_ms": t0.elapsed().as_millis() as u64,
                    "session_dropped": true,
                }));
            }
            Ok(json!({
                "ok": status != "error",
                "key": key,
                "status": status,
                "log": e.to_string(),
                "duration_ms": t0.elapsed().as_millis() as u64,
                "wire_active": rustmadoka_core::wire::is_active(),
                "wire_dir": rustmadoka_core::wire::current_dir().map(|p| p.display().to_string()),
            }))
        }
    };
    if let Some(list) = client.mst.cached_list("get_quest_stage_mst_list") {
        let _ = save_quest_stage_cache(data_dir, ch.as_str(), list);
    }
    pool.release(skey, skind, client, false).await;
    rustmadoka_core::wire::stop();
    out
}

/// 解析组队参与名单：与现有卡片取交；删卡自动降级；同 channel。
/// Docs: GROUP_RAID §8.3
pub fn resolve_group_raid_members(
    group_accounts: &[GameAccount],
    requested_aliases: &[String],
    default_party: &str,
) -> Result<(Vec<GroupRaidMember>, Vec<String>)> {
    let mut notes = Vec::new();
    let mut members = Vec::new();
    let mut channel0: Option<String> = None;
    for alias in requested_aliases {
        let a = alias.trim();
        if a.is_empty() {
            continue;
        }
        let Some(acc) = group_accounts.iter().find(|x| x.alias == a) else {
            notes.push(format!("卡片「{a}」已不存在，已从参与名单移除"));
            continue;
        };
        let ch = Channel::from_user(&acc.channel);
        if !ch.login_implemented() {
            notes.push(format!("卡片「{a}」台服登录未实现，已跳过"));
            continue;
        }
        let chs = acc.channel.trim().to_ascii_lowercase();
        if let Some(ref c0) = channel0 {
            if &chs != c0 {
                notes.push(format!(
                    "卡片「{a}」服务器与首张参与卡不一致（{} vs {c0}），已跳过",
                    acc.channel
                ));
                continue;
            }
        } else {
            channel0 = Some(chs);
        }
        let party = if default_party.trim().is_empty() {
            acc.config
                .get("start_raid_party")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        } else {
            default_party.to_string()
        };
        members.push(GroupRaidMember {
            alias: acc.alias.clone(),
            channel: acc.channel.clone(),
            migration_code: acc.username.clone(),
            party,
        });
    }
    if members.is_empty() {
        bail!("组队 Raid：没有有效参与账号（可能卡片已全部删除）");
    }
    if members.len() == 1 {
        notes.push("单号模式：将自动打满该号今日团战召唤次数（以服务端上限为准）".into());
    }
    Ok((members, notes))
}

/// 组队团战执行：支持 1+ 人；删卡降级；单号打满日次数。
pub async fn exec_group_raid(
    data_dir: &Path,
    fp_url: &str,
    gate: &TaskGate,
    group: &str,
    group_password: Option<&str>,
    aliases_csv: &str,
    room_open: &str,
    party: &str,
    leave_after_support: bool,
) -> Result<Value> {
    let alias_list: Vec<String> = aliases_csv
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if alias_list.is_empty() {
        bail!("group-raid：请指定至少一个别名 --aliases");
    }
    let store = Store::open(data_dir)?;
    let g = store.load_group(group, group_password)?;
    let (members, degrade_notes) =
        resolve_group_raid_members(&g.accounts, &alias_list, party)?;
    let n = members.len();
    let room = RoomOpenMode::parse_for_count(room_open, n)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let mut lock_pairs: Vec<(String, String)> = Vec::new();
    for m in &members {
        let ch = Channel::from_user(&m.channel);
        lock_pairs.push((ch.as_str().to_string(), m.migration_code.clone()));
    }
    let _multi = gate
        .try_begin_many(&lock_pairs, "group_raid", group)
        .map_err(|e| anyhow::anyhow!(e))?;

    let pool = process_pool();
    let mut clients = HashMap::new();
    let mut lease: Vec<(String, crate::session_pool::SessionKind, String)> = Vec::new();
    for m in &members {
        let ch = Channel::from_user(&m.channel);
        let fp = load_fp(data_dir, fp_url, ch.as_str()).await?;
        tracing::info!(alias = %m.alias, "group_raid: acquire session");
        let password = g
            .accounts
            .iter()
            .find(|a| a.alias == m.alias)
            .map(|a| a.password.as_str())
            .unwrap_or("");
        let (skey, client, skind) = pool
            .acquire_full(ch.as_str(), &m.migration_code, password, fp, data_dir)
            .await?;
        lease.push((skey, skind, m.alias.clone()));
        clients.insert(m.alias.clone(), client);
    }

    let cfg = GroupRaidConfig {
        owner_group: group.to_string(),
        room_open: room,
        leave_after_support,
        battle_result: 3,
        prefer_stamina_recover: true,
        members,
    };
    let report = run_group_raid(&mut clients, &cfg)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()));
    let drop_sess = report
        .as_ref()
        .err()
        .map(should_drop_session)
        .unwrap_or(false);
    for (skey, skind, alias) in lease {
        if let Some(client) = clients.remove(&alias) {
            pool.release(skey, skind, client, drop_sess).await;
        }
    }
    let mut v = serde_json::to_value(&report?)?;
    if let Some(obj) = v.as_object_mut() {
        if !degrade_notes.is_empty() {
            obj.insert("degrade_notes".into(), json!(degrade_notes));
        }
        obj.insert("solo_mode".into(), json!(n == 1));
        obj.insert("effective_member_count".into(), json!(n));
    }
    Ok(v)
}

/// 读取用户组组队配置列表（多卡片）+ 现有游戏账号别名。
/// Docs: PLAN_GROUP_RAID_UI · GROUP_RAID §8.1
pub fn load_group_raid_panel(
    data_dir: &Path,
    group: &str,
    group_password: Option<&str>,
) -> Result<Value> {
    let store = Store::open(data_dir)?;
    let g = store.load_group(group, group_password)?;
    let existing: Vec<String> = g.accounts.iter().map(|a| a.alias.clone()).collect();
    let entries: Vec<Value> = g
        .group_raid
        .entries
        .iter()
        .map(|e| {
            let mut aliases = e.aliases.clone();
            aliases.retain(|a| existing.iter().any(|x| x == a));
            json!({
                "id": e.id,
                "name": e.name,
                "aliases": aliases,
                "room_open": e.room_open,
                "party": e.party,
                "leave_after_support": e.leave_after_support,
            })
        })
        .collect();
    Ok(json!({
        "ok": true,
        "group": group,
        "entries": entries,
        // 兼容旧前端：第一份配置镜像到 config
        "config": entries.first().cloned().unwrap_or_else(|| json!({
            "id": "",
            "name": "",
            "aliases": [],
            "room_open": "",
            "party": "",
            "leave_after_support": false,
        })),
        "available_aliases": existing,
        "accounts": g.accounts.iter().map(|a| json!({
            "alias": a.alias,
            "channel": a.channel,
        })).collect::<Vec<_>>(),
    }))
}

/// 全量替换组队配置列表。
pub fn save_group_raid_panel(
    data_dir: &Path,
    group: &str,
    group_password: Option<&str>,
    cfg: GroupRaidPanelConfig,
) -> Result<Value> {
    let store = Store::open(data_dir)?;
    let mut g = store.load_group(group, group_password)?;
    let existing: Vec<String> = g.accounts.iter().map(|a| a.alias.clone()).collect();
    let mut panel = GroupRaidPanelConfig::default();
    for mut e in cfg.entries {
        e.aliases.retain(|a| existing.iter().any(|x| x == a));
        if e.aliases.is_empty() {
            bail!("组队配置「{}」：请至少保留 1 个有效游戏账号", e.name);
        }
        let mut ch0: Option<String> = None;
        for al in &e.aliases {
            let acc = g
                .accounts
                .iter()
                .find(|a| &a.alias == al)
                .ok_or_else(|| anyhow::anyhow!("内部：别名丢失 {al}"))?;
            let ch = acc.channel.trim().to_ascii_lowercase();
            if let Some(ref c0) = ch0 {
                if &ch != c0 {
                    bail!("组队配置「{}」：参与账号必须同一服务器", e.name);
                }
            } else {
                ch0 = Some(ch);
            }
        }
        RoomOpenMode::parse_for_count(&e.room_open, e.aliases.len())
            .map_err(|err| anyhow::anyhow!(err.to_string()))?;
        if e.id.trim().is_empty() {
            e.id = format!("gr_{}", uuid_simple());
        }
        if e.name.trim().is_empty() {
            e.name = "组队配置".into();
        }
        panel.entries.push(e);
    }
    g.group_raid = panel;
    store.save_group(&g)?;
    load_group_raid_panel(data_dir, group, group_password)
}

/// 新建或更新单张组队配置卡。
pub fn upsert_group_raid_entry(
    data_dir: &Path,
    group: &str,
    group_password: Option<&str>,
    mut entry: rustmadoka_core::GroupRaidConfigEntry,
) -> Result<Value> {
    let store = Store::open(data_dir)?;
    let mut g = store.load_group(group, group_password)?;
    let existing: Vec<String> = g.accounts.iter().map(|a| a.alias.clone()).collect();
    entry.aliases.retain(|a| existing.iter().any(|x| x == a));
    if entry.aliases.is_empty() {
        bail!("组队配置：请至少选择 1 个仍存在的游戏账号");
    }
    // 同 channel
    let mut ch0: Option<String> = None;
    for al in &entry.aliases {
        let acc = g
            .accounts
            .iter()
            .find(|a| &a.alias == al)
            .ok_or_else(|| anyhow::anyhow!("内部：别名丢失 {al}"))?;
        let ch = acc.channel.trim().to_ascii_lowercase();
        if let Some(ref c0) = ch0 {
            if &ch != c0 {
                bail!("组队配置：参与账号必须同一服务器（「{al}」与首张不一致）");
            }
        } else {
            ch0 = Some(ch);
        }
    }
    let n = entry.aliases.len();
    // 与运行时同一套开放范围校验（多号禁止仅自己；未选多号拒绝）
    RoomOpenMode::parse_for_count(&entry.room_open, n)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    if entry.id.trim().is_empty() {
        entry.id = format!("gr_{}", uuid_simple());
    }
    if entry.name.trim().is_empty() {
        entry.name = "组队配置".into();
    }
    g.group_raid.upsert(entry.clone());
    store.save_group(&g)?;
    Ok(json!({
        "ok": true,
        "entry": {
            "id": entry.id,
            "name": entry.name,
            "aliases": entry.aliases,
            "room_open": entry.room_open,
            "party": entry.party,
            "leave_after_support": entry.leave_after_support,
        },
    }))
}

/// 删除一张组队配置卡。
pub fn delete_group_raid_entry(
    data_dir: &Path,
    group: &str,
    group_password: Option<&str>,
    id: &str,
) -> Result<Value> {
    let id = id.trim();
    if id.is_empty() {
        bail!("组队配置：删除需要 id");
    }
    let store = Store::open(data_dir)?;
    let mut g = store.load_group(group, group_password)?;
    if !g.group_raid.remove(id) {
        bail!("组队配置不存在：{id}");
    }
    store.save_group(&g)?;
    Ok(json!({ "ok": true, "deleted": id }))
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{t:x}")
}

/// 按配置 id 执行组队（删卡降级用配置内 aliases）。
pub async fn exec_group_raid_by_config_id(
    data_dir: &Path,
    fp_url: &str,
    gate: &TaskGate,
    group: &str,
    group_password: Option<&str>,
    config_id: &str,
) -> Result<Value> {
    let store = Store::open(data_dir)?;
    let g = store.load_group(group, group_password)?;
    let entry = g
        .group_raid
        .find(config_id)
        .ok_or_else(|| anyhow::anyhow!("组队配置不存在：{config_id}"))?
        .clone();
    if entry.aliases.is_empty() {
        bail!("组队配置「{}」没有参与别名（卡片可能都已删除，请重新编辑）", entry.name);
    }
    let aliases = entry.aliases.join(",");
    let mut v = exec_group_raid(
        data_dir,
        fp_url,
        gate,
        group,
        group_password,
        &aliases,
        &entry.room_open,
        &entry.party,
        entry.leave_after_support,
    )
    .await?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert("config_id".into(), json!(entry.id));
        obj.insert("config_name".into(), json!(entry.name));
    }
    Ok(v)
}

pub async fn export_account_session(
    data_dir: &Path,
    fp_url: &str,
    gate: Option<&TaskGate>,
    group: &str,
    group_password: Option<&str>,
    alias: &str,
    out: Option<&Path>,
) -> Result<Value> {
    let store = Store::open(data_dir)?;
    let g = store.load_group(group, group_password)?;
    let acc = g
        .accounts
        .iter()
        .find(|a| a.alias == alias)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("game account not found: {alias}"))?;
    let ch = Channel::from_user(&acc.channel);
    let _guard = if let Some(gate) = gate {
        Some(
            gate.try_begin_owned(ch.as_str(), &acc.username, "export", group)
                .map_err(|e| anyhow::anyhow!(e))?,
        )
    } else {
        None
    };
    let fp = load_fp(data_dir, fp_url, ch.as_str()).await?;
    let pool = process_pool();
    let (skey, client, skind) = pool
        .acquire_full(ch.as_str(), &acc.username, &acc.password, fp, data_dir)
        .await?;
    let exports_root = out
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| data_dir.join("exports"));
    let meta = SessionExportMeta {
        group: group.to_string(),
        alias: alias.to_string(),
        channel: acc.channel.clone(),
        app_version: None,
        build_stamp: None,
    };
    let opts = SessionExportOptions::default();
    let written = write_session_export(&client, &exports_root, &meta, &opts)
        .map_err(|e| anyhow::anyhow!(e.to_string()));
    let user_id = client.user_id;
    let drop = written.as_ref().err().map(should_drop_session).unwrap_or(false);
    pool.release(skey, skind, client, drop).await;
    let written = written?;
    Ok(json!({
        "ok": true,
        "dir": written.dir.display().to_string(),
        "user_id": user_id,
        "alias": acc.alias,
        "group": group,
        "channel": acc.channel,
        "manifest": written.manifest,
    }))
}
