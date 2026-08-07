//! Gree SDK（日服 / 国际服）— 设备注册、引继迁移、OAuth 授权、token 缓存。
//!
//! # 职责
//! - `login_or_migrate`：读/写 `cache/token/{引继}_{pwdMD5}.json`；失败则 register+migrate
//! - **`device_id` 按游戏账号卡片复用**（channel 无关、键为引继码；见 `device_by_account`）
//! - 对加密后的游戏 body 提供 RSA 签名材料（由 GameClient 调 ApiCrypto 路径）
//!
//! # 签名阶段机（L1 · 禁止混淆）
//! - `/auth/initialize`：私钥**尚未**挂上 → **HMAC-SHA1(APP_SECRET)**，无 xoauth_as_hash
//! - 之后：RSA-SHA1 **Prehashed(SHA1)**（DigestInfo 前缀，不是 raw unprefixed）
//! - JSON body hash 依赖键序 → 工作区 serde_json `preserve_order`
//!
//! # 文档
//! - `docs/tech/SDK_AND_LOGIN.md` §2 · §5（设备身份）
//! - `docs/tech/GROUP_RAID_AND_DEVICE_IDENTITY.md` §6
//! - `docs/tech/LESSONS_RUST_PORT.md` L1 · L10
//! - `docs/tech/AUTOMADOKA_RESEARCH_AND_RUST_GAP.md` §2.2
//!
//! # 对照
//! `archive/.../sdk/greeclient.py` · `sdkclients.py`

use crate::crypto::{self, b_encode, generate_gree_rsa};
use crate::diag::network_from_reqwest;
use crate::error::{CoreError, Result};
use crate::fingerprint::Fingerprint;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use hmac::{Hmac, Mac};
use md5::{Digest as Md5Digest, Md5};
use rand::RngCore;
use rsa::pkcs8::DecodePrivateKey;
use rsa::{Pkcs1v15Sign, RsaPrivateKey};
use serde_json::{json, Value};
use sha1::Sha1;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

type HmacSha1 = Hmac<Sha1>;

#[derive(Clone, Copy, Debug)]
pub enum GreeRegion {
    Japan,
    Global,
}

impl GreeRegion {
    pub fn app_id(self) -> &'static str {
        match self {
            Self::Japan => "863165203288142",
            Self::Global => "934835692267709",
        }
    }
    pub fn app_secret(self) -> &'static str {
        match self {
            Self::Japan => "858931807c393c548db2a5f725bb6b45",
            Self::Global => "c1d8e8d0bfe9a9026cd21c83a9584586",
        }
    }
    pub fn base_url(self) -> &'static str {
        match self {
            Self::Japan => "https://gl-pkl-jp-payment.gree-apps.net/v1.0",
            Self::Global => "https://gl-pkl-us-payment.gree-apps.net/v1.0",
        }
    }
    pub fn api_root(self) -> &'static str {
        match self {
            Self::Japan => "https://api.mmme.pokelabo.jp",
            Self::Global => "https://api-gl.mmme.pokelabo.jp",
        }
    }
}

pub struct GreeSession {
    pub region: GreeRegion,
    pub private_key_der: Option<Vec<u8>>,
    pub public_key_pem: String,
    pub uuid: String,
    pub device_id: String,
    http: reqwest::Client,
}

fn generate_device_id() -> String {
    let mut buf = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut buf);
    // Python: B_encode(buf.hex())
    b_encode(&hex::encode(buf))
}

/// 游戏账号卡片级设备身份（**不**再全安装共用一个 device_id）。
///
/// - 路径：`{token_dir}/../device_by_account/{引继安全名}.json`
/// - 同一引继码永远复用同一 `device_id`；不同引继码必须不同 id
/// - token 缓存内也写 `device_id`；token 丢失时靠本文件恢复
/// - 旧文件 `cache/device_profile.json` 仅历史遗留，**不再**作为新号来源
///
/// 文档：`docs/tech/GROUP_RAID_AND_DEVICE_IDENTITY.md` §6 · `SDK_AND_LOGIN.md`
fn account_device_path(token_dir: &Path, migration_code: &str) -> PathBuf {
    let safe: String = migration_code
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let name = if safe.is_empty() {
        "unknown".into()
    } else {
        safe
    };
    token_dir
        .parent()
        .unwrap_or(token_dir)
        .join("device_by_account")
        .join(format!("{name}.json"))
}

