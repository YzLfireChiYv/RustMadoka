//! 用户组与游戏账号持久化（产品模型，非原版 qid 文件夹的逐字段克隆）。
//!
//! # 职责
//! - **权威读写**：`users/{组名}.json`（schema 2 信封；可选 AES vault）
//! - **layout2 旁路（save 时镜像）**：
//!   - `accounts/{card_id}/identity.json` — 明文组的引继+密码（同 device 键=引继）
//!   - `groups/{组}/meta.json` — 成员 alias→card_id、组队配置
//!   - `groups/{组}/cards/{别名}/settings.json` — 无凭证模块设置（可拷同步）
//!   - `groups/{组}/settings/shared.json` — 纯设置候选
//! - **加密组**：不写明文 identity；凭证仅在 users 信封
//! - 渠道枚举：jp / en / tw（tw 登录未实现）
//!
//! # 文档
//! - `docs/tech/DATA_FOLDER_LAYOUT.md` §0.1 · §1.1
//! - `docs/PLAN_R3_ACCOUNT_CLI_UX.md` · `docs/tech/SECURITY_AND_PRIVACY_AUDIT.md`
//! - 对照：`archive/.../module/accountmgr.py`
//!
//! # 安全
//! - 引继/密码不进 git（P8）；明文组磁盘明文属产品选择（P9）

use crate::error::{CoreError, Result as CoreResult};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const KDF_ITERS: u32 = 120_000;
const VAULT_MAGIC: &str = "automadoka-vault-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    En,
    Jp,
    /// 台服（Sonet SDK；登录实现与 Gree 不同，见 SDK_AND_LOGIN）
    Tw,
}

impl Channel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Jp => "jp",
            Self::Tw => "tw",
        }
    }
    pub fn from_user(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "jp" | "日服" | "bsdk" | "japan" => Self::Jp,
            "tw" | "台服" | "sonet" | "taiwan" => Self::Tw,
            _ => Self::En,
        }
    }
    pub fn display(&self) -> &'static str {
        match self {
            Self::En => "国际服",
            Self::Jp => "日服",
            Self::Tw => "台服",
        }
    }
    /// 是否已实现 SDK 登录（台服 Sonet 尚未移植）

    pub fn login_implemented(&self) -> bool {
        !matches!(self, Self::Tw)
    }
}

/// 游戏账号（内存中始终为解密后的明文形态）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameAccount {
    pub alias: String,
    /// 引继码

    pub username: String,
    pub password: String,
    pub channel: String,
    /// 上次成功获取信息后的昵称

    #[serde(default)]
    pub game_name: String,
    #[serde(default)]
    pub level: i64,
    /// 上次成功 info 的 ISO 时间

    #[serde(default)]
    pub info_fetched_at: Option<String>,
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

/// 单份组队配置（卡片；用户组内可有多份）
/// Docs: `docs/PLAN_GROUP_RAID_UI.md` · GROUP_RAID §8.1
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GroupRaidConfigEntry {
    /// 稳定 id（UUID 或 gr_n）
    #[serde(default)]
    pub id: String,
    /// 卡片显示名
    #[serde(default)]
    pub name: String,
    /// 参与别名（运行时与现有卡片取交；被删自动降级）
    #[serde(default)]
    pub aliases: Vec<String>,
    /// guild / friend / all / self
    #[serde(default)]
    pub room_open: String,
    /// 统一队伍码（名或 id）；空则各卡用默认
    #[serde(default)]
    pub party: String,
    #[serde(default)]
    pub leave_after_support: bool,
}

/// 用户组内组队 Raid **多配置**（落盘）
///
/// 旧版单例字段（aliases/room_open/…）反序列化时自动迁入 `entries` 一条。
/// Docs: `docs/tech/GROUP_RAID_AND_DEVICE_IDENTITY.md` §8 · PLAN_GROUP_RAID_UI
#[derive(Debug, Clone, Serialize, Default)]
pub struct GroupRaidPanelConfig {
    #[serde(default)]
    pub entries: Vec<GroupRaidConfigEntry>,
}

