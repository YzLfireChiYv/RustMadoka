//! Owner ↔ Client 本机 IPC（Windows 命名管道；非 HTTP）
//!
//! 文档: docs/tech/INSTANCE_AND_CLI.md
//! 避免 CLI 走 HTTP 栈 / 系统代理带来的怪异问题。

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

/// 客户端 → Owner
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum IpcRequest {
    Ping,
    RunInfo {
        group: String,
        #[serde(default)]
        group_password: Option<String>,
        alias: String,
    },
    RunDaily {
        group: String,
        #[serde(default)]
        group_password: Option<String>,
        alias: String,
    },
    /// 单模块（与 HTTP/CLI `run module` 对齐；有 Owner 时走 IPC）
    RunModule {
        group: String,
        #[serde(default)]
        group_password: Option<String>,
        alias: String,
        key: String,
        #[serde(default)]
        safe_raid_damage: bool,
    },
    /// E1 会话快照导出
    ExportSession {
        group: String,
        #[serde(default)]
        group_password: Option<String>,
        alias: String,
        #[serde(default)]
        include_session_id: bool,
        #[serde(default)]
        include_device_debug: bool,
        #[serde(default)]
        out: Option<String>,
    },
    /// 运行控制（须 Owner 内 RunHub）
    RunPause,
    RunResume,
    RunAbort,
    RunStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    pub ok: bool,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
}

pub fn pipe_name_for_data_dir(data_dir: &Path) -> String {
    let canon = data_dir
        .canonicalize()
        .unwrap_or_else(|_| data_dir.to_path_buf());
    let s = canon.to_string_lossy().to_lowercase();
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    let hex = hex::encode(h.finalize());
    format!(r"\\.\pipe\rustmadoka_{}", &hex[..24])
}

/// Client：发送一条命令并读一条响应（默认 120s）
#[cfg(windows)]
pub async fn client_call(data_dir: &Path, req: &IpcRequest) -> Result<IpcResponse> {
    client_call_timeout(data_dir, req, 120).await
}

/// Client：可指定超时（E1 full_login 导出可能较久）
#[cfg(windows)]
pub async fn client_call_timeout(
    data_dir: &Path,
    req: &IpcRequest,
    timeout_secs: u64,
) -> Result<IpcResponse> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::windows::named_pipe::ClientOptions;

    let name = pipe_name_for_data_dir(data_dir);
    let client = ClientOptions::new()
        .open(&name)
        .with_context(|| format!("连接 Owner IPC 失败（是否未启动运行面板？）: {name}"))?;

    let mut client = client;
    let line = serde_json::to_string(req)? + "\n";
    client.write_all(line.as_bytes()).await?;
    client.flush().await?;

    let mut reader = BufReader::new(client);
    let mut resp_line = String::new();
    let n = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        reader.read_line(&mut resp_line),
    )
    .await
    .context("等待 Owner 响应超时")??;
    if n == 0 {
        bail!("Owner 关闭了 IPC 连接");
    }
    let resp: IpcResponse = serde_json::from_str(resp_line.trim())?;
    Ok(resp)
}

#[cfg(not(windows))]
pub async fn client_call(_data_dir: &Path, _req: &IpcRequest) -> Result<IpcResponse> {
    bail!("IPC Client 当前仅实现 Windows 命名管道")
}

#[cfg(not(windows))]
pub async fn client_call_timeout(
    data_dir: &Path,
    req: &IpcRequest,
    _timeout_secs: u64,
) -> Result<IpcResponse> {
    client_call(data_dir, req).await
}

/// Owner：后台接受 IPC
#[cfg(windows)]
pub fn spawn_ipc_server<F, Fut>(data_dir: std::path::PathBuf, handler: F)
where
    F: Fn(IpcRequest) -> Fut + Send + Sync + 'static + Clone,
    Fut: std::future::Future<Output = IpcResponse> + Send + 'static,
{
    tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::windows::named_pipe::ServerOptions;

        let name = pipe_name_for_data_dir(&data_dir);
        tracing::info!(%name, "IPC server listening (named pipe)");
        let mut first = true;
        loop {
            let mut opts = ServerOptions::new();
            if first {
                opts.first_pipe_instance(true);
                first = false;
            }
            let mut server = match opts.create(&name) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(error = %e, "IPC create pipe failed");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    first = true;
                    continue;
                }
            };
            if let Err(e) = server.connect().await {
                tracing::warn!(error = %e, "IPC connect wait failed");
                continue;
            }
            let mut line = String::new();
            {
                let mut reader = BufReader::new(&mut server);
                match reader.read_line(&mut line).await {
                    Ok(0) => continue,
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!(error = %e, "IPC read failed");
                        continue;
                    }
                }
            }
            let req: IpcRequest = match serde_json::from_str(line.trim()) {
                Ok(r) => r,
                Err(e) => {
                    let resp = IpcResponse {
                        ok: false,
                        error: Some(format!("bad request: {e}")),
                        result: None,
                    };
                    let body = serde_json::to_string(&resp).unwrap_or_default() + "\n";
                    let _ = server.write_all(body.as_bytes()).await;
                    continue;
                }
            };
            let resp = handler.clone()(req).await;
            let body = serde_json::to_string(&resp).unwrap_or_else(|_| {
                r#"{"ok":false,"error":"serialize"}"#.into()
            }) + "\n";
            let _ = server.write_all(body.as_bytes()).await;
            let _ = server.flush().await;
        }
    });
}

#[cfg(not(windows))]
pub fn spawn_ipc_server<F, Fut>(_data_dir: std::path::PathBuf, _handler: F)
where
    F: Fn(IpcRequest) -> Fut + Send + Sync + 'static + Clone,
    Fut: std::future::Future<Output = IpcResponse> + Send + 'static,
{
    tracing::warn!("IPC server skipped (non-Windows)");
}
