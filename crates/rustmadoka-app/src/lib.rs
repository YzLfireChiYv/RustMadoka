//! RustMadoka desktop host (Windows first): CLI + loopback HTTP + Owner + static web UI.
//!
//! # Architecture
//! - **This crate**: process host, HTTP, CLI, Owner, TaskGate, IPC (platform shell).
//! - **rustmadoka-core**: protocol + game modules (platform-agnostic).
//! - **static/index.html**: browser web frontend (reuse allowed).
//!
//! # Docs
//! - `docs/PLAN_RUSTMADOKA_FULL_REWRITE.md`
//! - `docs/tech/INSTANCE_AND_CLI.md` · `docs/HANDOFF.md`
//!
//! # Defaults
//! - Data: `RustMadoka_data` · Port: **14103**

mod app_settings;
mod config_pack;
mod data_layout;
mod settings_files;
mod fp_load;
mod fp_slots;
mod http_server;
mod ipc;
mod occupancy;
mod owner_lock;
mod paths;
mod run_control;
mod run_ops;
mod session_pool;
mod task_gate;
mod system_toast;
mod task_log;
mod wire_scope;

use anyhow::{bail, Context, Result};
use app_settings::{prepare_sources, AppSettings};
use clap::{Parser, Subcommand};
use ipc::{IpcRequest, IpcResponse};
use paths::resolve_data_dir;
use rustmadoka_core::account::{Channel, GameAccount, Store};
use rustmadoka_core::fingerprint::fetch_fingerprint;
use rustmadoka_core::safety::assert_daily_allowed;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use task_gate::TaskGate;
use wire_scope::should_record_wire;

pub const DEFAULT_FP_URL: &str =
    "https://raw.githubusercontent.com/YzLfireChiYv/rules/main/automadoka.json";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUILD_STAMP: &str = env!("AUTOMADOKA_BUILD_STAMP");
pub const PRODUCT_EDITION: &str = if cfg!(feature = "wire_record") {
    "debug"
} else {
    "release"
};

/// Android / 嵌入式：启动 loopback Owner（无浏览器）。
/// Outbound: `http_server::run_embedded_serve`
pub use http_server::run_embedded_serve;

// --- CLI -------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(
    name = "RustMadoka",
    version = APP_VERSION,
    about = "RustMadoka：本机清日常与自动化（Windows 优先）",
    long_about = "RustMadoka 本机工具。默认无子命令时启动 Owner 服务与浏览器网页前端。\n\
数据文件夹默认在可执行文件旁路 RustMadoka_data；默认 HTTP 端口 14103。\n\
文档：docs/HANDOFF.md · docs/tech/INSTANCE_AND_CLI.md"
)]
struct Cli {
    /// 覆盖数据文件夹路径（默认：exe 旁路 RustMadoka_data）
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    /// 覆盖监听端口（默认见数据文件夹 app 设置或 14103）
    #[arg(long, global = true)]
    port: Option<u16>,
    /// 指纹拉取 URL（默认 rules 仓 automadoka.json）
    #[arg(long, global = true, default_value = DEFAULT_FP_URL)]
    fp_url: String,
    /// 默认渠道 en/jp/tw（登录与指纹选择）
    #[arg(long, global = true, default_value = "en")]
    default_channel: String,
    /// 启动服务时不自动打开浏览器
    #[arg(long, global = true)]
    no_browser: bool,
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// 启动 Owner：程序运行面板终端 + 本机 HTTP + 浏览器网页前端
    Serve,
    /// 拉取并保存游戏版本指纹
    FetchFp {
        #[arg(long, default_value = DEFAULT_FP_URL)]
        url: String,
        #[arg(long, default_value = "en")]
        channel: String,
    },
    /// 用户组：列表 / 创建 / 改密
    Group {
        #[command(subcommand)]
        action: GroupCmd,
    },
    /// 游戏账号卡片：列表 / 添加 / 删除 / 配置
    Account {
        #[command(subcommand)]
        action: AccountCmd,
    },
    /// 执行：info / 日常 / 单模块 / 组队团战
    Run {
        #[command(subcommand)]
        action: RunCmd,
    },
    /// 导出会话快照（E1）
    Export {
        #[command(subcommand)]
        action: ExportCmd,
    },
    /// 指纹槽：列表 / 刷新默认源 / 启用 / 重置内嵌（P23 · P29）
    Fp {
        #[command(subcommand)]
        action: FpCmd,
    },
    /// 任务日志：列表 / 查看 / 按天清理（P23）
    TaskLog {
        #[command(subcommand)]
        action: TaskLogCmd,
    },
    /// 运行中任务控制：暂停 / 继续 / 放弃 / 状态（须已有 Owner；走 IPC）
    Control {
        #[command(subcommand)]
        action: ControlCmd,
    },
    /// 通知：设置变更历史 + Windows 系统 toast 配置（默认关）
    Notify {
        #[command(subcommand)]
        action: NotifyCmd,
    },
    /// Master 表查询：关卡 ID↔名称等（对照原版 mst；可缓存到数据文件夹）
    Mst {
        #[command(subcommand)]
        action: MstCmd,
    },
}

