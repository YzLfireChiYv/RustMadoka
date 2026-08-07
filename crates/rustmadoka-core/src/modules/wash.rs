//! 快速洗词条（工具 · 原版 tool 区 super_wash）。
//!
//! # 职责
//! - 目标 style 来自 **mst 全表**（character ∩ figure ∩ style），非账号持有列表（L3）
//! - 未持有 style 应 Abort；逐轮 ProgressEvent（NDJSON 流）
//!
//! # 文档
//! - `docs/tech/WASH_CHARACTER_LIST.md` · `docs/tech/PHASE_R2_MODULE_PARITY.md` U1
//! - `docs/tech/AUTOMADOKA_RESEARCH_AND_RUST_GAP.md` §3.4
//!
//! # 对照
//! `archive/.../module/modules/wash.py`

use crate::client::GameClient;
use crate::error::{CoreError, Result};
use super::progress::{emit, ProgressEvent, ProgressTx};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

/// UI：目标角色下拉（全 mst 拼装）
pub fn style_choices(client: &GameClient) -> Vec<(String, i64)> {
    let mut char_dict: HashMap<i64, String> = HashMap::new();
    for c in &client.mst.character_list {
        if let (Some(id), Some(name)) = (
            c.get("characterMstId").and_then(|v| v.as_i64()),
            c.get("name").and_then(|v| v.as_str()),
        ) {
            char_dict.insert(id, name.to_string());
        }
    }
    let mut figure_dict: HashMap<i64, String> = HashMap::new();
    for f in &client.mst.figure_list {
        if let (Some(fid), Some(cid)) = (
            f.get("styleFigureMstId").and_then(|v| v.as_i64()),
            f.get("characterMstId").and_then(|v| v.as_i64()),
        ) {
            let name = char_dict
                .get(&cid)
                .cloned()
                .unwrap_or_else(|| format!("未知角色({cid})"));
            figure_dict.insert(fid, name);
        }
    }
    let mut out = Vec::new();
    out.push(("".into(), 0));
    for s in &client.mst.style_list {
        let sid = s.get("styleMstId").and_then(|v| v.as_i64()).unwrap_or(0);
        let sname = s.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let fid = s.get("styleFigureMstId").and_then(|v| v.as_i64()).unwrap_or(0);
        let cname = figure_dict
            .get(&fid)
            .cloned()
            .unwrap_or_else(|| format!("未知角色({fid})"));
        out.push((format!("{sid}:[{sname}]{cname}"), sid));
    }
    out
}

/// UI：副词条下拉 type==2
pub fn sub_selection_choices(client: &GameClient) -> Vec<(String, i64)> {
    let mut out = vec![("未使用".into(), 0)];
    for item in &client.mst.selection_ability_list {
        if item.get("selectionAbilityType").and_then(|v| v.as_i64()) == Some(2) {
            let id = item
                .get("selectionAbilityMstId")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            out.push((format!("{id}:{name}"), id));
        }
    }
    out
}

pub async fn run_super_wash(
    client: &mut GameClient,
    style_id: i64,
    selection_index: i64,
    repeat_times: i64,
    target_ids: &[i64],
    or_logic: bool,
) -> Result<String> {
    run_super_wash_with_progress(
        client,
        style_id,
        selection_index,
        repeat_times,
        target_ids,
        or_logic,
        &None,
    )
    .await
}

