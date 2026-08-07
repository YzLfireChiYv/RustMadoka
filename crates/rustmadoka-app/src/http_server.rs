//! Owner HTTP server + SPA static page.
//!
//! Docs: `docs/tech/HTTP_SERVER.md` · `docs/PLAN_RUSTMADOKA_FULL_REWRITE.md`

use crate::app_settings::{prepare_sources, AppSettings, DEFAULT_LISTEN_PORT};
use crate::config_pack;
use crate::data_layout;
use crate::fp_slots;
use crate::ipc::{self, IpcRequest, IpcResponse};
use crate::occupancy;
use crate::owner_lock;
use crate::run_control::RunHub;
use crate::run_ops;
use crate::task_gate::TaskGate;
use crate::task_log;
use crate::{exec_run_cmd_owner, RunCmd, APP_VERSION, BUILD_STAMP, DEFAULT_FP_URL, PRODUCT_EDITION};
use anyhow::{bail, Context, Result};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{delete, get, post, put}; // put: group-raid/config
use axum::{Json, Router};
use futures_util::StreamExt;
use rustmadoka_core::account::{Channel, GameAccount, Store};
use rustmadoka_core::client::GameClient;
use rustmadoka_core::fingerprint::Fingerprint;
use rustmadoka_core::modules::{
    daily_catalog, daily_modules_info, low_risk_module_keys, merge_run_config,
    resolve_enabled_from_store, run_daily_with_progress, run_super_wash_with_progress,
    style_choices, ProgressEvent, ProgressTx, RunControlFlags,
};
use rustmadoka_core::safety::{assert_daily_allowed, assert_tool_allowed, daily_allowed, gates_json, tool_allowed};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub data_dir: PathBuf,
    pub fp_url: String,
    pub default_channel: String,
    pub listen_port: u16,
    pub task_gate: TaskGate,
    pub run_hub: RunHub,
    pub inner: Arc<RwLock<Inner>>,
}

pub struct SessionData {
    pub group: String,
    pub group_password: Option<String>,
}

impl Clone for SessionData {
    fn clone(&self) -> Self {
        Self {
            group: self.group.clone(),
            group_password: self.group_password.clone(),
        }
    }
}

pub struct Inner {
    pub fingerprint: Option<Fingerprint>,
    pub sessions: HashMap<String, SessionData>,
}

/// Android / 嵌入式宿主入口：固定端口、不自动开系统浏览器。
///
/// `fp_url` / `default_channel` 为 None 时用产品默认（rules 指纹源 · en）。
/// Docs: `docs/tech/ANDROID_DUAL_PLATFORM.md` · R7 · `rustmadoka-mobile`
pub async fn run_embedded_serve(
    data_dir: PathBuf,
    port: u16,
    fp_url: Option<String>,
    default_channel: Option<String>,
) -> Result<()> {
    run_owner_serve(
        data_dir,
        fp_url.unwrap_or_else(|| crate::DEFAULT_FP_URL.to_string()),
        default_channel.unwrap_or_else(|| "en".into()),
        Some(port),
        true, // no_browser：WebView 自行打开
        None,
    )
    .await
}

pub async fn run_owner_serve(
    data_dir: PathBuf,
    fp_url: String,
    default_channel: String,
    cli_port: Option<u16>,
    no_browser: bool,
    bootstrap_cmd: Option<RunCmd>,
) -> Result<()> {
    // 布局先于 Owner 锁：保证 users/cache 等存在；layout_schema 向后兼容检查
    let layout = data_layout::ensure_data_layout(&data_dir, crate::APP_VERSION)?;
    tracing::info!(
        layout_schema = layout.layout_schema,
        data_dir = %data_dir.display(),
        "data layout ensured"
    );

    let _occ = occupancy::OccupancyGuard::start(&data_dir)?;
    let _owner = owner_lock::try_acquire(&data_dir)
        .with_context(|| format!("cannot become Owner of {}", data_dir.display()))?;

    let mut settings = AppSettings::load(&data_dir);
    prepare_sources(&mut settings);
    if let Some(p) = cli_port {
        settings.listen_port = p;
        let _ = settings.save(&data_dir);
    }
    let port = settings.listen_port;
    let listener = bind_http_listener(&data_dir, port).await?;
    let addr = listener.local_addr()?;
    tracing::info!(%addr, edition = PRODUCT_EDITION, "RustMadoka Owner listening");
    // PROC-MONITOR-TERMINAL（程序运行面板终端）：启动只读说明 + 完整监视流镜像
    // Docs: docs/tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md §3 · docs/tech/UI_ROUTING_AND_TASK_LOGS.md
    eprintln!();
    eprintln!("======== RustMadoka 程序运行面板终端 ========");
    eprintln!("  本窗口用于监视与异常确认（端口占用 / 跨路径占用）。");
    eprintln!("  正常跑任务时：下方打印【完整监视流】（与浏览器主页 stream_lines 同源）。");
    eprintln!("  非异常状态：本窗口只读，不可在此暂停/放弃；请用浏览器网页前端控制任务。");
    eprintln!("  关闭本窗口 = 退出程序（叉掉即可，无二次确认）。");
    eprintln!("  浏览器网页前端: http://127.0.0.1:{}/", addr.port());
    eprintln!("  数据文件夹: {}", data_dir.display());
    eprintln!("  版本: {} · {}", PRODUCT_EDITION, BUILD_STAMP);
    eprintln!("============================================");
    eprintln!();

    let fp = Fingerprint::load_version_json(&data_dir.join("cache/version.json")).ok();
    let task_gate = TaskGate::new();
    let run_hub = RunHub::new();
    // 完整流按 seq 增量打印 + 忙线摘要变化行（正常态不可控制任务）
    {
        let hub_mon = run_hub.clone();
        tokio::spawn(async move {
            let mut last_seq: u64 = 0;
            let mut last_busy_summary = String::new();
            let mut was_busy = false;
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                let new_lines = hub_mon.stream_lines_after(last_seq);
                for line in &new_lines {
                    eprintln!(
                        "[流] {} 组={} 别名={} ({}) {}",
                        line.ts, line.group, line.alias, line.kind, line.text
                    );
                    last_seq = last_seq.max(line.seq);
                }
                let b = hub_mon.bundle(None);
                if b.busy_any {
                    was_busy = true;
                    let summary = b
                        .runs
                        .iter()
                        .filter(|r| r.busy)
                        .map(|r| {
                            let step = if r.total > 0 {
                                format!(" {}/{}", r.round, r.total)
                            } else {
                                String::new()
                            };
                            format!(
                                "组={} 别名={} {}{} {}",
                                r.group.as_deref().unwrap_or("?"),
                                r.alias.as_deref().unwrap_or("?"),
                                r.kind.as_deref().unwrap_or("?"),
                                step,
                                r.current_name.as_deref().unwrap_or("")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" | ");
                    if summary != last_busy_summary {
                        eprintln!("[监视·摘要] {summary}");
                        last_busy_summary = summary;
                    }
                } else if was_busy {
                    eprintln!("[监视·摘要] 全部空闲");
                    last_busy_summary.clear();
                    was_busy = false;
                }
            }
        });
    }
    let state = AppState {
        data_dir: data_dir.clone(),
        fp_url: fp_url.clone(),
        default_channel: default_channel.clone(),
        listen_port: addr.port(),
        task_gate: task_gate.clone(),
        run_hub: run_hub.clone(),
        inner: Arc::new(RwLock::new(Inner {
            fingerprint: fp,
            sessions: HashMap::new(),
        })),
    };

    let data_dir_ipc = data_dir.clone();
    let fp_url_ipc = fp_url.clone();
    let gate_ipc = task_gate.clone();
    let hub_ipc = run_hub.clone();
    ipc::spawn_ipc_server(data_dir_ipc.clone(), move |req| {
        let data_dir = data_dir_ipc.clone();
        let fp_url = fp_url_ipc.clone();
        let gate = gate_ipc.clone();
        let hub = hub_ipc.clone();
        async move {
            match req {
                IpcRequest::Ping => IpcResponse {
                    ok: true,
                    error: None,
                    result: Some(json!({"pong": true})),
                },
                IpcRequest::RunPause => match hub.pause() {
                    Ok(()) => IpcResponse {
                        ok: true,
                        error: None,
                        result: Some(json!({"ok": true, "message": "已请求暂停"})),
                    },
                    Err(e) => IpcResponse {
                        ok: false,
                        error: Some(e),
                        result: None,
                    },
                },
                IpcRequest::RunResume => match hub.resume() {
                    Ok(()) => IpcResponse {
                        ok: true,
                        error: None,
                        result: Some(json!({"ok": true, "message": "已继续"})),
                    },
                    Err(e) => IpcResponse {
                        ok: false,
                        error: Some(e),
                        result: None,
                    },
                },
                IpcRequest::RunAbort => match hub.abort() {
                    Ok(()) => IpcResponse {
                        ok: true,
                        error: None,
                        result: Some(json!({"ok": true, "message": "已请求放弃"})),
                    },
                    Err(e) => IpcResponse {
                        ok: false,
                        error: Some(e),
                        result: None,
                    },
                },
                IpcRequest::RunStatus => {
                    let b = hub.bundle(None);
                    IpcResponse {
                        ok: true,
                        error: None,
                        result: Some(serde_json::to_value(b).unwrap_or(json!({}))),
                    }
                },
                IpcRequest::RunInfo {
                    group,
                    group_password,
                    alias,
                } => match exec_run_cmd_owner(
                    &data_dir,
                    &fp_url,
                    &gate,
                    RunCmd::Info {
                        group,
                        group_password,
                        alias,
                        json: true,
                        wire: false,
                    },
                )
                .await
                {
                    Ok(v) => IpcResponse {
                        ok: true,
                        error: None,
                        result: Some(v),
                    },
                    Err(e) => IpcResponse {
                        ok: false,
                        error: Some(e.to_string()),
                        result: None,
                    },
                },
                IpcRequest::RunDaily {
                    group,
                    group_password,
                    alias,
                } => match exec_run_cmd_owner(
                    &data_dir,
                    &fp_url,
                    &gate,
                    RunCmd::Daily {
                        group,
                        group_password,
                        alias,
                        json: true,
                        wire: false,
                        all_modules: false,
                        safe_raid_damage: false,
                    },
                )
                .await
                {
                    Ok(v) => IpcResponse {
                        ok: true,
                        error: None,
                        result: Some(v),
                    },
                    Err(e) => IpcResponse {
                        ok: false,
                        error: Some(e.to_string()),
                        result: None,
                    },
                },
                IpcRequest::RunModule {
                    group,
                    group_password,
                    alias,
                    key,
                    safe_raid_damage,
                } => match exec_run_cmd_owner(
                    &data_dir,
                    &fp_url,
                    &gate,
                    RunCmd::Module {
                        group,
                        group_password,
                        alias,
                        key,
                        json: true,
                        wire: false,
                        safe_raid_damage,
                    },
                )
                .await
                {
                    Ok(v) => IpcResponse {
                        ok: true,
                        error: None,
                        result: Some(v),
                    },
                    Err(e) => IpcResponse {
                        ok: false,
                        error: Some(e.to_string()),
                        result: None,
                    },
                },
                IpcRequest::ExportSession {
                    group,
                    group_password,
                    alias,
                    ..
                } => match run_ops::export_account_session(
                    &data_dir,
                    &fp_url,
                    Some(&gate),
                    &group,
                    group_password.as_deref(),
                    &alias,
                    None,
                )
                .await
                {
                    Ok(v) => IpcResponse {
                        ok: true,
                        error: None,
                        result: Some(v),
                    },
                    Err(e) => IpcResponse {
                        ok: false,
                        error: Some(e.to_string()),
                        result: None,
                    },
                },
            }
        }
    });

    if let Some(cmd) = bootstrap_cmd {
        match exec_run_cmd_owner(&data_dir, &fp_url, &task_gate, cmd).await {
            Ok(v) => {
                println!("{}", serde_json::to_string_pretty(&v)?);
                eprintln!(
                    "command done; panel serves http://127.0.0.1:{}/",
                    addr.port()
                );
            }
            Err(e) => eprintln!("command failed: {e}"),
        }
    }

    // 默认源静默日检：Owner 启动时最多每天一次（PLAN_NEXT_FOOLPROOF §8.4）
    {
        let data_dir_fp = data_dir.clone();
        tokio::spawn(async move {
            let day = crate::app_settings::today_day_plus08();
            let mut settings = AppSettings::load(&data_dir_fp);
            if settings.last_version_check_day.as_deref() == Some(day.as_str()) {
                tracing::info!(%day, "skip silent fp refresh (already checked today)");
                return;
            }
            match crate::fp_slots::refresh_default_source(&data_dir_fp, true).await {
                Ok(v) => {
                    let status = v
                        .pointer("/result/status")
                        .and_then(|x| x.as_str())
                        .unwrap_or(if v.get("ok") == Some(&json!(true)) {
                            "ok"
                        } else {
                            "unreachable"
                        });
                    tracing::info!(%day, %status, "silent default fingerprint refresh");
                    settings.last_version_check_day = Some(day);
                    settings.last_version_check = Some(v);
                    let _ = settings.save(&data_dir_fp);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "silent fingerprint refresh failed");
                    settings.last_version_check_day = Some(day);
                    settings.last_version_check =
                        Some(json!({"ok": false, "status": "error", "error": e.to_string()}));
                    let _ = settings.save(&data_dir_fp);
                }
            }
        });
    }

    let app = build_router(state);
    if !no_browser {
        let url = format!("http://127.0.0.1:{}/", addr.port());
        let _ = open::that(&url);
        eprintln!("RustMadoka browser UI: {url}");
    }
    eprintln!(
        "RustMadoka Owner | {PRODUCT_EDITION} | data={} | http://127.0.0.1:{}/",
        data_dir.display(),
        addr.port()
    );
    axum::serve(listener, app).await?;
    Ok(())
}

