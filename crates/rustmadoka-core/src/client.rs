//! 游戏 API 客户端 — 合并原版 apiclient + sessionmgr + pcrclient 主路径。
//!
//! # 职责
//! - Gree 登录后：LoginApi → mst bootstrap → 初始化 API 串 → 业务 `request`
//! - 加密封包 + `x-post-signature`（私钥来自 GreeSession）
//! - 维护 `session_id` / `user_id` / `init_data` / `game_config` / `MstCache`
//! - `login_for_info`：轻量登录（产品优化；原版无对等路径）
//! - `battle_log_from_units`：对照 datamgr.generate_battle_log
//!
//! # 不变量
//! - 请求串行（内部 Mutex）；envelope 字段与 actionTime 对齐 Python
//! - 台服 channel 在入口拒绝（Sonet 未移植）
//! - 全量登录串顺序与 `sessionmgr._login` 一致（见 SDK_AND_LOGIN §3）
//!
//! # 文档
//! - `docs/tech/PROTOCOL_STACK.md` · `docs/tech/SDK_AND_LOGIN.md`
//! - `docs/tech/INIT_AND_RESPONSE_PAYLOADS.md` — full_login 对照 sessionmgr 串
//! - `docs/tech/API_INVENTORY.md` · `docs/tech/UPSTREAM_SOURCE_AND_WIRE.md`
//! - `docs/tech/AUTOMADOKA_RESEARCH_AND_RUST_GAP.md` §2 · §5
//! - 错误中文：`docs/tech/ERROR_DIAGNOSTICS.md` · `crate::diag`
//!
//! # 对照
//! `archive/.../core/{apiclient,sessionmgr,pcrclient,datamgr}.py`
//!
//! # 稳定性
//! - HTTP connect/整体超时，避免无限挂起
//! - 非 200 经 `CoreError::http_status` 净化 body

use crate::crypto::{self, pklb_hash_key};
use crate::diag::network_from_reqwest;
use crate::error::{CoreError, Result};
use crate::fingerprint::Fingerprint;
use crate::gree::{GreeRegion, GreeSession};
use crate::mst::MstCache;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Windows FILETIME 风格 actionTime（与 Python actionTime 一致）
fn action_time() -> i64 {
    const EPOCH_DIFF: i64 = 11_644_473_600;
    let unix = chrono::Utc::now().timestamp() as i64;
    (unix + EPOCH_DIFF) * 10_000_000
}

pub struct GameClient {
    pub fp: Fingerprint,
    pub gree: GreeSession,
    pub session_id: Option<String>,
    pub user_id: i64,
    pub uuid: String,
    pub last_home_access: String,
    /// 登录后 `get_init_data_list` 全量（party / item / style 等）

    pub init_data: Value,
    /// `/api/config/get_config` 全量（体力上限、团战配置等）

    pub game_config: Value,
    pub mst: MstCache,
    http: reqwest::Client,
    crypt_key: String,
    lock: Arc<Mutex<()>>,
}

impl GameClient {
    pub async fn login(
        channel: &str,
        migration_code: &str,
        password: &str,
        fp: Fingerprint,
        data_dir: &Path,
    ) -> Result<Self> {
        let ch = crate::account::Channel::from_user(channel);
        if !ch.login_implemented() {
            return Err(CoreError::Login(
                "台服（Sonet）登录尚未实现。指纹通道已支持 tw；请使用日服或国际服，或等待 Sonet 移植。"
                    .into(),
            ));
        }
        let region = match ch {
            crate::account::Channel::Jp => GreeRegion::Japan,
            _ => GreeRegion::Global,
        };
        // Debug：无差别通讯录制会话（已有则复用）
        // Docs: docs/tech/WIRE_AND_DEBUG_PROBES.md
        let _ = crate::wire::ensure_started(data_dir, migration_code, ch.as_str(), "full_login");
        crate::wire::record_probe(
            "login_begin",
            json!({ "channel": ch.as_str(), "mode": "full_login" }),
        );
        let token_dir = data_dir.join("cache").join("token");
        let gree =
            GreeSession::login_or_migrate(region, migration_code, password, &token_dir, &fp).await?;
        let http = Self::build_http()?;
        let mut client = Self {
            uuid: gree.uuid.clone(),
            gree,
            fp,
            session_id: None,
            user_id: 0,
            last_home_access: "0".into(),
            init_data: Value::Null,
            game_config: Value::Null,
            mst: MstCache::default(),
            http,
            crypt_key: pklb_hash_key(),
            lock: Arc::new(Mutex::new(())),
        };
        client.full_login().await?;
        crate::wire::record_probe(
            "login_ok",
            json!({ "channel": ch.as_str(), "mode": "full_login", "user_id": client.user_id }),
        );
        Ok(client)
    }

