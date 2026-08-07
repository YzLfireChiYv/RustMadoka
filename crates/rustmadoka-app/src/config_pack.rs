//! Config short-code packs (AM2) for shop / settings share.
//!
//! Docs: `docs/tech/UI_ROUTING_AND_TASK_LOGS.md` · NORMS config pack schema

use rustmadoka_core::safety::{CONFIG_PACK_SCHEMA, UPSTREAM_COMPAT};
use serde_json::{json, Value};
use std::collections::HashMap;

const FORBIDDEN_KEYS: &[&str] = &["username", "password", "migration", "token", "privateKey"];

fn crc8(data: &[u8]) -> u8 {
    let mut crc = 0u8;
    for &b in data {
        crc ^= b;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ 0x07;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

fn b64url_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(data)
}

fn b64url_decode(s: &str) -> Result<Vec<u8>, String> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD
        .decode(s.trim())
        .map_err(|e| format!("b64 decode: {e}"))
}

fn upstream_wire() -> String {
    UPSTREAM_COMPAT.replace('.', "_")
}

fn seal(kind: &str, payload_b64: &str) -> String {
    let core = format!(
        "{kind}.u{}.s{}.{payload_b64}",
        upstream_wire(),
        CONFIG_PACK_SCHEMA
    );
    let c = crc8(core.as_bytes());
    format!("AM2.{core}.{c:02X}")
}

fn open(s: &str) -> Result<(String, String), String> {
    let s = s.trim();
    let rest = s
        .strip_prefix("AM2.")
        .ok_or_else(|| "not AM2 pack".to_string())?;
    let (body, crc_s) = rest
        .rsplit_once('.')
        .ok_or_else(|| "pack missing checksum".to_string())?;
    let expect = u8::from_str_radix(crc_s, 16).map_err(|_| "bad checksum".to_string())?;
    if crc8(body.as_bytes()) != expect {
        return Err("checksum failed".into());
    }
    let parts: Vec<&str> = body.splitn(4, '.').collect();
    if parts.len() != 4 {
        return Err("bad pack format".into());
    }
    let kind = parts[0].to_string();
    let upstream = parts[1]
        .strip_prefix('u')
        .ok_or_else(|| "missing upstream".to_string())?
        .replace('_', ".");
    let schema: u32 = parts[2]
        .strip_prefix('s')
        .ok_or_else(|| "missing schema".to_string())?
        .parse()
        .map_err(|_| "invalid schema".to_string())?;
    if upstream != UPSTREAM_COMPAT {
        return Err("upstream version mismatch".into());
    }
    if schema != CONFIG_PACK_SCHEMA {
        return Err("schema mismatch".into());
    }
    let payload = String::from_utf8(b64url_decode(parts[3])?)
        .map_err(|_| "payload not utf-8".to_string())?;
    Ok((kind, payload))
}

/// Encode shop-related config keys into a share code.
pub fn encode_shop(config: &HashMap<String, Value>, kind: &str) -> Result<String, String> {
    let keys: Vec<&str> = match kind {
        "SHOPe" => vec!["event_shop", "shop_event"],
        _ => vec!["event_shop", "raid_shop", "arena_shop", "shop"],
    };
    let mut obj = serde_json::Map::new();
    for k in keys {
        if let Some(v) = config.get(k) {
            obj.insert(k.to_string(), v.clone());
        }
        // also copy keys that start with prefix
        for (ck, cv) in config {
            if ck.starts_with(k) {
                obj.insert(ck.clone(), cv.clone());
            }
        }
    }
    let raw = serde_json::to_vec(&Value::Object(obj)).map_err(|e| e.to_string())?;
    Ok(seal(if kind == "SHOPe" { "SHOPe" } else { "SHOP3" }, &b64url_encode(&raw)))
}

/// Encode full non-secret config.
pub fn encode_config(config: &HashMap<String, Value>) -> Result<String, String> {
    let mut obj = serde_json::Map::new();
    for (k, v) in config {
        if FORBIDDEN_KEYS.iter().any(|f| k.eq_ignore_ascii_case(f)) {
            continue;
        }
        obj.insert(k.clone(), v.clone());
    }
    let raw = serde_json::to_vec(&Value::Object(obj)).map_err(|e| e.to_string())?;
    Ok(seal("CFG", &b64url_encode(&raw)))
}

/// Decode any AM2 pack → (kind, flat patch map).
pub fn decode_any(code: &str) -> Result<(String, HashMap<String, Value>), String> {
    let (kind, payload) = open(code)?;
    let v: Value = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
    let obj = v
        .as_object()
        .ok_or_else(|| "payload not object".to_string())?;
    let mut map = HashMap::new();
    for (k, val) in obj {
        map.insert(k.clone(), val.clone());
    }
    Ok((kind, map))
}

pub fn apply_shop_patch(
    config: &mut HashMap<String, Value>,
    kind: &str,
    patch: &HashMap<String, Value>,
) -> Result<(), String> {
    let _ = kind;
    for (k, v) in patch {
        if FORBIDDEN_KEYS.iter().any(|f| k.eq_ignore_ascii_case(f)) {
            continue;
        }
        config.insert(k.clone(), v.clone());
    }
    Ok(())
}

pub fn apply_config_patch(config: &mut HashMap<String, Value>, patch: HashMap<String, Value>) {
    for (k, v) in patch {
        if FORBIDDEN_KEYS.iter().any(|f| k.eq_ignore_ascii_case(f)) {
            continue;
        }
        config.insert(k, v);
    }
}
