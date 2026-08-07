# Version fingerprint (version / sign / libcount / sm) — complete

| Item | Content |
|------|---------|
| **Wall clock** | 2026-08-07 23:53（产品实现状态增补） |
| **Outbound** | `archive/pre-rust-2026-08/autopcr/core/version.py` · Rust `crates/rustmadoka-core/src/fingerprint.rs` · `publish/automadoka.json` · app `fp_slots.rs` · `fp_load.rs` · `build.rs` embed |
| **Authority git** | `origin/main` @ `9826135` |
| **Inbound** | [PROTOCOL_STACK.md](./PROTOCOL_STACK.md) · [SDK_AND_LOGIN.md](./SDK_AND_LOGIN.md) · NORMS **P15 · P29** · [CLI_WEB_PARITY.md](./CLI_WEB_PARITY.md) |
| **MAY CONTAIN ERRORS** | Yes — 无主人点测不写 FIXED |

---

## 1. Three fields + sm

| Field | How obtained from XAPK | Used as |
|-------|------------------------|---------|
| version | `manifest.json` → `version_name` | LoginApi `appVersion`; Gree payload appVersion |
| sign | **base** split APK entire file **MD5** hex | sm |
| libcount | **config.arm64_v8a** zip count of `lib/arm64-v8a/*` | sm |

```text
sm = f"d{sign}o{libcount}1E88A0177575728C9A399A9BD1F43A11D4100065n"
```

Source property: `AppInfo.sm` in `version.py`.  
Injection: every `RequestBase.prepare()` sets `self.sm = version_info.sm`.

Optional metadata (product publish JSON): `package_id` / `channel`.

## 2. Disk layout

| Path | Content |
|------|---------|
| Python `cache/version.json` | version, sign, libcount |
| Rust `automadoka_data/cache/version.json` | same shape via Fingerprint helpers |
| Product remote | rules raw URL only (P15) |
| Embed | `publish/automadoka.json` → `EMBEDDED_COMBINED_JSON` |

## 3. Upstream update path (`version._update_version_sync`)

Default upstream behavior (product **does not** use as main path):

1. Stream XAPK from APKPure sample URL (EN package in source).  
2. Read manifest version_name; if same as current, skip.  
3. MD5 base APK; count arm64 libs; save version.json.

Triggered on HTTP **428** → `update_version()` → `VersionUpdatedException` → sessionmgr re-login.

Product replacement: cloud fingerprint JSON (rules) + local XAPK extract CLI + embed.

## 4. Extract algorithm (Rust `extract_from_xapk`, aligned with Python)

1. Open XAPK as zip; read `manifest.json`.  
2. Find split id `base` and `config.arm64_v8a`.  
3. MD5 full base APK bytes → sign.  
4. Nested zip: count names starting with `lib/arm64-v8a/`.  
5. Infer channel from package_name suffix `.jp` / `.en` / `.tw`.

## 5. Published sample (this repo)

See `publish/automadoka.json` and prior 3.13.0 table in history; regenerate with product CLI when packages update.

## 6. Empirical open questions

| ID | Question |
|----|----------|
| E-V1 | Hotfix without store version change → 428? |
| E-V2 | Refreshing triple alone recovers login? |
| E-V3 | Split id names stable across stores? |

## 7. Product implementation status（RustMadoka · 2026-08-07）

| 产品要求 | 实现 | 说明 |
|----------|------|------|
| exe **内嵌** combined 指纹 | **CODE** | 编译期 `EMBEDDED_COMBINED_JSON`；槽 `default_embedded`；拷走 exe 即带保底 |
| GitHub **rules raw 热更新** | **CODE** | `refresh_default_source` → 数据夹槽 `default_pulled` + cache；**不改写 exe 字节** |
| 刷新后登录用新指纹 | **CODE 方向** | 拉取成功自动启用拉取槽；`fp_load` 槽优先 |
| Owner **静默日检** | **CODE** | 每天最多一次 |
| 网页展示版本/刷新结果 | **CODE（API/SPA）** | CLI 全槽操作面仍弱于网页（P23 债） |
| 仅拷 data 不含 exe | 拉取槽在 data | 与「exe 单独可拷」分工明确 |

**禁止：** 把「内嵌存在」单独写成产品全部完成；P29 四条须一起看。

---

## 8. Revision

| Date | Content |
|------|---------|
| 2026-08-06 | First topic doc |
| 2026-08-06 | 3.13.0 samples |
| 2026-08-07 06:04 | DOC-FULL-01: full extract + sm + product vs upstream |
| 2026-08-07 23:53 | §7 产品实现状态（内嵌+热更+拷贝模型） |