async fn bind_http_listener(
    data_dir: &std::path::Path,
    port: u16,
) -> Result<tokio::net::TcpListener> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => Ok(l),
        Err(e) => {
            eprintln!("bind {port} failed: {e}");
            eprintln!("Type exactly: 我知道端口被占用");
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            if line.trim() != "我知道端口被占用" {
                bail!("port confirm cancelled");
            }
            eprintln!("Enter new port:");
            line.clear();
            std::io::stdin().read_line(&mut line)?;
            let new_port: u16 = line.trim().parse().context("invalid port")?;
            let mut settings = AppSettings::load(data_dir);
            settings.listen_port = new_port;
            settings.save(data_dir)?;
            let addr = SocketAddr::from(([127, 0, 0, 1], new_port));
            Ok(tokio::net::TcpListener::bind(addr).await?)
        }
    }
}

fn build_router(state: AppState) -> Router {
    // SPA: `/` and any non-API path must serve index.html so browser refresh / deep link /
    // history back re-entry works (docs/tech/UI_ROUTING_AND_TASK_LOGS.md §1).
    // API routes are registered first; GET catch-all is last.
    Router::new()
        .route("/", get(index_html))
        .route("/api/health", get(api_health))
        .route("/api/gates", get(|| async { Json(gates_json()) }))
        .route("/api/groups", get(api_list_groups).post(api_create_group))
        // 改密：PLAN_UI_ROUTING §2 · tech UI_ROUTING API 表 · 成功后废会话
        .route("/api/groups/password", post(api_group_set_password))
        .route("/api/login", post(api_login))
        .route("/api/logout", post(api_logout))
        .route(
            "/api/accounts",
            get(api_list_accounts).post(api_add_account),
        )
        .route("/api/accounts/:alias", delete(api_delete_account))
        .route("/api/accounts/:alias/info", post(api_account_info))
        .route("/api/accounts/:alias/parties", get(api_parties_get))
        .route(
            "/api/accounts/:alias/parties/refresh",
            post(api_parties_refresh),
        )
        // 关卡 ID↔名称：账号渠道缓存优先；?refresh=1 对服拉取
        .route(
            "/api/accounts/:alias/mst/quest-stages",
            get(api_account_quest_stages),
        )
        // 仅读数据文件夹缓存（不登录）；?channel=en|jp
        .route("/api/mst/quest-stages", get(api_mst_quest_stages_cache))
        .route(
            "/api/accounts/:alias/config",
            get(api_get_config).post(api_set_config),
        )
        .route("/api/accounts/:alias/config/auto", post(api_config_auto))
        .route("/api/accounts/:alias/daily", post(api_account_daily))
        .route(
            "/api/accounts/:alias/daily/stream",
            post(api_account_daily_stream),
        )
        .route(
            "/api/accounts/:alias/module/:key/run",
            post(api_module_run),
        )
        .route(
            "/api/accounts/:alias/task_logs",
            get(api_task_logs).delete(api_task_logs_clear),
        )
        .route("/api/accounts/:alias/task_logs/:id", get(api_task_log_one))
        .route(
            "/api/accounts/:alias/task_logs/:id/progress",
            get(api_task_log_progress),
        )
        .route("/api/accounts/:alias/wash_options", get(api_wash_options))
        .route("/api/accounts/:alias/wash", post(api_wash))
        .route(
            "/api/accounts/:alias/export_settings",
            get(api_export_settings),
        )
        .route(
            "/api/accounts/:alias/import_settings",
            post(api_import_settings),
        )
        .route("/api/accounts/:alias/copy_config", post(api_copy_config))
        .route(
            "/api/accounts/:alias/shop_pack",
            get(api_shop_pack_get).post(api_shop_pack_post),
        )
        .route(
            "/api/accounts/:alias/config_pack",
            get(api_config_pack_get).post(api_config_pack_post),
        )
        .route(
            "/api/accounts/:alias/config_file",
            get(api_config_file_get).post(api_config_file_post),
        )
        .route("/api/run/status", get(api_run_status))
        .route("/api/run/pause", post(api_run_pause))
        .route("/api/run/resume", post(api_run_resume))
        .route("/api/run/abort", post(api_run_abort))
        .route("/api/run/clear_report", post(api_run_clear))
        .route("/api/group-raid", post(api_group_raid))
        .route(
            "/api/group-raid/config",
            get(api_group_raid_config_get).put(api_group_raid_config_put),
        )
        .route(
            "/api/group-raid/entry",
            post(api_group_raid_entry_upsert).delete(api_group_raid_entry_delete),
        )
        .route("/api/version", get(api_version))
        .route("/api/version/check", post(api_version_check))
        .route("/api/version/fetch", post(api_version_fetch))
        .route("/api/version/paste", post(api_version_paste))
        .route("/api/version/sources", post(api_version_sources_save))
        .route("/api/version/sources/add", post(api_version_sources_add))
        .route(
            "/api/version/sources/delete",
            post(api_version_sources_delete),
        )
        .route("/api/version/sources/test", post(api_version_sources_test))
        .route("/api/fp/slots", get(api_fp_slots))
        .route("/api/fp/default/refresh", post(api_fp_refresh))
        .route("/api/fp/slots/activate", post(api_fp_activate))
        .route("/api/fp/slots/clear", post(api_fp_clear))
        .route("/api/fp/slots/fill", post(api_fp_fill))
        .route("/api/fp/slots/reset", post(api_fp_reset))
        .route("/api/diagnose", post(api_diagnose))
        .route(
            "/api/features/settings/notifications",
            get(api_settings_notifications),
        )
        .route(
            "/api/system-toast",
            get(api_system_toast_get).post(api_system_toast_set),
        )
        // Deep links: /{group}, /{group}/{alias}, /{group}/检测, … → same SPA shell
        .route("/*path", get(index_html))
        .fallback(get(index_html))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Browser web frontend shell. Must be returned for deep URLs (not only `/`).
async fn index_html() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn session_of(st: &AppState, headers: &HeaderMap) -> Option<SessionData> {
    let t = headers.get("x-token")?.to_str().ok()?.to_string();
    st.inner.read().await.sessions.get(&t).cloned()
}

fn require_session(s: Option<SessionData>) -> Result<SessionData, Response> {
    s.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "login required"})),
        )
            .into_response()
    })
}

async fn load_acc(
    st: &AppState,
    headers: &HeaderMap,
    alias: &str,
) -> Result<(SessionData, GameAccount, Store), Response> {
    let sess = require_session(session_of(st, headers).await)?;
    let store = Store::open(&st.data_dir).map_err(err_resp)?;
    let g = store
        .load_group(&sess.group, sess.group_password.as_deref())
        .map_err(err_resp)?;
    let acc = g
        .accounts
        .iter()
        .find(|a| a.alias == alias)
        .cloned()
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"ok": false, "error": "alias not found"})),
            )
                .into_response()
        })?;
    Ok((sess, acc, store))
}

fn err_resp(e: impl std::fmt::Display) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"ok": false, "error": e.to_string()})),
    )
        .into_response()
}

async fn api_health(State(st): State<AppState>) -> Json<Value> {
    let settings = AppSettings::load(&st.data_dir);
    // Debug 探针摘要（普通版 wire_built=false）
    // Docs: docs/tech/WIRE_AND_DEBUG_PROBES.md
    Json(json!({
        "ok": true,
        "product": "RustMadoka",
        "edition": PRODUCT_EDITION,
        "version": APP_VERSION,
        "build_stamp": BUILD_STAMP,
        "listen_port": st.listen_port,
        "default_port": DEFAULT_LISTEN_PORT,
        "port_is_default": st.listen_port == DEFAULT_LISTEN_PORT,
        "data_dir": st.data_dir.display().to_string(),
        "run": st.run_hub.snapshot(),
        "settings_port": settings.listen_port,
        "debug": {
            "wire_built": rustmadoka_core::wire::is_built_with_wire(),
            "wire_active": rustmadoka_core::wire::is_active(),
            "wire_dir": rustmadoka_core::wire::current_dir().map(|p| p.display().to_string()),
            "doc": "docs/tech/WIRE_AND_DEBUG_PROBES.md",
        },
        "session_pool": crate::session_pool::process_pool().stats_json().await,
    }))
}

async fn api_list_groups(State(st): State<AppState>) -> impl IntoResponse {
    match Store::open(&st.data_dir).and_then(|s| s.list_groups()) {
        Ok(list) => Json(json!({"ok": true, "groups": list})).into_response(),
        Err(e) => err_resp(e),
    }
}

#[derive(Deserialize)]
struct NamePw {
    name: String,
    #[serde(default)]
    password: String,
}

async fn api_create_group(
    State(st): State<AppState>,
    Json(body): Json<NamePw>,
) -> impl IntoResponse {
    let store = match Store::open(&st.data_dir) {
        Ok(s) => s,
        Err(e) => return err_resp(e),
    };
    let pw = if body.password.is_empty() {
        None
    } else {
        Some(body.password.as_str())
    };
    match store.create_group(&body.name, pw) {
        Ok(_) => Json(json!({"ok": true, "group": body.name})).into_response(),
        Err(e) => err_resp(e),
    }
}

#[derive(Deserialize)]
struct SetGroupPasswordBody {
    name: String,
    #[serde(default)]
    old_password: String,
    /// 空字符串 = 清除密码改回明文组
    #[serde(default)]
    new_password: String,
}

