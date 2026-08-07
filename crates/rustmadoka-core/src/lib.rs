//! rustmadoka-core — game protocol and business modules (platform-agnostic).
//!
//! # Responsibility
//! - Channel login (Gree JP/EN), fingerprint `sm`, AES+msgpack game API
//! - User group / game account model, daily modules, wash, safety gates, diagnostics
//! - Group raid orchestration
//!
//! # R2 layout (semantic partitions)
//! - [`protocol`] — Gree / AES / client / fingerprint / wire
//! - [`domain`] — account / modules / mst / safety / notify / session export
//! - [`diag`] + [`error`] — diagnostics and result types
//! Root-level modules remain for stable `use rustmadoka_core::gree` paths.
//!
//! # Docs (bidirectional)
//! - Plan: `docs/PLAN_RUSTMADOKA_FULL_REWRITE.md`
//! - Protocol: `docs/tech/PROTOCOL_STACK.md` · `docs/tech/SDK_AND_LOGIN.md`
//! - Group raid: `docs/tech/GROUP_RAID_AND_DEVICE_IDENTITY.md`
//! - Modules: `docs/MODULES.md` · `docs/tech/ERROR_DIAGNOSTICS.md`
//!
//! # Reference (read-only)
//! `archive/pre-rust-2026-08/autopcr/`
//!
//! # Invariants
//! - TW / Sonet login not implemented (`Channel::Tw`)
//! - Daily catalog defaults all off (product safety, P17)

pub mod crypto;
pub mod diag;
pub mod error;
pub mod fingerprint;
pub mod gree;
pub mod client;
pub mod account;
pub mod modules;
pub mod mst;
pub mod safety;
pub mod notify;
pub mod session_export;
pub mod wire;

/// R2: protocol stack namespace
pub mod protocol;
/// R2: product domain namespace
pub mod domain;

pub use account::{
    Channel, GameAccount, GroupListItem, GroupRaidConfigEntry, GroupRaidPanelConfig, Store,
    ToolUser, UserGroup,
};
pub use notify::{
    append_settings_notify, sanitize_feature, ChangeAfter, FeatureNotifyFile, NotifyEntry,
    FEATURE_SETTINGS,
};
pub use client::{party_summaries_from_init, GameClient};
pub use session_export::{
    write_session_export, SessionExportMeta, SessionExportOptions, SessionExportResult,
};
pub use diag::{
    classify_http, diagnose_text, format_anyhow_zh, network_from_reqwest, DiagReport, ErrorCode,
};
pub use error::{CoreError, Result};
pub use fingerprint::{
    apply_sm, build_combined_publish_json, channel_versions_from_text, extract_from_xapk,
    fetch_fingerprint, fetch_fingerprint_text, fingerprint_from_combined_text,
    parse_fingerprint_file_text, Fingerprint, FingerprintFile, EMBEDDED_COMBINED_JSON,
};
pub use modules::{
    all_setting_defaults, daily_catalog, daily_keys, daily_modules_info, flatten_for_save,
    is_low_risk_module, low_risk_module_keys, merge_run_config, module_config_fields,
    resolve_enabled, resolve_enabled_from_store, shop_item_categories, upstream_shop_priority_patch,
    run_daily, run_daily_with_progress,
    run_group_raid, run_player_info, run_single_module, run_single_module_with_progress,
    run_super_wash,
    run_super_wash_with_progress, style_choices, sub_selection_choices, is_partial_success_log,
    tool_catalog, ConfigField, DailyReport, GroupRaidConfig, GroupRaidMember, GroupRaidReport,
    ModuleCatalogEntry, ModuleInfo, ModuleResult, ProgressEvent, ProgressTx, RoomOpenMode,
    RunControlFlags,
};
pub use safety::{
    assert_daily_allowed, assert_tool_allowed, daily_allowed, gates_json, tool_allowed,
    CONFIG_PACK_SCHEMA, UPSTREAM_COMPAT, ALLOW_DAILY_RUN, ALLOW_TOOL_RUN,
};
pub use mst::{
    filter_quest_stages, format_quest_stage_label, quest_stage_name, summarize_quest_stage,
};