/// 读取或创建**该游戏账号卡片**的稳定 device_id
fn load_or_create_device_id_for_account(token_dir: &Path, migration_code: &str) -> String {
    let path = account_device_path(token_dir, migration_code);
    if let Ok(t) = std::fs::read_to_string(&path) {
        if let Ok(v) = serde_json::from_str::<Value>(&t) {
            if let Some(id) = v.get("device_id").and_then(|x| x.as_str()) {
                if !id.is_empty() {
                    return id.to_string();
                }
            }
        }
    }
    let id = generate_device_id();
    let _ = save_account_device_id(token_dir, migration_code, &id);
    id
}

fn save_account_device_id(token_dir: &Path, migration_code: &str, device_id: &str) -> Result<()> {
    let path = account_device_path(token_dir, migration_code);
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let v = json!({
        "schema": 2,
        "migration_code": migration_code,
        "device_id": device_id,
        "deviceModel": "Asus ASUS_I003DD",
        "osType": 2,
        "osVersion": "Android OS 9 / API-28 (PI/rel.cjw.20220518.114133)",
        "note": "Per game-account-card device identity. Reused only for this migration code.",
    });
    std::fs::write(path, serde_json::to_string_pretty(&v)?)?;
    Ok(())
}

/// 登录用固定设备画像（与 register / LoginApi 保持一致）
pub fn fixed_device_model() -> &'static str {
    "Asus ASUS_I003DD"
}
pub fn fixed_os_version() -> &'static str {
    "Android OS 9 / API-28 (PI/rel.cjw.20220518.114133)"
}

