# Protocol stack: game HTTP pipeline and crypto (complete, source-backed)

| Item | Content |
|------|---------|
| **Wall clock** | 2026-08-07 06:04 |
| **Outbound (Python authority)** | `archive/pre-rust-2026-08/autopcr/core/base.py` · `apiclient.py` · `crypto.py` · `misc.py` · `model/modelbase.py` · `util/freqlimiter.py` · `util/aiorequests.py` · `util/type_utils.py` · `constants.py` |
| **Authority git** | `origin/main` @ `9826135` |
| **Outbound (Rust)** | `crates/rustmadoka-core/src/client.rs` · `crypto.rs` · `error.rs` · `diag.rs` |
| **Inbound** | [SDK_AND_LOGIN.md](./SDK_AND_LOGIN.md) · [VERSION_FINGERPRINT.md](./VERSION_FINGERPRINT.md) · [API_INVENTORY.md](./API_INVENTORY.md) · [INIT_AND_RESPONSE_PAYLOADS.md](./INIT_AND_RESPONSE_PAYLOADS.md) · [UPSTREAM_SOURCE_AND_WIRE.md](./UPSTREAM_SOURCE_AND_WIRE.md) · [LESSONS_RUST_PORT.md](./LESSONS_RUST_PORT.md) L1/L2/L8 · [UPSTREAM_FILE_MAP.md](./UPSTREAM_FILE_MAP.md) |
| **MAY CONTAIN ERRORS** | Yes — constants from client source; server may change without notice |

---

## 0. 中文总览

游戏业务请求**不是**明文 JSON。流程是：

1. 业务对象填字段 → `prepare()` 写入 **`sm` 指纹**  
2. 包进外层 envelope（uuid / sessionId / userId / actionTime …）  
3. **msgpack** 序列化 → **AES-CBC** 加密（固定 IV 策略 + PKLB 密钥）  
4. 对**密文字节**做 RSA 签名 → HTTP 头 **`x-post-signature`**  
5. POST 到 `{apiroot}{url}`，Content-Type `application/x-msgpack`  
6. 响应同样解密解包；`errors` 非空则业务失败  

本文件按源码逐步写清，**不用「详见源码」代替步骤**。

---

## 1. Container onion (`base.py`)

`Container.register(component)` wraps handlers so the **last registered** runs **outermost**.

`pcrclient.__init__` registration order (source `pcrclient.py`):

1. `errorhandler()` — from `misc.py`  
2. `self.data` (`datamgr`)  
3. `self.session` (`sessionmgr`)  
4. `mutexhandler()` — from `misc.py`  

Call graph for `await client.request(req)`:

```text
mutexhandler.request
  → sessionmgr.request   # may run full login first
    → datamgr.request      # on return: resp.update(datamgr)
      → errorhandler.request # network retry loop
        → apiclient._request_internal  # real HTTP
```

### 1.1 errorhandler (`misc.py`)

| Behavior | Detail |
|----------|--------|
| NetworkException | Retry up to 5 times |
| ApiException containing `"维护"` | Raise `PanicError` |
| Other ApiException | Re-raise |
| Other Exception | Log and re-raise |

### 1.2 mutexhandler (`misc.py`)

Single `asyncio.Lock` around the rest of the chain (in addition to `apiclient._lck` on the wire call).

---

## 2. Single game request (`apiclient._request_internal`)

Source steps (abbreviated only by omitting probe instrumentation lines):