    /// 构建游戏 HTTP 客户端。
    /// 单请求超时加长：快速刷图等多轮战斗 + 登录串会超过 25s（网页曾因 20s 前端超时显示「本机无响应」）。
    /// Docs: docs/tech/WIRE_AND_DEBUG_PROBES.md · docs/tech/ERROR_DIAGNOSTICS.md

    fn build_http() -> Result<reqwest::Client> {
        reqwest::Client::builder()
            .user_agent("UnityRequest   (Asus ASUS_I003DD Android OS 9 / API-28)")
            .connect_timeout(std::time::Duration::from_secs(20))
            // 单次 API 上限；整模块时长由前端/宿主更长超时覆盖
            .timeout(std::time::Duration::from_secs(120))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .build()
            .map_err(|e| {
                CoreError::Network(format!("NET_OTHER|构建HTTP客户端|{}", e))
            })
    }

    /// 仅获取账号信息用的轻量登录：Gree + LoginApi + userParam / init 最小集
    /// **跳过** mst 全表与大量初始化请求，避免「一次成功后第二次卡死」与长时间无响应。
    /// 文档: docs/PLAN_R3_ACCOUNT_CLI_UX.md · 安全门禁下只允许 info

    pub async fn login_for_info(
        channel: &str,
        migration_code: &str,
        password: &str,
        fp: Fingerprint,
        data_dir: &Path,
    ) -> Result<Self> {
        let ch = crate::account::Channel::from_user(channel);
        if !ch.login_implemented() {
            return Err(CoreError::Login(
                "台服（Sonet）登录尚未实现。指纹通道已支持 tw；请使用日服或国际服，或等待 Sonet 移植。"
                    .into(),
            ));
        }
        let region = match ch {
            crate::account::Channel::Jp => GreeRegion::Japan,
            _ => GreeRegion::Global,
        };
        let _ = crate::wire::ensure_started(data_dir, migration_code, ch.as_str(), "login_for_info");
        crate::wire::record_probe(
            "login_begin",
            json!({ "channel": ch.as_str(), "mode": "login_for_info" }),
        );
        let token_dir = data_dir.join("cache").join("token");
        tracing::info!(channel, "login_for_info: gree…");
        let gree =
            GreeSession::login_or_migrate(region, migration_code, password, &token_dir, &fp).await?;
        let http = Self::build_http()?;
        let mut client = Self {
            uuid: gree.uuid.clone(),
            gree,
            fp,
            session_id: None,
            user_id: 0,
            last_home_access: "0".into(),
            init_data: Value::Null,
            game_config: Value::Null,
            mst: MstCache::default(),
            http,
            crypt_key: pklb_hash_key(),
            lock: Arc::new(Mutex::new(())),
        };
        tracing::info!("login_for_info: game login api…");
        client.light_login().await?;
        tracing::info!("login_for_info: ok");
        Ok(client)
    }

    /// 最小登录链：/api/login → get_init_data_list 或 get_user_param_data

