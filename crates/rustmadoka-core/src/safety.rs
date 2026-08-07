//! 产品安全门禁（编译期常量 + 运行前 assert）。
//!
//! # 职责
//! - `ALLOW_DAILY_RUN` / `ALLOW_TOOL_RUN`：总开关；UI/API 执行前必须 `assert_*`
//! - 暴露 `gates_json` / 兼容短码 schema 供设置包与关于页
//!
//! # 产品安全仍靠（门打开之后）
//! - 模块默认全关、商店优先级 0、队伍默认空（须用户显式配置）— P17 · C5
//! - 二次确认键、低风险白名单 — UI 层
//!
//! # 文档
//! - `docs/PLAN_R3_ACCOUNT_CLI_UX.md` · `docs/tech/AUTOMADOKA_RESEARCH_AND_RUST_GAP.md` §3.3
//! - NORMS P17 · 教训 C5/C6

/// 清日常是否允许执行。
/// 打开后仍只跑**用户已启用**的模块；目录默认全部为关，避免一键误耗资源。
pub const ALLOW_DAILY_RUN: bool = true;

/// 工具区写操作（洗词条等）。
pub const ALLOW_TOOL_RUN: bool = true;

/// 上游配置/商店目录兼容号（手维；对照归档 autopcr Web ~1.7 与 shop.py 类别表）
/// 短码导入必须一致，防止类别错位误兑。
pub const UPSTREAM_COMPAT: &str = "1.7";

/// 我方配置短码 schema（改编码格式时 bump；与 build_stamp 无关）
pub const CONFIG_PACK_SCHEMA: u32 = 2;

pub fn daily_allowed() -> bool {
    ALLOW_DAILY_RUN
}

pub fn tool_allowed() -> bool {
    ALLOW_TOOL_RUN
}

/// 尝试执行清日常前必须调用；失败则拒绝。
pub fn assert_daily_allowed() -> Result<(), String> {
    if ALLOW_DAILY_RUN {
        Ok(())
    } else {
        Err(
            "清日常未开放（ALLOW_DAILY_RUN=false）。仅允许「获取账号信息」。"
                .into(),
        )
    }
}

pub fn assert_tool_allowed() -> Result<(), String> {
    if ALLOW_TOOL_RUN {
        Ok(())
    } else {
        Err("测试期未开放工具写操作（洗词条等）。仅允许「获取账号信息」。".into())
    }
}

/// 给 API/UI 的状态摘要
pub fn gates_json() -> serde_json::Value {
    serde_json::json!({
        "allow_daily_run": ALLOW_DAILY_RUN,
        "allow_tool_run": ALLOW_TOOL_RUN,
        "upstream_compat": UPSTREAM_COMPAT,
        "config_pack_schema": CONFIG_PACK_SCHEMA,
        "message": ""
    })
}