/// 关卡/Master 表 CLI（P23 · 产品化底层 mst）
///
/// Docs: `docs/tech/BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS.md` · `DATA_AND_MST.md`
#[derive(Subcommand, Debug)]
enum MstCmd {
    /// 关卡 ID 与名称对照表（原版 get_quest_stage_mst_list）
    ///
    /// 默认优先读数据文件夹缓存；无缓存或 --refresh 时登录账号对服拉取并写入
    /// `RustMadoka_data/cache/mst/{channel}/quest_stage.json`。
    /// 仅查缓存不登录：加 --from-cache 并指定 --channel。
    QuestStages {
        #[arg(short = 'g', long, help = "用户组（登录拉取时需要）")]
        group: Option<String>,
        #[arg(long, help = "加密用户组密码")]
        group_password: Option<String>,
        #[arg(short = 'a', long, help = "游戏账号别名（登录拉取时需要）")]
        alias: Option<String>,
        #[arg(long, default_value = "en", help = "仅 --from-cache 时使用的渠道 en/jp/tw")]
        channel: String,
        #[arg(long, default_value_t = false, help = "强制对服刷新并写缓存")]
        refresh: bool,
        #[arg(long, default_value_t = false, help = "只读本地缓存，不登录")]
        from_cache: bool,
        #[arg(long, help = "精确关卡 ID，如 411102")]
        id: Option<i64>,
        #[arg(long, help = "精确 questGroupMstId，如 101=キオク")]
        group_id: Option<i64>,
        #[arg(long, help = "名称子串过滤，如 キオク / 晶花 / Magic")]
        filter: Option<String>,
        #[arg(long, default_value_t = 0, help = "最多返回条数；0=全部（过滤后）")]
        limit: usize,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 按关卡 ID 查一条（名称/耗体/组）；等价 quest-stages --id
    QuestLookup {
        #[arg(long, help = "关卡 ID")]
        id: i64,
        #[arg(short = 'g', long)]
        group: Option<String>,
        #[arg(long)]
        group_password: Option<String>,
        #[arg(short = 'a', long)]
        alias: Option<String>,
        #[arg(long, default_value = "en")]
        channel: String,
        #[arg(long, default_value_t = false)]
        refresh: bool,
        #[arg(long, default_value_t = false)]
        from_cache: bool,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum FpCmd {
    /// 列出指纹槽与当前启用槽
    Slots {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 从默认 rules 源拉取并启用
    Refresh {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 启用指定槽 id（如 default_embedded / default_pulled / custom_0）
    Activate {
        #[arg(long)]
        id: String,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 重置为程序内置槽
    Reset {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 向自定义槽写入指纹 JSON（路径 `@file` 或内联；槽 custom_0 / custom_1）
    Fill {
        #[arg(long, help = "custom_0 或 custom_1")]
        id: String,
        /// 指纹 JSON 文本，或以 `@path` 读文件
        #[arg(long)]
        text: String,
        #[arg(long, default_value = "")]
        note: String,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum TaskLogCmd {
    /// 列出任务日志索引
    List {
        #[arg(short = 'g', long)]
        group: String,
        #[arg(short = 'a', long)]
        alias: String,
        #[arg(long, help = "触发过滤：one_click_daily / single_module / cli")]
        trigger: Option<String>,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 查看已定稿完整日志
    Show {
        #[arg(short = 'g', long)]
        group: String,
        #[arg(short = 'a', long)]
        alias: String,
        #[arg(long)]
        id: String,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 查看进行中进度（或定稿摘要）
    Progress {
        #[arg(short = 'g', long)]
        group: String,
        #[arg(short = 'a', long)]
        alias: String,
        #[arg(long)]
        id: String,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 按天清理：删除早于 retain_days 天的日志
    ClearOlder {
        #[arg(short = 'g', long)]
        group: String,
        #[arg(short = 'a', long)]
        alias: String,
        #[arg(long, default_value_t = 30)]
        retain_days: u32,
        #[arg(long, default_value_t = false, help = "仅清理一键清日常")]
        only_one_click: bool,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ControlCmd {
    /// 请求暂停当前运行中的对服任务
    Pause {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 继续
    Resume {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 放弃
    Abort {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 查看 Owner 内运行状态摘要
    Status {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum NotifyCmd {
    /// 列出设置变更通知历史（feature=settings）
    Settings {
        #[arg(long)]
        group: Option<String>,
        #[arg(long)]
        alias: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 读取 Windows 系统 toast 配置
    SystemGet {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 写入 Windows 系统 toast 配置（默认 enabled=false）
    SystemSet {
        #[arg(long)]
        enabled: Option<bool>,
        #[arg(long)]
        on_task_success: Option<bool>,
        #[arg(long)]
        on_task_error: Option<bool>,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// 强制弹出一次系统 toast（测试；不依赖 enabled）
    SystemTest {
        #[arg(long, default_value = "RustMadoka")]
        title: String,
        #[arg(long, default_value = "系统通知测试")]
        body: String,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum GroupCmd {
    /// 列出用户组（不含密码）
    List,
    /// 创建用户组；--password 可选（有则加密组）
    Create {
        name: String,
        #[arg(long)]
        password: Option<String>,
    },
    /// 修改用户组密码（空 --new-password = 改回明文组；成功后须重新登录网页）
    SetPassword {
        name: String,
        #[arg(long)]
        old_password: Option<String>,
        #[arg(long)]
        new_password: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum AccountCmd {
    List {
        #[arg(short = 'g', long)]
        group: String,
        #[arg(long)]
        group_password: Option<String>,
    },
    Add {
        #[arg(short = 'g', long)]
        group: String,
        #[arg(long)]
        group_password: Option<String>,
        #[arg(short = 'a', long)]
        alias: String,
        #[arg(long, default_value = "en")]
        channel: String,
        #[arg(long)]
        code: String,
        #[arg(long)]
        password: String,
    },
    Remove {
        #[arg(short = 'g', long)]
        group: String,
        #[arg(long)]
        group_password: Option<String>,
        #[arg(short = 'a', long)]
        alias: String,
    },
    /// 合并写入账号扁平 config（JSON 对象字符串或 @文件路径）
    Config {
        #[arg(short = 'g', long)]
        group: String,
        #[arg(long)]
        group_password: Option<String>,
        #[arg(short = 'a', long)]
        alias: String,
        /// JSON 对象；或以 `@path` 读文件（UTF-8）
        #[arg(long)]
        merge: String,
        #[arg(long)]
        json: bool,
    },
    /// 仅打印账号 config
    ShowConfig {
        #[arg(short = 'g', long)]
        group: String,
        #[arg(long)]
        group_password: Option<String>,
        #[arg(short = 'a', long)]
        alias: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum RunCmd {
    /// 登录并获取角色信息（昵称/等级等）
    Info {
        #[arg(short = 'g', long, help = "用户组名")]
        group: String,
        #[arg(long, help = "加密用户组密码")]
        group_password: Option<String>,
        #[arg(short = 'a', long, help = "游戏账号别名")]
        alias: String,
        #[arg(long, help = "JSON 输出")]
        json: bool,
        #[arg(long, default_value_t = false, help = "开发版录制通讯（wire）")]
        wire: bool,
    },
    /// 一键清日常（默认仅已开启模块；--all-modules 强制全开跑测）
    Daily {
        #[arg(short = 'g', long)]
        group: String,
        #[arg(long)]
        group_password: Option<String>,
        #[arg(short = 'a', long)]
        alias: String,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = false)]
        wire: bool,
        #[arg(long, default_value_t = false, help = "忽略配置开关，启用全部日常模块（测试用）")]
        all_modules: bool,
        #[arg(long, default_value_t = false, help = "团战伤害用安全下限（冒烟）")]
        safe_raid_damage: bool,
    },
    /// 运行单个模块（--key 模块键，如 loginbonus）
    Module {
        #[arg(short = 'g', long)]
        group: String,
        #[arg(long)]
        group_password: Option<String>,
        #[arg(short = 'a', long)]
        alias: String,
        #[arg(long, help = "模块键，如 loginbonus / event_shop")]
        key: String,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = false)]
        wire: bool,
        #[arg(long, default_value_t = false)]
        safe_raid_damage: bool,
    },
    /// 组队团战：1 人=打满今日次数；2+ 人=互援（§8 · 多配置卡）
    GroupRaid {
        #[arg(short = 'g', long)]
        group: String,
        #[arg(long)]
        group_password: Option<String>,
        #[arg(
            long,
            default_value = "",
            help = "组队配置卡 id（与 --aliases 二选一；优先 config-id）"
        )]
        config_id: String,
        #[arg(
            long,
            default_value = "",
            help = "参与别名，逗号分隔；1 个=单号打满日次数，2+=组队"
        )]
        aliases: String,
        #[arg(
            long,
            default_value = "",
            help = "房间开放 guild/friend/all；单号可用 self；空=单号默认 self"
        )]
        room_open: String,
        #[arg(long, default_value = "", help = "队伍名/id（可选）")]
        party: String,
        #[arg(long, default_value_t = false, help = "援助后退出房间（默认关，需结算奖励）")]
        leave_after_support: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ExportCmd {
    /// 导出会话业务数据（不含引继/密码；路径在数据文件夹 exports/）
    Session {
        #[arg(short = 'g', long)]
        group: String,
        #[arg(long)]
        group_password: Option<String>,
        #[arg(short = 'a', long)]
        alias: String,
        #[arg(long, help = "输出目录（可选）")]
        out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}

// --- entry -----------------------------------------------------------------

/// Called from `main.rs`.
pub async fn cli_main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,rustmadoka_core=info".into()),
        )
        .init();

    let cli = Cli::parse();
    let data_dir = resolve_data_dir(cli.data_dir.clone());
    std::fs::create_dir_all(&data_dir)?;
    // 正式自用：布局版本 + 约定目录（不删用户文件）· docs/tech/DATA_FOLDER_LAYOUT.md · NORMS P32
    let _layout = data_layout::ensure_data_layout(&data_dir, APP_VERSION)
        .context("data folder layout ensure failed")?;
    let _ = write_runtime_stamp(&data_dir);

    match cli.cmd.unwrap_or(Cmd::Serve) {
        Cmd::Serve => {
            http_server::run_owner_serve(
                data_dir,
                cli.fp_url,
                cli.default_channel,
                cli.port,
                cli.no_browser,
                None,
            )
            .await
        }
        Cmd::FetchFp { url, channel } => {
            let fp = fetch_fingerprint(&url, &channel).await?;
            let _ = fp.save_version_json(&data_dir.join("cache/version.json"));
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "channel": channel,
                    "version": fp.version,
                }))?
            );
            Ok(())
        }
        Cmd::Group { action } => cmd_group(&data_dir, action),
        Cmd::Account { action } => cmd_account(&data_dir, action),
        Cmd::Run { action } => {
            cmd_run(
                &data_dir,
                &cli.fp_url,
                cli.port,
                cli.default_channel,
                cli.no_browser,
                action,
            )
            .await
        }
        Cmd::Export { action } => cmd_export(&data_dir, &cli.fp_url, action).await,
        Cmd::Fp { action } => cmd_fp(&data_dir, action).await,
        Cmd::TaskLog { action } => cmd_task_log(&data_dir, action),
        Cmd::Control { action } => cmd_control(&data_dir, action).await,
        Cmd::Notify { action } => cmd_notify(&data_dir, action),
        Cmd::Mst { action } => cmd_mst(&data_dir, &cli.fp_url, action).await,
    }
}

/// Master 表 CLI：关卡对照等。
async fn cmd_mst(data_dir: &std::path::Path, fp_url: &str, action: MstCmd) -> Result<()> {
    match action {
        MstCmd::QuestStages {
            group,
            group_password,
            alias,
            channel,
            refresh,
            from_cache,
            id,
            group_id,
            filter,
            limit,
            json,
        } => {
            let v = mst_quest_stages_value(
                data_dir,
                fp_url,
                group.as_deref(),
                group_password.as_deref(),
                alias.as_deref(),
                &channel,
                refresh,
                from_cache,
                id,
                group_id,
                filter.as_deref(),
                limit,
            )
            .await?;
            print_mst_result(&v, json)?;
            Ok(())
        }
        MstCmd::QuestLookup {
            id,
            group,
            group_password,
            alias,
            channel,
            refresh,
            from_cache,
            json,
        } => {
            let v = mst_quest_stages_value(
                data_dir,
                fp_url,
                group.as_deref(),
                group_password.as_deref(),
                alias.as_deref(),
                &channel,
                refresh,
                from_cache,
                Some(id),
                None,
                None,
                1,
            )
            .await?;
            print_mst_result(&v, json)?;
            Ok(())
        }
    }
}

async fn mst_quest_stages_value(
    data_dir: &std::path::Path,
    fp_url: &str,
    group: Option<&str>,
    group_password: Option<&str>,
    alias: Option<&str>,
    channel: &str,
    refresh: bool,
    from_cache: bool,
    id: Option<i64>,
    group_id: Option<i64>,
    filter: Option<&str>,
    limit: usize,
) -> Result<Value> {
    if from_cache {
        return run_ops::query_quest_stages_from_cache(
            data_dir, channel, id, group_id, filter, limit,
        );
    }
    // 无账号时：有缓存则读缓存；无缓存且未 refresh 则提示
    if group.is_none() || alias.is_none() {
        if let Ok(v) = run_ops::query_quest_stages_from_cache(
            data_dir, channel, id, group_id, filter, limit,
        ) {
            return Ok(v);
        }
        bail!(
            "需要 --from-cache --channel <en|jp> 且本地已有缓存，或提供 -g/-a 登录拉取。例：\
mst quest-stages -g 123456 -a en_w1 --group-password *** --refresh --filter キオク"
        );
    }
    let group = group.unwrap();
    let alias = alias.unwrap();
    let store = Store::open(data_dir)?;
    let g = store.load_group(group, group_password)?;
    let acc = g
        .accounts
        .iter()
        .find(|a| a.alias == alias)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("account not found: {alias}"))?;
    run_ops::query_quest_stages(
        data_dir,
        fp_url,
        &acc,
        refresh,
        id,
        group_id,
        filter,
        limit,
    )
    .await
}

fn print_mst_result(v: &Value, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(v)?);
        return Ok(());
    }
    let count = v.get("count").and_then(|x| x.as_u64()).unwrap_or(0);
    let total = v
        .get("total_in_source")
        .and_then(|x| x.as_u64())
        .unwrap_or(count);
    let source = v.get("source").and_then(|x| x.as_str()).unwrap_or("?");
    let ch = v.get("channel").and_then(|x| x.as_str()).unwrap_or("?");
    println!(
        "关卡对照 channel={ch} source={source} 命中={count} 源表行数≈{total}"
    );
    if let Some(stages) = v.get("stages").and_then(|x| x.as_array()) {
        for s in stages {
            let id = s
                .get("questStageMstId")
                .and_then(|x| x.as_i64())
                .unwrap_or(0);
            let gid = s
                .get("questGroupMstId")
                .and_then(|x| x.as_i64())
                .unwrap_or(0);
            let name = s.get("name").and_then(|x| x.as_str()).unwrap_or("");
            let st = s.get("useStamina").and_then(|x| x.as_i64()).unwrap_or(0);
            let diff = s.get("difficulty").and_then(|x| x.as_i64()).unwrap_or(0);
            let power = s
                .get("recommendationPartyPower")
                .and_then(|x| x.as_i64())
                .unwrap_or(0);
            println!(
                "{id}\t组={gid}\t耗体={st}\t难度={diff}\t推荐战力={power}\t{name}"
            );
        }
    }
    if count == 0 {
        println!("（无匹配行。可试 --filter キオク / --group-id 101 / --id 411102）");
    }
    Ok(())
}

async fn cmd_fp(data_dir: &std::path::Path, action: FpCmd) -> Result<()> {
    match action {
        FpCmd::Slots { json } => {
            let store = fp_slots::FpSlotStore::load(data_dir);
            let v = store.to_public_json();
            if json {
                println!("{}", serde_json::to_string_pretty(&v)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&v)?);
            }
            Ok(())
        }
        FpCmd::Refresh { json } => {
            let v = fp_slots::refresh_default_source(data_dir, true).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&v)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&v)?);
            }
            Ok(())
        }
        FpCmd::Activate { id, json } => {
            let mut store = fp_slots::FpSlotStore::load(data_dir);
            store.activate(&id)?;
            store.save(data_dir)?;
            let v = json!({"ok": true, "active_slot_id": id, "store": store.to_public_json()});
            println!("{}", serde_json::to_string_pretty(&v)?);
            let _ = json;
            Ok(())
        }
        FpCmd::Reset { json } => {
            let mut store = fp_slots::FpSlotStore::load(data_dir);
            store.reset_to_default_embedded()?;
            store.save(data_dir)?;
            let v = json!({
                "ok": true,
                "active_slot_id": "default_embedded",
                "store": store.to_public_json()
            });
            println!("{}", serde_json::to_string_pretty(&v)?);
            let _ = json;
            Ok(())
        }
        FpCmd::Fill {
            id,
            text,
            note,
            json,
        } => {
            let body = if let Some(path) = text.strip_prefix('@') {
                std::fs::read_to_string(path)
                    .with_context(|| format!("读取指纹文件失败: {path}"))?
            } else {
                text
            };
            let mut store = fp_slots::FpSlotStore::load(data_dir);
            store.fill_custom(&id, &body, &note)?;
            store.activate(&id)?;
            store.save(data_dir)?;
            let v = json!({
                "ok": true,
                "active_slot_id": id,
                "store": store.to_public_json()
            });
            println!("{}", serde_json::to_string_pretty(&v)?);
            let _ = json;
            Ok(())
        }
    }
}

fn cmd_task_log(data_dir: &std::path::Path, action: TaskLogCmd) -> Result<()> {
    match action {
        TaskLogCmd::List {
            group,
            alias,
            trigger,
            json,
        } => {
            let list = task_log::list_sessions(data_dir, &group, &alias, trigger.as_deref())?;
            let v = serde_json::to_value(&list)?;
            println!("{}", serde_json::to_string_pretty(&v)?);
            let _ = json;
            Ok(())
        }
        TaskLogCmd::Show {
            group,
            alias,
            id,
            json,
        } => {
            let sess = task_log::load_full_session(data_dir, &group, &alias, &id)?;
            println!("{}", serde_json::to_string_pretty(&sess)?);
            let _ = json;
            Ok(())
        }
        TaskLogCmd::Progress {
            group,
            alias,
            id,
            json,
        } => {
            let v = task_log::load_progress(data_dir, &group, &alias, &id)?;
            println!("{}", serde_json::to_string_pretty(&v)?);
            let _ = json;
            Ok(())
        }
        TaskLogCmd::ClearOlder {
            group,
            alias,
            retain_days,
            only_one_click,
            json,
        } => {
            let n = task_log::clear_sessions_older_than(
                data_dir,
                &group,
                &alias,
                retain_days,
                only_one_click,
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "removed": n,
                    "retain_days": retain_days,
                    "only_one_click": only_one_click,
                }))?
            );
            let _ = json;
            Ok(())
        }
    }
}

async fn cmd_control(data_dir: &std::path::Path, action: ControlCmd) -> Result<()> {
    let req = match &action {
        ControlCmd::Pause { .. } => IpcRequest::RunPause,
        ControlCmd::Resume { .. } => IpcRequest::RunResume,
        ControlCmd::Abort { .. } => IpcRequest::RunAbort,
        ControlCmd::Status { .. } => IpcRequest::RunStatus,
    };
    match ipc::client_call(data_dir, &req).await {
        Ok(resp) => {
            if resp.ok {
                if let Some(v) = resp.result {
                    println!("{}", serde_json::to_string_pretty(&v)?);
                } else {
                    println!("{}", json!({"ok": true}));
                }
                Ok(())
            } else {
                bail!(
                    "{}",
                    resp.error
                        .unwrap_or_else(|| "Owner 控制失败".into())
                )
            }
        }
        Err(e) => bail!(
            "无法连接 Owner IPC（请先双击 RustMadoka.exe 保持程序运行面板终端运行）：{e}"
        ),
    }
}

fn cmd_notify(data_dir: &std::path::Path, action: NotifyCmd) -> Result<()> {
    match action {
        NotifyCmd::Settings {
            group,
            alias,
            limit,
            json,
        } => {
            let file = rustmadoka_core::FeatureNotifyFile::load(
                data_dir,
                rustmadoka_core::FEATURE_SETTINGS,
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;
            let entries = file.query_filtered(
                None,
                alias.as_deref(),
                group.as_deref(),
                Some(limit),
            );
            println!("{}", serde_json::to_string_pretty(&entries)?);
            let _ = json;
            Ok(())
        }
        NotifyCmd::SystemGet { json } => {
            let s = system_toast::SystemToastSettings::load(data_dir);
            println!("{}", serde_json::to_string_pretty(&s)?);
            let _ = json;
            Ok(())
        }
        NotifyCmd::SystemSet {
            enabled,
            on_task_success,
            on_task_error,
            json,
        } => {
            let mut s = system_toast::SystemToastSettings::load(data_dir);
            if let Some(v) = enabled {
                s.enabled = v;
            }
            if let Some(v) = on_task_success {
                s.on_task_success = v;
            }
            if let Some(v) = on_task_error {
                s.on_task_error = v;
            }
            s.save(data_dir)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "settings": s,
                }))?
            );
            let _ = json;
            Ok(())
        }
        NotifyCmd::SystemTest { title, body, json } => {
            system_toast::show_toast(&title, &body)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "title": title,
                    "body": body,
                    "note": "已强制弹出（不检查 enabled）",
                }))?
            );
            let _ = json;
            Ok(())
        }
    }
}