/// POST /api/groups/password — 改密后废该组全部 HTTP 会话（须重新验密）
async fn api_group_set_password(
    State(st): State<AppState>,
    Json(body): Json<SetGroupPasswordBody>,
) -> impl IntoResponse {
    let store = match Store::open(&st.data_dir) {
        Ok(s) => s,
        Err(e) => return err_resp(e),
    };
    if !store.group_exists(&body.name) {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "ok": false,
                "error": format!("用户组不存在：{}", body.name),
                "code": "group_not_found",
            })),
        )
            .into_response();
    }
    let old = if body.old_password.is_empty() {
        None
    } else {
        Some(body.old_password.as_str())
    };
    let new = if body.new_password.is_empty() {
        None
    } else {
        Some(body.new_password.as_str())
    };
    match store.set_group_password(&body.name, old, new) {
        Ok(g) => {
            // 作废该组所有 token（含其它标签页）
            let mut inner = st.inner.write().await;
            inner.sessions.retain(|_, s| s.group != body.name);
            drop(inner);
            Json(json!({
                "ok": true,
                "group": body.name,
                "has_password": g.has_password,
                "must_relogin": true,
                "message": "密码已更新，请重新登录该用户组",
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": e.to_string(), "code": "password_change_failed"})),
        )
            .into_response(),
    }
}

async fn api_login(State(st): State<AppState>, Json(body): Json<NamePw>) -> impl IntoResponse {
    let store = match Store::open(&st.data_dir) {
        Ok(s) => s,
        Err(e) => return err_resp(e),
    };
    let pw = if body.password.is_empty() {
        None
    } else {
        Some(body.password.as_str())
    };
    // 不存在的组：404 语义，便于前端「错 path 回退到登录/选组」
    if !store.group_exists(&body.name) {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "ok": false,
                "error": format!("用户组不存在：{}", body.name),
                "code": "group_not_found",
            })),
        )
            .into_response();
    }
    match store.load_group(&body.name, pw) {
        Ok(mut g) => {
            // 有密码却空密码：明确拒绝（明文组 has_password=false 才允许空密码）
            if g.has_password && body.password.is_empty() {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "ok": false,
                        "error": "该用户组需要密码",
                        "code": "password_required",
                        "has_password": true,
                    })),
                )
                    .into_response();
            }
            let _ = store.touch_login(&mut g);
            let token = uuid::Uuid::new_v4().to_string();
            st.inner.write().await.sessions.insert(
                token.clone(),
                SessionData {
                    group: body.name.clone(),
                    group_password: if body.password.is_empty() {
                        None
                    } else {
                        Some(body.password)
                    },
                },
            );
            Json(json!({
                "ok": true,
                "token": token,
                "group": body.name,
                "has_password": g.has_password,
                "gates": gates_json(),
            }))
            .into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let code = if msg.contains("不存在") {
                "group_not_found"
            } else if msg.contains("密码") {
                "bad_password"
            } else {
                "login_failed"
            };
            let status = if code == "group_not_found" {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::UNAUTHORIZED
            };
            (
                status,
                Json(json!({"ok": false, "error": msg, "code": code})),
            )
                .into_response()
        }
    }
}

/// 注销当前 token（切换用户组 / 改密后客户端配合）
async fn api_logout(State(st): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Some(t) = headers
        .get("x-token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
    {
        st.inner.write().await.sessions.remove(&t);
    }
    Json(json!({"ok": true})).into_response()
}

async fn api_list_accounts(State(st): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let sess = match require_session(session_of(&st, &headers).await) {
        Ok(s) => s,
        Err(r) => return r,
    };
    let store = match Store::open(&st.data_dir) {
        Ok(s) => s,
        Err(e) => return err_resp(e),
    };
    let g = match store.load_group(&sess.group, sess.group_password.as_deref()) {
        Ok(g) => g,
        Err(e) => return err_resp(e),
    };
    // MULTI_GROUP §5：按 game_id_hash 投影运行态；跨组时文案含发起用户组（不回传引继明文）
    let hub_bundle = st.run_hub.bundle(None);
    let accounts: Vec<_> = g
        .accounts
        .iter()
        .map(|a| {
            let gid = TaskGate::game_id_hash(&a.channel, &a.username);
            let run = hub_bundle.runs.iter().find(|r| {
                r.game_id_hash.as_deref() == Some(gid.as_str())
                    || (r.busy && r.alias.as_deref() == Some(a.alias.as_str()))
            });
            let (busy, run_group, run_kind, run_label) = match run {
                Some(r) if r.busy => {
                    let rg = r.group.clone().unwrap_or_default();
                    let kind = r.kind.clone().unwrap_or_else(|| "任务".into());
                    let name = r
                        .current_name
                        .clone()
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| kind.clone());
                    let label = if !rg.is_empty() && rg != sess.group {
                        format!("在用户组「{rg}」中正在运行：{name}")
                    } else {
                        format!("{name} 运行中")
                    };
                    (true, Some(rg), Some(kind), Some(label))
                }
                _ => (false, None, None, None),
            };
            json!({
                "alias": a.alias,
                "channel": a.channel,
                "game_name": a.game_name,
                "level": a.level,
                "info_fetched_at": a.info_fetched_at,
                "game_id_hash": gid,
                "busy": busy,
                "run_group": run_group,
                "run_kind": run_kind,
                "run_label": run_label,
            })
        })
        .collect();
    // group 必须回传，供前端校验「会话用户组 === URL 用户组」（PLAN_UI_ROUTING · C23）
    Json(json!({
        "ok": true,
        "group": sess.group,
        "has_password": g.has_password,
        "accounts": accounts,
    }))
    .into_response()
}

/// GET 队伍缓存（PARTY-SELECT）；无缓存时 parties=[] 并提示刷新
async fn api_parties_get(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
) -> impl IntoResponse {
    let (_, acc, _) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    let ch = Channel::from_user(&acc.channel);
    match run_ops::load_party_cache(&st.data_dir, ch.as_str(), &acc.username) {
        Some(doc) => Json(json!({
            "ok": true,
            "from_cache": true,
            "parties": doc.get("parties").cloned().unwrap_or(json!([])),
            "fetched_at": doc.get("fetched_at"),
            "game_id_hash": doc.get("game_id_hash"),
        }))
        .into_response(),
        None => Json(json!({
            "ok": true,
            "from_cache": false,
            "parties": [],
            "message": "尚无队伍缓存。请点「刷新队伍列表」（完整登录）或先跑一次信息/日常。",
        }))
        .into_response(),
    }
}

/// POST 完整登录刷新 partyDataList 并落盘缓存
async fn api_parties_refresh(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
) -> impl IntoResponse {
    let (sess, acc, _) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    let ch = Channel::from_user(&acc.channel);
    let _guard = match st
        .task_gate
        .try_begin_owned(ch.as_str(), &acc.username, "parties", &sess.group)
    {
        Ok(g) => g,
        Err(e) => return err_resp(e),
    };
    match run_ops::refresh_party_list(&st.data_dir, &st.fp_url, &acc).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_resp(e),
    }
}

/// 关卡对照查询参数（CLI `mst quest-stages` 对等）
#[derive(Deserialize)]
struct QuestStageQuery {
    #[serde(default)]
    id: Option<i64>,
    #[serde(default)]
    group_id: Option<i64>,
    #[serde(default)]
    filter: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    /// 1/true 时对服刷新
    #[serde(default)]
    refresh: Option<String>,
    #[serde(default)]
    channel: Option<String>,
}

fn quest_stage_limit(q: &QuestStageQuery) -> usize {
    q.limit.unwrap_or(200)
}

fn quest_stage_refresh(q: &QuestStageQuery) -> bool {
    matches!(
        q.refresh.as_deref().map(|s| s.trim().to_ascii_lowercase()),
        Some(s) if s == "1" || s == "true" || s == "yes"
    )
}

/// GET /api/accounts/:alias/mst/quest-stages — 优先缓存，?refresh=1 登录拉 mst
///
/// Docs: BASIC_SUPER_SWEEP · CLI `mst quest-stages`
async fn api_account_quest_stages(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
    Query(q): Query<QuestStageQuery>,
) -> impl IntoResponse {
    let (sess, acc, _) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    let refresh = quest_stage_refresh(&q);
    if refresh {
        let ch = Channel::from_user(&acc.channel);
        let _guard = match st.task_gate.try_begin_owned(
            ch.as_str(),
            &acc.username,
            "mst_quest_stages",
            &sess.group,
        ) {
            Ok(g) => g,
            Err(e) => return err_resp(e),
        };
    }
    match run_ops::query_quest_stages(
        &st.data_dir,
        &st.fp_url,
        &acc,
        refresh,
        q.id,
        q.group_id,
        q.filter.as_deref(),
        quest_stage_limit(&q),
    )
    .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_resp(e),
    }
}

/// GET /api/mst/quest-stages?channel=jp&filter=キオク — 只读本地缓存，不登录
async fn api_mst_quest_stages_cache(
    State(st): State<AppState>,
    Query(q): Query<QuestStageQuery>,
) -> impl IntoResponse {
    let channel = q.channel.as_deref().unwrap_or("en");
    match run_ops::query_quest_stages_from_cache(
        &st.data_dir,
        channel,
        q.id,
        q.group_id,
        q.filter.as_deref(),
        quest_stage_limit(&q),
    ) {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_resp(e),
    }
}

#[derive(Deserialize)]
struct AddAcc {
    alias: String,
    username: String,
    password: String,
    #[serde(default = "default_en")]
    channel: String,
}
fn default_en() -> String {
    "en".into()
}

async fn api_add_account(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AddAcc>,
) -> impl IntoResponse {
    let sess = match require_session(session_of(&st, &headers).await) {
        Ok(s) => s,
        Err(r) => return r,
    };
    let store = match Store::open(&st.data_dir) {
        Ok(s) => s,
        Err(e) => return err_resp(e),
    };
    let mut g = match store.load_group(&sess.group, sess.group_password.as_deref()) {
        Ok(g) => g,
        Err(e) => return err_resp(e),
    };
    if g.accounts.iter().any(|a| a.alias == body.alias) {
        return err_resp("alias exists");
    }
    g.accounts.push(GameAccount {
        alias: body.alias.clone(),
        channel: body.channel,
        username: body.username,
        password: body.password,
        game_name: String::new(),
        level: 0,
        info_fetched_at: None,
        config: HashMap::new(),
    });
    if let Err(e) = store.save_group(&g) {
        return err_resp(e);
    }
    Json(json!({"ok": true, "alias": body.alias})).into_response()
}

async fn api_delete_account(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
) -> impl IntoResponse {
    let sess = match require_session(session_of(&st, &headers).await) {
        Ok(s) => s,
        Err(r) => return r,
    };
    let store = match Store::open(&st.data_dir) {
        Ok(s) => s,
        Err(e) => return err_resp(e),
    };
    let mut g = match store.load_group(&sess.group, sess.group_password.as_deref()) {
        Ok(g) => g,
        Err(e) => return err_resp(e),
    };
    g.accounts.retain(|a| a.alias != alias);
    if let Err(e) = store.save_group(&g) {
        return err_resp(e);
    }
    Json(json!({"ok": true})).into_response()
}

async fn api_account_info(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
) -> impl IntoResponse {
    let (sess, acc, store) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    let ch = Channel::from_user(&acc.channel);
    let _guard = match st
        .task_gate
        .try_begin_owned(ch.as_str(), &acc.username, "info", &sess.group)
    {
        Ok(g) => g,
        Err(e) => return err_resp(e),
    };
    match run_ops::fetch_account_info(&st.data_dir, &st.fp_url, &acc).await {
        Ok(info) => {
            if let Ok(mut g) = store.load_group(&sess.group, sess.group_password.as_deref()) {
                if let Some(a) = g.accounts.iter_mut().find(|a| a.alias == alias) {
                    a.game_name = info["name"].as_str().unwrap_or("").to_string();
                    a.level = info["level"].as_i64().unwrap_or(0);
                    a.info_fetched_at = Some(chrono::Utc::now().to_rfc3339());
                }
                let _ = store.save_group(&g);
            }
            Json(json!({"ok": true, "info": info})).into_response()
        }
        Err(e) => err_resp(e),
    }
}

