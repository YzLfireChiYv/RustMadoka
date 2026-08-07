# SDK and login pipeline (Gree / Sonet / sessionmgr) — complete source-backed

| Item | Content |
|------|---------|
| **Wall clock** | 2026-08-07 06:04 |
| **Outbound** | `archive/pre-rust-2026-08/autopcr/core/sessionmgr.py` · `sdkclient.py` · `sdk/sdkclients.py` · `sdk/greeclient.py` · `sdk/sonetclient.py` · `core/crypto.py` · `constants.py` |
| **Authority git** | `origin/main` @ `9826135` |
| **Rust** | `crates/rustmadoka-core/src/gree.rs` · `client.rs` · `account.rs` (`Channel`) · `fingerprint.rs` |
| **Inbound** | [PROTOCOL_STACK.md](./PROTOCOL_STACK.md) · [CHANNELS.md](./CHANNELS.md) · [INIT_AND_RESPONSE_PAYLOADS.md](./INIT_AND_RESPONSE_PAYLOADS.md) · [UPSTREAM_SOURCE_AND_WIRE.md](./UPSTREAM_SOURCE_AND_WIRE.md) · [VERSION_FINGERPRINT.md](./VERSION_FINGERPRINT.md) · [LESSONS_RUST_PORT.md](./LESSONS_RUST_PORT.md) L1/L10 · [UPSTREAM_FILE_MAP.md](./UPSTREAM_FILE_MAP.md) |
| **MAY CONTAIN ERRORS** | Yes — live Gree/Sonet may change; migrate Sonet incomplete in upstream |

---

## 0. 中文总览

登录分两截：

1. **渠道 SDK（Gree 或 Sonet）**：证明你是「某个引继码账号」的设备，得到 **uuid**（Gree 还要 **RSA 私钥**）。  
2. **游戏 sessionmgr**：用 uuid 调游戏 **LoginApi**，再拉 mst + init 大包（队伍、道具等）——详见 [INIT_AND_RESPONSE_PAYLOADS.md](./INIT_AND_RESPONSE_PAYLOADS.md)。

游戏业务 HTTP 的 `x-post-signature` 对 **AES 密文**签名；Gree 的 `/auth/initialize` 阶段却是 **HMAC**，二者不能混（教训 **L1**）。

---

## 1. Channel map / 渠道映射

Source: `constants.py` channel names · `sdkclients.py` classes.

| Constant | Display | SDK class | Game `apiroot` | AES key | Game post_sign |
|----------|---------|-----------|----------------|---------|----------------|
| `BSDK` | 日服 | `bsdkclient` | `https://api.mmme.pokelabo.jp` | `PKLB_HASH_KEY` | `ApiCrypto.sign(RSA)` |
| `QSDK` | 国际服 | `qsdkclient` | `https://api-gl.mmme.pokelabo.jp` | PKLB | same, US Gree |
| `BSDKRSA` | 日服 RSA 捷径 | `bsdkrsaclient` | JP | PKLB | password field = private key B64; login returns username as uuid |
| `QSDKRSA` | 国际 RSA 捷径 | `qsdkrsaclient` | GL | PKLB | same |
| `SONET` | 台服 | `sonetsdkclient` | `https://app-mme.so-net.tw` | `SONET_HASH_KEY` | JWT string; LoginApi may set `jwttoken` |

`sdkclients.create(channel, account)`: unknown channel falls back to BSDK with warning (source).

Rust product channels: `jp` / `en` / `tw` (`account::Channel`). **`tw` login not implemented** (`login_implemented() == false`).

---

## 2. Abstract SDK (`sdkclient.py`)

| Member | Role |
|--------|------|
| `account` (username/password/platform) | Migration code + game password |
| `login() -> uuid` | Channel login |
| `register(password)` | New account |
| `get_crypto_key()` | AES material for game pack |
| `post_sign(bytes) -> str` | Game body signature |
| `modify_request(request)` | e.g. Sonet JWT on LoginApi |
| `header()` | DEFAULT_HEADERS + `x-game-server-url=apiroot` |
| `apiroot` / `region` | Per subclass |
| `invoke_post_login` | Run and clear post-login callbacks |

---

## 3. Gree client (`greeclient.py`) — full flow

### 3.1 Hosts and app credentials

Subclasses `JpGreeClient` / `UsGreeClient` provide:

| Property | Role |
|----------|------|
| APP_ID | OAuth consumer key |
| APP_SECRET | HMAC secret / xoauth material |
| BASE_URL | e.g. `https://gl-pkl-jp-payment.gree-apps.net/v1.0` (JP) |

**【实测预留】** whether APP_ID/SECRET rotate with app versions.

### 3.2 Device id（产品：按游戏账号卡片复用）

```text
generate_device_id: 8 random bytes → hex
device_id stored as B_encode(hex)   # rot13(reverse(base64(utf8)))
```

**RustMadoka 现行规则（2026-08-07）：**

| 项 | 规则 |
|----|------|
| **复用单位** | **游戏账号卡片** = 引继码（token 文件键） |
| **主存** | `cache/token/{引继}_{pwdMD5}.json` 内字段 `device_id` |
| **卡片侧备份** | `cache/device_by_account/{引继安全名}.json` — token 重建后仍绑同一卡片 |
| **禁止** | 全数据文件夹共用一个安装级 `device_id` 给所有卡片 |
| **旧文件** | `cache/device_profile.json` 仅历史；**不再**作为新号来源 |

源码：`crates/rustmadoka-core/src/gree.rs` · 规格全文：[GROUP_RAID_AND_DEVICE_IDENTITY.md](./GROUP_RAID_AND_DEVICE_IDENTITY.md) §6  

多账号并行时**不**刻意插入等待秒数（同规格 §1.3）。

### 3.3 register()

1. Generate **512-bit RSA** key (`generate_512bit_rsa_key`).  
2. Export public key PEM (SubjectPublicKeyInfo).  
3. `POST /auth/initialize` body:
   - `device_id`
   - `token` = public_key_pem  
   - `payload` = JSON string of device/app fields **including `sm` and `appVersion`**  
4. Store `private_key` DER, `uuid` from result.  

**Signature for initialize:** `private_key` is still **None** during first posts → OAuth uses **HMAC-SHA1(APP_SECRET)** (not RSA). **L1.**

### 3.4 migrate_from(migration_code, password)

```text
POST /migration/code/verify
  migration_code, migration_password=B_encode(password)
→ migration_token, src_uuid

POST /migration
  migration_token, src_uuid, device_id, token=public_key_pem, dst_uuid=self.uuid
→ self.uuid = src_uuid
```

### 3.5 register_password / get_migration_code

| Call | Path |
|------|------|
| register_password | `POST /migration/password/register` |
| get_migration_code | `GET /migration/code` → `migration_code` |

### 3.6 login() authorize

```text
POST /auth/authorize
on "Inactive Device": POST /linked/active/update then authorize again
```

### 3.7 OAuth request construction (`GreeClient._request`)

For each Gree HTTP call:

| Piece | Content |
|-------|---------|
| Body | JSON compact separators `(',', ':')` or empty |
| oauth_body_hash | Base64(SHA1(body)) |
| oauth_consumer_key | APP_ID |
| oauth_nonce | random 64-bit |
| oauth_timestamp | unix seconds string |
| oauth_version | 1.0 |
| If private_key set | oauth_signature_method=RSA-SHA1; xoauth_as_hash = RSA-SHA1-Prehashed(SHA1(APP_SECRET+ts)); xoauth_requestor_id=uuid; oauth_signature = RSA over normalized base string |
| If no private_key | HMAC-SHA1(APP_SECRET) over normalized base string |
| Headers | Authorization: OAuth …; X-GREE-GAMELIB=…appVersion…; User-Agent Android WebView style |

Success: JSON `result == "OK"`.

### 3.8 sdkclientbase.login (Gree day-global)

Source `sdkclients.sdkclientbase.login`:

```text
for attempt in 1..5:
  try load cache/token/{migration}_{md5(password)}.json
      gclient = ClientType(privateKey, uuid)
      await gclient.login()  # authorize
      break
  except missing/fail:
      gclient = ClientType()
      await gclient.register()
      await gclient.migrate_from(username, password)
      await gclient.login()
      save privateKey+uuid
return uuid
```

Migration code format check: `^[A-Za-z0-9]{16}$`.

Cache JSON keys: `privateKey` (base64 DER), `uuid`. (Rust may also store device_id.)

### 3.9 Game post_sign after Gree

`bsdkclient.post_sign` / `qsdkclient.post_sign`:

```text
ApiCrypto.sign(encrypted_game_body, private_key_bytes=gclient.private_key)
```

---

## 4. Sonet (`sonetclient.py` + `sonetsdkclient`)

### 4.1 SDK HTTP

