//! 客户端包指纹：version / sign / libcount → 请求字段 `sm`。
//!
//! # 职责
//! - 构造 `sm = d{sign}o{libcount}{SM_TAIL}`（与 Python `AppInfo.sm` 字节级一致）
//! - 解析远程 JSON：单 channel 或 `channels` 多服合一（publish / rules 仓）
//! - 从 XAPK 提取 base APK MD5 + arm64 lib 数量；内置 `EMBEDDED_COMBINED_JSON`
//!
//! # 产品钉死（P15 / L4 / C1）
//! - **默认远程源**仅 rules 仓 raw（应用层常量）；本模块不绑 URL
//! - 用户主路径 **不**依赖 APKPure 整包（原版 version.update 的坑）
//!
//! # 文档
//! - `docs/tech/VERSION_FINGERPRINT.md` · `docs/tech/AUTOMADOKA_RESEARCH_AND_RUST_GAP.md` §2.5
//! - 对照：`archive/.../core/version.py`

use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 常量尾缀（与 Python AppInfo.sm 一致）
const SM_TAIL: &str = "1E88A0177575728C9A399A9BD1F43A11D4100065n";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fingerprint {
    pub version: String,
    pub sign: String,
    pub libcount: i64,
    #[serde(default)]
    pub channel: Option<String>,
    /// 可选元数据（发布 JSON 用；运行不强制）

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,
}

impl Fingerprint {
    pub fn sm(&self) -> String {
        format!("d{}o{}{}", self.sign, self.libcount, SM_TAIL)
    }

    pub fn validate(&self) -> Result<()> {
        if self.version.is_empty() {
            return Err(CoreError::Fingerprint("empty version".into()));
        }
        let sign = self.sign.to_lowercase();
        if sign.len() != 32 || !sign.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(CoreError::Fingerprint(format!("bad sign: {}", self.sign)));
        }
        if self.libcount <= 0 {
            return Err(CoreError::Fingerprint("libcount <= 0".into()));
        }
        Ok(())
    }

    pub fn to_version_json(&self) -> serde_json::Value {
        serde_json::json!({
            "version": self.version,
            "sign": self.sign.to_lowercase(),
            "libcount": self.libcount,
        })
    }

    pub fn load_version_json(path: &Path) -> Result<Self> {
        let t = std::fs::read_to_string(path)?;
        let v: serde_json::Value = serde_json::from_str(&t)?;
        Ok(Self {
            version: v["version"].as_str().unwrap_or("").to_string(),
            sign: v["sign"].as_str().unwrap_or("").to_lowercase(),
            libcount: v["libcount"].as_i64().unwrap_or(0),
            channel: v["channel"].as_str().map(|s| s.to_string()),
            package_id: v["package_id"].as_str().map(|s| s.to_string()),
        })
    }

    pub fn save_version_json(&self, path: &Path) -> Result<()> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(path, serde_json::to_string(&self.to_version_json())?)?;
        Ok(())
    }
}

/// 远程文件：单对象或 { channels: { en, jp }, default }
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FingerprintFile {
    Single(Fingerprint),
    Multi {
        #[serde(default)]
        default: Option<String>,
        channels: std::collections::HashMap<String, Fingerprint>,
    },
}

impl FingerprintFile {
    pub fn pick(&self, prefer: &str) -> Result<Fingerprint> {
        match self {
            Self::Single(fp) => {
                let mut f = fp.clone();
                f.sign = f.sign.to_lowercase();
                f.validate()?;
                Ok(f)
            }
            Self::Multi { default, channels } => {
                let key = if channels.contains_key(prefer) {
                    prefer.to_string()
                } else if let Some(d) = default {
                    d.clone()
                } else {
                    channels
                        .keys()
                        .next()
                        .cloned()
                        .ok_or_else(|| CoreError::Fingerprint("empty channels".into()))?
                };
                let mut fp = channels
                    .get(&key)
                    .ok_or_else(|| CoreError::Fingerprint(format!("no channel {key}")))?
                    .clone();
                fp.channel = Some(key);
                fp.sign = fp.sign.to_lowercase();
                fp.validate()?;
                Ok(fp)
            }
        }
    }
}

/// 编译期嵌入的默认合一指纹（`publish/automadoka.json`）
/// 文档: docs/PLAN_NEXT_FOOLPROOF_AND_DIAG.md §8.5 · VERSION_FINGERPRINT.md
pub const EMBEDDED_COMBINED_JSON: &str = include_str!("../../../publish/automadoka.json");

/// 解析指纹 JSON 文本为 FingerprintFile（不合格返回 Fingerprint 错误）
pub fn parse_fingerprint_file_text(text: &str) -> Result<FingerprintFile> {
    let file: FingerprintFile = serde_json::from_str(text)
        .map_err(|e| CoreError::Fingerprint(format!("指纹 JSON 格式错误: {e}")))?;
    // 至少有一个合法 channel（jp/en 或单对象）
    if file.pick("jp").is_err() && file.pick("en").is_err() {
        // 单对象或其它 default
        match &file {
            FingerprintFile::Single(fp) => {
                let mut f = fp.clone();
                f.sign = f.sign.to_lowercase();
                f.validate()?;
            }
            FingerprintFile::Multi { channels, default } => {
                if channels.is_empty() {
                    return Err(CoreError::Fingerprint("channels 为空".into()));
                }
                let key = default
                    .clone()
                    .or_else(|| channels.keys().next().cloned())
                    .unwrap_or_default();
                file.pick(&key)?;
            }
        }
    }
    Ok(file)
}