async fn api_get_config(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
) -> impl IntoResponse {
    let (_, acc, _) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    // Frontend openSettingsData expects `modules` + `low_risk` (schema with config fields).
    Json(json!({
        "ok": true,
        "config": acc.config,
        "channel": acc.channel,
        "alias": acc.alias,
        "defaults": rustmadoka_core::all_setting_defaults(),
        "catalog": daily_catalog(),
        "modules": daily_modules_info(),
        "low_risk": low_risk_module_keys(),
        "gates": gates_json(),
    }))
    .into_response()
}

#[derive(Deserialize)]
struct ConfigBody {
    #[serde(default)]
    config: HashMap<String, Value>,
}

async fn api_set_config(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
    Json(body): Json<ConfigBody>,
) -> impl IntoResponse {
    let (sess, _, store) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    let mut g = match store.load_group(&sess.group, sess.group_password.as_deref()) {
        Ok(g) => g,
        Err(e) => return err_resp(e),
    };
    if let Some(a) = g.accounts.iter_mut().find(|a| a.alias == alias) {
        a.config = body.config;
        let _ = crate::settings_files::mirror_account_settings(
            &st.data_dir,
            &sess.group,
            &alias,
            &a.config,
        );
    }
    if let Err(e) = store.save_group(&g) {
        return err_resp(e);
    }
    Json(json!({"ok": true})).into_response()
}

#[derive(Deserialize)]
struct ConfigAuto {
    #[serde(default)]
    patch: HashMap<String, Value>,
    /// 可选：前端变更说明（仅通知用）
    #[serde(default)]
    changes: Vec<Value>,
}

async fn api_config_auto(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
    Json(body): Json<ConfigAuto>,
) -> impl IntoResponse {
    let (sess, _, store) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    let mut g = match store.load_group(&sess.group, sess.group_password.as_deref()) {
        Ok(g) => g,
        Err(e) => return err_resp(e),
    };
    let mut saved_cfg = HashMap::new();
    if let Some(a) = g.accounts.iter_mut().find(|a| a.alias == alias) {
        for (k, v) in body.patch {
            a.config.insert(k, v);
        }
        saved_cfg = a.config.clone();
        let _ = crate::settings_files::mirror_account_settings(
            &st.data_dir,
            &sess.group,
            &alias,
            &a.config,
        );
    }
    if let Err(e) = store.save_group(&g) {
        return err_resp(e);
    }
    // 多窗口同步：落盘通知（本窗不因 toast 抢焦点；其它窗靠签名轮询）
    {
        use rustmadoka_core::{append_settings_notify, ChangeAfter};
        let changes: Vec<ChangeAfter> = body
            .changes
            .iter()
            .filter_map(|c| {
                let key = c.get("key")?.as_str()?.to_string();
                let label = c
                    .get("label")
                    .and_then(|x| x.as_str())
                    .unwrap_or(key.as_str())
                    .to_string();
                let after = c.get("after").cloned().unwrap_or(Value::Null);
                Some(ChangeAfter { key, label, after })
            })
            .collect();
        if !changes.is_empty() {
            let _ = append_settings_notify(&st.data_dir, &sess.group, &alias, changes);
        }
    }
    Json(json!({
        "ok": true,
        "message": "已保存",
        "config": saved_cfg,
    }))
    .into_response()
}

/// 一键/单次日常请求体：enabled 与 config 覆盖磁盘（save 仅前端语义，后端不因 save 改开关）
#[derive(Deserialize, Default)]
struct DailyRunBody {
    #[serde(default)]
    enabled: HashMap<String, bool>,
    #[serde(default)]
    config: HashMap<String, Value>,
    #[serde(default)]
    save: bool,
}

async fn api_account_daily(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
    body: Option<Json<DailyRunBody>>,
) -> impl IntoResponse {
    let body = body.map(|j| j.0).unwrap_or_default();
    let (sess, acc, _) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    if let Err(e) = assert_daily_allowed() {
        return err_resp(e);
    }
    let ch = Channel::from_user(&acc.channel);
    let _guard = match st
        .task_gate
        .try_begin_owned(ch.as_str(), &acc.username, "daily", &sess.group)
    {
        Ok(g) => g,
        Err(e) => return err_resp(e),
    };
    let acc_key = TaskGate::account_key(ch.as_str(), &acc.username);
    let flags = Arc::new(RunControlFlags::default());
    let sid = uuid::Uuid::new_v4().to_string();
    st.run_hub.begin(
        flags,
        sid,
        sess.group.clone(),
        alias.clone(),
        "daily".into(),
        acc_key.clone(),
    );
    let enabled = resolve_enabled_from_store(
        &run_ops::catalog_default_pairs(),
        &acc.config,
        &body.enabled,
    );
    let config = merge_run_config(&acc.config, &body.config);
    let snap = json!({
        "enabled": enabled,
        "config": config,
        "source": "http_daily",
        "save_flag_ignored_for_enabled": true,
        "client_save": body.save,
    });
    let mut tlog = match task_log::begin_session_with_snapshot(
        &st.data_dir,
        &sess.group,
        &alias,
        task_log::TaskTrigger::OneClickDaily,
        None,
        Some(snap),
    ) {
        Ok(s) => s,
        Err(e) => {
            st.run_hub
                .end_with_report(&acc_key, e.to_string(), false);
            return err_resp(e);
        }
    };
    let data_dir = st.data_dir.clone();
    let fp_url = st.fp_url.clone();
    let hub = st.run_hub.clone();
    let acc2 = acc.clone();
    match run_ops::run_account_daily(&data_dir, &fp_url, &acc2, &enabled, &config).await {
        Ok(r) => {
            for row in &r.results {
                tlog.modules.push(task_log::ModuleLogEntry {
                    key: row.key.clone(),
                    name: row.name.clone(),
                    status: row.status.clone(),
                    log: row.log.clone(),
                    started_at: None,
                    finished_at: None,
                });
            }
            let msg = format!(
                "成功{} 部分{} 跳过{} 中止{} 错误{}",
                r.success, r.partial, r.skipped, r.aborted, r.errors
            );
            let stt = if r.ok {
                task_log::TaskStatus::Success
            } else {
                task_log::TaskStatus::Error
            };
            let _ = task_log::finalize_session(&data_dir, &mut tlog, stt, msg.clone());
            hub.end_with_report(&acc_key, msg.clone(), r.ok);
            Json(json!({
                "ok": r.ok,
                "message": msg,
                "results": r.results,
                "task_session_id": tlog.id,
            }))
            .into_response()
        }
        Err(e) => {
            let _ = task_log::finalize_session(
                &data_dir,
                &mut tlog,
                task_log::TaskStatus::Error,
                e.to_string(),
            );
            hub.end_with_report(&acc_key, e.to_string(), false);
            err_resp(e)
        }
    }
}

async fn api_account_daily_stream(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
    body: Option<Json<DailyRunBody>>,
) -> impl IntoResponse {
    let body = body.map(|j| j.0).unwrap_or_default();
    let (sess, acc, _) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    if let Err(e) = assert_daily_allowed() {
        return err_resp(e);
    }
    let ch = Channel::from_user(&acc.channel);
    let guard = match st
        .task_gate
        .try_begin_owned(ch.as_str(), &acc.username, "daily", &sess.group)
    {
        Ok(g) => g,
        Err(e) => return err_resp(e),
    };
    let acc_key = TaskGate::account_key(ch.as_str(), &acc.username);
    let flags = Arc::new(RunControlFlags::default());
    let sid = uuid::Uuid::new_v4().to_string();
    st.run_hub.begin(
        flags.clone(),
        sid,
        sess.group.clone(),
        alias.clone(),
        "daily".into(),
        acc_key.clone(),
    );
    // 请求体 enabled/config 必须生效（浏览器网页前端一键清日常依赖此契约）
    let enabled = resolve_enabled_from_store(
        &run_ops::catalog_default_pairs(),
        &acc.config,
        &body.enabled,
    );
    let config = merge_run_config(&acc.config, &body.config);
    let snap = json!({
        "enabled": enabled,
        "config": config,
        "source": "http_daily_stream",
        "client_save": body.save,
    });
    let tlog = match task_log::begin_session_with_snapshot(
        &st.data_dir,
        &sess.group,
        &alias,
        task_log::TaskTrigger::OneClickDaily,
        None,
        Some(snap),
    ) {
        Ok(s) => s,
        Err(e) => {
            st.run_hub
                .end_with_report(&acc_key, e.to_string(), false);
            return err_resp(e);
        }
    };
    // 进度双写：NDJSON 给发起页 + RunHub 给跨组设置页（避免发起方进度条超前一拍）
    let (tx_mod, mut rx_mod) = mpsc::unbounded_channel::<ProgressEvent>();
    let progress_tx = Some(tx_mod);
    let (tx_out, rx_out) = mpsc::unbounded_channel::<ProgressEvent>();
    let data_dir = st.data_dir.clone();
    let fp_url = st.fp_url.clone();
    let hub = st.run_hub.clone();
    let acc2 = acc.clone();
    let group_name = sess.group.clone();
    let alias_name = alias.clone();
    let acc_key_fwd = acc_key.clone();
    let hub_fwd = hub.clone();
    let tx_client = tx_out.clone();
    tokio::spawn(async move {
        while let Some(ev) = rx_mod.recv().await {
            if !ev.done {
                hub_fwd.update_progress(
                    &acc_key_fwd,
                    ev.round,
                    ev.total,
                    &ev.key,
                    &ev.name,
                    &ev.status,
                    &ev.message,
                );
            }
            if tx_client.send(ev).is_err() {
                break;
            }
        }
    });
    tokio::spawn(async move {
        let _guard = guard;
        let mut tlog = tlog;
        let ch = Channel::from_user(&acc2.channel);
        let _wire = crate::wire_scope::WireScope::enter(
            &data_dir,
            &alias_name,
            ch.as_str(),
            "daily_stream",
        );
        hub.update_progress(
            &acc_key,
            0,
            0,
            "daily",
            "清日常",
            "running",
            "登录并准备中…",
        );
        let result = async {
            let fp = crate::fp_load::load_fp(&data_dir, &fp_url, ch.as_str()).await?;
            let pool = crate::session_pool::process_pool();
            let (skey, mut client, skind) = pool
                .acquire_full(
                    ch.as_str(),
                    &acc2.username,
                    &acc2.password,
                    fp,
                    &data_dir,
                )
                .await?;
            let report = run_daily_with_progress(
                &mut client,
                &enabled,
                &config,
                &progress_tx,
                Some(flags),
            )
            .await;
            pool.release(skey, skind, client, false).await;
            Ok::<_, anyhow::Error>(report)
        }
        .await;
        drop(progress_tx);
        match result {
            Ok(r) => {
                for row in &r.results {
                    tlog.modules.push(task_log::ModuleLogEntry {
                        key: row.key.clone(),
                        name: row.name.clone(),
                        status: row.status.clone(),
                        log: row.log.clone(),
                        started_at: None,
                        finished_at: None,
                    });
                }
                let msg = format!(
                    "成功{} 部分{} 跳过{} 中止{} 错误{}",
                    r.success, r.partial, r.skipped, r.aborted, r.errors
                );
                let stt = if r.ok {
                    task_log::TaskStatus::Success
                } else {
                    task_log::TaskStatus::Error
                };
                let _ = task_log::finalize_session(&data_dir, &mut tlog, stt, msg.clone());
                hub.end_with_report(&acc_key, msg.clone(), r.ok);
                let _ = tx_out.send(ProgressEvent {
                    kind: "daily".into(),
                    key: "daily".into(),
                    name: "清日常".into(),
                    round: 0,
                    total: 0,
                    status: if r.ok { "done".into() } else { "error".into() },
                    message: format!("{msg} · 日志 {tlog_id}", tlog_id = tlog.id),
                    done: true,
                });
                rustmadoka_core::wire::stop();
            }
            Err(e) => {
                let _ = task_log::finalize_session(
                    &data_dir,
                    &mut tlog,
                    task_log::TaskStatus::Error,
                    e.to_string(),
                );
                hub.end_with_report(&acc_key, e.to_string(), false);
                let _ = tx_out.send(ProgressEvent {
                    kind: "daily".into(),
                    key: "daily".into(),
                    name: "清日常".into(),
                    round: 0,
                    total: 0,
                    status: "error".into(),
                    message: e.to_string(),
                    done: true,
                });
                rustmadoka_core::wire::stop();
            }
        }
        let _ = group_name;
    });
    let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx_out).map(|ev| {
        let line = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".into());
        Ok::<_, std::io::Error>(format!("{line}\n"))
    });
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "application/x-ndjson; charset=utf-8",
        )],
        axum::body::Body::from_stream(stream),
    )
        .into_response()
}