fn write_runtime_stamp(data_dir: &std::path::Path) -> Result<()> {
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let doc = json!({
        "schema": 1,
        "build_stamp": BUILD_STAMP,
        "cargo_version": APP_VERSION,
        "edition": PRODUCT_EDITION,
        "exe_path": exe,
        "started_at": chrono::Utc::now().to_rfc3339(),
    });
    std::fs::write(
        data_dir.join("app_runtime.json"),
        serde_json::to_string_pretty(&doc)?,
    )?;
    Ok(())
}

fn cmd_group(data_dir: &std::path::Path, action: GroupCmd) -> Result<()> {
    let store = Store::open(data_dir)?;
    match action {
        GroupCmd::List => {
            println!("{}", serde_json::to_string_pretty(&store.list_groups()?)?);
        }
        GroupCmd::Create { name, password } => {
            store.create_group(&name, password.as_deref())?;
            println!("{}", json!({"ok": true, "group": name}));
        }
        GroupCmd::SetPassword {
            name,
            old_password,
            new_password,
        } => {
            let g = store.set_group_password(
                &name,
                old_password.as_deref(),
                new_password.as_deref(),
            )?;
            println!(
                "{}",
                json!({
                    "ok": true,
                    "group": name,
                    "has_password": g.has_password,
                    "must_relogin": true,
                    "message": "密码已更新；HTTP 会话须重新登录（CLI 无 token 缓存）",
                })
            );
        }
    }
    Ok(())
}