    async fn light_login(&mut self) -> Result<()> {
        let login_body = json!({
            "lastHomeAccessTime": self.last_home_access,
            "sm": self.fp.sm(),
            "appVersion": self.fp.version,
            "urlParam": null,
            "deviceModel": crate::gree::fixed_device_model(),
            "osType": 2,
            "osVersion": crate::gree::fixed_os_version(),
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
            "xuid": 0
        });
        let resp = self.request_raw("/api/login", login_body).await?;
        self.session_id = resp
            .pointer("/payload/sessionId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        self.user_id = resp
            .pointer("/payload/userId")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // 优先轻量 userParam；失败再拉 init 全量
        match self
            .request("/api/user/get_user_param_data", json!({}))
            .await
        {
            Ok(param) => {
                if let Some(upd) = param.get("userParamData") {
                    self.init_data = json!({ "userParamData": upd });
                } else {
                    self.init_data = param;
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "get_user_param_data failed, try init_data_list");
                let init = self
                    .request("/api/user/get_init_data_list", json!({}))
                    .await?;
                self.init_data = init;
            }
        }
        Ok(())
    }

    /// 全量登录串 — 对照 `sessionmgr._login` / docs/tech/SDK_AND_LOGIN.md §3
    /// 顺序：LoginApi → bootstrap_mst → init/party/character/collection/style/userParam/config/option/webpay/terms

    async fn full_login(&mut self) -> Result<()> {
        // LoginApiLoginRequest 字段集与 Python sessionmgr._ensure_token 对齐
        let login_body = json!({
            "lastHomeAccessTime": self.last_home_access,
            "sm": self.fp.sm(),
            "appVersion": self.fp.version,
            "urlParam": null,
            // 固定设备画像 — 与 gree::fixed_device_* / device_profile 一致，迭代勿改
            "deviceModel": crate::gree::fixed_device_model(),
            "osType": 2,
            "osVersion": crate::gree::fixed_os_version(),
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
            "xuid": 0
        });
        let resp = self
            .request_raw("/api/login", login_body)
            .await?;
        self.session_id = resp
            .pointer("/payload/sessionId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        self.user_id = resp
            .pointer("/payload/userId")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // mst bootstrap（角色/词条下拉依赖）
        self.bootstrap_mst().await?;

        // init chain — docs/tech/SDK_AND_LOGIN.md
        let init = self
            .request("/api/user/get_init_data_list", json!({}))
            .await?;
        self.init_data = init;
        let _ = self
            .request("/api/party/get_character_build_data_list", json!({}))
            .await;
        let _ = self
            .request("/api/character/get_character_list", json!({}))
            .await;
        let _ = self
            .request(
                "/api/collection/get_collection_param_up_achieve_data_list",
                json!({}),
            )
            .await;
        let _ = self
            .request("/api/collection/get_collection_data_list", json!({}))
            .await;
        let _ = self
            .request("/api/style/get_style_data_list", json!({}))
            .await;
        if let Ok(param) = self
            .request("/api/user/get_user_param_data", json!({}))
            .await
        {
            // 合并最新 userParamData 进 init_data，供 stamina/等级读数
            if let Some(upd) = param.get("userParamData") {
                if let Some(obj) = self.init_data.as_object_mut() {
                    obj.insert("userParamData".into(), upd.clone());
                }
            }
        }
        if let Ok(cfg) = self.request("/api/config/get_config", json!({})).await {
            self.game_config = cfg;
        }
        let _ = self.request("/api/user/load_option", json!({})).await;
        let _ = self.request("/api/web_pay/cancel_latest", json!({})).await;
        let _ = self
            .request("/api/terms/get_updated_terms", json!({ "storeType": 2 }))
            .await;
        Ok(())
    }

    /// 刷新 userParamData（买体力/扫荡后体力变化）

    pub async fn refresh_user_param(&mut self) -> Result<()> {
        let param = self
            .request("/api/user/get_user_param_data", json!({}))
            .await?;
        if let Some(upd) = param.get("userParamData") {
            if let Some(obj) = self.init_data.as_object_mut() {
                obj.insert("userParamData".into(), upd.clone());
            }
        }
        Ok(())
    }

    /// 从 allyBattleUnitList 生成 finalize 用 battleLog（对齐 datamgr.generate_battle_log）
    /// 文档: archive/.../core/datamgr.py

    pub fn battle_log_from_units(units: &[Value]) -> String {
        let result_units: Vec<Value> = units
            .iter()
            .map(|unit| {
                let style = unit.get("styleMstId").cloned().unwrap_or(json!(0));
                let atk = unit.get("atk").cloned().unwrap_or(json!(0));
                let speed = unit.get("speed").cloned().unwrap_or(json!(0));
                let id = unit
                    .get("battleUnitDataId")
                    .cloned()
                    .unwrap_or(json!(0));
                let special = unit
                    .pointer("/specialAttackInfo/skillMstId")
                    .cloned()
                    .unwrap_or(json!(0));
                let normal = unit
                    .pointer("/normalAttackInfo/skillMstId")
                    .cloned()
                    .unwrap_or(json!(0));
                let actives: Vec<Value> = unit
                    .get("attackInfoList")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|s| s.get("skillMstId").cloned())
                            .collect()
                    })
                    .unwrap_or_default();
                json!({
                    "serializeBattleParameter": {
                        "StyleMstId": style,
                        "ATK": atk,
                        "Speed": speed
                    },
                    "Id": id,
                    "SkillSet": {
                        "specialAttackMstId": special,
                        "normalAttackMstId": normal,
                        "activeSkillMstIds": actives
                    }
                })
            })
            .collect();
        // 与 Python json.dumps 键序接近即可；服务端主要校验结构
        serde_json::to_string(&json!({
            "Commands": [],
            "ResultBattleUnits": result_units,
            "ResultRound": 1
        }))
        .unwrap_or_else(|_| {
            r#"{"Commands":[],"ResultBattleUnits":[],"ResultRound":1}"#.into()
        })
    }

    pub fn simple_battle_log() -> &'static str {
        r#"{"Commands":[],"ResultBattleUnits":[],"ResultRound":1}"#
    }

    /// 业务 payload（自动补 lastHomeAccessTime + sm），返回 payload 对象

    pub async fn request(&mut self, url: &str, mut payload: Value) -> Result<Value> {
        if let Some(obj) = payload.as_object_mut() {
            obj.entry("lastHomeAccessTime")
                .or_insert_with(|| json!(self.last_home_access));
            obj.insert("sm".into(), json!(self.fp.sm()));
        }
        let full = self.request_raw(url, payload).await?;
        if let Some(errs) = full.get("errors").and_then(|e| e.as_array()) {
            if !errs.is_empty() {
                // W3 R3：业务码 → Skip；见 CoreError::from_game_api_errors
                return Err(CoreError::from_game_api_errors(errs));
            }
        }
        if url == "/api/home/get_home_info" {
            self.last_home_access = chrono::Utc::now().timestamp().to_string();
        }
        full.get("payload")
            .cloned()
            .ok_or_else(|| CoreError::Api("missing payload".into()))
    }

    async fn request_raw(&self, url: &str, payload: Value) -> Result<Value> {
        let _g = self.lock.lock().await;
        let t0 = std::time::Instant::now();
        let envelope = json!({
            "payload": payload,
            "uuid": self.uuid,
            "userId": self.user_id,
            "sessionId": self.session_id,
            "actionToken": null,
            "ctag": null,
            "actionTime": action_time(),
        });
        let crypted = crypto::pack_value(&envelope, &self.crypt_key)?;
        let sig = self.gree.sign_game_body(&crypted)?;
        let api_root = self.gree.region.api_root();
        let full_url = format!("{api_root}{url}");

        // 与 constants.DEFAULT_HEADERS 一致：默认日文区；国际服实测可用 EN 指纹 + US API
        let (x_region, x_lang) = match self.gree.region {
            GreeRegion::Japan => ("JP", "ja-Jpan"),
            // 上游 DEFAULT 常为 JP；国际服实测可用 EN 指纹 + US API
            GreeRegion::Global => ("JP", "ja-Jpan"),
        };

        let wire_rec = |status: u16,
                        plain: Option<&Value>,
                        cipher: Option<&[u8]>,
                        err: Option<&str>| {
            if crate::wire::is_active() {
                let ms = t0.elapsed().as_millis() as u64;
                crate::wire::record_game_api_timed(
                    "POST",
                    &full_url,
                    url,
                    &payload,
                    &envelope,
                    &crypted,
                    status,
                    plain,
                    cipher,
                    err,
                    Some(ms),
                );
            }
        };

        let resp = self
            .http
            .post(&full_url)
            .header("content-type", "application/x-msgpack")
            .header("x-timezone-offset", "28800")
            .header("x-language", x_lang)
            .header("x-unity-version", "2022.3.21f1")
            .header("x-region", x_region)
            .header("x-game-server-url", api_root)
            .header("x-post-signature", sig)
            .header(
                "user-agent",
                "UnityRequest   (Asus ASUS_I003DD Android OS 9 / API-28 (PI/rel.cjw.20220518.114133))",
            )
            .body(crypted.clone())
            .send()
            .await
            .map_err(|e| {
                wire_rec(0, None, None, Some(&format!("request send failed: {e}")));
                network_from_reqwest(&e, "游戏API请求")
            })?;

        let status = resp.status().as_u16();
        let bytes = resp.bytes().await.map_err(|e| {
            wire_rec(
                status,
                None,
                None,
                Some(&format!("read body failed: {e}")),
            );
            network_from_reqwest(&e, "游戏API读响应")
        })?;
        if status == 428 {
            // 指纹 version/sign 与服务器不一致 — 见 ERROR_DIAGNOSTICS HTTP_428_VERSION
            wire_rec(status, None, Some(&bytes), Some("HTTP 428"));
            return Err(CoreError::http_status(428, &bytes, "游戏API"));
        }
        if status == 401 {
            wire_rec(status, None, Some(&bytes), Some("HTTP 401"));
            return Err(CoreError::http_status(401, &bytes, "游戏API会话"));
        }
        if status != 200 {
            wire_rec(
                status,
                None,
                Some(&bytes),
                Some(&format!("HTTP {status}")),
            );
            return Err(CoreError::http_status(status, &bytes, &format!("游戏API {url}")));
        }
        match crypto::unpack_value(&bytes, &self.crypt_key) {
            Ok(val) => {
                wire_rec(status, Some(&val), Some(&bytes), None);
                Ok(val)
            }
            Err(e) => {
                // 解密失败时附带提示：可能实为网关 HTML 被当密文
                wire_rec(status, None, Some(&bytes), Some(&format!("decrypt: {e}")));
                match e {
                    CoreError::Crypto(msg) if msg.to_lowercase().contains("unpad") => {
                        Err(CoreError::Crypto(format!(
                            "Unpad Error（解密失败）。可能：指纹/渠道错误，或响应不是游戏密文。url={url} raw={msg}"
                        )))
                    }
                    other => Err(other),
                }
            }
        }
    }

    pub fn user_name_level(&self) -> (String, i64) {
        let name = self
            .init_data
            .pointer("/userParamData/name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let level = self
            .init_data
            .pointer("/userParamData/level")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        (name, level)
    }

    /// 从 full_login 的 `init_data.partyDataList` 提取队伍摘要（PARTY-SELECT / C10）。
    /// light_login 可能没有完整列表 → 返回空。
    /// Docs: `docs/PLAN_PARTY_SELECT_UX.md` · `docs/tech/PARTY_TEAM_RESOLVE.md` · `docs/tech/INIT_AND_RESPONSE_PAYLOADS.md`
    pub fn party_summaries(&self) -> Vec<Value> {
        party_summaries_from_init(&self.init_data)
    }

    /// 当前体力（含按配置的自然回复估算，对齐 pcrclient.stamina）
    /// 对照: archive/.../core/pcrclient.py · docs/tech/MODULES_RUNTIME.md

    pub fn stamina(&self) -> i64 {
        let raw = self
            .init_data
            .pointer("/userParamData/stamina")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let limit = self
            .game_config
            .pointer("/userConfig/staminaUpperLimit")
            .and_then(|v| v.as_i64())
            .unwrap_or(9999);
        if raw >= limit {
            return raw;
        }
        let recover_sec = self
            .game_config
            .pointer("/userConfig/staminaRecoverSec")
            .and_then(|v| v.as_i64())
            .unwrap_or(300);
        if recover_sec <= 0 {
            return raw;
        }
        let updated = self
            .init_data
            .pointer("/userParamData/staminaUpdatedTime")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .or_else(|| {
                self.init_data
                    .pointer("/userParamData/staminaUpdatedTime")
                    .and_then(|v| v.as_str())
                    .and_then(|s| {
                        chrono::DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f%z").ok()
                    })
            });
        let Some(updated) = updated else {
            return raw;
        };
        let now = chrono::Utc::now();
        let delta = now.signed_duration_since(updated.with_timezone(&chrono::Utc));
        let recover_times = delta.num_seconds() / recover_sec;
        (raw + recover_times).min(limit)
    }

    /// 本地扣体力（扫荡成功后避免重复拉参；最终以 refresh 为准）

    pub fn apply_stamina_delta(&mut self, delta: i64) {
        if let Some(v) = self.init_data.pointer_mut("/userParamData/stamina") {
            if let Some(n) = v.as_i64() {
                *v = json!((n + delta).max(0));
            }
        }
    }

    /// 团战 LP 估算（对齐 pcrclient.raid_stamina；无配置时退回 raw）

    pub fn raid_stamina(&self, multi_raid_user: &Value) -> i64 {
        let raw = multi_raid_user
            .get("stamina")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let max_stamina = self
            .game_config
            .pointer("/multiRaidConfig/maxStamina")
            .and_then(|v| v.as_i64())
            .unwrap_or(raw);
        if raw >= max_stamina {
            return raw;
        }
        let recover_sec = self
            .game_config
            .pointer("/multiRaidConfig/staminaRecoverSec")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let recover_num = self
            .game_config
            .pointer("/multiRaidConfig/staminaRecoverNum")
            .and_then(|v| v.as_i64())
            .unwrap_or(1);
        if recover_sec <= 0 {
            return raw;
        }
        let updated = multi_raid_user
            .get("staminaUpdatedTime")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
        let Some(updated) = updated else {
            return raw;
        };
        let now = chrono::Utc::now();
        let delta = now.signed_duration_since(updated.with_timezone(&chrono::Utc));
        let v40 = delta.num_seconds() / recover_sec;
        let need = (max_stamina - raw).max(0);
        let v42 = if recover_num > 0 {
            ((need + recover_num - 1) / recover_num).min(v40)
        } else {
            0
        };
        raw + v42 * recover_num
    }
}