impl<'de> Deserialize<'de> for GroupRaidPanelConfig {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(default)]
            entries: Vec<GroupRaidConfigEntry>,
            /// 兼容早期命名
            #[serde(default)]
            configs: Vec<GroupRaidConfigEntry>,
            // —— 旧版单例字段 ——
            #[serde(default)]
            aliases: Vec<String>,
            #[serde(default)]
            room_open: String,
            #[serde(default)]
            party: String,
            #[serde(default)]
            leave_after_support: bool,
        }
        let raw = Raw::deserialize(deserializer)?;
        let mut entries = if !raw.entries.is_empty() {
            raw.entries
        } else if !raw.configs.is_empty() {
            raw.configs
        } else if !raw.aliases.is_empty()
            || !raw.room_open.trim().is_empty()
            || !raw.party.trim().is_empty()
            || raw.leave_after_support
        {
            vec![GroupRaidConfigEntry {
                id: "legacy".into(),
                name: "默认组队".into(),
                aliases: raw.aliases,
                room_open: raw.room_open,
                party: raw.party,
                leave_after_support: raw.leave_after_support,
            }]
        } else {
            vec![]
        };
        for (i, e) in entries.iter_mut().enumerate() {
            if e.id.trim().is_empty() {
                e.id = format!("gr_{}", i + 1);
            }
            if e.name.trim().is_empty() {
                e.name = format!("组队配置 {}", i + 1);
            }
        }
        Ok(GroupRaidPanelConfig { entries })
    }
}

impl GroupRaidPanelConfig {
    pub fn find(&self, id: &str) -> Option<&GroupRaidConfigEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn upsert(&mut self, entry: GroupRaidConfigEntry) {
        if let Some(slot) = self.entries.iter_mut().find(|e| e.id == entry.id) {
            *slot = entry;
        } else {
            self.entries.push(entry);
        }
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let n = self.entries.len();
        self.entries.retain(|e| e.id != id);
        self.entries.len() < n
    }
}

/// 用户组账号（内存解密态）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserGroup {
    pub name: String,
    /// 是否设置了用户组密码（加密开关）

    #[serde(default)]
    pub has_password: bool,
    /// 仅内存：登录时填入，不落盘明文

    #[serde(skip)]
    pub session_password: Option<String>,
    #[serde(default)]
    pub last_login_at: Option<String>,
    #[serde(default)]
    pub accounts: Vec<GameAccount>,
    /// 组队 Raid 面板配置
    #[serde(default)]
    pub group_raid: GroupRaidPanelConfig,
}

/// 列表用公开元数据（无需密码）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupListItem {
    pub name: String,
    pub has_password: bool,
    pub last_login_at: Option<String>,
    pub account_count: usize,
    pub aliases: Vec<String>,
}

/// 旧版兼容
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LegacyToolUser {
    #[serde(default)]
    name: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    accounts: Vec<GameAccount>,
}

/// 落盘信封
#[derive(Debug, Serialize, Deserialize)]
struct StoredGroup {
    #[serde(default = "schema_v2")]
    schema: u32,
    name: String,
    #[serde(default)]
    has_password: bool,
    #[serde(default)]
    last_login_at: Option<String>,
    /// 明文模式

    #[serde(default)]
    accounts: Option<Vec<GameAccount>>,
    /// 加密模式

    #[serde(default)]
    salt_b64: Option<String>,
    #[serde(default)]
    nonce_b64: Option<String>,
    #[serde(default)]
    ciphertext_b64: Option<String>,
    /// 无需解密即可展示的别名列表

    #[serde(default)]
    public_aliases: Vec<PublicAlias>,
    /// 组队面板（明文落盘；加密组也在信封外，无敏感字段）
    #[serde(default)]
    group_raid: Option<GroupRaidPanelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PublicAlias {
    alias: String,
    channel: String,
    #[serde(default)]
    info_fetched_at: Option<String>,
}

fn schema_v2() -> u32 {
    2
}

#[derive(Serialize, Deserialize)]
struct VaultPayload {
    magic: String,
    accounts: Vec<GameAccount>,
}

fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, KDF_ITERS, &mut key);
    key
}

fn encrypt_accounts(password: &str, accounts: &[GameAccount]) -> CoreResult<(String, String, String)> {
    let mut salt = [0u8; 16];
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let key = derive_key(password, &salt);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| CoreError::other(e.to_string()))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let payload = VaultPayload {
        magic: VAULT_MAGIC.into(),
        accounts: accounts.to_vec(),
    };
    let plain = serde_json::to_vec(&payload)?;
    let ct = cipher
        .encrypt(nonce, plain.as_ref())
        .map_err(|e| CoreError::other(format!("encrypt: {e}")))?;
    Ok((
        B64.encode(salt),
        B64.encode(nonce_bytes),
        B64.encode(ct),
    ))
}

