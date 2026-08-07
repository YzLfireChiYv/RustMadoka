//! Fingerprint loading order for RustMadoka.
//!
//! Order: fp slots → cache files → embed → remote default URL.
//! Docs: `docs/tech/VERSION_FINGERPRINT.md` · NORMS P15

use anyhow::Result;
use rustmadoka_core::fingerprint::{
    fetch_fingerprint, fingerprint_from_combined_text, Fingerprint, EMBEDDED_COMBINED_JSON,
};
use std::path::{Path, PathBuf};

use crate::fp_slots;

pub async fn load_fp(data_dir: &Path, fp_url: &str, channel: &str) -> Result<Fingerprint> {
    if let Ok(fp) = fp_slots::load_fp_from_slots(data_dir, channel) {
        let _ = fp.save_version_json(&data_dir.join("cache/version.json"));
        return Ok(fp);
    }
    let candidates = [
        data_dir.join("cache/automadoka.combined.json"),
        data_dir.join("cache/automadoka.json"),
        data_dir
            .parent()
            .map(|p| p.join("publish/automadoka.json"))
            .unwrap_or_else(|| PathBuf::from("publish/automadoka.json")),
    ];
    for path in &candidates {
        if path.is_file() {
            if let Ok(text) = std::fs::read_to_string(path) {
                if let Ok(fp) = fingerprint_from_combined_text(&text, channel) {
                    let _ = fp.save_version_json(&data_dir.join("cache/version.json"));
                    return Ok(fp);
                }
            }
        }
    }
    if let Ok(fp) = fingerprint_from_combined_text(EMBEDDED_COMBINED_JSON, channel) {
        let _ = fp.save_version_json(&data_dir.join("cache/version.json"));
        return Ok(fp);
    }
    let fp = fetch_fingerprint(fp_url, channel).await?;
    let _ = fp.save_version_json(&data_dir.join("cache/version.json"));
    Ok(fp)
}
