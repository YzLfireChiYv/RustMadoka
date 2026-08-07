//! 数据文件夹占用心跳（跨路径 / 云盘二次保险）
//!
//! # 与 owner.lock 的分工
//! - **同机同路径**：`owner_lock` 独占文件锁（本文件不替代）。
//! - **跨路径 / 同步盘**：`occupancy_heartbeat.json` 记录时间 + **数据文件夹路径**；
//!   约 1 分钟刷新；不同路径且心跳新鲜（默认 30 分钟）时默认拒绝接管，
//!   用户可在程序运行面板终端输入 **`我已知晓`** 强制启动。
//!
//! # 禁止
//! - 勿把心跳写进 `owner.lock`（独占后云盘无法实时同步该文件）。
//!
//! # 文档
//! - `docs/PLAN_AUDIT_COMMS_OCCUPANCY_PROGRESS.md` §4
//! - `docs/tech/INSTANCE_AND_CLI.md`

use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 心跳文件名（与 owner.lock 分离）
pub const HEARTBEAT_FILE: &str = "occupancy_heartbeat.json";

/// 不同数据路径下，默认拒绝接管的窗口
pub const STALE_MINUTES: i64 = 30;

/// 刷新间隔
pub const REFRESH_SECS: u64 = 60;

const FORCE_PHRASE: &str = "我已知晓";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OccupancyHeartbeat {
    /// UTC RFC3339

    pub updated_at: String,
    /// 占用方数据文件夹绝对路径（主判定字段）

    pub data_dir_path: String,
    /// 占用方 exe 绝对路径（信息性）

    #[serde(default)]
    pub exe_path: String,
    #[serde(default)]
    pub pid: u32,
    /// idle = 正常退出写闲置；active = 运行中

    #[serde(default = "default_active")]
    pub state: String,
}

fn default_active() -> String {
    "active".into()
}

fn heartbeat_path(data_dir: &Path) -> PathBuf {
    data_dir.join(HEARTBEAT_FILE)
}

fn canonical_data_dir(data_dir: &Path) -> String {
    data_dir
        .canonicalize()
        .unwrap_or_else(|_| data_dir.to_path_buf())
        .to_string_lossy()
        .to_lowercase()
}

fn exe_path_string() -> String {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| String::new())
}

/// 读取心跳（文件不存在则 None）
pub fn load(data_dir: &Path) -> Result<Option<OccupancyHeartbeat>> {
    let p = heartbeat_path(data_dir);
    if !p.is_file() {
        return Ok(None);
    }
    let t = std::fs::read_to_string(&p)?;
    let hb: OccupancyHeartbeat = serde_json::from_str(&t)?;
    Ok(Some(hb))
}

/// 写入心跳（短写短放，不独占锁，便于云盘同步）
pub fn write_active(data_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(data_dir)?;
    let hb = OccupancyHeartbeat {
        updated_at: Utc::now().to_rfc3339(),
        data_dir_path: canonical_data_dir(data_dir),
        exe_path: exe_path_string(),
        pid: std::process::id(),
        state: "active".into(),
    };
    let p = heartbeat_path(data_dir);
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&hb)?)?;
    std::fs::rename(&tmp, &p).or_else(|_| {
        std::fs::copy(&tmp, &p)?;
        std::fs::remove_file(&tmp)
    })?;
    Ok(())
}

/// 正常退出尽量写闲置
pub fn write_idle(data_dir: &Path) -> Result<()> {
    let mut hb = load(data_dir)?.unwrap_or(OccupancyHeartbeat {
        updated_at: Utc::now().to_rfc3339(),
        data_dir_path: canonical_data_dir(data_dir),
        exe_path: exe_path_string(),
        pid: std::process::id(),
        state: "idle".into(),
    });
    hb.updated_at = Utc::now().to_rfc3339();
    hb.state = "idle".into();
    hb.pid = std::process::id();
    hb.exe_path = exe_path_string();
    let p = heartbeat_path(data_dir);
    std::fs::write(&p, serde_json::to_string_pretty(&hb)?)?;
    Ok(())
}

fn parse_updated(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

/// 启动前检查：同数据路径立即允许；不同路径且 30 分钟内 active → 要求「我已知晓」
pub fn check_before_owner(data_dir: &Path) -> Result<()> {
    let self_path = canonical_data_dir(data_dir);
    let Some(hb) = load(data_dir)? else {
        return Ok(());
    };
    if hb.state == "idle" {
        return Ok(());
    }
    let other = hb.data_dir_path.to_lowercase();
    if other.is_empty() || other == self_path {
        // 同路径（本机重启等）立即允许
        return Ok(());
    }
    let Some(updated) = parse_updated(&hb.updated_at) else {
        return Ok(());
    };
    let age = Utc::now().signed_duration_since(updated);
    if age > Duration::minutes(STALE_MINUTES) {
        return Ok(());
    }
    let remain = STALE_MINUTES - age.num_minutes().max(0);
    eprintln!();
    eprintln!("========================================");
    eprintln!("  占用二次保险（跨路径 / 云盘同步）");
    eprintln!("  数据文件夹心跳显示：另一路径在约 {STALE_MINUTES} 分钟内曾占用本份数据。");
    eprintln!("  记录路径: {}", hb.data_dir_path);
    eprintln!("  本机路径: {self_path}");
    eprintln!("  心跳时间: {}（约剩余 {remain} 分钟窗口）", hb.updated_at);
    eprintln!("  默认不自动接管，以免你不知情时双开。");
    eprintln!("  若你**知道**自己在做什么，请输入完整短语：{FORCE_PHRASE}");
    eprintln!("  （单独 y / yes 无效）");
    eprintln!("========================================");
    eprint!("确认> ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    if line.trim() != FORCE_PHRASE {
        bail!(
            "未输入「{FORCE_PHRASE}」，拒绝接管数据文件夹。\n  {}",
            data_dir.display()
        );
    }
    tracing::warn!(
        data_dir = %data_dir.display(),
        other = %hb.data_dir_path,
        "occupancy force start with 我已知晓"
    );
    Ok(())
}

/// 后台刷新心跳，直到 stop 为 true
pub fn spawn_refresh_loop(data_dir: PathBuf, stop: Arc<AtomicBool>) {
    std::thread::Builder::new()
        .name("occupancy-heartbeat".into())
        .spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                if let Err(e) = write_active(&data_dir) {
                    tracing::warn!(error = %e, "occupancy heartbeat write failed");
                }
                for _ in 0..REFRESH_SECS {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
            let _ = write_idle(&data_dir);
        })
        .ok();
}

/// RAII：停止刷新线程标志
pub struct OccupancyGuard {
    stop: Arc<AtomicBool>,
    data_dir: PathBuf,
}

impl OccupancyGuard {
    pub fn start(data_dir: &Path) -> Result<Self> {
        check_before_owner(data_dir)?;
        write_active(data_dir)?;
        let stop = Arc::new(AtomicBool::new(false));
        spawn_refresh_loop(data_dir.to_path_buf(), stop.clone());
        Ok(Self {
            stop,
            data_dir: data_dir.to_path_buf(),
        })
    }
}

impl Drop for OccupancyGuard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = write_idle(&self.data_dir);
    }
}