| Step | Code behavior | Notes |
|-----:|---------------|-------|
| 1 | If `request` is falsy, return `None` | Used as login ping |
| 2 | `request.lastHomeAccessTime = self.lastHomeAccessTime` | String timestamp |
| 3 | `request.prepare()` | Sets `request.sm = version_info.sm` (`modelbase.RequestBase`) |
| 4 | `await self.modify_request(request)` | Sonet may inject JWT on LoginApi |
| 5 | Build envelope `Request(...)` | See §3 |
| 6 | `urlroot = self.servers[self.active_server]` | Set by sessionmgr to `sdk.apiroot` |
| 7 | `crypted = PackHelper.pack(req.dict(by_alias=True), PackHelper.get_iv(), self.get_crypto_key())` | msgpack + AES |
| 8 | `self._headers['x-post-signature'] = await self.post_sign(crypted)` | RSA or JWT per channel |
| 9 | `aiorequests.post(urlroot + request.url, data=crypted, headers=..., timeout=10)` | |
| 10 | Status handling | §4 |
| 11 | `PackHelper.unpack(body, crypto_key)` | decrypt + msgpack |
| 12 | Parse `Response[T]` via pydantic | `type_utils.find_type_base` |
| 13 | If `response.errors` | `ApiException` join reasons |
| 14 | Assert `payload is not None` | |
| 15 | If url is `/api/home/get_home_info` | `access_home()` updates lastHomeAccessTime |

Outer `request()`:

```text
async with self._lck:
    return await self._request_internal(request)
```

Rate limit decorator on `_request_internal`:

```text
@FreqLimiter(API_LIMIT_TIMES=5, API_LIMIT_INTERVAL=1)
```

From `constants.py`.

---

## 3. Envelope types (`modelbase.py`)

### 3.1 RequestBase (payload)

| Field | Set by |
|-------|--------|
| lastHomeAccessTime | apiclient before prepare |
| sm | `prepare()` ← `version_info.sm` |
| + subclass business fields | module / sessionmgr |

### 3.2 Outer `Request` model fields

| Field | Source |
|-------|--------|
| payload | RequestBase instance |
| uuid | `apiclient.uuid` (SDK) |
| userId | `apiclient.userId` |
| sessionId | `apiclient.sessionId` (optional before login) |
| actionToken | `None` in default path |
| ctag | `None` in default path |
| actionTime | `apiclient.actionTime()` |

### 3.3 actionTime formula (`apiclient.actionTime`)

```text
EPOCH_DIFFERENCE_SECONDS = 11644473600  # 1601 → 1970
filetime = int((unix_seconds + EPOCH_DIFFERENCE_SECONDS) * 10**7)
```

Windows FILETIME-style 100ns ticks.

### 3.4 Response envelope

| Field | Meaning |
|-------|---------|
| payload | Typed `TResponse` |
| url | Echo |
| status | Status |
| errors | Optional list of `ServerError` (domain, code, field, reason) |

---

## 4. HTTP status handling

| Status / case | Behavior | Exception |
|---------------|----------|-----------|
| **428** | `await update_version()` then raise | `VersionUpdatedException` — sessionmgr retries login |
| **401** | Unauthorized | `ApiException` result_code 401; sessionmgr clears `_logged` on business ApiException path |
| Other non-200 | `raise_for_status` → often `NetworkException` | |
| Body `errors` non-null | Join `reason` strings | `ApiException` |
| Unpack/network failure | traceback | `NetworkException` |

---

## 5. Crypto (`crypto.py`) — full mechanism

### 5.1 Key derivation (PKLB / day-global AES material)

| Symbol | Construction (source) |
|--------|----------------------|
| `StrCnv1.cnv` | Characters at odd indices of obfuscated strings in `Builtin._elements` |
| `AppCryptoConfig.hash_key/salt/crypto_key` | Built-in elements + HMAC-SHA256 style hash |
| **`PKLB_HASH_KEY`** | `Hash.hash_string("UVFBdDtWKhpESJj3", 16, (hash_salt, hash_key_bytes))` |
| **`SONET_HASH_KEY`** | `Hash.hash_string("ABCDEFGHIJKLMNOP", 16, ('System.Char[]', b'System.Char[]'))` |

Gree JP/US game API uses **PKLB_HASH_KEY** (`sdkclients.sdkclientbase.get_crypto_key`).  
Sonet uses **SONET_HASH_KEY**.

Golden product check (Rust lessons L2/L8): derived AES key string must match known vector used in port tests (documented in LESSONS as `/TZh+1VxrtkNiDEH` path result).