/// 单模块运行：NDJSON 流式进度（快速刷图逐轮）+ RunHub 实时更新 + 定稿 task_log。
/// Content-Type: application/x-ndjson（浏览器须用 fetch 读流，勿当 JSON 一次解析）。
/// Docs: docs/tech/UI_ROUTING_AND_TASK_LOGS.md · C7 · L6
async fn api_module_run(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath((alias, key)): AxumPath<(String, String)>,
) -> impl IntoResponse {
    let (sess, acc, _) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    if let Err(e) = assert_daily_allowed() {
        return err_resp(e);
    }
    let ch = Channel::from_user(&acc.channel);
    let guard = match st.task_gate.try_begin_owned(
        ch.as_str(),
        &acc.username,
        &format!("module:{key}"),
        &sess.group,
    ) {
        Ok(g) => g,
        Err(e) => return err_resp(e),
    };
    let acc_key = TaskGate::account_key(ch.as_str(), &acc.username);
    let flags = Arc::new(RunControlFlags::default());
    let sid = uuid::Uuid::new_v4().to_string();
    let mod_name = daily_catalog()
        .iter()
        .find(|e| e.key == key)
        .map(|e| e.name.to_string())
        .unwrap_or_else(|| key.clone());
    st.run_hub.begin(
        flags.clone(),
        sid,
        sess.group.clone(),
        alias.clone(),
        format!("module:{key}"),
        acc_key.clone(),
    );
    st.run_hub.update_progress(
        &acc_key,
        0,
        0,
        &key,
        &mod_name,
        "running",
        "登录并准备中…",
    );
    let mut tlog = match task_log::begin_session_with_snapshot(
        &st.data_dir,
        &sess.group,
        &alias,
        task_log::TaskTrigger::SingleModule,
        Some(key.clone()),
        Some(json!({"module": key, "source": "http_module_stream"})),
    ) {
        Ok(s) => s,
        Err(e) => {
            st.run_hub
                .end_with_report(&acc_key, e.to_string(), false);
            return err_resp(e);
        }
    };
    let _ = task_log::write_progress(&st.data_dir, &tlog);

    // module → progress_tx → 转发：RunHub + progress 文件 + NDJSON 客户端
    let (progress_tx_inner, mut progress_rx) = mpsc::unbounded_channel::<ProgressEvent>();
    let progress_tx: ProgressTx = Some(progress_tx_inner);
    let (tx_out, rx_out) = mpsc::unbounded_channel::<ProgressEvent>();
    let tx_client = tx_out.clone();

    let hub = st.run_hub.clone();
    let data_dir = st.data_dir.clone();
    let fp_url = st.fp_url.clone();
    let acc2 = acc.clone();
    let key2 = key.clone();
    let acc_key2 = acc_key.clone();
    let mod_name2 = mod_name.clone();

    tokio::spawn(async move {
        let _guard = guard;
        let hub_w = hub.clone();
        let acc_key_w = acc_key2.clone();
        let data_dir_w = data_dir.clone();
        let mut tlog_prog = tlog.clone();
        let tx_fwd = tx_out;

        let forward = tokio::spawn(async move {
            while let Some(ev) = progress_rx.recv().await {
                if !ev.done {
                    hub_w.update_progress(
                        &acc_key_w,
                        ev.round,
                        ev.total,
                        &ev.key,
                        &ev.name,
                        &ev.status,
                        &ev.message,
                    );
                    tlog_prog.message = format!(
                        "[{}] {} ({}/{})",
                        ev.status, ev.message, ev.round, ev.total
                    );
                    tlog_prog.status = task_log::TaskStatus::Running;
                    let _ = task_log::write_progress(&data_dir_w, &tlog_prog);
                }
                if tx_fwd.send(ev).is_err() {
                    break;
                }
            }
        });

        let _ = tx_client.send(ProgressEvent::info(
            "module",
            &key2,
            &mod_name2,
            "登录并准备中…",
        ));

        let t0 = std::time::Instant::now();
        let result = run_ops::run_account_module_with_progress(
            &data_dir,
            &fp_url,
            &acc2,
            &key2,
            &HashMap::new(),
            &progress_tx,
            t0,
        )
        .await;
        drop(progress_tx);
        let _ = forward.await;

        match result {
            Ok(v) => {
                let status = v
                    .get("status")
                    .and_then(|x| x.as_str())
                    .unwrap_or("error")
                    .to_string();
                let log = v
                    .get("log")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let ok = v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
                let msg = format!("[{status}] {mod_name2}：{log}");
                tlog.modules.push(task_log::ModuleLogEntry {
                    key: key2.clone(),
                    name: mod_name2.clone(),
                    status: status.clone(),
                    log: log.clone(),
                    started_at: None,
                    finished_at: None,
                });
                let stt = match status.as_str() {
                    "success" | "skip" | "partial" => task_log::TaskStatus::Success,
                    "abort" => task_log::TaskStatus::Aborted,
                    _ if ok => task_log::TaskStatus::Success,
                    _ => task_log::TaskStatus::Error,
                };
                let _ = task_log::finalize_session(&data_dir, &mut tlog, stt, msg.clone());
                hub.end_with_report(&acc_key2, msg.clone(), ok || status == "skip");
                let _ = tx_client.send(ProgressEvent::finished(
                    "module",
                    ok || status == "skip",
                    msg,
                ));
            }
            Err(e) => {
                let em = e.to_string();
                let _ = task_log::finalize_session(
                    &data_dir,
                    &mut tlog,
                    task_log::TaskStatus::Error,
                    em.clone(),
                );
                hub.end_with_report(&acc_key2, em.clone(), false);
                let _ = tx_client.send(ProgressEvent::finished("module", false, em));
            }
        }
    });

    let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx_out).map(|ev| {
        let line = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".into());
        Ok::<_, std::io::Error>(format!("{line}\n"))
    });
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "application/x-ndjson; charset=utf-8",
        )],
        axum::body::Body::from_stream(stream),
    )
        .into_response()
}

async fn api_task_logs(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
) -> impl IntoResponse {
    let (sess, _, _) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    match task_log::list_sessions(&st.data_dir, &sess.group, &alias, None) {
        Ok(list) => Json(json!({"ok": true, "sessions": list})).into_response(),
        Err(e) => err_resp(e),
    }
}

async fn api_task_log_one(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath((alias, id)): AxumPath<(String, String)>,
) -> impl IntoResponse {
    let (sess, _, _) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    match task_log::load_full_session(&st.data_dir, &sess.group, &alias, &id) {
        Ok(s) => Json(json!({"ok": true, "session": s})).into_response(),
        Err(e) => err_resp(e),
    }
}

/// 进行中进度快照（任务未定稿时用；对照 UI_ROUTING API 表）
async fn api_task_log_progress(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath((alias, id)): AxumPath<(String, String)>,
) -> impl IntoResponse {
    let (sess, _, _) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    match task_log::load_progress(&st.data_dir, &sess.group, &alias, &id) {
        Ok(p) => Json(json!({"ok": true, "progress": p})).into_response(),
        Err(e) => err_resp(e),
    }
}

/// 日志清理请求（按天；主人 2026-08-07 新规，不再默认按条数）
/// Docs: docs/tech/UI_ROUTING_AND_TASK_LOGS.md
#[derive(Deserialize)]
struct ClearLogs {
    /// 保留最近多少天（7 或 30）；删除更早的
    #[serde(default = "default_retain_days")]
    retain_days: u32,
    /// true=仅清理「一键清日常」类；false=本账号全部触发类型
    #[serde(default)]
    only_one_click: bool,
    /// 兼容旧客户端：若传 keep_latest 且无 retain_days 语义，仍可走条数清理
    #[serde(default)]
    keep_latest: Option<usize>,
}
fn default_retain_days() -> u32 {
    7
}

async fn api_task_logs_clear(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
    Json(body): Json<ClearLogs>,
) -> impl IntoResponse {
    let (sess, _, _) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    // 优先按天；仅当显式 keep_latest 且 retain_days 为 0 时回退条数（兼容）
    if body.retain_days > 0 {
        match task_log::clear_sessions_older_than(
            &st.data_dir,
            &sess.group,
            &alias,
            body.retain_days,
            body.only_one_click,
        ) {
            Ok(n) => Json(json!({
                "ok": true,
                "removed": n,
                "retain_days": body.retain_days,
                "only_one_click": body.only_one_click,
                "message": format!("已删除 {n} 条早于 {} 天前的任务日志", body.retain_days),
            }))
            .into_response(),
            Err(e) => err_resp(e),
        }
    } else if let Some(k) = body.keep_latest {
        match task_log::clear_sessions(
            &st.data_dir,
            &sess.group,
            &alias,
            body.only_one_click,
            Some(k),
        ) {
            Ok(n) => Json(json!({"ok": true, "removed": n, "keep_latest": k})).into_response(),
            Err(e) => err_resp(e),
        }
    } else {
        err_resp("请指定 retain_days（7 或 30）或 keep_latest".to_string())
    }
}

async fn api_wash_options(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
) -> impl IntoResponse {
    let (sess, acc, _) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    if let Err(e) = assert_tool_allowed() {
        return err_resp(e);
    }
    let ch = Channel::from_user(&acc.channel);
    let _guard = match st.task_gate.try_begin_owned(
        ch.as_str(),
        &acc.username,
        "wash_options",
        &sess.group,
    ) {
        Ok(g) => g,
        Err(e) => return err_resp(e),
    };
    match async {
        let fp = crate::fp_load::load_fp(&st.data_dir, &st.fp_url, ch.as_str()).await?;
        let pool = crate::session_pool::process_pool();
        let (skey, client, skind) = pool
            .acquire_full(ch.as_str(), &acc.username, &acc.password, fp, &st.data_dir)
            .await?;
        let v = json!({"ok": true, "styles": style_choices(&client)});
        pool.release(skey, skind, client, false).await;
        Ok::<_, anyhow::Error>(v)
    }
    .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_resp(e),
    }
}

