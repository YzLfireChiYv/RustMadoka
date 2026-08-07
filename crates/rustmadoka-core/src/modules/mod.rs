//! 业务模块：日常全序 + 快速洗词条（工具区仅 wash；其余工具原版有 Rust 未移植）。
//!
//! # 对照
//! - 注册表：`archive/pre-rust-2026-08/autopcr/module/modules/__init__.py`
//! - 实现：`modules/{common,stamina,tool,raid,sweep,shop,gacha,collection,wash}.py`
//! - 原版原理与缺口：`docs/tech/AUTOMADOKA_RESEARCH_AND_RUST_GAP.md` §3 · §5
//!
//! # 文档
//! - `docs/MODULES.md` · `docs/tech/PHASE_R2_MODULE_PARITY.md` · `docs/tech/MODULES_RUNTIME.md`
//! - 语义分类：`docs/tech/MODULE_SEMANTIC_CLASSIFICATION.md`
//! - W2/W3 变更与验收：`docs/tech/W2_WIRE_ANALYSIS_AND_REWRITE_LIST.md`
//! - 失败中文：`CoreError::module_log_zh` · `docs/tech/ERROR_DIAGNOSTICS.md`
//!
//! # 一键清日常调度（相对原版 ModuleManager.do_daily）
//! 1. 顺序与 Python `daily_modules` 一致（含挂在日常下的 `super_sweep`）
//! 2. **产品默认全部关闭**（与原版多数 `@default(True)` 不同；P17/C5）
//! 3. 请求体 `enabled` / 账号 config 可覆盖；缺省 key 用目录默认（全 false）
//! 4. **Skip / Error / Abort 均不中断**整次日常（原版仅 PANIC break）
//! 5. 可选 `ProgressTx` 逐模块推送（原版无 NDJSON 流）
//! 6. **工具缺口：** cron 定时、raid_support（由组队 group-raid 上位替代）、secret/auto_register（后置）、
//!    clear_dungeon_event **已移植**（`run module --key clear_dungeon_event`）
//!
//! # 模块结果标签（当前实现 · 非完备本体）
//! 调度层展示标签：`成功` / `部分完成` / `跳过` / `中止` / `错误`。
//! `Ok(log)` 且 log 以 `【部分完成】` 开头 → **部分完成**（计入独立计数，不并入「成功」）。
//! 原版 Python 另有 `警告` / `致命`。用户整次放弃见 `RunControlFlags`。
//! 勿把标签写成「游戏全部结局」。

pub mod config_catalog;
mod daily;
pub mod group_raid;
mod progress;
mod wash;

use crate::client::GameClient;
use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub use config_catalog::{
    all_setting_defaults, flatten_for_save, merge_run_config, module_config_fields,
    module_description, resolve_enabled_from_store, shop_item_categories,
    upstream_shop_priority_patch, ConfigField,
};
pub use group_raid::{
    run_group_raid, split_group_raid_damages, GroupRaidConfig, GroupRaidMember, GroupRaidReport,
    RoomOpenMode,
};
pub use progress::{emit, ProgressEvent, ProgressTx};
pub use wash::{
    run_super_wash, run_super_wash_with_progress, style_choices, sub_selection_choices,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleResult {
    pub key: String,
    pub name: String,
    /// 展示标签：`成功` | `部分完成` | `跳过` | `中止` | `错误`

    pub status: String,
    pub log: String,
}

/// `Ok(log)` 是否为「部分完成」（log 以【部分完成】开头）
pub fn is_partial_success_log(log: &str) -> bool {
    log.trim_start().starts_with("【部分完成】")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReport {
    pub results: Vec<ModuleResult>,
    /// 无模块级「中止/错误」且用户未整次放弃时为 true；存在跳过/部分完成仍可为 true

    pub ok: bool,
    /// 实际调度的模块数

    pub total: usize,
    /// 完整成功（非部分完成）

    pub success: usize,
    /// 部分完成（至少做了一步，未满计划等）

    #[serde(default)]
    pub partial: usize,
    /// `CoreError::Skip`

    pub skipped: usize,
    /// `CoreError::Abort`

    #[serde(default)]
    pub aborted: usize,
    /// 其它 `CoreError`

    #[serde(default)]
    pub errors: usize,
    /// `aborted + errors`

    pub failed: usize,
}

impl DailyReport {
    /// 汇总短句（分开展示各档）

    pub fn summary_counts_zh(&self) -> String {
        format!(
            "成功{} 部分完成{} 跳过{} 中止{} 错误{}",
            self.success, self.partial, self.skipped, self.aborted, self.errors
        )
    }
}

/// 模块目录项（静态元数据；配置字段运行时从 config_catalog 展开）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleCatalogEntry {
    pub key: &'static str,
    pub name: &'static str,
    pub default_enabled: bool,
    /// 会消耗体力/石/门票等；UI 应提示

    pub resource_heavy: bool,
}

/// 发给前端的完整模块信息（含设置 schema）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub key: String,
    pub name: String,
    pub description: String,
    pub default_enabled: bool,
    pub resource_heavy: bool,
    /// 设置字段 schema（不含模块开关）

    pub config: Vec<ConfigField>,
    pub config_order: Vec<String>,
}

