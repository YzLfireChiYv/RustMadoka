//! Android JNI 壳 → 启动与 Win 同源的 HTTP Owner（`rustmadoka_app::run_embedded_serve`）。
//!
//! # 职责
//! - 在 Android 进程内起 loopback HTTP，WebView / 系统浏览器访问浏览器网页前端
//! - 协议与业务在 `rustmadoka-core`；本 crate 仅 JNI 入口与路径/端口注入
//!
//! # 产品约定
//! - 包名：`com.rustmadoka.android.NativeBridge`（勿覆盖主人日常包 `com.automadoka.app`，P12）
//! - 产物：`librustmadoka_mobile.so`（arm64-v8a / x86_64）
//! - 默认端口 **14103**，数据目录注入为 `RustMadoka_data` 语义路径
//!
//! # 文档（双向链接）
//! - `docs/tech/ANDROID_DUAL_PLATFORM.md` · `docs/PLAN_ANDROID_AND_DUAL_PLATFORM.md`
//! - 对照：`archive/pre-rust-2026-08/android` BackendService（Python 时代）
//!
//! Outbound: `crates/rustmadoka-mobile/src/lib.rs`

use android_logger::Config;
use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;
use log::LevelFilter;
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::runtime::Runtime;

/// 默认 loopback 端口（与 Win 产品默认一致，便于文档与浏览器网页前端习惯）
pub const DEFAULT_PORT: u16 = 14103;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static STARTED: AtomicBool = AtomicBool::new(false);
static LISTEN_PORT: AtomicU16 = AtomicU16::new(0);
static LOG_INIT: AtomicBool = AtomicBool::new(false);

fn ensure_log() {
    if LOG_INIT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        android_logger::init_once(
            Config::default()
                .with_max_level(LevelFilter::Info)
                .with_tag("automadoka"),
        );
        let _ = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_env_filter("info,rustmadoka_core=info,rustmadoka_app=info")
            .try_init();
        log::info!("rustmadoka-mobile log init");
    }
}

fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("tokio runtime for rustmadoka-mobile")
    })
}

/// 闃诲绛夊埌 health 鎴栬秴鏃讹紱杩斿洖 JSON 瀛楃涓茬姸鎬併€
fn start_server(data_dir: PathBuf, port: u16) -> String {
    ensure_log();
    if STARTED.load(Ordering::SeqCst) {
        let p = LISTEN_PORT.load(Ordering::SeqCst);
        return json!({
            "ok": true,
            "listening": true,
            "note": "already_running",
            "port": p,
            "url": format!("http://127.0.0.1:{p}/"),
            "data_dir": data_dir.display().to_string(),
        })
        .to_string();
    }

    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        return json!({
            "ok": false,
            "listening": false,
            "error": format!("create data_dir: {e}"),
        })
        .to_string();
    }

    let data_for_task = data_dir.clone();
    let rt = runtime();
    rt.spawn(async move {
        log::info!(
            "run_embedded_serve start data_dir={} port={}",
            data_for_task.display(),
            port
        );
        match rustmadoka_app::run_embedded_serve(data_for_task, port, None, None).await {
            Ok(()) => log::info!("run_embedded_serve exited cleanly"),
            Err(e) => log::error!("run_embedded_serve failed: {e:#}"),
        }
        STARTED.store(false, Ordering::SeqCst);
        LISTEN_PORT.store(0, Ordering::SeqCst);
    });

    // 杞 /api/health
    let url = format!("http://127.0.0.1:{port}/api/health");
    let deadline = std::time::Instant::now() + Duration::from_secs(45);
    let client = reqwest_blocking_get_health(&url, deadline);

    match client {
        Ok(()) => {
            STARTED.store(true, Ordering::SeqCst);
            LISTEN_PORT.store(port, Ordering::SeqCst);
            json!({
                "ok": true,
                "listening": true,
                "note": "started",
                "port": port,
                "url": format!("http://127.0.0.1:{port}/"),
                "data_dir": data_dir.display().to_string(),
                "build_stamp": rustmadoka_app::BUILD_STAMP,
                "version": rustmadoka_app::APP_VERSION,
            })
            .to_string()
        }
        Err(e) => json!({
            "ok": false,
            "listening": false,
            "error": e,
            "port": port,
            "data_dir": data_dir.display().to_string(),
        })
        .to_string(),
    }
}