async fn api_wash(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let (sess, acc, _) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    if let Err(e) = assert_tool_allowed() {
        return err_resp(e);
    }
    let ch = Channel::from_user(&acc.channel);
    let _guard = match st
        .task_gate
        .try_begin_owned(ch.as_str(), &acc.username, "wash", &sess.group)
    {
        Ok(g) => g,
        Err(e) => return err_resp(e),
    };
    let style_id = body.get("style_id").and_then(|v| v.as_i64()).unwrap_or(0);
    let selection_index = body
        .get("selection_index")
        .and_then(|v| v.as_i64())
        .unwrap_or(1);
    let repeat_times = body
        .get("repeat_times")
        .and_then(|v| v.as_i64())
        .unwrap_or(1);
    let target_ids: Vec<i64> = body
        .get("target_ids")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_i64()).collect())
        .unwrap_or_default();
    let or_logic = body
        .get("or_logic")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    match async {
        let fp = crate::fp_load::load_fp(&st.data_dir, &st.fp_url, ch.as_str()).await?;
        let pool = crate::session_pool::process_pool();
        let (skey, mut client, skind) = pool
            .acquire_full(ch.as_str(), &acc.username, &acc.password, fp, &st.data_dir)
            .await?;
        let (tx, _rx) = mpsc::unbounded_channel();
        let wash_res = run_super_wash_with_progress(
            &mut client,
            style_id,
            selection_index,
            repeat_times,
            &target_ids,
            or_logic,
            &Some(tx),
        )
        .await;
        let drop = wash_res
            .as_ref()
            .err()
            .map(crate::session_pool::should_drop_core)
            .unwrap_or(false);
        pool.release(skey, skind, client, drop).await;
        let report = wash_res?;
        Ok::<_, anyhow::Error>(json!({"ok": true, "report": report}))
    }
    .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_resp(e),
    }
}

async fn api_export_settings(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
) -> impl IntoResponse {
    let (_, acc, _) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    Json(json!({
        "ok": true,
        "text": serde_json::to_string_pretty(&acc.config).unwrap_or_default(),
        "config": acc.config,
    }))
    .into_response()
}

#[derive(Deserialize)]
struct ImportBody {
    text: String,
}

async fn api_import_settings(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
    Json(body): Json<ImportBody>,
) -> impl IntoResponse {
    let (sess, _, store) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    let parsed: HashMap<String, Value> = match serde_json::from_str(&body.text) {
        Ok(v) => v,
        Err(e) => return err_resp(e),
    };
    let mut g = match store.load_group(&sess.group, sess.group_password.as_deref()) {
        Ok(g) => g,
        Err(e) => return err_resp(e),
    };
    if let Some(a) = g.accounts.iter_mut().find(|a| a.alias == alias) {
        a.config = parsed;
        let _ = crate::settings_files::mirror_account_settings(
            &st.data_dir,
            &sess.group,
            &alias,
            &a.config,
        );
    }
    if let Err(e) = store.save_group(&g) {
        return err_resp(e);
    }
    Json(json!({"ok": true})).into_response()
}

#[derive(Deserialize)]
struct CopyBody {
    to: String,
}

async fn api_copy_config(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
    Json(body): Json<CopyBody>,
) -> impl IntoResponse {
    let (sess, _, store) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    let mut g = match store.load_group(&sess.group, sess.group_password.as_deref()) {
        Ok(g) => g,
        Err(e) => return err_resp(e),
    };
    let cfg = g
        .accounts
        .iter()
        .find(|a| a.alias == alias)
        .map(|a| a.config.clone());
    let Some(cfg) = cfg else {
        return err_resp("from not found");
    };
    if let Some(a) = g.accounts.iter_mut().find(|a| a.alias == body.to) {
        a.config = cfg;
        let _ = crate::settings_files::mirror_account_settings(
            &st.data_dir,
            &sess.group,
            &body.to,
            &a.config,
        );
    } else {
        return err_resp("to not found");
    }
    if let Err(e) = store.save_group(&g) {
        return err_resp(e);
    }
    Json(json!({"ok": true})).into_response()
}

async fn api_shop_pack_get(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
    Query(q): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let (_, acc, _) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    let scope = q.get("scope").map(|s| s.as_str()).unwrap_or("SHOP3");
    match config_pack::encode_shop(&acc.config, scope) {
        Ok(code) => Json(json!({"ok": true, "code": code})).into_response(),
        Err(e) => err_resp(e),
    }
}

#[derive(Deserialize)]
struct PackBody {
    code: String,
    #[serde(default)]
    kind: String,
}

async fn api_shop_pack_post(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
    Json(body): Json<PackBody>,
) -> impl IntoResponse {
    let (sess, _, store) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    let (kind, patch) = match config_pack::decode_any(&body.code) {
        Ok(p) => p,
        Err(e) => return err_resp(e),
    };
    let use_kind = if body.kind.is_empty() {
        kind
    } else {
        body.kind
    };
    let mut g = match store.load_group(&sess.group, sess.group_password.as_deref()) {
        Ok(g) => g,
        Err(e) => return err_resp(e),
    };
    if let Some(a) = g.accounts.iter_mut().find(|a| a.alias == alias) {
        if let Err(e) = config_pack::apply_shop_patch(&mut a.config, &use_kind, &patch) {
            return err_resp(e);
        }
        let _ = crate::settings_files::mirror_account_settings(
            &st.data_dir,
            &sess.group,
            &alias,
            &a.config,
        );
    }
    if let Err(e) = store.save_group(&g) {
        return err_resp(e);
    }
    Json(json!({"ok": true})).into_response()
}

async fn api_config_pack_get(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
) -> impl IntoResponse {
    let (_, acc, _) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    match config_pack::encode_config(&acc.config) {
        Ok(code) => Json(json!({"ok": true, "code": code})).into_response(),
        Err(e) => err_resp(e),
    }
}

async fn api_config_pack_post(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
    Json(body): Json<PackBody>,
) -> impl IntoResponse {
    let (sess, _, store) = match load_acc(&st, &headers, &alias).await {
        Ok(x) => x,
        Err(r) => return r,
    };
    let (_kind, patch) = match config_pack::decode_any(&body.code) {
        Ok(p) => p,
        Err(e) => return err_resp(e),
    };
    let mut g = match store.load_group(&sess.group, sess.group_password.as_deref()) {
        Ok(g) => g,
        Err(e) => return err_resp(e),
    };
    if let Some(a) = g.accounts.iter_mut().find(|a| a.alias == alias) {
        config_pack::apply_config_patch(&mut a.config, patch);
        let _ = crate::settings_files::mirror_account_settings(
            &st.data_dir,
            &sess.group,
            &alias,
            &a.config,
        );
    }
    if let Err(e) = store.save_group(&g) {
        return err_resp(e);
    }
    Json(json!({"ok": true})).into_response()
}

async fn api_config_file_get(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
) -> impl IntoResponse {
    api_export_settings(State(st), headers, AxumPath(alias)).await
}

async fn api_config_file_post(
    State(st): State<AppState>,
    headers: HeaderMap,
    AxumPath(alias): AxumPath<String>,
    Json(body): Json<ImportBody>,
) -> impl IntoResponse {
    api_import_settings(State(st), headers, AxumPath(alias), Json(body)).await
}

#[derive(Deserialize)]
struct RunQ {
    alias: Option<String>,
    /// 主页精简监视：只返回该用户组发起的运行（跨组仅出现在卡片 run_label，不进本组流）
    /// Docs: docs/tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md §3.1·§4.1
    group: Option<String>,
}

async fn api_run_status(State(st): State<AppState>, Query(q): Query<RunQ>) -> Json<Value> {
    // 带 group 时：runs 过滤本组 + stream_lines 仅本组完整监视流（主页面板用）
    let mut b = st
        .run_hub
        .bundle_with_stream(q.alias.as_deref(), None, q.group.as_deref());
    let runs_all = if q.group.is_some() {
        st.run_hub.bundle(None).runs
    } else {
        b.runs.clone()
    };
    if let Some(ref g) = q.group {
        b.runs
            .retain(|r| r.group.as_deref() == Some(g.as_str()));
        b.busy_any = b.runs.iter().any(|r| r.busy);
        b.run = b
            .runs
            .iter()
            .find(|r| r.busy)
            .cloned()
            .unwrap_or_default();
    }
    Json(json!({
        "ok": true,
        "busy_any": b.busy_any,
        "run": b.run,
        "runs": b.runs,
        "runs_all": runs_all,
        // 完整行缓冲（类似程序运行面板终端；非进度条）
        "stream_lines": b.stream_lines,
        "stream_count": b.stream_lines.len(),
    }))
}

#[derive(Deserialize, Default)]
struct RunCtrlBody {
    /// 请求停止的用户组（须等于发起用户组 owner_group）
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    alias: Option<String>,
    #[serde(default)]
    channel: Option<String>,
    /// 仅当已知 migration 时服务端用 gate 校验；无则仅用 run_hub 的 group 字段
    #[serde(default)]
    migration: Option<String>,
}

/// 暂停/继续/放弃：**必须**带用户组；仅任务发起用户组可控制。
/// Docs: docs/tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md · INSTANCE_AND_CLI · P16
fn run_ctrl_auth(st: &AppState, body: &RunCtrlBody) -> Result<(), String> {
    let req_group = body
        .group
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "缺少用户组参数：仅「发起该任务的用户组」可暂停/继续/放弃。请从正确用户组页面操作。"
                .to_string()
        })?;
    if let (Some(ch), Some(mig)) = (body.channel.as_deref(), body.migration.as_deref()) {
        return st.task_gate.may_stop(req_group, ch, mig);
    }
    let b = st.run_hub.bundle(body.alias.as_deref());
    let busy = b
        .runs
        .iter()
        .find(|r| r.busy)
        .or(if b.run.busy { Some(&b.run) } else { None });
    let Some(snap) = busy else {
        return Err("当前没有进行中的任务".into());
    };
    match snap.group.as_deref() {
        Some(og) if og == req_group => Ok(()),
        Some(og) => Err(format!(
            "仅发起用户组「{og}」可暂停/继续/放弃该任务；当前请求组为「{req_group}」。其它用户组只能查看进度。"
        )),
        None => Err("任务未记录发起用户组，拒绝控制（请升级程序后重试）".into()),
    }
}

async fn api_run_pause(
    State(st): State<AppState>,
    body: Option<Json<RunCtrlBody>>,
) -> impl IntoResponse {
    let body = body.map(|j| j.0).unwrap_or_default();
    if let Err(e) = run_ctrl_auth(&st, &body) {
        return err_resp(e);
    }
    match st.run_hub.pause() {
        Ok(()) => Json(json!({"ok": true, "message": "已请求暂停"})).into_response(),
        Err(e) => err_resp(e),
    }
}
async fn api_run_resume(
    State(st): State<AppState>,
    body: Option<Json<RunCtrlBody>>,
) -> impl IntoResponse {
    let body = body.map(|j| j.0).unwrap_or_default();
    if let Err(e) = run_ctrl_auth(&st, &body) {
        return err_resp(e);
    }
    match st.run_hub.resume() {
        Ok(()) => Json(json!({"ok": true, "message": "已继续"})).into_response(),
        Err(e) => err_resp(e),
    }
}
async fn api_run_abort(
    State(st): State<AppState>,
    body: Option<Json<RunCtrlBody>>,
) -> impl IntoResponse {
    let body = body.map(|j| j.0).unwrap_or_default();
    if let Err(e) = run_ctrl_auth(&st, &body) {
        return err_resp(e);
    }
    match st.run_hub.abort() {
        Ok(()) => Json(json!({"ok": true, "message": "已请求放弃"})).into_response(),
        Err(e) => err_resp(e),
    }
}
async fn api_run_clear(State(st): State<AppState>) -> Json<Value> {
    st.run_hub.clear_report();
    st.run_hub.clear_stream();
    Json(json!({"ok": true, "message": "已清空运行摘要与监视流显示"}))
}

// --- end run control ---

#[derive(Deserialize)]
struct GroupRaidBody {
    /// 优先：按配置卡 id 启动
    #[serde(default)]
    config_id: String,
    /// 逗号分隔别名；也可 aliases_list
    #[serde(default)]
    aliases: String,
    #[serde(default)]
    aliases_list: Vec<String>,
    #[serde(default)]
    room_open: String,
    #[serde(default)]
    party: String,
    #[serde(default)]
    leave_after_support: bool,
}

