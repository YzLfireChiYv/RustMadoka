//! R2 分区：`protocol` — 对游戏服务器的登录、加密封包、指纹、通讯录制。
//!
//! # 现状（2026-08-08 收口口径）
//! - **语义命名空间已完成**：`rustmadoka_core::protocol::*` 可用。
//! - **物理迁入 `src/protocol/`**：演进式保留根旁路实现文件，避免 L1/L2 级盲搬风险。
//! - **约束**：本分区**不得**依赖 Win-only API（R7）；设备 id 按游戏账号卡片（引继）。
//!
//! # Docs
//! - `docs/PLAN_RUSTMADOKA_FULL_REWRITE.md` 阶段 R2 · R7
//! - `docs/tech/PROTOCOL_STACK.md` · `docs/tech/SDK_AND_LOGIN.md` · `docs/tech/VERSION_FINGERPRINT.md`
//!
//! # Outbound 源码
//! `crate::{crypto,gree,client,fingerprint,wire}`

pub use crate::client::{party_summaries_from_init, GameClient};
pub use crate::crypto;
pub use crate::fingerprint::{
    apply_sm, build_combined_publish_json, channel_versions_from_text, extract_from_xapk,
    fetch_fingerprint, fetch_fingerprint_text, fingerprint_from_combined_text,
    parse_fingerprint_file_text, Fingerprint, FingerprintFile, EMBEDDED_COMBINED_JSON,
};
pub use crate::gree;
pub use crate::wire;