/// Probe Owner health without reqwest: plain TcpStream HTTP GET.
/// Docs: docs/PLAN_RUSTMADOKA_FULL_REWRITE.md R7 · crates/rustmadoka-mobile
fn reqwest_blocking_get_health(url: &str, deadline: std::time::Instant) -> Result<(), String> {
    // url = http://127.0.0.1:PORT/api/health
    let port_path = url
        .strip_prefix("http://127.0.0.1:")
        .ok_or_else(|| format!("bad health url: {url}"))?;
    let (port_s, path) = port_path
        .split_once('/')
        .ok_or_else(|| format!("bad health url path: {url}"))?;
    let port: u16 = port_s
        .parse()
        .map_err(|e| format!("bad port in url: {e}"))?;
    let path = format!("/{path}");

    let mut last_err = String::from("timeout waiting for health");
    while std::time::Instant::now() < deadline {
        match try_http_get_ok(port, &path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = e;
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
    Err(last_err)
}

fn try_http_get_ok(port: u16, path: &str) -> Result<(), String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .map_err(|e| format!("connect: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .ok();
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .ok();
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| format!("write: {e}"))?;
    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).map_err(|e| format!("read: {e}"))?;
    let text = String::from_utf8_lossy(&buf[..n]);
    if text.contains("200") && text.contains("\"ok\"") {
        Ok(())
    } else if text.starts_with("HTTP/1.1 200") || text.starts_with("HTTP/1.0 200") {
        Ok(())
    } else {
        Err(format!("health not ready: {}", text.chars().take(120).collect::<String>()))
    }
}

fn status_json() -> String {
    let started = STARTED.load(Ordering::SeqCst);
    let port = LISTEN_PORT.load(Ordering::SeqCst);
    json!({
        "ok": started,
        "listening": started,
        "port": port,
        "url": if port > 0 {
            format!("http://127.0.0.1:{port}/")
        } else {
            String::new()
        },
        "build_stamp": rustmadoka_app::BUILD_STAMP,
        "version": rustmadoka_app::APP_VERSION,
    })
    .to_string()
}

// --- JNI ---

/// `NativeBridge.nativeStart(dataDir: String, port: Int): String`
#[no_mangle]
pub extern "system" fn Java_com_rustmadoka_android_NativeBridge_nativeStart(
    mut env: JNIEnv,
    _class: JClass,
    data_dir: JString,
    port: jni::sys::jint,
) -> jstring {
    let result = (|| -> Result<String, String> {
        let dir: String = env
            .get_string(&data_dir)
            .map_err(|e| format!("data_dir: {e}"))?
            .into();
        let p = if port <= 0 || port > 65535 {
            DEFAULT_PORT
        } else {
            port as u16
        };
        Ok(start_server(PathBuf::from(dir), p))
    })();

    let s = match result {
        Ok(s) => s,
        Err(e) => json!({"ok": false, "listening": false, "error": e}).to_string(),
    };
    env.new_string(s)
        .expect("jni new_string")
        .into_raw()
}

/// `NativeBridge.nativeStatus(): String`
#[no_mangle]
pub extern "system" fn Java_com_rustmadoka_android_NativeBridge_nativeStatus(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    ensure_log();
    env.new_string(status_json())
        .expect("jni new_string")
        .into_raw()
}

/// `NativeBridge.nativeBuildStamp(): String`
#[no_mangle]
pub extern "system" fn Java_com_rustmadoka_android_NativeBridge_nativeBuildStamp(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    env.new_string(rustmadoka_app::BUILD_STAMP)
        .expect("jni new_string")
        .into_raw()
}