/// 组队团战 HTTP 入口（1+ 卡；删卡降级；可 config_id）
/// Docs: docs/tech/GROUP_RAID_AND_DEVICE_IDENTITY.md §8 · PLAN_GROUP_RAID_UI
async fn api_group_raid(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<GroupRaidBody>,
) -> impl IntoResponse {
    let sess = match require_session(session_of(&st, &headers).await) {
        Ok(s) => s,
        Err(r) => return r,
    };
    if let Err(e) = assert_daily_allowed() {
        return err_resp(e);
    }
    if !body.config_id.trim().is_empty() {
        return match run_ops::exec_group_raid_by_config_id(
            &st.data_dir,
            &st.fp_url,
            &st.task_gate,
            &sess.group,
            sess.group_password.as_deref(),
            body.config_id.trim(),
        )
        .await
        {
            Ok(v) => Json(v).into_response(),
            Err(e) => err_resp(e),
        };
    }
    let aliases = if !body.aliases_list.is_empty() {
        body.aliases_list.join(",")
    } else {
        body.aliases.clone()
    };
    match run_ops::exec_group_raid(
        &st.data_dir,
        &st.fp_url,
        &st.task_gate,
        &sess.group,
        sess.group_password.as_deref(),
        &aliases,
        &body.room_open,
        &body.party,
        body.leave_after_support,
    )
    .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_resp(e),
    }
}

async fn api_group_raid_config_get(
    State(st): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let sess = match require_session(session_of(&st, &headers).await) {
        Ok(s) => s,
        Err(r) => return r,
    };
    match run_ops::load_group_raid_panel(
        &st.data_dir,
        &sess.group,
        sess.group_password.as_deref(),
    ) {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_resp(e),
    }
}

#[derive(Deserialize)]
struct GroupRaidConfigBody {
    #[serde(default)]
    entries: Option<Vec<GroupRaidEntryBody>>,
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    room_open: String,
    #[serde(default)]
    party: String,
    #[serde(default)]
    leave_after_support: bool,
}

#[derive(Deserialize, Clone)]
struct GroupRaidEntryBody {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    room_open: String,
    #[serde(default)]
    party: String,
    #[serde(default)]
    leave_after_support: bool,
}

async fn api_group_raid_config_put(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<GroupRaidConfigBody>,
) -> impl IntoResponse {
    let sess = match require_session(session_of(&st, &headers).await) {
        Ok(s) => s,
        Err(r) => return r,
    };
    let entries: Vec<rustmadoka_core::GroupRaidConfigEntry> = if let Some(list) = body.entries {
        list.into_iter()
            .map(|e| rustmadoka_core::GroupRaidConfigEntry {
                id: e.id,
                name: e.name,
                aliases: e.aliases,
                room_open: e.room_open,
                party: e.party,
                leave_after_support: e.leave_after_support,
            })
            .collect()
    } else {
        vec![rustmadoka_core::GroupRaidConfigEntry {
            id: if body.id.is_empty() {
                "legacy".into()
            } else {
                body.id
            },
            name: if body.name.is_empty() {
                "默认组队".into()
            } else {
                body.name
            },
            aliases: body.aliases,
            room_open: body.room_open,
            party: body.party,
            leave_after_support: body.leave_after_support,
        }]
    };
    match run_ops::save_group_raid_panel(
        &st.data_dir,
        &sess.group,
        sess.group_password.as_deref(),
        rustmadoka_core::GroupRaidPanelConfig { entries },
    ) {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_resp(e),
    }
}

async fn api_group_raid_entry_upsert(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<GroupRaidEntryBody>,
) -> impl IntoResponse {
    let sess = match require_session(session_of(&st, &headers).await) {
        Ok(s) => s,
        Err(r) => return r,
    };
    match run_ops::upsert_group_raid_entry(
        &st.data_dir,
        &sess.group,
        sess.group_password.as_deref(),
        rustmadoka_core::GroupRaidConfigEntry {
            id: body.id,
            name: body.name,
            aliases: body.aliases,
            room_open: body.room_open,
            party: body.party,
            leave_after_support: body.leave_after_support,
        },
    ) {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_resp(e),
    }
}

#[derive(Deserialize)]
struct GroupRaidEntryDeleteBody {
    id: String,
}

async fn api_group_raid_entry_delete(
    State(st): State<AppState>,
    headers: HeaderMap,
    uri: axum::http::Uri,
    body: Option<Json<GroupRaidEntryDeleteBody>>,
) -> impl IntoResponse {
    let sess = match require_session(session_of(&st, &headers).await) {
        Ok(s) => s,
        Err(r) => return r,
    };
    // 优先 JSON body；部分环境 DELETE 无 body 时用 ?id=
    let mut id = body.map(|j| j.0.id).unwrap_or_default();
    if id.trim().is_empty() {
        if let Some(q) = uri.query() {
            for pair in q.split('&') {
                if let Some(v) = pair.strip_prefix("id=") {
                    id = urlencoding_decode(v);
                    break;
                }
            }
        }
    }
    match run_ops::delete_group_raid_entry(
        &st.data_dir,
        &sess.group,
        sess.group_password.as_deref(),
        &id,
    ) {
        Ok(v) => Json(v).into_response(),
        Err(e) => err_resp(e),
    }
}

fn urlencoding_decode(s: &str) -> String {
    // 轻量：别名 id 通常无复杂编码；%XX 简单处理
    let mut out = String::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = &s[i + 1..i + 3];
            if let Ok(v) = u8::from_str_radix(hex, 16) {
                out.push(v as char);
                i += 3;
                continue;
            }
        }
        if b[i] == b'+' {
            out.push(' ');
        } else {
            out.push(b[i] as char);
        }
        i += 1;
    }
    out
}

async fn api_version(State(st): State<AppState>) -> Json<Value> {
    let mut settings = AppSettings::load(&st.data_dir);
    prepare_sources(&mut settings);
    Json(json!({
        "ok": true,
        "product": "RustMadoka",
        "edition": PRODUCT_EDITION,
        "cargo_version": APP_VERSION,
        "build_stamp": BUILD_STAMP,
        "listen_port": st.listen_port,
        "default_port": DEFAULT_LISTEN_PORT,
        "info_sources": settings.info_sources,
        "manual_version_note": settings.manual_version_note,
        "last_remote_info": settings.last_remote_info,
        "last_version_check": settings.last_version_check,
    }))
}

async fn api_version_check(State(st): State<AppState>) -> Json<Value> {
    let mut settings = AppSettings::load(&st.data_dir);
    prepare_sources(&mut settings);
    let _ = fp_slots::refresh_default_source(&st.data_dir, false).await;
    settings.last_version_check_day = Some(crate::app_settings::today_day_plus08());
    settings.last_version_check = Some(json!({"ok": true, "at": chrono::Utc::now().to_rfc3339()}));
    let _ = settings.save(&st.data_dir);
    Json(json!({"ok": true, "check": settings.last_version_check}))
}

async fn api_version_fetch(State(st): State<AppState>) -> impl IntoResponse {
    match rustmadoka_core::fetch_fingerprint(&st.fp_url, &st.default_channel).await {
        Ok(fp) => {
            let _ = fp.save_version_json(&st.data_dir.join("cache/version.json"));
            let mut settings = AppSettings::load(&st.data_dir);
            settings.last_remote_fetched_at = Some(chrono::Utc::now().to_rfc3339());
            settings.last_remote_info =
                Some(json!({"version": fp.version, "channel": fp.channel}));
            let _ = settings.save(&st.data_dir);
            Json(json!({"ok": true, "version": fp.version})).into_response()
        }
        Err(e) => err_resp(e),
    }
}

#[derive(Deserialize)]
struct PasteBody {
    text: String,
}

async fn api_version_paste(
    State(st): State<AppState>,
    Json(body): Json<PasteBody>,
) -> impl IntoResponse {
    let mut settings = AppSettings::load(&st.data_dir);
    settings.manual_version_note = body.text;
    let _ = settings.save(&st.data_dir);
    Json(json!({"ok": true})).into_response()
}

#[derive(Deserialize)]
struct SourcesBody {
    sources: Vec<crate::app_settings::InfoSource>,
}

async fn api_version_sources_save(
    State(st): State<AppState>,
    Json(body): Json<SourcesBody>,
) -> impl IntoResponse {
    let mut settings = AppSettings::load(&st.data_dir);
    settings.info_sources = body.sources;
    prepare_sources(&mut settings);
    let _ = settings.save(&st.data_dir);
    Json(json!({"ok": true})).into_response()
}

#[derive(Deserialize)]
struct SourceAdd {
    name: String,
    url: String,
    #[serde(default = "default_fp_kind")]
    kind: String,
}
fn default_fp_kind() -> String {
    "fingerprint".into()
}

async fn api_version_sources_add(
    State(st): State<AppState>,
    Json(body): Json<SourceAdd>,
) -> impl IntoResponse {
    let mut settings = AppSettings::load(&st.data_dir);
    settings.info_sources.push(crate::app_settings::InfoSource {
        id: uuid::Uuid::new_v4().to_string(),
        name: body.name,
        url: body.url,
        kind: body.kind,
        builtin: false,
        enabled: true,
        last_test_ok: None,
        last_test_at: None,
        last_test_message: None,
    });
    let _ = settings.save(&st.data_dir);
    Json(json!({"ok": true})).into_response()
}

#[derive(Deserialize)]
struct IdBody {
    id: String,
}

async fn api_version_sources_delete(
    State(st): State<AppState>,
    Json(body): Json<IdBody>,
) -> impl IntoResponse {
    let mut settings = AppSettings::load(&st.data_dir);
    settings
        .info_sources
        .retain(|s| s.id != body.id || s.builtin);
    let _ = settings.save(&st.data_dir);
    Json(json!({"ok": true})).into_response()
}

async fn api_version_sources_test(
    State(st): State<AppState>,
    Json(body): Json<IdBody>,
) -> impl IntoResponse {
    let mut settings = AppSettings::load(&st.data_dir);
    let url = settings
        .info_sources
        .iter()
        .find(|s| s.id == body.id)
        .map(|s| s.url.clone());
    let Some(url) = url else {
        return err_resp("source not found");
    };
    let ok = rustmadoka_core::fetch_fingerprint(&url, &st.default_channel)
        .await
        .is_ok();
    if let Some(s) = settings.info_sources.iter_mut().find(|s| s.id == body.id) {
        s.last_test_ok = Some(ok);
        s.last_test_at = Some(chrono::Utc::now().to_rfc3339());
        s.last_test_message = Some(if ok { "ok".into() } else { "fail".into() });
    }
    let _ = settings.save(&st.data_dir);
    Json(json!({"ok": true, "test_ok": ok})).into_response()
}

async fn api_fp_slots(State(st): State<AppState>) -> impl IntoResponse {
    let s = fp_slots::FpSlotStore::load(&st.data_dir);
    Json(json!({"ok": true, "store": s.to_public_json()})).into_response()
}

async fn api_fp_refresh(State(st): State<AppState>) -> impl IntoResponse {
    match fp_slots::refresh_default_source(&st.data_dir, true).await {
        Ok(v) => {
            // Network failure returns ok:false with store still useful for UI.
            let status = if v.get("ok").and_then(|x| x.as_bool()) == Some(false) {
                StatusCode::OK
            } else {
                StatusCode::OK
            };
            (status, Json(v)).into_response()
        }
        Err(e) => err_resp(e),
    }
}

async fn api_fp_activate(State(st): State<AppState>, Json(body): Json<IdBody>) -> impl IntoResponse {
    let mut s = fp_slots::FpSlotStore::load(&st.data_dir);
    if let Err(e) = s.activate(&body.id) {
        return err_resp(e);
    }
    if let Err(e) = s.save(&st.data_dir) {
        return err_resp(e);
    }
    Json(json!({"ok": true, "store": s.to_public_json()})).into_response()
}