/// 从 init_data JSON 提取队伍摘要（可单测、可无 GameClient 调用）。
/// Docs: `docs/PLAN_PARTY_SELECT_UX.md` · `docs/tech/PARTY_TEAM_RESOLVE.md`
pub fn party_summaries_from_init(init_data: &Value) -> Vec<Value> {
    let list = match init_data.get("partyDataList") {
        Some(Value::Array(a)) => a,
        _ => return Vec::new(),
    };
    let mut out = Vec::new();
    for p in list {
        let id = p
            .get("partyDataId")
            .and_then(|v| v.as_i64())
            .or_else(|| p.get("partyDataId").and_then(|v| v.as_u64()).map(|u| u as i64))
            .unwrap_or(0);
        let index = p
            .get("partyIndex")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let name = p
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let party_type = p
            .get("partyType")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        // 跳过完全空壳（无 id 且无名称）
        if id == 0 && name.is_empty() {
            continue;
        }
        out.push(json!({
            "partyDataId": id,
            "partyIndex": index,
            "name": name,
            "partyType": party_type,
            "label": format!("序号{index} · {name} · id={id}"),
        }));
    }
    out
}

#[cfg(test)]
mod party_summary_tests {
    use super::*;

    #[test]
    fn party_summaries_parse_basic() {
        let init = json!({
            "partyDataList": [
                {"partyDataId": 100, "partyIndex": 1, "name": "日常队", "partyType": 1},
                {"partyDataId": 0, "partyIndex": 2, "name": "", "partyType": 0},
            ]
        });
        let s = party_summaries_from_init(&init);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0]["name"], "日常队");
        assert_eq!(s[0]["partyDataId"], 100);
    }
}