fn decrypt_accounts(
    password: &str,
    salt_b64: &str,
    nonce_b64: &str,
    ct_b64: &str,
) -> CoreResult<Vec<GameAccount>> {
    let salt = B64
        .decode(salt_b64)
        .map_err(|e| CoreError::other(e.to_string()))?;
    let nonce_bytes = B64
        .decode(nonce_b64)
        .map_err(|e| CoreError::other(e.to_string()))?;
    let ct = B64
        .decode(ct_b64)
        .map_err(|e| CoreError::other(e.to_string()))?;
    if nonce_bytes.len() != 12 {
        return Err(CoreError::other("bad nonce len"));
    }
    let key = derive_key(password, &salt);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| CoreError::other(e.to_string()))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plain = cipher
        .decrypt(nonce, ct.as_ref())
        .map_err(|_| CoreError::other("用户组密码错误或数据损坏"))?;
    let payload: VaultPayload = serde_json::from_slice(&plain)?;
    if payload.magic != VAULT_MAGIC {
        return Err(CoreError::other("vault magic mismatch"));
    }
    Ok(payload.accounts)
}

fn public_aliases_from(accounts: &[GameAccount]) -> Vec<PublicAlias> {
    accounts
        .iter()
        .map(|a| PublicAlias {
            alias: a.alias.clone(),
            channel: a.channel.clone(),
            info_fetched_at: a.info_fetched_at.clone(),
        })
        .collect()
}

pub struct Store {
    /// 数据文件夹根（RustMadoka_data）
    data_dir: PathBuf,
    /// 兼容权威：`users/{组}.json`
    root: PathBuf,
}