async fn api_fp_clear(State(st): State<AppState>, Json(body): Json<IdBody>) -> impl IntoResponse {
    let mut s = fp_slots::FpSlotStore::load(&st.data_dir);
    if let Err(e) = s.clear_custom(&body.id) {
        return err_resp(e);
    }
    if let Err(e) = s.save(&st.data_dir) {
        return err_resp(e);
    }
    Json(json!({"ok": true, "store": s.to_public_json()})).into_response()
}

#[derive(Deserialize)]
struct FillBody {
    id: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    url: String,
}

async fn api_fp_fill(State(st): State<AppState>, Json(body): Json<FillBody>) -> impl IntoResponse {
    let text = if !body.text.trim().is_empty() {
        body.text.clone()
    } else if !body.url.trim().is_empty() {
        match rustmadoka_core::fingerprint::fetch_fingerprint_text(body.url.trim()).await {
            Ok(t) => t,
            Err(e) => return err_resp(e),
        }
    } else {
        return err_resp(anyhow::anyhow!("请提供粘贴 JSON 或 URL"));
    };
    let mut s = fp_slots::FpSlotStore::load(&st.data_dir);
    if let Err(e) = s.fill_custom(&body.id, &text, "") {
        return err_resp(e);
    }
    if let Err(e) = s.save(&st.data_dir) {
        return err_resp(e);
    }
    Json(json!({
        "ok": true,
        "message": "已写入自定义槽，请手动点「启用」",
        "store": s.to_public_json(),
    }))
    .into_response()
}

async fn api_fp_reset(State(st): State<AppState>) -> impl IntoResponse {
    let mut s = fp_slots::FpSlotStore::load(&st.data_dir);
    if let Err(e) = s.reset_to_default_embedded() {
        return err_resp(e);
    }
    if let Err(e) = s.save(&st.data_dir) {
        return err_resp(e);
    }
    Json(json!({"ok": true, "store": s.to_public_json()})).into_response()
}

#[derive(Deserialize)]
struct DiagBody {
    #[serde(default)]
    alias: String,
    /// true = try login path (needs alias); false = environment only
    #[serde(default)]
    with_login: Option<bool>,
}

async fn api_diagnose(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DiagBody>,
) -> impl IntoResponse {
    let mut steps: Vec<Value> = Vec::new();
    let mut report_lines: Vec<String> = Vec::new();

    // 1) data dir
    let data_ok = st.data_dir.is_dir()
        || std::fs::create_dir_all(&st.data_dir).is_ok();
    steps.push(json!({
        "title": "数据文件夹",
        "ok": data_ok,
        "detail": st.data_dir.display().to_string(),
    }));
    report_lines.push(format!(
        "[{}] 数据文件夹 {}",
        if data_ok { "OK" } else { "FAIL" },
        st.data_dir.display()
    ));

    // 2) gates
    let daily_ok = daily_allowed();
    let tool_ok = tool_allowed();
    steps.push(json!({
        "title": "清日常门禁",
        "ok": daily_ok,
        "detail": if daily_ok { "ALLOW_DAILY_RUN=true" } else { "已关闭" },
    }));
    steps.push(json!({
        "title": "工具门禁",
        "ok": tool_ok,
        "detail": if tool_ok { "ALLOW_TOOL_RUN=true" } else { "已关闭" },
    }));
    report_lines.push(format!(
        "[{}] 清日常门禁 daily={}",
        if daily_ok { "OK" } else { "FAIL" },
        daily_ok
    ));
    report_lines.push(format!(
        "[{}] 工具门禁 tool={}",
        if tool_ok { "OK" } else { "FAIL" },
        tool_ok
    ));

    // 3) fingerprint slots + versions
    let store = fp_slots::FpSlotStore::load(&st.data_dir);
    let pub_store = store.to_public_json();
    let jp = pub_store
        .get("jp_version")
        .and_then(|v| v.as_str())
        .unwrap_or("—");
    let en = pub_store
        .get("en_version")
        .and_then(|v| v.as_str())
        .unwrap_or("—");
    let active = pub_store
        .get("active_label")
        .and_then(|v| v.as_str())
        .unwrap_or("—");
    let fp_ok = store.active_combined_text().is_ok();
    steps.push(json!({
        "title": "指纹槽（当前启用）",
        "ok": fp_ok,
        "detail": format!("启用={active} · 日服 {jp} · 国际服 {en}"),
    }));
    report_lines.push(format!(
        "[{}] 指纹 启用={} JP={} EN={}",
        if fp_ok { "OK" } else { "FAIL" },
        active,
        jp,
        en
    ));

    // 4) port / edition
    steps.push(json!({
        "title": "本机服务",
        "ok": true,
        "detail": format!(
            "端口 {} · 版本 {} · 构建 {} · 包 {}",
            st.listen_port, APP_VERSION, BUILD_STAMP, PRODUCT_EDITION
        ),
    }));
    report_lines.push(format!(
        "[OK] 服务 port={} edition={} stamp={}",
        st.listen_port, PRODUCT_EDITION, BUILD_STAMP
    ));

    // 5) optional account + login smoke
    let want_login = body.with_login.unwrap_or(!body.alias.is_empty());
    let mut summary_parts = vec![format!(
        "环境：数据夹{} · 指纹{} · 日服{} · 国际服{}",
        if data_ok { "可写" } else { "异常" },
        if fp_ok { "可用" } else { "不可用" },
        jp,
        en
    )];

    if want_login && !body.alias.is_empty() {
        match load_acc(&st, &headers, &body.alias).await {
            Ok((sess, acc, _)) => {
                let ch = Channel::from_user(&acc.channel);
                let busy = st.task_gate.is_busy(ch.as_str(), &acc.username);
                let busy_txt = match &busy {
                    Some((task, og)) => format!("是 task={task} owner_group={og}"),
                    None => "否".into(),
                };
                steps.push(json!({
                    "title": "账号卡片",
                    "ok": true,
                    "detail": format!(
                        "组={} 别名={} 服={} 占用={}",
                        sess.group, acc.alias, acc.channel, busy_txt
                    ),
                }));
                report_lines.push(format!(
                    "[OK] 账号 group={} alias={} channel={} busy={}",
                    sess.group, acc.alias, acc.channel, busy_txt
                ));

                // Soft login check: load fp only (full game login is longer; still valuable)
                match crate::fp_load::load_fp(&st.data_dir, &st.fp_url, ch.as_str()).await {
                    Ok(fp) => {
                        steps.push(json!({
                            "title": "账号渠道指纹加载",
                            "ok": true,
                            "detail": format!("channel={} version={}", ch.as_str(), fp.version),
                        }));
                        report_lines.push(format!(
                            "[OK] 渠道指纹 version={} sign_len={}",
                            fp.version,
                            fp.sign.len()
                        ));
                        // Optional light info login
                        match run_ops::fetch_account_info(&st.data_dir, &st.fp_url, &acc).await {
                            Ok(info) => {
                                steps.push(json!({
                                    "title": "试登录（获取角色信息）",
                                    "ok": true,
                                    "detail": info,
                                }));
                                report_lines.push(format!(
                                    "[OK] 试登录 {}",
                                    serde_json::to_string(&info).unwrap_or_default()
                                ));
                                summary_parts.push("试登录成功".into());
                            }
                            Err(e) => {
                                let es = e.to_string();
                                let pwd = es.contains("密码")
                                    || es.to_lowercase().contains("password")
                                    || es.contains("401")
                                    || es.contains("Unauthorized");
                                steps.push(json!({
                                    "title": if pwd { "试登录（密码/鉴权）" } else { "试登录（获取角色信息）" },
                                    "ok": false,
                                    "detail": es,
                                }));
                                report_lines.push(format!("[FAIL] 试登录 {es}"));
                                summary_parts.push(if pwd {
                                    "试登录失败：请检查游戏密码/引继".into()
                                } else {
                                    format!("试登录失败：{es}")
                                });
                            }
                        }
                    }
                    Err(e) => {
                        steps.push(json!({
                            "title": "账号渠道指纹加载",
                            "ok": false,
                            "detail": e.to_string(),
                        }));
                        report_lines.push(format!("[FAIL] 渠道指纹 {e}"));
                        summary_parts.push(format!("指纹加载失败：{e}"));
                    }
                }
            }
            Err(r) => return r,
        }
    } else if want_login {
        steps.push(json!({
            "title": "试登录",
            "ok": false,
            "detail": "未选择账号卡片",
        }));
        report_lines.push("[FAIL] 试登录：未选择账号".into());
        summary_parts.push("未选择账号，跳过试登录".into());
    } else {
        steps.push(json!({
            "title": "试登录",
            "ok": true,
            "detail": "本轮仅环境检测，未试登录",
        }));
        report_lines.push("[OK] 仅环境，未试登录".into());
    }

    let fail_n = steps.iter().filter(|s| s.get("ok") == Some(&json!(false))).count();
    let summary_zh = if fail_n == 0 {
        format!("环境检测通过。{}", summary_parts.join(" · "))
    } else {
        format!("发现 {fail_n} 项异常。{}", summary_parts.join(" · "))
    };
    let report_text = format!(
        "RustMadoka 环境检测报告\nbuild={BUILD_STAMP} edition={PRODUCT_EDITION}\n{}\n\n{}",
        chrono::Utc::now().to_rfc3339(),
        report_lines.join("\n")
    );

    Json(json!({
        "ok": fail_n == 0,
        "steps": steps,
        "summary_zh": summary_zh,
        "report_text": report_text,
        "edition": PRODUCT_EDITION,
        "data_dir": st.data_dir.display().to_string(),
        "listen_port": st.listen_port,
        "fp_store": pub_store,
    }))
    .into_response()
}

async fn api_settings_notifications(
    State(st): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let sess = match require_session(session_of(&st, &headers).await) {
        Ok(s) => s,
        Err(r) => return r,
    };
    let alias = q.get("alias").cloned().unwrap_or_default();
    let limit = q
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(100);
    match rustmadoka_core::FeatureNotifyFile::load(
        &st.data_dir,
        rustmadoka_core::FEATURE_SETTINGS,
    ) {
        Ok(file) => {
            let entries = file.query_filtered(
                None,
                if alias.is_empty() {
                    None
                } else {
                    Some(alias.as_str())
                },
                Some(sess.group.as_str()),
                Some(limit),
            );
            Json(json!({
                "ok": true,
                "feature": rustmadoka_core::FEATURE_SETTINGS,
                "max_keep": file.max_keep,
                "entries": entries,
            }))
            .into_response()
        }
        Err(e) => err_resp(e.to_string()),
    }
}

/// Windows 系统 toast 配置（默认 enabled=false）
async fn api_system_toast_get(State(st): State<AppState>) -> impl IntoResponse {
    let s = crate::system_toast::SystemToastSettings::load(&st.data_dir);
    Json(json!({"ok": true, "settings": s})).into_response()
}

#[derive(Deserialize)]
struct SystemToastBody {
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    on_task_success: Option<bool>,
    #[serde(default)]
    on_task_error: Option<bool>,
}

async fn api_system_toast_set(
    State(st): State<AppState>,
    Json(body): Json<SystemToastBody>,
) -> impl IntoResponse {
    let mut s = crate::system_toast::SystemToastSettings::load(&st.data_dir);
    if let Some(v) = body.enabled {
        s.enabled = v;
    }
    if let Some(v) = body.on_task_success {
        s.on_task_success = v;
    }
    if let Some(v) = body.on_task_error {
        s.on_task_error = v;
    }
    match s.save(&st.data_dir) {
        Ok(()) => Json(json!({"ok": true, "settings": s})).into_response(),
        Err(e) => err_resp(e.to_string()),
    }
}