fn cmd_account(data_dir: &std::path::Path, action: AccountCmd) -> Result<()> {
    let store = Store::open(data_dir)?;
    match action {
        AccountCmd::List {
            group,
            group_password,
        } => {
            let g = store.load_group(&group, group_password.as_deref())?;
            let rows: Vec<_> = g
                .accounts
                .iter()
                .map(|a| {
                    json!({
                        "alias": a.alias,
                        "channel": a.channel,
                        "game_name": a.game_name,
                        "level": a.level,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&rows)?);
        }
        AccountCmd::Add {
            group,
            group_password,
            alias,
            channel,
            code,
            password,
        } => {
            let mut g = store.load_group(&group, group_password.as_deref())?;
            if g.accounts.iter().any(|a| a.alias == alias) {
                bail!("alias already exists: {alias}");
            }
            g.accounts.push(GameAccount {
                alias: alias.clone(),
                channel,
                username: code,
                password,
                game_name: String::new(),
                level: 0,
                info_fetched_at: None,
                config: HashMap::new(),
            });
            store.save_group(&g)?;
            println!("{}", json!({"ok": true, "alias": alias}));
        }
        AccountCmd::Remove {
            group,
            group_password,
            alias,
        } => {
            let mut g = store.load_group(&group, group_password.as_deref())?;
            let n = g.accounts.len();
            g.accounts.retain(|a| a.alias != alias);
            if g.accounts.len() == n {
                bail!("alias not found: {alias}");
            }
            store.save_group(&g)?;
            println!("{}", json!({"ok": true, "removed": alias}));
        }
        AccountCmd::Config {
            group,
            group_password,
            alias,
            merge,
            json: _,
        } => {
            let raw = if let Some(path) = merge.strip_prefix('@') {
                std::fs::read_to_string(path)
                    .with_context(|| format!("read config file {path}"))?
            } else {
                merge
            };
            let patch: HashMap<String, Value> = serde_json::from_str(&raw)
                .context("merge must be a JSON object of config keys")?;
            let mut g = store.load_group(&group, group_password.as_deref())?;
            let acc = g
                .accounts
                .iter_mut()
                .find(|a| a.alias == alias)
                .ok_or_else(|| anyhow::anyhow!("account not found: {alias}"))?;
            for (k, v) in patch {
                acc.config.insert(k, v);
            }
            let n = acc.config.len();
            store.save_group(&g)?;
            println!(
                "{}",
                json!({
                    "ok": true,
                    "alias": alias,
                    "config_keys": n,
                })
            );
        }
        AccountCmd::ShowConfig {
            group,
            group_password,
            alias,
        } => {
            let g = store.load_group(&group, group_password.as_deref())?;
            let acc = g
                .accounts
                .iter()
                .find(|a| a.alias == alias)
                .ok_or_else(|| anyhow::anyhow!("account not found: {alias}"))?;
            println!("{}", serde_json::to_string_pretty(&acc.config)?);
        }
    }
    Ok(())
}

/// 是否必须在本进程独占执行（不走 Owner 拉起 / IPC）。
///
/// 产品主路径（主人设计）：CLI **优先**附着已有 Owner（IPC）；无 Owner 时 **拉起** 程序运行面板
/// 并长期 serve。仅 wire 录制、组队多号等必须本机独占时才强制 local。
///
/// Docs: `docs/tech/INSTANCE_AND_CLI.md` · `docs/PLAN_INSTANCE_CLI_PORT.md`
fn run_wants_local(action: &RunCmd) -> bool {
    match action {
        RunCmd::Info { wire, .. } => should_record_wire(*wire) || *wire,
        RunCmd::Daily { wire, .. } => should_record_wire(*wire) || *wire,
        // 单模块：默认走 Owner/IPC；仅 --wire 时本机录制
        RunCmd::Module { wire, .. } => should_record_wire(*wire) || *wire,
        // 组队多号编排暂本机独占（IPC 可后扩）
        RunCmd::GroupRaid { .. } => true,
    }
}

/// CLI `run *` 路径（**产品设计 · 非 Bug**）：
///
/// 1. 须本机独占（wire 等）→ 抢 Owner 锁、执行、退出  
/// 2. 已有 Owner → **命名管道 IPC**，Client 打印结果后退出  
/// 3. **无 Owner → 本进程升级为 Owner**：执行本次命令后 **继续 serve**（拉起程序并保持）
///
/// 2026-08-07 曾误把「3」当成挂死修成「执行后退出」，违反主人设计；已恢复。
/// AI 冒烟若需「执行即退出」：应先有 Owner，或显式 `serve` 后再 IPC，或使用 `--wire` 独占路径。
///
/// Docs: `docs/tech/INSTANCE_AND_CLI.md` · `docs/PLAN_INSTANCE_CLI_PORT.md`
async fn cmd_run(
    data_dir: &std::path::Path,
    fp_url: &str,
    cli_port: Option<u16>,
    default_channel: String,
    no_browser: bool,
    action: RunCmd,
) -> Result<()> {
    if run_wants_local(&action) {
        let _owner =
            owner_lock::try_acquire(data_dir).context("need exclusive Owner for this command")?;
        let gate = TaskGate::new();
        let v = exec_run_cmd_owner(data_dir, fp_url, &gate, action).await?;
        println!("{}", serde_json::to_string_pretty(&v)?);
        return Ok(());
    }

    let ipc_req = match &action {
        RunCmd::Info {
            group,
            group_password,
            alias,
            ..
        } => IpcRequest::RunInfo {
            group: group.clone(),
            group_password: group_password.clone(),
            alias: alias.clone(),
        },
        RunCmd::Daily {
            group,
            group_password,
            alias,
            ..
        } => IpcRequest::RunDaily {
            group: group.clone(),
            group_password: group_password.clone(),
            alias: alias.clone(),
        },
        RunCmd::Module {
            group,
            group_password,
            alias,
            key,
            safe_raid_damage,
            ..
        } => IpcRequest::RunModule {
            group: group.clone(),
            group_password: group_password.clone(),
            alias: alias.clone(),
            key: key.clone(),
            safe_raid_damage: *safe_raid_damage,
        },
        RunCmd::GroupRaid { .. } => unreachable!("group-raid always local"),
    };

    // 已有 Owner：Client 经 IPC 投递，不双开、不抢锁
    if let Ok(resp) = ipc::client_call(data_dir, &ipc_req).await {
        if !resp.ok {
            bail!("{}", resp.error.unwrap_or_else(|| "IPC failed".into()));
        }
        println!(
            "{}",
            serde_json::to_string_pretty(resp.result.as_ref().unwrap_or(&json!({})))?
        );
        return Ok(());
    }

    // 无 Owner：拉起程序本体（Owner + 程序运行面板终端 + HTTP），执行本次命令后**保持运行**
    tracing::info!("no Owner; starting Owner panel (CLI pull-up design)");
    http_server::run_owner_serve(
        data_dir.to_path_buf(),
        fp_url.to_string(),
        default_channel,
        cli_port,
        no_browser,
        Some(action),
    )
    .await
}

async fn cmd_export(
    data_dir: &std::path::Path,
    fp_url: &str,
    action: ExportCmd,
) -> Result<()> {
    match action {
        ExportCmd::Session {
            group,
            group_password,
            alias,
            out,
            json,
        } => {
            let gate = TaskGate::new();
            let v = run_ops::export_account_session(
                data_dir,
                fp_url,
                Some(&gate),
                &group,
                group_password.as_deref(),
                &alias,
                out.as_deref(),
            )
            .await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&v)?);
            } else {
                println!("{v}");
            }
            Ok(())
        }
    }
}

