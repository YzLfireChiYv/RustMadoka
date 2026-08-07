//! Data-folder Owner lock — only one Owner process per `RustMadoka_data`.
//!
//! Docs: `docs/tech/INSTANCE_AND_CLI.md` · `docs/PLAN_RUSTMADOKA_FULL_REWRITE.md`
//! Implementation: exclusive `owner.lock` (released on process exit/crash by OS).

use anyhow::{bail, Result};
use fs2::FileExt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 持有即表示本进程是该 data 的 Owner
pub struct OwnerGuard {
    _file: std::fs::File,
    pub data_dir: PathBuf,
}

/// 尝试成为 Owner；失败表示已有其它进程绑定该 data
pub fn try_acquire(data_dir: &Path) -> Result<OwnerGuard> {
    std::fs::create_dir_all(data_dir)?;
    let lock_path = data_dir.join("owner.lock");
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    match file.try_lock_exclusive() {
        Ok(()) => {
            let _ = file.set_len(0);
            let _ = writeln!(
                file,
                "pid={}\nstarted={}",
                std::process::id(),
                chrono::Utc::now().to_rfc3339()
            );
            let _ = file.sync_all();
            Ok(OwnerGuard {
                _file: file,
                data_dir: data_dir.to_path_buf(),
            })
        }
        Err(_) => {
            bail!(
                "data folder already owned by another process:\n  {}\n\
                 Only one Owner per RustMadoka_data. Use the open process, or close it first.",
                data_dir.display()
            )
        }
    }
}
