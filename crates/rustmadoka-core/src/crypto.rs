//! 游戏 API 加密封包与 Gree/API 签名原语。
//!
//! # 职责
//! - 从内置混淆串派生 **PKLB AES-128 密钥**（须与 Python 黄金向量一致）
//! - msgpack 序列化后 AES-CBC+PKCS7：密文形态 = IV || ciphertext
//! - `ApiCrypto` 风格 RSA-SHA1（Prehashed）供 `x-post-signature`
//! - `B_encode` / Gree 512-bit RSA 生成等辅助
//!
//! # 不变量（踩坑 L2/L8）
//! - 密钥派生结果须对齐 `/TZh+1VxrtkNiDEH`（当前 AppCryptoConfig 路径）
//! - 业务侧 msgpack 用 **rmpv** 形状对齐 Python `packb`，勿用会改字段名的路径
//!
//! # 文档
//! - `docs/tech/PROTOCOL_STACK.md` · `docs/tech/LESSONS_RUST_PORT.md`（L2/L8）
//! - `docs/tech/AUTOMADOKA_RESEARCH_AND_RUST_GAP.md` §2
//!
//! # 对照
//! `archive/pre-rust-2026-08/autopcr/core/crypto.py`

use crate::error::{CoreError, Result};
use aes::Aes128;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use hmac::{Hmac, Mac};
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::{Pkcs1v15Sign, RsaPrivateKey};
use sha1::{Digest, Sha1};
use sha2::Sha256;

type Aes128CbcEnc = cbc::Encryptor<Aes128>;
type Aes128CbcDec = cbc::Decryptor<Aes128>;
type HmacSha256 = Hmac<Sha256>;

/// 与 Python StrCnv1.cnv 一致：取奇数下标
fn str_cnv1(src: &str) -> String {
    src.chars()
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, c)| c)
        .collect()
}

fn builtin_key(index: usize) -> String {
    const ELEMENTS: [&str; 3] = [
        "4dn9Sycy!ev)8f%_,Yay~pAj)~k4q!hNz,FHuWHFQe%+P*eW24Ac)yTAGeF$pJ)!7BU!9#ke%|3Ai%*jMa(Vi~B2j*L(uyvE/9cE$E_,WwV4irL$5RXgaC4ufu/4FB5p",
        "j%.i.LL|rL,+d6JA",
        "EZTv,6~NZQv(X9DU",
    ];
    str_cnv1(ELEMENTS[index])
}

fn hash_bytes(text: &str, max_length: usize, salt: &str, key: &[u8]) -> Vec<u8> {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(key).expect("hmac key");
    mac.update(format!("{salt}{text}").as_bytes());
    let v15 = mac.finalize().into_bytes().to_vec();
    if max_length == 0 || v15.len() < max_length {
        return v15;
    }
    let offset = (v15.len() - max_length) / 2;
    v15[offset..offset + max_length].to_vec()
}

fn hash_string(text: &str, max_length: usize, salt: &str, key: &[u8]) -> String {
    let hb = hash_bytes(text, max_length, salt, key);
    let b64 = B64.encode(hb);
    if b64.len() > max_length {
        let start = (b64.len() - max_length) / 2;
        b64[start..start + max_length].to_string()
    } else {
        b64
    }
}

/// 日/国服游戏 API AES 密钥材料（`crypto.PKLB_HASH_KEY`）
///
/// Python 钉死：
/// `Hash.hash_string("UVFBdDtWKhpESJj3", 16, (hash_salt, hash_key.encode()))`
/// **不是** `AppCryptoConfig.crypto_key()`（那是另一路，输入为 builtin[2]）。
pub fn pklb_hash_key() -> String {
    let hash_key = builtin_key(0);
    let hash_salt = builtin_key(1);
    hash_string("UVFBdDtWKhpESJj3", 16, &hash_salt, hash_key.as_bytes())
}

/// 固定 IV（PackHelper.get_iv）
pub fn pack_iv() -> [u8; 16] {
    [
        0x88, 0x46, 0x51, 0x55, 0x30, 0x61, 0x67, 0x82, 0x55, 0x2c, 0xab, 0x5e, 0x1d, 0x7c, 0x85,
        0x0f,
    ]
}

pub fn encrypt(crypto_key: &str, data: &[u8], iv: &[u8]) -> Result<Vec<u8>> {
    let key = crypto_key.as_bytes();
    if key.len() != 16 {
        return Err(CoreError::Crypto(format!(
            "key len {} want 16",
            key.len()
        )));
    }
    if iv.len() != 16 {
        return Err(CoreError::Crypto("iv len".into()));
    }
    let enc = Aes128CbcEnc::new_from_slices(key, iv)
        .map_err(|e| CoreError::Crypto(e.to_string()))?;
    let ciphertext = enc.encrypt_padded_vec_mut::<Pkcs7>(data);
    let mut out = iv.to_vec();
    out.extend(ciphertext);
    Ok(out)
}

