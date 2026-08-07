# Channels and default HTTP headers (source-backed)

| Item | Content |
|------|---------|
| **Wall clock** | 2026-08-07 06:04 |
| **Outbound** | `archive/pre-rust-2026-08/autopcr/constants.py` · `core/sdkclient.py` · `sdk/sdkclients.py` · Rust `crates/rustmadoka-core/src/account.rs` · `gree.rs` · `client.rs` |
| **Authority git** | `origin/main` @ `9826135` |
| **Inbound** | [SDK_AND_LOGIN.md](./SDK_AND_LOGIN.md) · [PROTOCOL_STACK.md](./PROTOCOL_STACK.md) · [UPSTREAM_SOURCE_AND_WIRE.md](./UPSTREAM_SOURCE_AND_WIRE.md) · [UPSTREAM_FILE_MAP.md](./UPSTREAM_FILE_MAP.md) |
| **MAY CONTAIN ERRORS** | Yes |

---

## 1. Channel constants (`constants.py`)

| Constant | String value |
|----------|--------------|
| BSDK | `日服` |
| QSDK | `国际服` |
| BSDKRSA | `日服（RSA登录）` |
| QSDKRSA | `国际服（RSA登录）` |
| SONET | `台服` |
| CHANNEL_OPTION | list of the five above |

## 2. Game API roots and crypto (sdkclients.py)

| Channel | apiroot | AES key helper | post_sign |
|---------|---------|----------------|----------|
| BSDK | https://api.mmme.pokelabo.jp | PKLB_HASH_KEY | RSA ApiCrypto.sign |
| QSDK | https://api-gl.mmme.pokelabo.jp | PKLB | RSA |
| BSDKRSA / QSDKRSA | same JP/GL | PKLB | key from password field |
| SONET | https://app-mme.so-net.tw | SONET_HASH_KEY | JWT |

Gree payment bases (greeclient subclasses): JP/US `gl-pkl-*-payment.gree-apps.net/v1.0`.  
Sonet SDK: `https://mme-sdk.so-net.tw`.

## 3. DEFAULT_HEADERS (Android)

| Header | Value in source |
|--------|-----------------|
| content-type | application/x-msgpack |
| x-timezone-offset | 28800 |
| x-language | ja-Jpan |
| x-unity-version | 2022.3.21f1 |
| x-region | JP |
| user-agent | UnityRequest … ASUS_I003DD Android OS 9 / API-28 … |
| x-game-server-url | set at runtime to apiroot |

`IOS_HEADERS = {}` (empty).

**【实测预留】** whether Global/Sonet should override x-region / x-language in production clients.

## 4. Rate limits and ports (`constants.py`)

| Name | Value |
|------|------:|
| SERVER_PORT default | 13200 (Python product; RustMadoka formal port **14103** (historical transitional 13220)) |
| API_LIMIT_TIMES / INTERVAL | 5 / 1s |
| LOGIN_LIMIT_TIMES / INTERVAL | 5 / 30s |
| CLIENT_POOL_* | pool sizing constants (Python hosting) |

## 5. Rust product channels

| UI / config | Maps to | Login |
|-------------|---------|-------|
| jp / 日服 | Gree Japan | Implemented |
| en / 国际服 | Gree Global | Implemented |
| tw / 台服 | Sonet | **Not implemented** (fingerprint may still list tw) |

## 6. Revision

| Date | Content |
|------|---------|
| 2026-08-06 | First version |
| 2026-08-07 06:04 | DOC-FULL-01: full tables from constants/sdkclients |