/// 日常 26 项（顺序 = 原版 daily_modules）
/// 对照: archive/.../modules/__init__.py
pub fn daily_catalog() -> &'static [ModuleCatalogEntry] {
    &[
        ModuleCatalogEntry {
            key: "loginbonus",
            name: "领取登陆奖励",
            default_enabled: false,
            resource_heavy: false,
        },
        ModuleCatalogEntry {
            key: "stamina_buy",
            name: "购买体力",
            default_enabled: false,
            resource_heavy: true,
        },
        ModuleCatalogEntry {
            key: "super_sweep",
            name: "快速刷图",
            default_enabled: false,
            resource_heavy: true,
        },
        ModuleCatalogEntry {
            key: "raid_reward",
            name: "魔女舔盒",
            default_enabled: false,
            resource_heavy: false,
        },
        ModuleCatalogEntry {
            key: "self_raid",
            name: "魔女召唤",
            default_enabled: false,
            resource_heavy: true,
        },
        ModuleCatalogEntry {
            key: "support_raid",
            name: "魔女援助",
            default_enabled: false,
            resource_heavy: true,
        },
        ModuleCatalogEntry {
            key: "like_raid",
            name: "魔女点赞",
            default_enabled: false,
            resource_heavy: false,
        },
        ModuleCatalogEntry {
            key: "solo_raid",
            name: "扫荡总力战",
            default_enabled: false,
            resource_heavy: false,
        },
        ModuleCatalogEntry {
            key: "high_score",
            name: "扫荡打分",
            default_enabled: false,
            resource_heavy: false,
        },
        ModuleCatalogEntry {
            key: "arena",
            name: "自动PVP投降",
            default_enabled: false,
            resource_heavy: true,
        },
        ModuleCatalogEntry {
            key: "basic",
            name: "智能体力扫荡",
            default_enabled: false,
            resource_heavy: true,
        },
        ModuleCatalogEntry {
            key: "event",
            name: "扫荡活动",
            default_enabled: false,
            resource_heavy: false,
        },
        ModuleCatalogEntry {
            key: "archive",
            name: "扫荡档案活动",
            default_enabled: false,
            resource_heavy: false,
        },
        // 兑换商店：消耗兑换币 → 标耗资源（主人 2026-08-06）
        ModuleCatalogEntry {
            key: "event_shop",
            name: "清空活动兑换币",
            default_enabled: false,
            resource_heavy: true,
        },
        ModuleCatalogEntry {
            key: "raid_shop",
            name: "清空raid兑换币",
            default_enabled: false,
            resource_heavy: true,
        },
        ModuleCatalogEntry {
            key: "arena_shop",
            name: "清空jjc兑换币",
            default_enabled: false,
            resource_heavy: true,
        },
        ModuleCatalogEntry {
            key: "tower",
            name: "扫荡露娜塔",
            default_enabled: false,
            resource_heavy: false,
        },
        ModuleCatalogEntry {
            key: "heart",
            name: "扫荡心之器",
            default_enabled: false,
            resource_heavy: false,
        },
        ModuleCatalogEntry {
            key: "gather",
            name: "收集宝箱",
            default_enabled: false,
            resource_heavy: false,
        },
        ModuleCatalogEntry {
            key: "freegacha",
            name: "免费扭蛋",
            default_enabled: false,
            resource_heavy: false,
        },
        ModuleCatalogEntry {
            key: "eventscenario",
            name: "阅读活动剧情",
            default_enabled: false,
            resource_heavy: false,
        },
        ModuleCatalogEntry {
            key: "collection",
            name: "阅读光之间",
            default_enabled: false,
            resource_heavy: false,
        },
        ModuleCatalogEntry {
            key: "battle_mission",
            name: "完成战斗任务",
            default_enabled: false,
            resource_heavy: false,
        },
        ModuleCatalogEntry {
            key: "mission",
            name: "领取任务",
            default_enabled: false,
            resource_heavy: false,
        },
        ModuleCatalogEntry {
            key: "present",
            name: "领取礼物",
            default_enabled: false,
            resource_heavy: false,
        },
        ModuleCatalogEntry {
            key: "info",
            name: "玩家信息",
            default_enabled: false,
            resource_heavy: false,
        },
    ]
}