/// 带逐轮进度推送的洗词条（U1）
pub async fn run_super_wash_with_progress(
    client: &mut GameClient,
    style_id: i64,
    selection_index: i64,
    repeat_times: i64,
    target_ids: &[i64],
    or_logic: bool,
    progress: &ProgressTx,
) -> Result<String> {
    if style_id == 0 {
        return Err(CoreError::Abort("请先选择一个角色".into()));
    }
    let field = format!("subSelectionAbilityMstIds{selection_index}");
    let filter: HashSet<String> = target_ids
        .iter()
        .filter(|&&x| x != 0)
        .map(|x| x.to_string())
        .collect();

    let res = client
        .request(
            "/api/selection_ability/get_selection_ability_data_list",
            json!({}),
        )
        .await?;
    let list = res
        .get("selectionAbilityDataList")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut data: Option<Value> = list
        .into_iter()
        .find(|x| x.get("styleMstId").and_then(|v| v.as_i64()) == Some(style_id));

    let data_ref = data
        .as_ref()
        .ok_or_else(|| CoreError::Abort(format!("没有找到角色 {style_id} 的技能石数据（账号可能未持有该风格）")))?;

    let init_sub = data_ref
        .get(&field)
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let init_set: HashSet<String> = if init_sub.is_empty() {
        HashSet::new()
    } else {
        init_sub.split(',').map(|s| s.to_string()).collect()
    };
    let hit = if or_logic {
        filter.intersection(&init_set).next().is_some()
    } else {
        filter.is_subset(&init_set)
    };
    if !filter.is_empty() && hit {
        return Ok("词条已符合，无需洗练".into());
    }

    let lock_field = format!("subSelectionLocks{selection_index}");
    let main_field = format!("mainSelectionAbilityMstId{selection_index}");
    let lock_str = data_ref.get(&lock_field).and_then(|v| v.as_str()).unwrap_or("");
    let mst_id = data_ref
        .get(&main_field)
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let permanent_lock: Vec<i64> = if lock_str.is_empty() {
        vec![]
    } else {
        lock_str
            .split(',')
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let mut name_map: HashMap<i64, String> = HashMap::new();
    for x in &client.mst.selection_ability_list {
        if let (Some(id), Some(n)) = (
            x.get("selectionAbilityMstId").and_then(|v| v.as_i64()),
            x.get("name").and_then(|v| v.as_str()),
        ) {
            name_map.insert(id, n.to_string());
        }
    }
    let style_name = client
        .mst
        .style_list
        .iter()
        .find(|s| s.get("styleMstId").and_then(|v| v.as_i64()) == Some(style_id))
        .and_then(|s| s.get("name").and_then(|v| v.as_str()))
        .unwrap_or("?")
        .to_string();

    let mut acquires: HashMap<String, u32> = HashMap::new();
    let mut logs = Vec::new();
    logs.push(format!(
        "开始洗练 style={style_id}({style_name}) 槽={selection_index} 计划次数={repeat_times} 目标={filter:?} OR={or_logic}"
    ));
    if !permanent_lock.is_empty() {
        logs.push(format!("应用永久锁定词条: {permanent_lock:?}"));
    }
    if filter.is_empty() {
        logs.push("未设目标词条：将跑满计划次数后结束（不会无限刷）".into());
    }

    emit(
        progress,
        ProgressEvent::info(
            "wash",
            "super_wash",
            "快速洗词条",
            format!("开始 style={style_id}({style_name}) 计划 {repeat_times} 次"),
        ),
    );

    let mut done_rounds = 0i64;
    for i in 0..repeat_times {
        // 对齐 Python FreqLimiter（约 5 次/秒），避免打爆服或假死
        if i > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(220)).await;
        }
        tracing::info!(round = i + 1, total = repeat_times, "wash learn");
        emit(
            progress,
            ProgressEvent::running(
                "wash",
                "super_wash",
                "快速洗词条",
                i + 1,
                repeat_times,
                format!("第 {}/{} 次洗练…", i + 1, repeat_times),
            ),
        );
        let res = client
            .request(
                "/api/selection_ability/learn_sub_selection_ability",
                json!({
                    "styleMstId": style_id,
                    "selectionAbilityNum": selection_index - 1,
                    "lockIds": [],
                    "permanentLockIds": permanent_lock,
                    "selectionAbilityMstId": mst_id
                }),
            )
            .await;
        let res = match res {
            Ok(v) => v,
            Err(e) => {
                let msg = format!("洗词条失败 round {}/{}: {e}", i + 1, repeat_times);
                logs.push(msg.clone());
                emit(
                    progress,
                    ProgressEvent::module_done(
                        "wash",
                        "super_wash",
                        "快速洗词条",
                        i + 1,
                        repeat_times,
                        "error",
                        msg,
                    ),
                );
                break;
            }
        };
        done_rounds += 1;
        // payload 可能直接是 selectionAbilityData，或包一层
        let sel = res
            .get("selectionAbilityData")
            .cloned()
            .or_else(|| {
                if res.get(&field).is_some() || res.get("styleMstId").is_some() {
                    Some(res.clone())
                } else {
                    None
                }
            })
            .unwrap_or(Value::Null);
        // 最新 selection 写回 data，供循环后扩展字段读取（当前主汇总用 acquires）
        data = Some(sel.clone());
        let _ = data.as_ref(); // 显式保留 data 路径，避免 unused_assignments 误报
        let sub = sel
            .get(&field)
            .and_then(|v| v.as_str())
            .or_else(|| sel.get(&field).and_then(|v| v.as_i64()).map(|_| ""))
            .unwrap_or("");
        // 有的响应是数组字段
        let current: HashSet<String> = if let Some(arr) = sel.get(&field).and_then(|v| v.as_array())
        {
            arr.iter()
                .filter_map(|x| {
                    x.as_i64()
                        .map(|n| n.to_string())
                        .or_else(|| x.as_str().map(|s| s.to_string()))
                })
                .collect()
        } else if sub.is_empty() {
            HashSet::new()
        } else {
            sub.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
        };
        let mut round_names = Vec::new();
        for sid in &current {
            if let Ok(id) = sid.parse::<i64>() {
                let n = name_map
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| format!("未知({id})"));
                *acquires.entry(n.clone()).or_default() += 1;
                round_names.push(n);
            }
        }
        let round_msg = format!(
            "[{}/{}] 当前副词条: {}",
            i + 1,
            repeat_times,
            if round_names.is_empty() {
                "(空/未解析到字段)".into()
            } else {
                round_names.join(", ")
            }
        );
        logs.push(format!("  {round_msg}"));
        emit(
            progress,
            ProgressEvent::module_done(
                "wash",
                "super_wash",
                "快速洗词条",
                i + 1,
                repeat_times,
                "success",
                round_msg,
            ),
        );
        let done = if filter.is_empty() {
            false
        } else if or_logic {
            filter.intersection(&current).next().is_some()
        } else {
            filter.is_subset(&current)
        };
        if done {
            logs.push("已洗到目标词条，提前结束".into());
            break;
        }
    }

    logs.push(format!(
        "洗练结束：实际完成 {done_rounds}/{repeat_times} 次，风格 {style_name} 汇总："
    ));
    if acquires.is_empty() {
        logs.push("  （本轮未统计到词条名；若每轮显示空，请核对 selection 槽位与响应字段）".into());
    } else {
        let mut items: Vec<_> = acquires.into_iter().collect();
        items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        for (k, c) in items {
            logs.push(format!("  - {k} x{c}"));
        }
    }
    let summary = logs.join("\n");
    emit(
        progress,
        ProgressEvent::finished("wash", true, summary.clone()),
    );
    Ok(summary)
}