impl Store {
    pub fn open(data_dir: &Path) -> std::io::Result<Self> {
        let root = data_dir.join("users");
        std::fs::create_dir_all(&root)?;
        std::fs::create_dir_all(data_dir.join("accounts"))?;
        std::fs::create_dir_all(data_dir.join("groups"))?;
        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            root,
        })
    }

    /// 数据文件夹根路径
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    fn safe_segment(name: &str) -> String {
        let mut s: String = name
            .chars()
            .map(|c| {
                if c.is_control()
                    || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
                {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        if s.is_empty() {
            s = "_".into();
        }
        s
    }

    fn user_path(&self, name: &str) -> PathBuf {
        self.root.join(format!("{}.json", Self::safe_segment(name)))
    }

    /// 游戏账号卡片 id：渠道 + 引继安全名（与 device_id 卡片一致，跨组同一卡同一 id）
    pub fn card_id_for(channel: &str, migration_code: &str) -> String {
        let ch = Channel::from_user(channel).as_str();
        format!("{ch}_{}", Self::safe_segment(migration_code))
    }

    fn account_identity_path(&self, card_id: &str) -> PathBuf {
        self.data_dir
            .join("accounts")
            .join(Self::safe_segment(card_id))
            .join("identity.json")
    }

    fn group_meta_path(&self, group: &str) -> PathBuf {
        self.data_dir
            .join("groups")
            .join(Self::safe_segment(group))
            .join("meta.json")
    }

    fn group_card_settings_path(&self, group: &str, alias: &str) -> PathBuf {
        self.data_dir
            .join("groups")
            .join(Self::safe_segment(group))
            .join("cards")
            .join(Self::safe_segment(alias))
            .join("settings.json")
    }

    fn group_card_link_path(&self, group: &str, alias: &str) -> PathBuf {
        self.data_dir
            .join("groups")
            .join(Self::safe_segment(group))
            .join("cards")
            .join(Self::safe_segment(alias))
            .join("link.json")
    }

    fn group_shared_settings_path(&self, group: &str) -> PathBuf {
        self.data_dir
            .join("groups")
            .join(Self::safe_segment(group))
            .join("settings")
            .join("shared.json")
    }

    fn write_json_atomic(path: &Path, value: &serde_json::Value) -> CoreResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(value)?)?;
        if std::fs::rename(&tmp, path).is_err() {
            std::fs::copy(&tmp, path)?;
            let _ = std::fs::remove_file(&tmp);
        }
        Ok(())
    }

    /// layout2 旁路：账号身份 + 组 meta + 卡片设置（无引继进 settings）。
    ///
    /// **产品安全（主人钉死 · 家用）：** 加密用户组 = 小孩打不开该组、拿不到组内账号密码。
    /// 因此加密组**不**在磁盘另写明文 `identity.json`（否则小孩翻文件夹就看见密码）。
    /// 无密码用户组：可写 identity 方便拷贝/对照。
    /// **同角色同登录：** 与是否加密组无关——游戏会话/device_id 键永远是 **渠道+引继**，
    /// 同一引继在加密组与明文组之间共用会话池与设备 id（便利优先，不搞多余安全军备）。
    /// Docs: `docs/tech/DATA_FOLDER_LAYOUT.md` §1.1 · NORMS 家用安全口径
    fn mirror_layout2(&self, group: &UserGroup) -> CoreResult<()> {
        let mut members = Vec::new();
        let mut shared_map: HashMap<String, serde_json::Value> = HashMap::new();

        for acc in &group.accounts {
            let card_id = Self::card_id_for(&acc.channel, &acc.username);
            members.push(serde_json::json!({
                "alias": acc.alias,
                "card_id": card_id,
                "channel": Channel::from_user(&acc.channel).as_str(),
            }));

            // 仅无密码组写明文 identity；加密组凭证只在 users 信封（防小孩翻盘）
            if !group.has_password {
                let id_path = self.account_identity_path(&card_id);
                let id_doc = serde_json::json!({
                    "schema": 1,
                    "kind": "account_identity",
                    "card_id": card_id,
                    "channel": Channel::from_user(&acc.channel).as_str(),
                    "username": acc.username,
                    "password": acc.password,
                    "game_name": acc.game_name,
                    "level": acc.level,
                    "info_fetched_at": acc.info_fetched_at,
                    "note": "引继+密码。device_id 见 cache/device_by_account。同引继=同一游戏登录身份。",
                });
                Self::write_json_atomic(&id_path, &id_doc)?;
            }

            let settings_path = self.group_card_settings_path(&group.name, &acc.alias);
            let settings_doc = serde_json::json!({
                "schema": 1,
                "kind": "card_settings",
                "group": group.name,
                "alias": acc.alias,
                "card_id": card_id,
                "note": "无引继/密码；可复制到另一别名目录同步配置",
                "config": acc.config,
            });
            Self::write_json_atomic(&settings_path, &settings_doc)?;

            let link_path = self.group_card_link_path(&group.name, &acc.alias);
            let link_doc = serde_json::json!({
                "schema": 1,
                "alias": acc.alias,
                "card_id": card_id,
                "channel": Channel::from_user(&acc.channel).as_str(),
            });
            Self::write_json_atomic(&link_path, &link_doc)?;

            // shared 候选：模块开关类键
            for (k, v) in &acc.config {
                if is_layout2_shared_key(k) {
                    shared_map.insert(k.clone(), v.clone());
                }
            }
        }

        let shared_path = self.group_shared_settings_path(&group.name);
        let shared_doc = serde_json::json!({
            "schema": 1,
            "kind": "shared_settings",
            "group": group.name,
            "note": "纯设置（无别名/无引继）。可整文件复制到另一组 settings/shared.json",
            "config": shared_map,
        });
        Self::write_json_atomic(&shared_path, &shared_doc)?;

        let meta_path = self.group_meta_path(&group.name);
        let meta_doc = serde_json::json!({
            "schema": 1,
            "kind": "group_meta",
            "name": group.name,
            "has_password": group.has_password,
            "last_login_at": group.last_login_at,
            "members": members,
            "group_raid": group.group_raid,
            "note": "无引继/密码正文；加密组凭证仅在 users/*.json 信封。权威读写仍经 Store::load/save_group。",
        });
        Self::write_json_atomic(&meta_path, &meta_doc)?;
        Ok(())
    }

    pub fn list_groups(&self) -> std::io::Result<Vec<GroupListItem>> {
        let mut v = Vec::new();
        for e in std::fs::read_dir(&self.root)? {
            let e = e?;
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            let meta = self.group_meta(&name).unwrap_or(GroupListItem {
                name: name.clone(),
                has_password: false,
                last_login_at: None,
                account_count: 0,
                aliases: vec![],
            });
            v.push(meta);
        }
        v.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(v)
    }

    pub fn group_meta(&self, name: &str) -> std::io::Result<GroupListItem> {
        let p = self.user_path(name);
        if !p.is_file() {
            return Ok(GroupListItem {
                name: name.into(),
                has_password: false,
                last_login_at: None,
                account_count: 0,
                aliases: vec![],
            });
        }
        let t = std::fs::read_to_string(&p)?;
        // 新格式
        if let Ok(stored) = serde_json::from_str::<StoredGroup>(&t) {
            let aliases: Vec<String> = if !stored.public_aliases.is_empty() {
                stored.public_aliases.iter().map(|a| a.alias.clone()).collect()
            } else if let Some(acc) = &stored.accounts {
                acc.iter().map(|a| a.alias.clone()).collect()
            } else {
                vec![]
            };
            return Ok(GroupListItem {
                name: stored.name,
                has_password: stored.has_password,
                last_login_at: stored.last_login_at,
                account_count: aliases.len(),
                aliases,
            });
        }
        // 旧 ToolUser
        if let Ok(legacy) = serde_json::from_str::<LegacyToolUser>(&t) {
            let aliases: Vec<_> = legacy.accounts.iter().map(|a| a.alias.clone()).collect();
            return Ok(GroupListItem {
                name: if legacy.name.is_empty() {
                    name.into()
                } else {
                    legacy.name
                },
                has_password: !legacy.password.is_empty(),
                last_login_at: None,
                account_count: aliases.len(),
                aliases,
            });
        }
        Ok(GroupListItem {
            name: name.into(),
            has_password: false,
            last_login_at: None,
            account_count: 0,
            aliases: vec![],
        })
    }

    /// 磁盘上是否存在该用户组文件（公开列表/路由校验用）。
    pub fn group_exists(&self, name: &str) -> bool {
        self.user_path(name).is_file()
    }

    /// 加载并解密用户组。`password`：无密组传 None 或 ""；有密组必填。
    ///
    /// **不存在的组名返回错误**（禁止静默当成空组登录，见 PLAN_UI_ROUTING 错 path 回退 · C23）。
    pub fn load_group(&self, name: &str, password: Option<&str>) -> CoreResult<UserGroup> {
        let p = self.user_path(name);
        if !p.is_file() {
            return Err(CoreError::other(format!("用户组不存在：{name}")));
        }
        let t = std::fs::read_to_string(&p).map_err(CoreError::from)?;

        if let Ok(stored) = serde_json::from_str::<StoredGroup>(&t) {
            return self.open_stored(stored, password);
        }
        // 旧版
        let legacy: LegacyToolUser =
            serde_json::from_str(&t).map_err(|e| CoreError::other(e.to_string()))?;
        let mut g = UserGroup {
            name: if legacy.name.is_empty() {
                name.into()
            } else {
                legacy.name
            },
            has_password: !legacy.password.is_empty(),
            session_password: None,
            last_login_at: None,
            accounts: legacy.accounts,
            group_raid: GroupRaidPanelConfig::default(),
        };
        if g.has_password {
            let pw = password.unwrap_or("");
            if pw != legacy.password {
                return Err(CoreError::other("用户组密码错误"));
            }
            g.session_password = Some(pw.to_string());
            // 迁移：下次 save 写成加密信封
        }
        Ok(g)
    }

    fn open_stored(&self, stored: StoredGroup, password: Option<&str>) -> CoreResult<UserGroup> {
        if !stored.has_password {
            let accounts = stored.accounts.unwrap_or_default();
            return Ok(UserGroup {
                name: stored.name,
                has_password: false,
                session_password: None,
                last_login_at: stored.last_login_at,
                accounts,
                group_raid: stored.group_raid.unwrap_or_default(),
            });
        }
        let pw = password.unwrap_or("");
        if pw.is_empty() {
            return Err(CoreError::other("该用户组需要密码"));
        }
        let salt = stored
            .salt_b64
            .as_deref()
            .ok_or_else(|| CoreError::other("encrypted group missing salt"))?;
        let nonce = stored
            .nonce_b64
            .as_deref()
            .ok_or_else(|| CoreError::other("encrypted group missing nonce"))?;
        let ct = stored
            .ciphertext_b64
            .as_deref()
            .ok_or_else(|| CoreError::other("encrypted group missing ciphertext"))?;
        let accounts = decrypt_accounts(pw, salt, nonce, ct)?;
        Ok(UserGroup {
            name: stored.name,
            has_password: true,
            session_password: Some(pw.to_string()),
            last_login_at: stored.last_login_at,
            accounts,
            group_raid: stored.group_raid.unwrap_or_default(),
        })
    }

    pub fn save_group(&self, group: &UserGroup) -> CoreResult<()> {
        let p = self.user_path(&group.name);
        let public_aliases = public_aliases_from(&group.accounts);
        let stored = if group.has_password {
            let pw = group
                .session_password
                .as_deref()
                .ok_or_else(|| CoreError::other("保存加密用户组需要 session 密码"))?;
            let (salt, nonce, ct) = encrypt_accounts(pw, &group.accounts)?;
            StoredGroup {
                schema: 2,
                name: group.name.clone(),
                has_password: true,
                last_login_at: group.last_login_at.clone(),
                accounts: None,
                salt_b64: Some(salt),
                nonce_b64: Some(nonce),
                ciphertext_b64: Some(ct),
                public_aliases,
                group_raid: Some(group.group_raid.clone()),
            }
        } else {
            StoredGroup {
                schema: 2,
                name: group.name.clone(),
                has_password: false,
                last_login_at: group.last_login_at.clone(),
                accounts: Some(group.accounts.clone()),
                salt_b64: None,
                nonce_b64: None,
                ciphertext_b64: None,
                public_aliases,
                group_raid: Some(group.group_raid.clone()),
            }
        };
        let text = serde_json::to_string_pretty(&stored)?;
        // 先写权威 users/，再旁路 layout2（失败不回滚 users，避免丢号；记错误）
        std::fs::write(&p, text)?;
        if let Err(e) = self.mirror_layout2(group) {
            tracing::warn!(
                group = %group.name,
                error = %e,
                "layout2 mirror after save_group failed (users/*.json already saved)"
            );
        }
        Ok(())
    }

    /// 删除用户组：去掉 `users/{组}.json`，并顺带删掉该组 layout2 目录与任务日志目录。
    ///
    /// **不**删除 `accounts/{card_id}/` 身份文件——同一角色可能仍挂在其它用户组
    /// （同引继=同登录身份）。主人口径：删组够清日志/组设置即可。
    /// Docs: DATA_FOLDER_LAYOUT · 主人 2026-08-08 家用便利
    pub fn delete_group(&self, name: &str) -> std::io::Result<()> {
        // 先读成员，便于日志说明（失败也不挡删）
        let _ = self.group_meta(name);

        let p = self.user_path(name);
        if p.is_file() {
            std::fs::remove_file(&p)?;
        }

        // groups/{组}/ 整树（settings、cards、meta）
        let gdir = self
            .data_dir
            .join("groups")
            .join(Self::safe_segment(name));
        if gdir.is_dir() {
            let _ = std::fs::remove_dir_all(&gdir);
        }

        // task_logs：目录名 = safe_seg(组名)，与 task_log.rs 一致（字母数字/_/- 保留，其它变 % + 哈希）
        let log_seg = task_log_safe_seg(name);
        let log_dir = self.data_dir.join("task_logs").join(&log_seg);
        if log_dir.is_dir() {
            let _ = std::fs::remove_dir_all(&log_dir);
        }

        Ok(())
    }

    pub fn create_group(&self, name: &str, password: Option<&str>) -> CoreResult<UserGroup> {
        if name.trim().is_empty() {
            return Err(CoreError::other("用户组名不能为空"));
        }
        let p = self.user_path(name);
        if p.is_file() {
            return Err(CoreError::other("用户组已存在"));
        }
        let has = password.map(|s| !s.is_empty()).unwrap_or(false);
        let g = UserGroup {
            name: name.to_string(),
            has_password: has,
            session_password: if has {
                Some(password.unwrap().to_string())
            } else {
                None
            },
            last_login_at: None,
            accounts: vec![],
            group_raid: GroupRaidPanelConfig::default(),
        };
        self.save_group(&g)?;
        Ok(g)
    }

    /// 设置/修改用户组密码（空密码 = 清除加密改回明文）

    pub fn set_group_password(
        &self,
        name: &str,
        old_password: Option<&str>,
        new_password: Option<&str>,
    ) -> CoreResult<UserGroup> {
        let mut g = self.load_group(name, old_password)?;
        let new_pw = new_password.unwrap_or("");
        if new_pw.is_empty() {
            g.has_password = false;
            g.session_password = None;
        } else {
            g.has_password = true;
            g.session_password = Some(new_pw.to_string());
        }
        self.save_group(&g)?;
        Ok(g)
    }

    pub fn touch_login(&self, group: &mut UserGroup) -> CoreResult<()> {
        group.last_login_at = Some(chrono::Utc::now().to_rfc3339());
        self.save_group(group)
    }
}