/// 日常模块完整信息（含每模块设置 schema）
pub fn daily_modules_info() -> Vec<ModuleInfo> {
    daily_catalog()
        .iter()
        .map(|e| {
            let config = module_config_fields(e.key);
            let config_order: Vec<String> = config.iter().map(|f| f.key.clone()).collect();
            ModuleInfo {
                key: e.key.into(),
                name: e.name.into(),
                description: module_description(e.key).into(),
                default_enabled: e.default_enabled,
                resource_heavy: e.resource_heavy,
                config,
                config_order,
            }
        })
        .collect()
}

/// 兼容旧 API：返回 (key, name) 列表
pub fn daily_keys() -> Vec<(&'static str, &'static str)> {
    daily_catalog()
        .iter()
        .map(|e| (e.key, e.name))
        .collect()
}

/// 「仅低风险建议项」：主人 2026-08-06 — 含活动/档案/塔；不含总力战/heart/商店等
pub fn low_risk_module_keys() -> &'static [&'static str] {
    &[
        "loginbonus",
        "raid_reward",
        "like_raid",
        "high_score",
        "event",
        "archive",
        "tower",
        "gather",
        "freegacha",
        "eventscenario",
        "collection",
        "battle_mission",
        "mission",
        "present",
        "info",
    ]
}

pub fn is_low_risk_module(key: &str) -> bool {
    low_risk_module_keys().contains(&key)
}

/// 解析启用表：请求覆盖 > 目录默认
pub fn resolve_enabled(enabled: &HashMap<String, bool>) -> HashMap<String, bool> {
    let defaults: Vec<(String, bool)> = daily_catalog()
        .iter()
        .map(|e| (e.key.to_string(), e.default_enabled))
        .collect();
    resolve_enabled_from_store(&defaults, &HashMap::new(), enabled)
}

/// 仅获取玩家信息（测试期允许的唯一游戏业务）
/// 对照: daily::info · PLAN_R3 安全门禁
pub async fn run_player_info(client: &mut GameClient) -> Result<String> {
    daily::info(client).await
}

/// 协作式暂停 / 中止（模块边界检查）
/// 文档: docs/tech/UI_ROUTING_AND_TASK_LOGS.md
#[derive(Debug, Default)]
pub struct RunControlFlags {
    pub pause: std::sync::atomic::AtomicBool,
    pub abort: std::sync::atomic::AtomicBool,
}