/// Shared by CLI and Owner IPC/HTTP.
pub(crate) async fn exec_run_cmd_owner(
    data_dir: &std::path::Path,
    fp_url: &str,
    gate: &TaskGate,
    action: RunCmd,
) -> Result<Value> {
    match action {
        RunCmd::Info {
            group,
            group_password,
            alias,
            ..
        } => {
            let store = Store::open(data_dir)?;
            let mut g = store.load_group(&group, group_password.as_deref())?;
            let acc = g
                .accounts
                .iter()
                .find(|a| a.alias == alias)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("account not found: {alias}"))?;
            let ch = Channel::from_user(&acc.channel);
            let _g = gate
                .try_begin_owned(ch.as_str(), &acc.username, "info", &group)
                .map_err(|e| anyhow::anyhow!(e))?;
            let info = run_ops::fetch_account_info(data_dir, fp_url, &acc).await?;
            if let Some(a) = g.accounts.iter_mut().find(|a| a.alias == alias) {
                a.game_name = info["name"].as_str().unwrap_or("").to_string();
                a.level = info["level"].as_i64().unwrap_or(0);
                a.info_fetched_at = Some(chrono::Utc::now().to_rfc3339());
            }
            store.save_group(&g)?;
            Ok(info)
        }
        RunCmd::Daily {
            group,
            group_password,
            alias,
            all_modules,
            safe_raid_damage,
            ..
        } => {
            assert_daily_allowed().map_err(|e| anyhow::anyhow!(e))?;
            let store = Store::open(data_dir)?;
            let g = store.load_group(&group, group_password.as_deref())?;
            let acc = g
                .accounts
                .iter()
                .find(|a| a.alias == alias)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("account not found: {alias}"))?;
            let ch = Channel::from_user(&acc.channel);
            let _g = gate
                .try_begin_owned(ch.as_str(), &acc.username, "daily", &group)
                .map_err(|e| anyhow::anyhow!(e))?;
            let mut request_enabled = HashMap::new();
            if all_modules {
                request_enabled = run_ops::all_modules_enabled_map();
            }
            let mut request_config = HashMap::new();
            if safe_raid_damage || all_modules {
                request_config.extend(run_ops::safe_raid_damage_config());
            }
            if all_modules {
                request_config.extend(run_ops::default_party_config("1"));
            }
            let snap = json!({
                "enabled": request_enabled,
                "config": request_config,
                "source": "cli_run_daily",
            });
            let mut tlog = task_log::begin_session_with_snapshot(
                data_dir,
                &group,
                &alias,
                task_log::TaskTrigger::Cli,
                None,
                Some(snap),
            )?;
            match run_ops::run_account_daily(
                data_dir,
                fp_url,
                &acc,
                &request_enabled,
                &request_config,
            )
            .await
            {
                Ok(report) => {
                    for r in &report.results {
                        tlog.modules.push(task_log::ModuleLogEntry {
                            key: r.key.clone(),
                            name: r.name.clone(),
                            status: r.status.clone(),
                            log: r.log.clone(),
                            started_at: None,
                            finished_at: None,
                        });
                    }
                    let st = if report.ok {
                        task_log::TaskStatus::Success
                    } else {
                        task_log::TaskStatus::Error
                    };
                    let msg = format!(
                        "success={} partial={} skipped={} aborted={} errors={}",
                        report.success,
                        report.partial,
                        report.skipped,
                        report.aborted,
                        report.errors
                    );
                    let _ = task_log::finalize_session(data_dir, &mut tlog, st, msg.clone());
                    Ok(json!({
                        "ok": report.ok,
                        "message": msg,
                        "success": report.success,
                        "partial": report.partial,
                        "skipped": report.skipped,
                        "aborted": report.aborted,
                        "errors": report.errors,
                        "results": report.results,
                        "task_session_id": tlog.id,
                    }))
                }
                Err(e) => {
                    let _ = task_log::finalize_session(
                        data_dir,
                        &mut tlog,
                        task_log::TaskStatus::Error,
                        e.to_string(),
                    );
                    Err(e)
                }
            }
        }
        RunCmd::Module {
            group,
            group_password,
            alias,
            key,
            safe_raid_damage,
            ..
        } => {
            assert_daily_allowed().map_err(|e| anyhow::anyhow!(e))?;
            let store = Store::open(data_dir)?;
            let g = store.load_group(&group, group_password.as_deref())?;
            let acc = g
                .accounts
                .iter()
                .find(|a| a.alias == alias)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("account not found: {alias}"))?;
            let ch = Channel::from_user(&acc.channel);
            let _g = gate
                .try_begin_owned(ch.as_str(), &acc.username, &format!("module {key}"), &group)
                .map_err(|e| anyhow::anyhow!(e))?;
            let mut request_config = HashMap::new();
            if safe_raid_damage || key == "self_raid" || key == "support_raid" {
                request_config.extend(run_ops::safe_raid_damage_config());
            }
            request_config.extend(run_ops::default_party_config("1"));
            run_ops::run_account_module(data_dir, fp_url, &acc, &key, &request_config).await
        }
        RunCmd::GroupRaid {
            group,
            group_password,
            config_id,
            aliases,
            room_open,
            party,
            leave_after_support,
            ..
        } => {
            assert_daily_allowed().map_err(|e| anyhow::anyhow!(e))?;
            if !config_id.trim().is_empty() {
                run_ops::exec_group_raid_by_config_id(
                    data_dir,
                    fp_url,
                    gate,
                    &group,
                    group_password.as_deref(),
                    config_id.trim(),
                )
                .await
            } else {
                run_ops::exec_group_raid(
                    data_dir,
                    fp_url,
                    gate,
                    &group,
                    group_password.as_deref(),
                    &aliases,
                    &room_open,
                    &party,
                    leave_after_support,
                )
                .await
            }
        }
    }
}
