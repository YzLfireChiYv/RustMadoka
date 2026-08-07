//! 构建时间版本号：精确到分钟（主人 2026-08-06 定稿）
//! 环境变量 AUTOMADOKA_BUILD_STAMP 可覆盖（CI 用）
//! 文档: docs/PLAN_RELEASE_AND_SELF_UPDATE.md · docs/tech/TECH_DOC_CONVENTION.md

fn main() {
    let stamp = std::env::var("AUTOMADOKA_BUILD_STAMP").unwrap_or_else(|_| {
        // 本地：用系统本地时间 YYYY.MM.DD.HHMM
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        // 简易 UTC 分解（避免 chrono 构建依赖）；构建机若需本地时区可设 env
        let secs = now + 8 * 3600; // 默认 +08:00 展示
        let days = secs / 86400;
        let tod = secs % 86400;
        let (y, m, d) = civil_from_days(days);
        let hh = tod / 3600;
        let mm = (tod % 3600) / 60;
        format!("{y:04}.{m:02}.{d:02}.{hh:02}{mm:02}")
    });
    println!("cargo:rustc-env=AUTOMADOKA_BUILD_STAMP={stamp}");
    println!("cargo:rerun-if-env-changed=AUTOMADOKA_BUILD_STAMP");
    // 内置指纹在 core include_str；变更 publish 时需重编 core
    println!("cargo:rerun-if-changed=../../publish/automadoka.json");
}

/// Howard Hinnant civil_from_days（UTC 日序 → y/m/d）
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}