impl RunControlFlags {
    pub fn new() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self::default())
    }
    pub fn request_pause(&self) {
        self.pause.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn request_resume(&self) {
        self.pause.store(false, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn request_abort(&self) {
        self.abort.store(true, std::sync::atomic::Ordering::SeqCst);
        self.pause.store(false, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn is_paused(&self) -> bool {
        self.pause.load(std::sync::atomic::Ordering::SeqCst)
    }
    pub fn is_aborted(&self) -> bool {
        self.abort.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// 无进度回调的一键清日常
///
/// **注意：** 调用方须先 `safety::assert_daily_allowed()`。
pub async fn run_daily(
    client: &mut GameClient,
    enabled: &HashMap<String, bool>,
    config: &HashMap<String, Value>,
) -> DailyReport {
    run_daily_with_progress(client, enabled, config, &None, None).await
}

/// 带进度推送的一键清日常
///
/// `config` 应为 **已 merge** 的扁平设置（含账号已存 + 请求覆盖 + 默认）。
/// `control`：模块间隙检查暂停/中止。见 `RunControlFlags`。
pub async fn run_daily_with_progress(
    client: &mut GameClient,
    enabled: &HashMap<String, bool>,
    config: &HashMap<String, Value>,
    progress: &ProgressTx,
    control: Option<std::sync::Arc<RunControlFlags>>,
) -> DailyReport {
    let resolved = resolve_enabled(enabled);
    let catalog = daily_catalog();
    let scheduled: Vec<_> = catalog
        .iter()
        .filter(|e| resolved.get(e.key).copied().unwrap_or(false))
        .collect();
    let total = scheduled.len() as i64;

    emit(
        progress,
        ProgressEvent::info(
            "daily",
            "daily",
            "清日常",
            format!(
                "将执行 {} 个已启用模块（目录默认全关；商店默认不兑换）",
                total
            ),
        ),
    );

    let mut results = Vec::new();
    let mut success = 0usize;
    let mut partial = 0usize;
    let mut skipped = 0usize;
    let mut module_aborted = 0usize;
    let mut errors = 0usize;
    let mut idx = 0i64;
    let mut user_aborted = false;

    for entry in catalog {
        if let Some(ref c) = control {
            while c.is_paused() && !c.is_aborted() {
                emit(
                    progress,
                    ProgressEvent::info("daily", "pause", "已暂停", "等待继续或放弃…"),
                );
                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            }
            if c.is_aborted() {
                user_aborted = true;
                emit(
                    progress,
                    ProgressEvent::info("daily", "abort", "已中止", "用户放弃，后续模块不再执行"),
                );
                break;
            }
        }
        let on = resolved.get(entry.key).copied().unwrap_or(false);
        if !on {
            continue;
        }
        idx += 1;
        emit(
            progress,
            ProgressEvent::running(
                "daily",
                entry.key,
                entry.name,
                idx,
                total,
                format!("开始：{}", entry.name),
            ),
        );

        // W1：wire 按模块打标签，便于 events.jsonl 过滤
        crate::wire::set_module_key(Some(entry.key));
        crate::wire::record_note(
            "module_start",
            serde_json::json!({
                "module_name": entry.name,
                "index": idx,
                "total": total,
            }),
        );

        let r = match run_one(client, entry.key, config, progress).await {
            Ok(log) => {
                if is_partial_success_log(&log) {
                    partial += 1;
                    ModuleResult {
                        key: entry.key.into(),
                        name: entry.name.into(),
                        status: "部分完成".into(),
                        log,
                    }
                } else {
                    success += 1;
                    ModuleResult {
                        key: entry.key.into(),
                        name: entry.name.into(),
                        status: "成功".into(),
                        log,
                    }
                }
            }
            Err(CoreError::Skip(m)) => {
                skipped += 1;
                ModuleResult {
                    key: entry.key.into(),
                    name: entry.name.into(),
                    status: "跳过".into(),
                    log: m,
                }
            }
            Err(CoreError::Abort(m)) => {
                module_aborted += 1;
                ModuleResult {
                    key: entry.key.into(),
                    name: entry.name.into(),
                    status: "中止".into(),
                    log: m,
                }
            }
            Err(e) => {
                errors += 1;
                ModuleResult {
                    key: entry.key.into(),
                    name: entry.name.into(),
                    status: "错误".into(),
                    log: e.module_log_zh(),
                }
            }
        };
        crate::wire::record_note(
            "module_end",
            serde_json::json!({
                "module_name": entry.name,
                "status": r.status,
                "log": r.log,
            }),
        );
        crate::wire::set_module_key(None);

        let status_en = match r.status.as_str() {
            "成功" => "success",
            "部分完成" => "partial",
            "跳过" => "skip",
            "中止" => "abort",
            _ => "error",
        };
        emit(
            progress,
            ProgressEvent::module_done(
                "daily",
                entry.key,
                entry.name,
                idx,
                total,
                status_en,
                r.log.clone(),
            ),
        );
        tracing::info!(key = entry.key, status = %r.status, "module done");
        results.push(r);
    }

    let failed = module_aborted + errors;
    let ok = failed == 0 && !user_aborted;
    let total = success + partial + skipped + failed;
    let counts = format!(
        "成功 {success} / 部分完成 {partial} / 跳过 {skipped} / 中止 {module_aborted} / 错误 {errors}"
    );
    let summary = if user_aborted {
        format!("清日常用户放弃：{counts} / 已跑 {}", results.len())
    } else {
        format!("清日常结束：{counts} / 共 {}", results.len())
    };
    emit(progress, ProgressEvent::finished("daily", ok, summary.clone()));

    DailyReport {
        results,
        ok,
        total,
        success,
        partial,
        skipped,
        aborted: module_aborted,
        errors,
        failed,
    }
}

/// 单模块执行（设置页「单独运行」）
pub async fn run_single_module(
    client: &mut GameClient,
    key: &str,
    config: &HashMap<String, Value>,
) -> Result<String> {
    run_one(client, key, config, &None).await
}

/// 单模块 + 可选进度通道（快速刷图等长任务逐轮推送）。
/// Docs: docs/tech/UI_ROUTING_AND_TASK_LOGS.md · C7
pub async fn run_single_module_with_progress(
    client: &mut GameClient,
    key: &str,
    config: &HashMap<String, Value>,
    progress: &ProgressTx,
) -> Result<String> {
    run_one(client, key, config, progress).await
}

async fn run_one(
    client: &mut GameClient,
    key: &str,
    config: &HashMap<String, Value>,
    progress: &ProgressTx,
) -> Result<String> {
    match key {
        "loginbonus" => daily::loginbonus(client).await,
        "stamina_buy" => daily::stamina_buy(client, config).await,
        "super_sweep" => daily::super_sweep_with_progress(client, config, progress).await,
        "raid_reward" => daily::raid_reward(client, config).await,
        "self_raid" => daily::self_raid(client, config).await,
        "support_raid" => daily::support_raid(client, config).await,
        "like_raid" => daily::like_raid(client, config).await,
        "solo_raid" => daily::solo_raid(client).await,
        "high_score" => daily::high_score(client).await,
        "arena" => daily::arena(client).await,
        "basic" => daily::basic(client, config).await,
        "event" => daily::event_sweep(client).await,
        "archive" => daily::archive_sweep(client).await,
        "event_shop" => daily::event_shop(client, config).await,
        "raid_shop" => daily::raid_shop(client, config).await,
        "arena_shop" => daily::arena_shop(client, config).await,
        "tower" => daily::tower(client).await,
        "heart" => daily::heart(client, config).await,
        "gather" => daily::gather(client).await,
        "freegacha" => daily::freegacha(client).await,
        "eventscenario" => daily::eventscenario(client).await,
        "collection" => daily::collection(client).await,
        "battle_mission" => daily::battle_mission(client).await,
        "mission" => daily::mission(client).await,
        "present" => daily::present(client).await,
        "info" => daily::info(client).await,
        // 工具区（非 daily 一键默认列表；可 CLI/网页单模块）
        "clear_dungeon_event" => daily::clear_dungeon_event(client).await,
        _ => Err(CoreError::Skip(format!("未知模块 {key}"))),
    }
}

/// 工具模块目录（非一键日常默认列表；可单独运行）
pub fn tool_catalog() -> &'static [ModuleCatalogEntry] {
    &[
        ModuleCatalogEntry {
            key: "super_wash",
            name: "快速洗词条",
            default_enabled: false,
            resource_heavy: true,
        },
        ModuleCatalogEntry {
            key: "clear_dungeon_event",
            name: "完成迷宫隐藏事件",
            default_enabled: false,
            resource_heavy: false,
        },
        ModuleCatalogEntry {
            key: "raid_support",
            name: "魔女救世（见组队）",
            default_enabled: false,
            resource_heavy: true,
        },
    ]
}
