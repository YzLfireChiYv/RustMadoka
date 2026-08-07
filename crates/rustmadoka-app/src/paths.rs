//! Data-folder and product path constants.
//!
//! Docs: `docs/PLAN_RUSTMADOKA_FULL_REWRITE.md` · `docs/tech/INSTANCE_AND_CLI.md`

use std::path::PathBuf;

/// Default runtime data folder name (beside the exe). No legacy `automadoka_data`.
pub const DATA_DIR_NAME: &str = "RustMadoka_data";

/// Resolve data directory from optional CLI override, else exe-sibling `RustMadoka_data`.
pub fn resolve_data_dir(cli_override: Option<PathBuf>) -> PathBuf {
    cli_override.unwrap_or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(DATA_DIR_NAME)))
            .unwrap_or_else(|| PathBuf::from(DATA_DIR_NAME))
    })
}