/// 与 Python Util.encode 一致（latin-1 逐字节 %XX）
fn oauth_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        let c = *b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~') {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// urllib.parse.quote 默认 safe='/' — OAuth 头里对 value 的编码
fn quote_oauth_value(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        let c = *b as char;
        if c.is_ascii_alphanumeric()
            || matches!(c, '-' | '.' | '_' | '~' | '/')
        {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Python: json.dumps(obj) 默认分隔符 ", " / ": "
fn py_json_dumps(v: &Value) -> String {
    // 手写紧凑替代不够；用 serde 后再近似空格形式
    // 对签名最敏感的是外层 separators=(",",":")；内层 payload 用默认带空格
    match v {
        Value::Object(map) => {
            let parts: Vec<String> = map
                .iter()
                .map(|(k, val)| format!("\"{}\": {}", escape_json_str(k), py_json_dumps(val)))
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
        Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(py_json_dumps).collect();
            format!("[{}]", parts.join(", "))
        }
        Value::String(s) => format!("\"{}\"", escape_json_str(s)),
        Value::Number(n) => n.to_string(),
        Value::Bool(true) => "true".into(),
        Value::Bool(false) => "false".into(),
        Value::Null => "null".into(),
    }
}

fn escape_json_str(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// 外层 body：Python separators=(",", ":")
fn py_json_dumps_compact(v: &Value) -> String {
    match v {
        Value::Object(map) => {
            let parts: Vec<String> = map
                .iter()
                .map(|(k, val)| {
                    format!("\"{}\":{}", escape_json_str(k), py_json_dumps_compact(val))
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(py_json_dumps_compact).collect();
            format!("[{}]", parts.join(","))
        }
        Value::String(s) => format!("\"{}\"", escape_json_str(s)),
        Value::Number(n) => n.to_string(),
        Value::Bool(true) => "true".into(),
        Value::Bool(false) => "false".into(),
        Value::Null => "null".into(),
    }
}

/// RSA-SHA1 Prehashed：DigestInfo + PKCS1v15（= cryptography Prehashed(SHA1)）
fn rsa_sign_sha1_prehash(der: &[u8], digest20: &[u8]) -> Result<Vec<u8>> {
    let key =
        RsaPrivateKey::from_pkcs8_der(der).map_err(|e| CoreError::Crypto(e.to_string()))?;
    let pad = Pkcs1v15Sign::new::<Sha1>();
    key.sign(pad, digest20)
        .map_err(|e| CoreError::Crypto(e.to_string()))
}

impl GreeSession {
    pub async fn login_or_migrate(
        region: GreeRegion,
        migration_code: &str,
        password: &str,
        token_dir: &Path,
        fp: &Fingerprint,
    ) -> Result<Self> {
        std::fs::create_dir_all(token_dir)?;
        let cache = token_path(token_dir, migration_code, password);
        // 必须设超时：否则 Gree 链路假死会卡死整个 exe（前端一起无响应）。
        // 登录多步可超过 25s；与游戏客户端对齐加长。Docs: docs/tech/WIRE_AND_DEBUG_PROBES.md
        let http = reqwest::Client::builder()
            .user_agent("rustmadoka/0.1")
            .connect_timeout(std::time::Duration::from_secs(20))
            .timeout(std::time::Duration::from_secs(90))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .build()
            .map_err(|e| CoreError::Network(format!("NET_OTHER|构建Gree客户端|{e}")))?;

        // 坏缓存会导致 Invalid Signature 循环：authorize 失败则删缓存重走注册
        if cache.is_file() {
            if let Ok(sess) = load_cache(&cache, region, http.clone()) {
                match sess
                    .request("POST", "/auth/authorize", None, &fp.version)
                    .await
                {
                    Ok(_) => {
                        tracing::info!("gree: cache authorize ok");
                        return Ok(sess);
                    }
                    Err(e) if format!("{e}").contains("Inactive") => {
                        let sess = sess;
                        let _ = sess
                            .request("POST", "/linked/active/update", None, &fp.version)
                            .await;
                        sess.request("POST", "/auth/authorize", None, &fp.version)
                            .await?;
                        return Ok(sess);
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "gree cache authorize failed, wipe and re-register");
                        let _ = std::fs::remove_file(&cache);
                    }
                }
            }
        }

        // 与 Python register：先只有公钥 PEM，private_key 在 initialize **之后**才挂上
        // device_id：按本引继码（游戏账号卡片）复用，见 GROUP_RAID_AND_DEVICE_IDENTITY §6
        let (der, pem) = generate_gree_rsa()?;
        let mut sess = Self {
            region,
            private_key_der: None, // initialize 必须 HMAC
            public_key_pem: pem,
            uuid: String::new(),
            device_id: load_or_create_device_id_for_account(token_dir, migration_code),
            http: http.clone(),
        };
        sess.register_device(fp).await?;
        // initialize 成功后挂上私钥（与 Python 顺序一致）
        sess.private_key_der = Some(der);
        sess.migrate_from(migration_code, password, &fp.version)
            .await?;
        sess.request("POST", "/auth/authorize", None, &fp.version)
            .await?;
        save_cache(&cache, &sess)?;
        let _ = save_account_device_id(token_dir, migration_code, &sess.device_id);
        tracing::info!("gree: migrate+authorize ok");
        Ok(sess)
    }

    async fn register_device(&mut self, fp: &Fingerprint) -> Result<()> {
        // 内层 payload：Python json.dumps 默认带空格；设备字段固定
        let payload = json!({
            "appVersion": fp.version,
            "urlParam": null,
            "deviceModel": fixed_device_model(),
            "osType": 2,
            "osVersion": fixed_os_version(),
            "storeType": 2,
            "graphicsDeviceId": 0,
            "graphicsDeviceVendorId": 0,
            "processorCount": 4,
            "processorType": "x86-64 SSE3 SSE4.1 SSE4.2 AVX",
            "supportedRenderTargetCount": 8,
            "supports3DTextures": true,
            "supportsAccelerometer": true,
            "supportsComputeShaders": true,
            "supportsGyroscope": true,
            "supportsImageEffects": true,
            "supportsInstancing": true,
            "supportsLocationService": true,
            "supportsRenderTextures": true,
            "supportsRenderToCubemap": true,
            "supportsShadows": true,
            "supportsSparseTextures": false,
            "supportsStencil": 1,
            "supportsVibration": false,
            "uuid": null,
            "xuid": 0,
            "sm": fp.sm(),
        });
        let payload_str = py_json_dumps(&payload);
        let body = json!({
            "device_id": self.device_id,
            "token": self.public_key_pem,
            "payload": payload_str,
        });
        let res = self
            .request("POST", "/auth/initialize", Some(body), &fp.version)
            .await?;
        self.uuid = res["uuid"]
            .as_str()
            .ok_or_else(|| CoreError::Login("no uuid from initialize".into()))?
            .to_string();
        Ok(())
    }

    async fn migrate_from(&mut self, code: &str, password: &str, app_ver: &str) -> Result<()> {
        let verify = self
            .request(
                "POST",
                "/migration/code/verify",
                Some(json!({
                    "migration_code": code,
                    "migration_password": b_encode(password),
                })),
                app_ver,
            )
            .await?;
        let migration_token = verify["migration_token"].as_str().unwrap_or("").to_string();
        let src_uuid = verify["src_uuid"].as_str().unwrap_or("").to_string();
        self.request(
            "POST",
            "/migration",
            Some(json!({
                "migration_token": migration_token,
                "src_uuid": src_uuid,
                "device_id": self.device_id,
                "token": self.public_key_pem,
                "dst_uuid": self.uuid,
            })),
            app_ver,
        )
        .await?;
        if !src_uuid.is_empty() {
            self.uuid = src_uuid;
        }
        Ok(())
    }

    pub async fn request(
        &self,
        method: &str,
        route: &str,
        body: Option<Value>,
        app_version: &str,
    ) -> Result<Value> {
        let url = format!("{}{}", self.region.base_url(), route);
        // Python: raw = "" if body is None else json.dumps(body, separators=(",",":"))
        let raw = match &body {
            None => String::new(),
            Some(b) => py_json_dumps_compact(b),
        };
        let ts = chrono::Utc::now().timestamp().to_string();

        let mut oauth: BTreeMap<String, String> = BTreeMap::new();
        let mut h = Sha1::new();
        h.update(raw.as_bytes());
        oauth.insert(
            "oauth_body_hash".into(),
            B64.encode(h.finalize()).trim().to_string(),
        );
        oauth.insert("oauth_consumer_key".into(), self.region.app_id().into());
        oauth.insert(
            "oauth_nonce".into(),
            // Python: str(random.getrandbits(64))
            format!("{}", rand::random::<u64>()),
        );
        oauth.insert("oauth_timestamp".into(), ts.clone());
        oauth.insert("oauth_version".into(), "1.0".into());

        if let Some(der) = &self.private_key_der {
            oauth.insert("oauth_signature_method".into(), "RSA-SHA1".into());

            // xoauth_as_hash = Sign(SHA1(APP_SECRET + ts))
            let v6 = format!("{}{}", self.region.app_secret(), ts);
            let mut h1 = Sha1::new();
            h1.update(v6.as_bytes());
            let hash1 = h1.finalize();
            let sig1 = rsa_sign_sha1_prehash(der, &hash1)?;
            oauth.insert("xoauth_as_hash".into(), B64.encode(sig1).trim().to_string());
            oauth.insert("xoauth_requestor_id".into(), self.uuid.clone());

            // oauth_signature = Sign(SHA1(normalize(...)))
            let norm = normalize(method, &url, &oauth);
            let mut h2 = Sha1::new();
            h2.update(norm.as_bytes());
            let sig2 = rsa_sign_sha1_prehash(der, &h2.finalize())?;
            oauth.insert("oauth_signature".into(), B64.encode(sig2).trim().to_string());
        } else {
            // HMAC-SHA1（initialize 阶段）
            oauth.insert("oauth_signature_method".into(), "HMAC-SHA1".into());
            let norm = normalize(method, &url, &oauth);
            let mut mac = <HmacSha1 as Mac>::new_from_slice(self.region.app_secret().as_bytes())
                .map_err(|e| CoreError::Crypto(e.to_string()))?;
            mac.update(norm.as_bytes());
            let sig = mac.finalize().into_bytes();
            oauth.insert("oauth_signature".into(), B64.encode(sig).trim().to_string());
        }

        // Authorization 头：按插入顺序在 Python 是 OrderedDict；服务端一般只校验签名字段。
        // 仍用稳定排序生成头，避免差异；签名 base string 已按 sorted 计算。
        let auth = format!(
            "OAuth {}",
            oauth
                .iter()
                .map(|(k, v)| format!(r#"{k}="{}""#, quote_oauth_value(v)))
                .collect::<Vec<_>>()
                .join(",")
        );

        let gree_hdr = format!(
            "authVersion%3D1.5.28%26billing%3D3%26storeType%3Dgoogle%26appVersion%3D{app_version}%26uaType%3Dandroid-app%26carrier%3DChina+Mobile+GSM%26compromised%3Dfalse%26countryCode%3DCN%26currencyCode%3DCNY%26model%3DAndroid-Phone"
        );

        let mut req = self.http.request(
            match method {
                "GET" => reqwest::Method::GET,
                _ => reqwest::Method::POST,
            },
            &url,
        );
        req = req
            .header("Authorization", auth)
            .header("X-GREE-GAMELIB", gree_hdr)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Linux; Android 9; ASUS_I003DD Build/PI; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/68.0.3440.70 Mobile Safari/537.36",
            );
        let req_body_for_wire = if method != "GET" {
            Some(raw.clone())
        } else {
            None
        };
        if method != "GET" {
            req = req
                .header("Content-Type", "application/json")
                .body(raw);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| {
                if crate::wire::is_active() {
                    crate::wire::record_sdk_http(
                        method,
                        &url,
                        route,
                        req_body_for_wire.as_deref(),
                        0,
                        &json!({"error": e.to_string()}),
                        Some("send failed"),
                    );
                }
                network_from_reqwest(&e, &format!("Gree {route}"))
            })?;
        let status = resp.status().as_u16();
        let text = resp
            .text()
            .await
            .map_err(|e| network_from_reqwest(&e, &format!("Gree读响应 {route}")))?;
        let val: Value = serde_json::from_str(&text).unwrap_or(json!({ "raw": text }));
        if val.get("result").and_then(|r| r.as_str()) != Some("OK") {
            if crate::wire::is_active() {
                crate::wire::record_sdk_http(
                    method,
                    &url,
                    route,
                    req_body_for_wire.as_deref(),
                    status,
                    &val,
                    Some("result!=OK"),
                );
            }
            // 保留 status + body 供 diagnose：Invalid Signature / 403 等
            // 文档: docs/tech/ERROR_DIAGNOSTICS.md · LESSONS L1
            return Err(CoreError::Login(format!(
                "gree {status} {route}: {}",
                crate::diag::sanitize_body(&text)
            )));
        }
        if crate::wire::is_active() {
            crate::wire::record_sdk_http(
                method,
                &url,
                route,
                req_body_for_wire.as_deref(),
                status,
                &val,
                None,
            );
        }
        Ok(val)
    }

    pub fn sign_game_body(&self, crypted: &[u8]) -> Result<String> {
        let der = self
            .private_key_der
            .as_ref()
            .ok_or_else(|| CoreError::Crypto("no private key for game sign".into()))?;
        crypto::sign_request(crypted, der)
    }
}

/// Util.normalize
fn normalize(method: &str, url: &str, params: &BTreeMap<String, String>) -> String {
    let items: Vec<String> = params
        .iter()
        .filter(|(k, _)| *k != "oauth_signature")
        .map(|(k, v)| format!("{}={}", oauth_encode(k), oauth_encode(v)))
        .collect();
    // BTreeMap already sorted
    let param_str = items.join("&");
    let base = {
        // scheme://host/path 无 query、默认端口省略
        if let Some(rest) = url.strip_prefix("https://") {
            let (hostpath, _) = rest.split_once('?').unwrap_or((rest, ""));
            format!("https://{hostpath}")
        } else if let Some(rest) = url.strip_prefix("http://") {
            let (hostpath, _) = rest.split_once('?').unwrap_or((rest, ""));
            format!("http://{hostpath}")
        } else {
            url.to_string()
        }
    };
    format!(
        "{}&{}&{}",
        oauth_encode(method),
        oauth_encode(&base),
        oauth_encode(&param_str)
    )
}

fn token_path(dir: &Path, code: &str, password: &str) -> PathBuf {
    let md5 = hex::encode(Md5::digest(password.as_bytes()));
    dir.join(format!("{code}_{md5}.json"))
}

fn save_cache(path: &Path, sess: &GreeSession) -> Result<()> {
    let der = sess
        .private_key_der
        .as_ref()
        .ok_or_else(|| CoreError::other("save without key"))?;
    let v = json!({
        "privateKey": B64.encode(der),
        "uuid": sess.uuid,
        "device_id": sess.device_id,
        "public_key_pem": sess.public_key_pem,
    });
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&v)?)?;
    Ok(())
}

/// 从 token 文件名 `{引继}_{md5}.json` 解析引继码（最后一个 `_` 前为引继；引继本身可含 `_` 时取最长前缀启发式）
fn migration_code_from_token_filename(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    // 密码 md5 为 32 hex；文件名 = `{code}_{32hex}`
    if stem.len() > 33 && stem.as_bytes().get(stem.len() - 33) == Some(&b'_') {
        return Some(stem[..stem.len() - 33].to_string());
    }
    stem.rsplit_once('_').map(|(code, _)| code.to_string())
}

fn load_cache(path: &Path, region: GreeRegion, http: reqwest::Client) -> Result<GreeSession> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let der = B64
        .decode(v["privateKey"].as_str().unwrap_or(""))
        .map_err(|e| CoreError::other(e.to_string()))?;
    let mut device_id = v["device_id"].as_str().unwrap_or("").to_string();
    let mig = migration_code_from_token_filename(path).unwrap_or_default();
    if let Some(token_dir) = path.parent() {
        if device_id.is_empty() {
            // 缺字段：按卡片文件恢复或新建（不读安装级 device_profile）
            if !mig.is_empty() {
                device_id = load_or_create_device_id_for_account(token_dir, &mig);
            } else {
                device_id = generate_device_id();
            }
        } else if !mig.is_empty() {
            // 已有 token 内 id：写回卡片文件，保证后续 token 重建仍绑同一卡片
            let _ = save_account_device_id(token_dir, &mig, &device_id);
        }
    }
    Ok(GreeSession {
        region,
        private_key_der: Some(der),
        public_key_pem: v["public_key_pem"].as_str().unwrap_or("").to_string(),
        uuid: v["uuid"].as_str().unwrap_or("").to_string(),
        device_id,
        http,
    })
}
