//! Debug-build wire session RAII.
//!
//! 使用 `ensure_started`：已有会话则复用，避免 HTTP 流式日常与 run_ops 双重 start 失败。
//! Drop 时 **不** stop：便于一次任务内多阶段共会话；进程结束或显式 stop 收尾。
//!
//! Docs: `docs/tech/WIRE_AND_DEBUG_PROBES.md` · `docs/logs/2026-08-07-dual-exe-wire.md`
//! Outbound: `crates/rustmadoka-core/src/wire.rs`

use std::path::Path;

/// Whether this package records wire (debug feature).
pub fn should_record_wire(_cli_wire: bool) -> bool {
    cfg!(feature = "wire_record")
}

/// Ensures wire session on enter (debug only). Does **not** stop on drop
/// so nested scopes and long HTTP tasks share one full capture.
pub struct WireScope {
    /// If true, this scope owns stop (legacy one-shot). Default false.
    stop_on_drop: bool,
}

impl WireScope {
    pub fn enter(data_dir: &Path, alias: &str, channel: &str, purpose: &str) -> Self {
        if should_record_wire(false) {
            if let Some(dir) =
                rustmadoka_core::wire::ensure_started(data_dir, alias, channel, purpose)
            {
                rustmadoka_core::wire::set_module_key(Some(purpose));
                rustmadoka_core::wire::record_probe(
                    "wire_scope_enter",
                    serde_json::json!({
                        "alias": alias,
                        "channel": channel,
                        "purpose": purpose,
                        "dir": dir.display().to_string(),
                    }),
                );
                tracing::info!(%alias, purpose, dir = %dir.display(), "wire scope on (ensure)");
            }
        }
        Self {
            stop_on_drop: false,
        }
    }

    /// 任务结束时显式收口（可选；CLI 可在 finalize 后调用）
    pub fn finish(self) {
        if self.stop_on_drop && should_record_wire(false) {
            rustmadoka_core::wire::stop();
        }
    }
}

impl Drop for WireScope {
    fn drop(&mut self) {
        if self.stop_on_drop && should_record_wire(false) {
            rustmadoka_core::wire::stop();
        }
    }
}