| Item | Value |
|------|-------|
| Host | `https://mme-sdk.so-net.tw` |
| gameid | 2601 |
| Sign | Sort `key=value&` pairs, append `sonet`, MD5 hex → field `sign` |
| register | `/api/register` → uuid, token, handoverid |
| set password | `/api/login/sethandoverpassword` with Bearer JWT |
| game login JWT | `/api/login/game` → jwtToken; cache until exp-120s |
| **migrate_from** | **`raise NotImplementedError`** in upstream |

### 4.2 sonetsdkclient game integration

| Method | Behavior |
|--------|----------|
| get_crypto_key | `SONET_HASH_KEY` |
| post_sign | return JWT string |
| modify_request | if LoginApiLoginRequest: set `jwttoken` |
| apiroot | `https://app-mme.so-net.tw` |
| cacheFile | token json with deviceid/uuid/token |

Rust: fingerprint channel `tw` may exist; **login path not implemented**.

---

## 5. sessionmgr game chain

Full ordered table and response fields: **[INIT_AND_RESPONSE_PAYLOADS.md](./INIT_AND_RESPONSE_PAYLOADS.md)** (canonical, not abbreviated here).

Triggers, rate limits, 428 retry, ApiException clearing `_logged`: see that document §1–§2 and PROTOCOL_STACK §4.

---

## 6. Account identifiers

| Layer | Identifier |
|-------|------------|
| Tool user (Python http_server) | Folder name under CONFIG_PATH |
| Game role | `AccountData.username` (migration code) + password + channel |
| sessionmgr.id / Account.id | `md5(migration_code)` |
| Gree | uuid + RSA |
| Sonet | uuid + token + deviceid |

---

## 7. Device identity model (Gree) — product vs official app

> Server copy about “logged in on another device” has **no public contract**. Below is **source-backed local identity**, not a guarantee.

### 7.1 What is persisted

| Layer | Fields | On-disk |
|-------|--------|---------|
| Gree device | device_id | Rust: `cache/device_profile.json`; also in token cache |
| Gree app identity | RSA private key + uuid | `cache/token/{code}_{pwdMD5}.json` |
| Game login portrait | fixed deviceModel/osVersion | Request body only |

### 7.2 Same device vs new device (inference)

| Scenario | device_id | RSA+uuid | Inference |
|----------|-----------|----------|-----------|
| Restart app / replace exe only | keep | keep | Same device |
| Delete token, keep device_profile | reuse | new | Same machine, new app registration |
| Copy whole data dir to another PC | same | same | Same materials → looks same device |
| Empty data dir | new | new | New device |
| Official game app vs this tool | real vs simulated | different | Different devices |

### 7.3 Operations advice

- One long-lived `automadoka_data` directory; upgrade only replaces binaries.  
- Do not run two data dirs against the same migration code.  
- Cross-machine backup: copy entire data dir including `cache/token` and `device_profile.json`.  
- Force “new device”: delete token + device_profile then re-login.

Python upstream often randomizes device_id on each register; Rust stabilizes via device_profile (product improvement).

---

## 8. RSA shortcut channels (BSDKRSA / QSDKRSA)

| Behavior | Detail |
|----------|--------|
| login | Returns `username` as uuid (no Gree migrate) |
| post_sign | `ApiCrypto.sign` with key = base64-decode(password field) |
| Audience | Power users who export keys; not normal product path |

Rust does not expose these as first-class UI channels.

---

## 9. Rust mapping

| Python | Rust |
|--------|------|
| sdkclientbase.login / migrate | `GreeSession::login_or_migrate` |
| initialize HMAC vs RSA | Enforced in `gree.rs` (L1) |
| sessionmgr chain | `GameClient::full_login` |
| light login | `login_for_info` |
| token path | `automadoka_data/cache/token/` |
| device_profile | `cache/device_profile.json` |
| Channel::Tw | Explicit login error until Sonet port |

---

## 10. Related lessons

| ID | Topic |
|----|-------|
| L1 | Invalid Signature / stage mix-up |
| L10 | Token cache is not “extra security feature” |
| C4 | Same as L10 product communication |
| L4 / P15 | Fingerprint source rules |

---

## 11. Revision

| Date | Content |
|------|---------|
| 2026-08-06 | First version |
| 2026-08-07 | § device identity |
| 2026-08-07 06:04 | DOC-FULL-01: expanded Gree/Sonet/OAuth/cache from source; chain details deferred to INIT doc |
