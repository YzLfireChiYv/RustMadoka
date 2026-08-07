//! Windows 系统通知（右下角 toast）。
//!
//! # 产品口径
//! - 配置落在**数据文件夹** `notifications/system_toast.json`
//! - **默认关闭**（`enabled: false`）
//! - 与浏览器网页 toast、设置变更历史（`FeatureNotifyFile`）是不同产品面
//!
//! # 文档
//! - `docs/tech/CLI_WEB_PARITY.md` §4.2 · `docs/tech/WINDOWS_SYSTEM_NOTIFY.md`
//! - NORMS：默认关；本机明文与协作卫生无关
//!
//! Outbound: `crates/rustmadoka-app/src/system_toast.rs`

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

const SCHEMA: u32 = 1;
const FILE_NAME: &str = "system_toast.json";

/// 数据文件夹内系统通知开关
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemToastSettings {
    #[serde(default = "schema_default")]
    pub schema: u32,
    /// 是否允许弹出 Windows 系统通知（默认关）
    #[serde(default)]
    pub enabled: bool,
    /// 任务成功结束时是否通知（仅 enabled 时生效）
    #[serde(default = "default_true")]
    pub on_task_success: bool,
    /// 任务失败/中止结束时是否通知
    #[serde(default = "default_true")]
    pub on_task_error: bool,
}

fn schema_default() -> u32 {
    SCHEMA
}
fn default_true() -> bool {
    true
}

impl Default for SystemToastSettings {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            enabled: false,
            on_task_success: true,
            on_task_error: true,
        }
    }
}

fn settings_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("notifications").join(FILE_NAME)
}

impl SystemToastSettings {
    pub fn load(data_dir: &Path) -> Self {
        let p = settings_path(data_dir);
        if !p.is_file() {
            return Self::default();
        }
        match std::fs::read_to_string(&p) {
            Ok(t) => serde_json::from_str(&t).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, data_dir: &Path) -> Result<()> {
        let p = settings_path(data_dir);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut s = self.clone();
        s.schema = SCHEMA;
        std::fs::write(&p, serde_json::to_string_pretty(&s)?)?;
        Ok(())
    }
}

/// 若已启用则尝试弹出系统通知。失败只记日志，不阻断业务。
pub fn notify_if_enabled(data_dir: &Path, title: &str, body: &str) {
    let s = SystemToastSettings::load(data_dir);
    if !s.enabled {
        return;
    }
    if let Err(e) = show_toast(title, body) {
        tracing::warn!(error = %e, "system toast failed");
    }
}

/// 任务结束钩子：按设置决定是否 toast
pub fn notify_task_finished(data_dir: &Path, ok: bool, title: &str, body: &str) {
    let s = SystemToastSettings::load(data_dir);
    if !s.enabled {
        return;
    }
    if ok && !s.on_task_success {
        return;
    }
    if !ok && !s.on_task_error {
        return;
    }
    if let Err(e) = show_toast(title, body) {
        tracing::warn!(error = %e, "system toast failed");
    }
}

/// 强制弹出一次（CLI 测试用；不检查 enabled）
pub fn show_toast(title: &str, body: &str) -> Result<()> {
    #[cfg(windows)]
    {
        show_toast_windows(title, body)
    }
    #[cfg(not(windows))]
    {
        let _ = (title, body);
        anyhow::bail!("系统通知当前仅在 Windows 上实现");
    }
}

#[cfg(windows)]
fn show_toast_windows(title: &str, body: &str) -> Result<()> {
    // 使用 PowerShell + Windows Runtime Toast（无额外 crate；默认关时几乎不走此路径）
    let title_esc = ps_single_quote(title);
    let body_esc = ps_single_quote(body);
    let script = format!(
        r#"
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
$template = @"
<toast>
  <visual>
    <binding template="ToastGeneric">
      <text>{title_esc}</text>
      <text>{body_esc}</text>
    </binding>
  </visual>
</toast>
"@
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml($template)
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier("RustMadoka").Show($toast)
"#
    );
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .status()
        .context("启动 PowerShell 弹出系统通知失败")?;
    if !status.success() {
        anyhow::bail!("PowerShell 系统通知退出码非 0: {status}");
    }
    Ok(())
}

fn ps_single_quote(s: &str) -> String {
    // 放入双引号 here-string 时仍转义 XML 特殊字符
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