/// 拉取 URL 的原始指纹 JSON 文本（校验可解析）
pub async fn fetch_fingerprint_text(url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("automadoka-rust/0.1")
        .connect_timeout(std::time::Duration::from_secs(12))
        .timeout(std::time::Duration::from_secs(25))
        .build()
        .map_err(|e| CoreError::Network(e.to_string()))?;
    let text = client
        .get(url)
        .send()
        .await
        .map_err(|e| CoreError::Network(e.to_string()))?
        .error_for_status()
        .map_err(|e| CoreError::Network(e.to_string()))?
        .text()
        .await
        .map_err(|e| CoreError::Network(e.to_string()))?;
    let _ = parse_fingerprint_file_text(&text)?;
    Ok(text)
}

/// 从 URL 拉取指纹（默认 GitHub raw）并 pick channel
pub async fn fetch_fingerprint(url: &str, channel: &str) -> Result<Fingerprint> {
    let text = fetch_fingerprint_text(url).await?;
    let file = parse_fingerprint_file_text(&text)?;
    file.pick(channel)
}

/// 从合一 JSON 文本取某 channel
pub fn fingerprint_from_combined_text(text: &str, channel: &str) -> Result<Fingerprint> {
    parse_fingerprint_file_text(text)?.pick(channel)
}

/// 从合一 JSON 提取 jp/en 版本号摘要
pub fn channel_versions_from_text(text: &str) -> (Option<String>, Option<String>) {
    let Ok(file) = parse_fingerprint_file_text(text) else {
        return (None, None);
    };
    let jp = file.pick("jp").ok().map(|f| f.version);
    let en = file.pick("en").ok().map(|f| f.version);
    (jp, en)
}

/// 注入 sm 到 payload 对象
pub fn apply_sm(payload: &mut serde_json::Value, fp: &Fingerprint) {
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("sm".into(), serde_json::json!(fp.sm()));
    }
}

/// 从本地 XAPK 提取三元组（发布器用）
pub fn extract_from_xapk(path: &Path) -> Result<(Fingerprint, String)> {
    use md5::{Digest, Md5};
    use std::fs::File;
    use std::io::Read;
    use zip::ZipArchive;

    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file).map_err(|e| CoreError::other(e.to_string()))?;
    let mut manifest_s = String::new();
    archive
        .by_name("manifest.json")
        .map_err(|e| CoreError::other(e.to_string()))?
        .read_to_string(&mut manifest_s)?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_s)?;
    let version = manifest["version_name"]
        .as_str()
        .ok_or_else(|| CoreError::Fingerprint("no version_name".into()))?
        .to_string();
    let package_id = manifest["package_name"].as_str().unwrap_or("").to_string();
    let splits = manifest["split_apks"]
        .as_array()
        .ok_or_else(|| CoreError::Fingerprint("no split_apks".into()))?;
    let base_name = splits
        .iter()
        .find(|s| s["id"].as_str() == Some("base"))
        .and_then(|s| s["file"].as_str())
        .ok_or_else(|| CoreError::Fingerprint("no base split".into()))?;
    let lib_name = splits
        .iter()
        .find(|s| s["id"].as_str() == Some("config.arm64_v8a"))
        .and_then(|s| s["file"].as_str())
        .ok_or_else(|| CoreError::Fingerprint("no arm64 split".into()))?;

    let mut base_data = Vec::new();
    archive
        .by_name(base_name)
        .map_err(|e| CoreError::other(e.to_string()))?
        .read_to_end(&mut base_data)?;
    let sign = hex::encode(Md5::digest(&base_data));

    let mut lib_apk = Vec::new();
    archive
        .by_name(lib_name)
        .map_err(|e| CoreError::other(e.to_string()))?
        .read_to_end(&mut lib_apk)?;
    let mut cursor = std::io::Cursor::new(&lib_apk);
    let mut lib_zip =
        ZipArchive::new(&mut cursor).map_err(|e| CoreError::other(e.to_string()))?;
    let mut libcount = 0i64;
    for i in 0..lib_zip.len() {
        let name = lib_zip.by_index(i).map_err(|e| CoreError::other(e.to_string()))?;
        if name.name().starts_with("lib/arm64-v8a/") {
            libcount += 1;
        }
    }

    let channel = if package_id.ends_with(".jp") {
        "jp"
    } else if package_id.ends_with(".en") {
        "en"
    } else if package_id.ends_with(".tw") {
        "tw"
    } else {
        "unknown"
    };

    let fp = Fingerprint {
        version,
        sign,
        libcount,
        channel: Some(channel.into()),
        package_id: Some(package_id.clone()),
    };
    fp.validate()?;
    Ok((fp, package_id))
}

/// 构建可上传 GitHub 的多 channel 指纹文件（en+jp…）
pub fn build_combined_publish_json(
    channels: &[(Fingerprint, String /*package_id*/)],
    default: &str,
) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (fp, pkg) in channels {
        let ch = fp.channel.clone().unwrap_or_else(|| "unknown".into());
        map.insert(
            ch.clone(),
            serde_json::json!({
                "channel": ch,
                "package_id": pkg,
                "version": fp.version,
                "sign": fp.sign.to_lowercase(),
                "libcount": fp.libcount,
            }),
        );
    }
    serde_json::json!({
        "schema": 1,
        "published_at": chrono::Utc::now().format("%Y-%m-%d").to_string(),
        "default": default,
        "note": "Upload as automadoka.json to GitHub raw. Client: fetch_fingerprint(url, channel). Extracted by automadoka extract-xapk.",
        "channels": map,
    })
}
