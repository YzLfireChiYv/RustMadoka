//! R2 分区：`domain` — 用户组/账号、日常与工具模块、组队团战、mst、安全门、会话导出。
//!
//! # 现状（2026-08-08 收口口径）
//! - **语义命名空间已完成**：`rustmadoka_core::domain::*` 与根路径并行可用。
//! - **物理迁入 `src/domain/`**：演进式；为降低协议正确性风险，实现文件仍在 crate 根旁路，
//!   产品语义以本模块 re-export 为界（PLAN R2 允许演进式，禁止盲重写 Gree/AES）。
//! - **DataPaths**：app 层 `paths::resolve_data_dir` + `data_layout::ensure_data_layout`；
//!   Store 持有 `data_dir` 并镜像 layout2 树。
//!
//! # Docs
//! - `docs/PLAN_RUSTMADOKA_FULL_REWRITE.md` 阶段 R2
//! - `docs/tech/DATA_FOLDER_LAYOUT.md`
//! - `docs/MODULES.md` · `docs/tech/MODULE_SEMANTIC_CLASSIFICATION.md`
//! - `docs/tech/GROUP_RAID_AND_DEVICE_IDENTITY.md`
//!
//! # Outbound
//! `crate::{account,modules,mst,safety,notify,session_export}`

pub use crate::account::{Channel, GameAccount, GroupListItem, Store, ToolUser, UserGroup};
pub use crate::modules::{
    all_setting_defaults, daily_catalog, daily_keys, daily_modules_info, flatten_for_save,
    is_low_risk_module, is_partial_success_log, low_risk_module_keys, merge_run_config,
    module_config_fields, resolve_enabled, resolve_enabled_from_store, run_daily,
    run_daily_with_progress, run_group_raid, run_player_info, run_single_module, run_super_wash,
    run_super_wash_with_progress, shop_item_categories, style_choices, sub_selection_choices,
    upstream_shop_priority_patch, ConfigField, DailyReport, GroupRaidConfig, GroupRaidMember,
    GroupRaidReport, ModuleCatalogEntry, ModuleInfo, ModuleResult, ProgressEvent, ProgressTx,
    RoomOpenMode, RunControlFlags,
};
pub use crate::mst;
pub use crate::notify::{
    append_settings_notify, sanitize_feature, ChangeAfter, FeatureNotifyFile, NotifyEntry,
    FEATURE_SETTINGS,
};
pub use crate::safety::{
    assert_daily_allowed, assert_tool_allowed, daily_allowed, gates_json, tool_allowed,
    ALLOW_DAILY_RUN, ALLOW_TOOL_RUN, CONFIG_PACK_SCHEMA, UPSTREAM_COMPAT,
};
pub use crate::session_export::{
    write_session_export, SessionExportMeta, SessionExportOptions, SessionExportResult,
};