pub fn decrypt(crypto_key: &str, data: &[u8]) -> Result<Vec<u8>> {
    let key = crypto_key.as_bytes();
    if data.len() < key.len() {
        return Err(CoreError::Crypto("cipher too short".into()));
    }
    let iv = &data[..key.len()];
    let ct = &data[key.len()..];
    let dec =
        Aes128CbcDec::new_from_slices(key, iv).map_err(|e| CoreError::Crypto(e.to_string()))?;
    dec.decrypt_padded_vec_mut::<Pkcs7>(ct)
        .map_err(|e| CoreError::Crypto(e.to_string()))
}

/// JSON Value → msgpack，对齐 Python `msgpack.packb(dict, raw=False)` 默认编码。
/// 注意：不可对 `serde_json::Value` 直接 `rmp_serde::to_vec_named`（会变成错误的 enum 形状）。
fn json_to_rmpv(v: &serde_json::Value) -> rmpv::Value {
    use rmpv::Value as Rv;
    match v {
        serde_json::Value::Null => Rv::Nil,
        serde_json::Value::Bool(b) => Rv::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Rv::Integer(i.into())
            } else if let Some(u) = n.as_u64() {
                Rv::Integer(u.into())
            } else {
                Rv::F64(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Rv::String(s.as_str().into()),
        serde_json::Value::Array(a) => Rv::Array(a.iter().map(json_to_rmpv).collect()),
        serde_json::Value::Object(m) => {
            // preserve_order：与 pydantic Request 字段序一致
            let pairs = m
                .iter()
                .map(|(k, val)| (Rv::String(k.as_str().into()), json_to_rmpv(val)))
                .collect();
            Rv::Map(pairs)
        }
    }
}

fn pack_msgpack(token: &serde_json::Value) -> Result<Vec<u8>> {
    let v = json_to_rmpv(token);
    let mut buf = Vec::new();
    rmpv::encode::write_value(&mut buf, &v).map_err(|e| CoreError::Crypto(e.to_string()))?;
    Ok(buf)
}

/// msgpack 序列化 + AES（对照 PackHelper.pack）
pub fn pack_value(token: &serde_json::Value, crypt_key: &str) -> Result<Vec<u8>> {
    let packed = pack_msgpack(token)?;
    encrypt(crypt_key, &packed, &pack_iv())
}

pub fn unpack_value(crypted: &[u8], crypt_key: &str) -> Result<serde_json::Value> {
    // 服务端偶发返回明文 JSON（错误页）时，避免只报 Unpad Error
    if crypted.first() == Some(&b'{') || crypted.first() == Some(&b'[') {
        if let Ok(s) = std::str::from_utf8(crypted) {
            if let Ok(j) = serde_json::from_str::<serde_json::Value>(s) {
                return Ok(j);
            }
        }
    }
    let raw = decrypt(crypt_key, crypted).map_err(|e| {
        CoreError::Crypto(format!(
            "{e}; body_len={} head_hex={}",
            crypted.len(),
            hex::encode(&crypted[..crypted.len().min(32)])
        ))
    })?;
    let v: rmpv::Value =
        rmpv::decode::read_value(&mut raw.as_slice()).map_err(|e| CoreError::Crypto(e.to_string()))?;
    rmpv_to_json(&v)
}

fn rmpv_to_json(v: &rmpv::Value) -> Result<serde_json::Value> {
    use rmpv::Value::*;
    Ok(match v {
        Nil => serde_json::Value::Null,
        Boolean(b) => serde_json::json!(*b),
        Integer(i) => {
            if let Some(n) = i.as_i64() {
                serde_json::json!(n)
            } else if let Some(n) = i.as_u64() {
                serde_json::json!(n)
            } else {
                serde_json::Value::Null
            }
        }
        F32(f) => serde_json::json!(*f),
        F64(f) => serde_json::json!(*f),
        String(s) => serde_json::json!(s.as_str().unwrap_or("")),
        Binary(b) => serde_json::json!(B64.encode(b)),
        Array(a) => {
            let arr: Result<Vec<_>> = a.iter().map(rmpv_to_json).collect();
            serde_json::Value::Array(arr?)
        }
        Map(m) => {
            let mut map = serde_json::Map::new();
            for (k, val) in m {
                let key = match k {
                    String(s) => s.as_str().unwrap_or("").to_string(),
                    Integer(i) => i.as_i64().unwrap_or(0).to_string(),
                    _ => k.to_string(),
                };
                map.insert(key, rmpv_to_json(val)?);
            }
            serde_json::Value::Object(map)
        }
        Ext(_, _) => serde_json::Value::Null,
    })
}

/// ApiCrypto.sign — x-post-signature
pub fn sign_request(encrypted: &[u8], private_key_der: &[u8]) -> Result<String> {
    let mut h = Sha1::new();
    h.update(encrypted);
    let data = B64.encode(h.finalize());

    let mut h2 = Sha1::new();
    h2.update(data.as_bytes());
    let digest = h2.finalize();

    let private_key = RsaPrivateKey::from_pkcs8_der(private_key_der)
        .map_err(|e| CoreError::Crypto(e.to_string()))?;
    // cryptography Prehashed(SHA1)：已哈希的 20 字节 + DigestInfo(SHA1) + PKCS1v15
    let padding = Pkcs1v15Sign::new::<Sha1>();
    let sig = private_key
        .sign(padding, &digest)
        .map_err(|e| CoreError::Crypto(e.to_string()))?;
    Ok(B64.encode(sig))
}

/// 生成 512-bit RSA（Gree register）— 与 Python generate_512bit_rsa_key 类似
pub fn generate_gree_rsa() -> Result<(Vec<u8>, String)> {
    use rand::rngs::OsRng;
    // 512-bit is weak but required by protocol
    let bits = 512;
    let private_key = RsaPrivateKey::new(&mut OsRng, bits)
        .map_err(|e| CoreError::Crypto(e.to_string()))?;
    let der = private_key
        .to_pkcs8_der()
        .map_err(|e| CoreError::Crypto(e.to_string()))?
        .as_bytes()
        .to_vec();
    let pub_pem = private_key
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .map_err(|e| CoreError::Crypto(e.to_string()))?;
    Ok((der, pub_pem))
}

/// B_encode: base64 → reverse → rot13（引继密码）
pub fn b_encode(input: &str) -> String {
    let b64 = B64.encode(input.as_bytes());
    let rev: String = b64.chars().rev().collect();
    rot13(&rev)
}

fn rot13(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' => ((c as u8 - b'A' + 13) % 26 + b'A') as char,
            'a'..='z' => ((c as u8 - b'a' + 13) % 26 + b'a') as char,
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sm_formula() {
        let fp = crate::fingerprint::Fingerprint {
            version: "3.13.0".into(),
            sign: "d929c89a96c474de5772d47848491c00".into(),
            libcount: 34,
            channel: Some("en".into()),
            package_id: None,
        };
        assert!(fp.sm().starts_with("dd929"));
        assert!(fp.sm().contains("o34"));
    }

    #[test]
    fn pklb_key_matches_python() {
        // 本机 Python crypto.PKLB_HASH_KEY 钉死
        let k = pklb_hash_key();
        assert_eq!(k, "/TZh+1VxrtkNiDEH", "key={k}");
        assert_eq!(k.len(), 16);
    }

    #[test]
    fn pack_roundtrip_matches_python_vector() {
        // Python: msgpack.packb + PackHelper.pack 黄金向量
        let token = serde_json::json!({
            "payload": {"sm": "test", "lastHomeAccessTime": "0"},
            "uuid": "abc",
            "userId": 0,
            "sessionId": null,
            "actionToken": null,
            "ctag": null,
            "actionTime": 12345,
        });
        let packed = pack_msgpack(&token).unwrap();
        let expect_msgpack = hex::decode(
            "87a77061796c6f616482a2736da474657374b26c617374486f6d6541636365737354696d65a130a475756964a3616263a675736572496400a973657373696f6e4964c0ab616374696f6e546f6b656ec0a463746167c0aa616374696f6e54696d65cd3039",
        )
        .unwrap();
        assert_eq!(
            packed, expect_msgpack,
            "msgpack mismatch\ngot  {}\nwant {}",
            hex::encode(&packed),
            hex::encode(&expect_msgpack)
        );
        let crypted = pack_value(&token, &pklb_hash_key()).unwrap();
        let expect_crypt = hex::decode(
            "8846515530616782552cab5e1d7c850f4b26af21ca9846119d2db77d4ed6459bf61f6e49b9a6b6285a2ce3e9d09f147e1431b136bc2e1727a07852e01058e1f41d06b0f7acbbb3b9261042d22f83021026153fc7c6ca01dd7788b23d50580cd6e6a394b9744e4bf4c74974760729d12cd89d672d2ec92f8ecd0af62f2a9607d2",
        )
        .unwrap();
        assert_eq!(
            crypted,
            expect_crypt,
            "cipher mismatch\ngot  {}\nwant {}",
            hex::encode(&crypted),
            hex::encode(&expect_crypt)
        );
        let back = unpack_value(&crypted, &pklb_hash_key()).unwrap();
        assert_eq!(back["payload"]["sm"], "test");
    }
}