/// 与 `task_log::safe_seg` 对齐的组目录名（core 不依赖 app，复制算法）
fn task_log_safe_seg(s: &str) -> String {
    let body: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c == ' ' {
                '_'
            } else {
                '%'
            }
        })
        .collect();
    let mut h = 2166136261u32;
    for b in s.as_bytes() {
        h ^= *b as u32;
        h = h.wrapping_mul(16777619);
    }
    format!("{body}_{h:x}")
}

/// layout2 shared.json 候选键（模块开关/确认/商店等；队伍名与关卡 ID 不入 shared）
fn is_layout2_shared_key(key: &str) -> bool {
    if key.starts_with("confirm_") {
        return true;
    }
    if key.ends_with("_shop") || key.contains("shop_priority") || key.contains("priority_") {
        return true;
    }
    matches!(
        key,
        "loginbonus"
            | "stamina_buy"
            | "super_sweep"
            | "raid_reward"
            | "self_raid"
            | "support_raid"
            | "like_raid"
            | "solo_raid"
            | "high_score"
            | "arena"
            | "basic"
            | "event"
            | "archive"
            | "event_shop"
            | "raid_shop"
            | "arena_shop"
            | "tower"
            | "heart"
            | "gather"
            | "freegacha"
            | "eventscenario"
            | "collection"
            | "battle_mission"
            | "mission"
            | "present"
            | "info"
            | "stamina_buy_count"
            | "basic_stamina_5star"
            | "basic_stamina_4star"
            | "basic_stamina_3star"
            | "log_auto_clean"
            | "log_keep_one_click"
            | "confirm_one_click_settings"
            | "confirm_one_click_home"
            | "confirm_one_click_daily"
    )
}