### 5.2 Fixed IV (`PackHelper.get_iv`)

Hex bytes:

```text
88 46 51 55 30 61 67 82 55 2c ab 5e 1d 7c 85 0f
```

### 5.3 pack / unpack

| Direction | Steps |
|-----------|-------|
| pack | `msgpack.packb(token)` → `ApiCrypto.encrypt(raw, iv, hk)` → `iv \|\| ciphertext` (BasicCrypto.encrypt prepends IV) |
| unpack | `ApiCrypto.decrypt` (IV is first `len(key)` bytes) → `msgpack.unpackb(..., raw=False)` |

AES: **AES-CBC + PKCS7**. Key length from crypto key string bytes (AES-128 material in practice for PKLB path).

### 5.4 ApiCrypto.sign (game `x-post-signature`)

Source `ApiCrypto.sign(encrypted, private_key_bytes)`:

1. `SHA1(encrypted_body)` → Base64 → `data`  
2. `SHA1(data)` → `digest`  
3. RSA PKCS1v15 sign with **Prehashed SHA1** on `digest`  
4. Base64 signature string → header  

Private key: DER PKCS#8 from Gree token cache.

**Not** the same as Gree OAuth HMAC stage (see SDK_AND_LOGIN / L1).

---

## 6. Default HTTP headers (`constants.py` + `sdkclient.header`)

| Header | Default (Android) |
|--------|-------------------|
| content-type | application/x-msgpack |
| x-timezone-offset | 28800 |
| x-language | ja-Jpan |
| x-unity-version | 2022.3.21f1 |
| x-region | JP |
| user-agent | UnityRequest … ASUS_I003DD Android 9 … |
| x-game-server-url | set to `sdk.apiroot` at runtime |

`IOS_HEADERS` is empty dict in source (iOS path incomplete).

---

## 7. Game API roots (channel)

| Channel display | Class | apiroot |
|-----------------|-------|---------|
| 日服 BSDK | bsdkclient | `https://api.mmme.pokelabo.jp` |
| 国际服 QSDK | qsdkclient | `https://api-gl.mmme.pokelabo.jp` |
| 日服 RSA | bsdkrsaclient | same JP |
| 国际 RSA | qsdkrsaclient | same GL |
| 台服 SONET | sonetsdkclient | `https://app-mme.so-net.tw` |

Full login/sign differences: [SDK_AND_LOGIN.md](./SDK_AND_LOGIN.md).

---

## 8. Complete API surface

**494** unique paths: [API_INVENTORY.md](./API_INVENTORY.md) (generated from `requests.py`, not a summary).

---

## 9. DEBUG_LOG vs probe

| Mechanism | Switch | Output |
|-----------|--------|--------|
| DEBUG_LOG | `AUTOPCR_SERVER_DEBUG_LOG` | Append headers+body to `req.log` (noisy, sensitive) |
| Probe | `AUTOPCR_PROBE=1` | Structured JSONL (`probe_capture.py`, archive-only addition) |

Prefer probe for analysis; never commit token dumps.

---

## 10. Rust implementation notes

| Concern | Rust location |
|---------|----------------|
| Serial mutex | `GameClient` internal Mutex |
| Envelope + pack | `client.rs` request_raw |
| Crypto | `crypto.rs` |
| Fingerprint sm | `fingerprint.rs` |
| Sign | Gree private key path |
| Timeouts | connect/total timeouts (product hardening) |
| Error Chinese | `diag.rs` / `error.rs` |

Preserve JSON key order for Gree OAuth body hash: workspace `serde_json` **preserve_order** (Cargo.toml comment).

---

## 11. Lessons cross-links

| ID | Topic |
|----|-------|
| L1 | Gree signature stage HMAC vs RSA |
| L2 | AES unpad / msgpack shape |
| L8 | Golden vectors checklist |

---

## 12. Revision

| Date | Content |
|------|---------|
| 2026-08-06 | First structural doc |
| 2026-08-07 06:04 | DOC-FULL-01: expanded step-by-step from source; no summary-only body |
