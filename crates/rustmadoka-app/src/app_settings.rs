//! 本机应用设置（落在 RustMadoka_data/app.json）
//!
//! 文档: docs/PLAN_RUSTMADOKA_FULL_REWRITE.md · docs/tech/INSTANCE_AND_CLI.md
//!
//! 文档: docs/tech/INSTANCE_AND_CLI.md · docs/PLAN_INSTANCE_CLI_PORT.md
//! - listen_port 持久化：不同 data 目录可多开不同端口

use serde::{Deserialize, Serialize};
use std::path::Path;

/// 默认 loopback 端口（与旧 automadoka 13220 无关）
pub const DEFAULT_LISTEN_PORT: u16 = 14103;
/// 主人日常包端口；选用时须警告 + 二次确认
pub const RESERVED_DAILY_PORT: u16 = 13200;

/// 版本/指纹信息源（GitHub raw 或自备 URL）— 一条一条管理
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoSource {
    /// 稳定 id（前端删改用）

    #[serde(default = "new_source_id")]
    pub id: String,
    pub name: String,
    pub url: String,
    /// fingerprint | release_manifest | notes

    #[serde(default = "default_source_kind")]
    pub kind: String,
    /// 是否为内置默认源（不可删，可禁用）

    #[serde(default)]
    pub builtin: bool,
    /// 用户可关

    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 最近一次测试结果

    #[serde(default)]
    pub last_test_ok: Option<bool>,
    #[serde(default)]
    pub last_test_at: Option<String>,
    #[serde(default)]
    pub last_test_message: Option<String>,
}

fn new_source_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn default_source_kind() -> String {
    "fingerprint".into()
}
fn default_true() -> bool {
    true
}

/// 内置默认指纹源 URL（主人 rules 仓；唯一默认源）
/// raw：https://raw.githubusercontent.com/YzLfireChiYv/rules/main/automadoka.json
pub const DEFAULT_FP_SOURCE_URL: &str =
    "https://raw.githubusercontent.com/YzLfireChiYv/rules/main/automadoka.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_port")]
    pub listen_port: u16,
    /// 信息源列表（逐条管理）

    #[serde(default = "default_info_sources")]
    pub info_sources: Vec<InfoSource>,
    /// 手动粘贴的版本/指纹备注（兜底）

    #[serde(default)]
    pub manual_version_note: String,
    /// 最近一次从远端拉取的摘要（JSON 字符串或说明）

    #[serde(default)]
    pub last_remote_info: Option<serde_json::Value>,
    #[serde(default)]
    pub last_remote_fetched_at: Option<String>,
    /// 每日版本检查：本地日 YYYY-MM-DD（+08）

    #[serde(default)]
    pub last_version_check_day: Option<String>,
    /// 最近一次多源检查结果摘要

    #[serde(default)]
    pub last_version_check: Option<serde_json::Value>,
}

fn default_port() -> u16 {
    DEFAULT_LISTEN_PORT
}

/// 默认：仅 rules 仓一条指纹源（主人指定）
pub fn default_info_sources() -> Vec<InfoSource> {
    vec![InfoSource {
        id: "builtin-fp".into(),
        name: "GitHub rules 指纹（默认）".into(),
        url: DEFAULT_FP_SOURCE_URL.into(),
        kind: "fingerprint".into(),
        builtin: true,
        enabled: true,
        last_test_ok: None,
        last_test_at: None,
        last_test_message: None,
    }]
}

/// 旧配置若仍指向 RustMadoka 或多余 builtin，收敛为 rules 单默认源
pub fn migrate_default_sources(sources: &mut Vec<InfoSource>) {
    let mut out = Vec::new();
    let mut has_rules_fp = false;
    for s in sources.drain(..) {
        let is_old_rustmadoka = s.url.contains("YzLfireChiYv/RustMadoka");
        let is_rules_fp = s.url.contains("YzLfireChiYv/rules")
            && s.url.contains("automadoka.json")
            && s.kind == "fingerprint";
        if s.builtin && is_old_rustmadoka {
            continue; // 丢掉旧内置
        }
        if is_rules_fp {
            has_rules_fp = true;
            let mut s = s;
            s.builtin = true;
            s.id = "builtin-fp".into();
            s.name = "GitHub rules 指纹（默认）".into();
            s.enabled = true;
            out.insert(0, s);
            continue;
        }
        if s.builtin && s.kind == "release_manifest" {
            continue; // 默认不再自带 RELEASES 源
        }
        out.push(s);
    }
    if !has_rules_fp {
        out.insert(0, default_info_sources().into_iter().next().unwrap());
    }
    *sources = out;
    normalize_sources(sources);
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            listen_port: DEFAULT_LISTEN_PORT,
            info_sources: default_info_sources(),
            manual_version_note: String::new(),
            last_remote_info: None,
            last_remote_fetched_at: None,
            last_version_check_day: None,
            last_version_check: None,
        }
    }
}

/// 迁移旧 sources（无 id）
pub fn normalize_sources(sources: &mut Vec<InfoSource>) {
    if sources.is_empty() {
        *sources = default_info_sources();
        return;
    }
    for s in sources.iter_mut() {
        if s.id.is_empty() {
            s.id = new_source_id();
        }
    }
}

/// load 后调用：normalize + 默认源迁移
pub fn prepare_sources(settings: &mut AppSettings) {
    if settings.info_sources.is_empty() {
        settings.info_sources = default_info_sources();
    }
    migrate_default_sources(&mut settings.info_sources);
}

/// 今日日期字符串（+08）
pub fn today_day_plus08() -> String {
    use chrono::{FixedOffset, Utc};
    let offset = FixedOffset::east_opt(8 * 3600).unwrap();
    Utc::now().with_timezone(&offset).format("%Y-%m-%d").to_string()
}

/// 简易版本比较：a > b → 1；相等 0；a < b → -1；无法比 0
pub fn cmp_game_version(a: &str, b: &str) -> i32 {
    let pa: Vec<u64> = a
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    let pb: Vec<u64> = b
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();
    let n = pa.len().max(pb.len());
    for i in 0..n {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        if x > y {
            return 1;
        }
        if x < y {
            return -1;
        }
    }
    0
}

impl AppSettings {
    pub fn path(data_dir: &Path) -> std::path::PathBuf {
        data_dir.join("app.json")
    }

    pub fn load(data_dir: &Path) -> Self {
        let p = Self::path(data_dir);
        if !p.is_file() {
            return Self::default();
        }
        match std::fs::read_to_string(&p) {
            Ok(t) => serde_json::from_str(&t).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, data_dir: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(data_dir)?;
        let t = serde_json::to_string_pretty(self)?;
        std::fs::write(Self::path(data_dir), t)?;
        Ok(())
    }
}

/// 解析最终监听端口：CLI 显式 > app.json > 默认
pub fn resolve_listen_port(cli_port: Option<u16>, data_dir: &Path) -> u16 {
    if let Some(p) = cli_port {
        return p;
    }
    let s = AppSettings::load(data_dir);
    if s.listen_port == 0 {
        DEFAULT_LISTEN_PORT
    } else {
        s.listen_port
    }
}

/// 校验用户输入的端口字符串
pub fn parse_port_input(s: &str) -> Result<u16, String> {
    let t = s.trim();
    if t.is_empty() {
        return Err("端口不能为空".into());
    }
    let p: u16 = t
        .parse()
        .map_err(|_| format!("不是有效端口数字: {t}"))?;
    if p < 1024 {
        return Err(format!("端口 {p} 过小（建议 ≥1024，且避开系统保留）"));
    }
    Ok(p)
}