// ─── 类型别名：兼容旧代码导出名 ───
pub type ToolUser = UserGroup;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn encrypt_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "automadoka-acct-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::open(&dir).unwrap();
        let mut g = store.create_group("加密组", Some("secret")).unwrap();
        g.accounts.push(GameAccount {
            alias: "主号".into(),
            username: "CODE123".into(),
            password: "pw".into(),
            channel: "jp".into(),
            game_name: "测试娘".into(),
            level: 42,
            info_fetched_at: Some("2026-08-06T00:00:00Z".into()),
            config: HashMap::new(),
        });
        store.save_group(&g).unwrap();

        let bad = store.load_group("加密组", Some("wrong"));
        assert!(bad.is_err());

        let ok = store.load_group("加密组", Some("secret")).unwrap();
        assert_eq!(ok.accounts.len(), 1);
        assert_eq!(ok.accounts[0].alias, "主号");
        assert_eq!(ok.accounts[0].username, "CODE123");
        assert_eq!(ok.accounts[0].game_name, "测试娘");
        assert_eq!(ok.accounts[0].level, 42);

        let meta = store.group_meta("加密组").unwrap();
        assert!(meta.has_password);
        assert_eq!(meta.aliases, vec!["主号".to_string()]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn group_raid_legacy_flat_migrates_to_entries() {
        let raw = r#"{
            "aliases": ["a", "b"],
            "room_open": "guild",
            "party": "30",
            "leave_after_support": false
        }"#;
        let cfg: GroupRaidPanelConfig = serde_json::from_str(raw).unwrap();
        assert_eq!(cfg.entries.len(), 1);
        assert_eq!(cfg.entries[0].aliases, vec!["a", "b"]);
        assert_eq!(cfg.entries[0].room_open, "guild");
        assert!(!cfg.entries[0].id.is_empty());
    }

    #[test]
    fn group_raid_entries_roundtrip() {
        let cfg = GroupRaidPanelConfig {
            entries: vec![GroupRaidConfigEntry {
                id: "gr_1".into(),
                name: "三号队".into(),
                aliases: vec!["en_w1".into()],
                room_open: "self".into(),
                party: "".into(),
                leave_after_support: false,
            }],
        };
        let s = serde_json::to_string(&cfg).unwrap();
        let back: GroupRaidPanelConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(back.entries.len(), 1);
        assert_eq!(back.entries[0].name, "三号队");
        assert_eq!(back.entries[0].id, "gr_1");
    }

    #[test]
    fn layout2_mirror_plain_group_writes_identity_and_settings() {
        let dir = std::env::temp_dir().join(format!(
            "rm-layout2-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::open(&dir).unwrap();
        let mut g = store.create_group("明文组", None).unwrap();
        let mut cfg = HashMap::new();
        cfg.insert("loginbonus".into(), serde_json::json!(true));
        cfg.insert("force_battle_team".into(), serde_json::json!("1"));
        g.accounts.push(GameAccount {
            alias: "日服主号".into(),
            username: "ABC123MIG".into(),
            password: "gamepw".into(),
            channel: "jp".into(),
            game_name: String::new(),
            level: 0,
            info_fetched_at: None,
            config: cfg,
        });
        store.save_group(&g).unwrap();

        let card_id = Store::card_id_for("jp", "ABC123MIG");
        let id_path = dir
            .join("accounts")
            .join(&card_id)
            .join("identity.json");
        assert!(id_path.is_file(), "identity missing: {}", id_path.display());
        let id_txt = std::fs::read_to_string(&id_path).unwrap();
        assert!(id_txt.contains("ABC123MIG"));
        assert!(id_txt.contains("gamepw"));

        let settings = dir
            .join("groups")
            .join("明文组")
            .join("cards")
            .join("日服主号")
            .join("settings.json");
        assert!(settings.is_file());
        let st = std::fs::read_to_string(&settings).unwrap();
        assert!(st.contains("loginbonus"));
        assert!(!st.contains("gamepw"), "settings must not contain password");

        let shared = dir
            .join("groups")
            .join("明文组")
            .join("settings")
            .join("shared.json");
        assert!(shared.is_file());
        let sh = std::fs::read_to_string(&shared).unwrap();
        assert!(sh.contains("loginbonus"));
        assert!(!sh.contains("force_battle_team"), "team is not shared key");

        // 加密组不写明文 identity
        let mut eg = store.create_group("加密组2", Some("secret")).unwrap();
        eg.accounts.push(GameAccount {
            alias: "x".into(),
            username: "SECRETCODE".into(),
            password: "spw".into(),
            channel: "en".into(),
            game_name: String::new(),
            level: 0,
            info_fetched_at: None,
            config: HashMap::new(),
        });
        store.save_group(&eg).unwrap();
        let secret_id = Store::card_id_for("en", "SECRETCODE");
        let secret_path = dir
            .join("accounts")
            .join(&secret_id)
            .join("identity.json");
        assert!(
            !secret_path.is_file(),
            "encrypted group must not write plaintext identity"
        );
        let meta = dir.join("groups").join("加密组2").join("meta.json");
        assert!(meta.is_file());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_group_removes_users_groups_tree_and_task_logs() {
        let dir = std::env::temp_dir().join(format!(
            "rm-delgrp-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Store::open(&dir).unwrap();
        let mut g = store.create_group("要删的组", None).unwrap();
        g.accounts.push(GameAccount {
            alias: "卡1".into(),
            username: "MIGDEL1".into(),
            password: "p".into(),
            channel: "jp".into(),
            game_name: String::new(),
            level: 0,
            info_fetched_at: None,
            config: HashMap::new(),
        });
        store.save_group(&g).unwrap();

        // 伪造任务日志目录（与 task_log_safe_seg 一致）
        let log_dir = dir
            .join("task_logs")
            .join(task_log_safe_seg("要删的组"))
            .join("dummy");
        std::fs::create_dir_all(&log_dir).unwrap();
        std::fs::write(log_dir.join("x.json"), "{}").unwrap();

        assert!(store.user_path("要删的组").is_file() || store.group_exists("要删的组"));
        store.delete_group("要删的组").unwrap();
        assert!(!store.group_exists("要删的组"));
        assert!(!dir.join("groups").join("要删的组").exists());
        assert!(!dir
            .join("task_logs")
            .join(task_log_safe_seg("要删的组"))
            .exists());
        // 身份文件可保留（同卡可能在其它组）
        let card_id = Store::card_id_for("jp", "MIGDEL1");
        // 仅一组时 identity 仍在——产品：不因删组清 accounts（避免误伤其它组）
        let _ = card_id;
        let _ = std::fs::remove_dir_all(&dir);
    }
}
