//! 进程内游戏会话池：同一游戏账号卡片在本进程存活期间复用 `GameClient`。
//!
//! # 产品意图（主人 2026-08-07）
//! 程序本体一次启动后，同一游戏账号**全程保持登录**，避免每次 info/日常/模块都冷启动 LoginApi。
//! 关进程 / 空闲超时 / 401 → 下次操作再登录（像真客户端休息后重登）。
//!
//! # 键
//! `TaskGate::game_id_hash(channel, migration_code)` — 游戏身份（渠道+引继），与跨组占用一致。
//!
//! # 用法
//! ```ignore
//! let (key, mut client, kind) = process_pool()
//!     .acquire_full(channel, code, pw, fp, data_dir).await?;
//! let r = run_daily(&mut client, ...).await;
//! process_pool().release(key, kind, client, is_session_err(&r));
//! ```
//!
//! # 文档
//! - `docs/tech/CLIENT_SESSION_SIMULATION_FEASIBILITY.md`
//! - `docs/tech/SDK_AND_LOGIN.md` · `docs/tech/INSTANCE_AND_CLI.md`
//! - Outbound: 本文件 · `run_ops.rs`

use anyhow::Result;
use rustmadoka_core::client::GameClient;
use rustmadoka_core::fingerprint::Fingerprint;
use rustmadoka_core::CoreError;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::task_gate::TaskGate;

/// 空闲超过此时长未使用则丢弃（默认 75 分钟）。
const IDLE_TTL: Duration = Duration::from_secs(75 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    /// `login_for_info`
    Light,
    /// `GameClient::login` 全量
    Full,
}

struct Pooled {
    client: GameClient,
    kind: SessionKind,
    last_used: Instant,
}

/// 进程内会话池。
#[derive(Clone, Default)]
pub struct SessionPool {
    inner: Arc<Mutex<HashMap<String, Pooled>>>,
}

impl SessionPool {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn account_key(channel: &str, migration_code: &str) -> String {
        TaskGate::game_id_hash(channel, migration_code)
    }

    pub async fn invalidate(&self, channel: &str, migration_code: &str) {
        let k = Self::account_key(channel, migration_code);
        let mut g = self.inner.lock().await;
        if g.remove(&k).is_some() {
            tracing::info!(%k, channel, "session_pool: invalidated");
        }
    }

    pub async fn clear_all(&self) {
        let mut g = self.inner.lock().await;
        let n = g.len();
        g.clear();
        if n > 0 {
            tracing::info!(count = n, "session_pool: cleared all");
        }
    }

    fn is_fresh(p: &Pooled) -> bool {
        p.last_used.elapsed() < IDLE_TTL
    }

    fn kind_ok(have: SessionKind, need: SessionKind) -> bool {
        matches!(
            (have, need),
            (SessionKind::Full, _) | (SessionKind::Light, SessionKind::Light)
        )
    }

    /// 取得 **Light** 客户端（可能复用 Full）。调用方用毕后必须 `release`。
    pub async fn acquire_light(
        &self,
        channel: &str,
        migration_code: &str,
        password: &str,
        fp: Fingerprint,
        data_dir: &Path,
    ) -> Result<(String, GameClient, SessionKind)> {
        self.acquire(
            SessionKind::Light,
            channel,
            migration_code,
            password,
            fp,
            data_dir,
        )
        .await
    }

    /// 取得 **Full** 客户端。调用方用毕后必须 `release`。
    pub async fn acquire_full(
        &self,
        channel: &str,
        migration_code: &str,
        password: &str,
        fp: Fingerprint,
        data_dir: &Path,
    ) -> Result<(String, GameClient, SessionKind)> {
        self.acquire(
            SessionKind::Full,
            channel,
            migration_code,
            password,
            fp,
            data_dir,
        )
        .await
    }

    async fn acquire(
        &self,
        need: SessionKind,
        channel: &str,
        migration_code: &str,
        password: &str,
        fp: Fingerprint,
        data_dir: &Path,
    ) -> Result<(String, GameClient, SessionKind)> {
        let key = Self::account_key(channel, migration_code);
        {
            let mut map = self.inner.lock().await;
            if let Some(p) = map.get(&key) {
                if Self::is_fresh(p) && Self::kind_ok(p.kind, need) {
                    let mut p = map.remove(&key).expect("entry");
                    p.last_used = Instant::now();
                    tracing::info!(
                        %key,
                        channel,
                        kind = ?p.kind,
                        need = ?need,
                        "session_pool: reuse"
                    );
                    return Ok((key, p.client, p.kind));
                }
                tracing::info!(
                    %key,
                    kind = ?p.kind,
                    need = ?need,
                    stale = !Self::is_fresh(p),
                    "session_pool: rebuild"
                );
                map.remove(&key);
            }
        }

        tracing::info!(%key, channel, need = ?need, "session_pool: login new");
        let (client, kind) = match need {
            SessionKind::Light => (
                GameClient::login_for_info(channel, migration_code, password, fp, data_dir).await?,
                SessionKind::Light,
            ),
            SessionKind::Full => (
                GameClient::login(channel, migration_code, password, fp, data_dir).await?,
                SessionKind::Full,
            ),
        };
        Ok((key, client, kind))
    }

    /// 归还会话。`drop_session=true` 时不入池（401 等）。
    pub async fn release(
        &self,
        key: String,
        kind: SessionKind,
        client: GameClient,
        drop_session: bool,
    ) {
        if drop_session {
            tracing::warn!(%key, "session_pool: release drop (session invalid)");
            return;
        }
        let mut map = self.inner.lock().await;
        map.insert(
            key,
            Pooled {
                client,
                kind,
                last_used: Instant::now(),
            },
        );
    }

    pub async fn stats_json(&self) -> serde_json::Value {
        let map = self.inner.lock().await;
        let mut full = 0u32;
        let mut light = 0u32;
        for p in map.values() {
            match p.kind {
                SessionKind::Full => full += 1,
                SessionKind::Light => light += 1,
            }
        }
        serde_json::json!({
            "entries": map.len(),
            "full": full,
            "light": light,
            "idle_ttl_secs": IDLE_TTL.as_secs(),
        })
    }
}

/// 是否应丢弃会话（401 / 会话类错误）。
pub fn should_drop_session(err: &anyhow::Error) -> bool {
    if let Some(ce) = err.downcast_ref::<CoreError>() {
        return should_drop_core(ce);
    }
    let s = err.to_string();
    s.contains("HTTP_401") || s.contains("http 401")
}

pub fn should_drop_core(err: &CoreError) -> bool {
    matches!(err, CoreError::Http { status: 401, .. })
        || err.to_string().contains("HTTP_401")
}

/// 本进程全局池（Owner serve + 同进程 IPC/`run` 共用）。
static PROCESS_POOL: OnceLock<SessionPool> = OnceLock::new();

pub fn process_pool() -> &'static SessionPool {
    PROCESS_POOL.get_or_init(SessionPool::new)
}
