# 协议与数据依赖笔记（摘要）

> 来源：归档 `archive/pre-rust-2026-08/autopcr` 静态阅读 + 探路验证。  
> **完整技术文档：** [tech/README.md](./tech/README.md)（总览/协议栈/登录/mst/模块/探针）。  
> **完整依赖以本地实跑为准**（阶段 0.5 · [tech/EMPIRICAL_CHECKLIST.md](./tech/EMPIRICAL_CHECKLIST.md)）。

## 1. 指纹（安装包 → 本地三字段）

| 字段 | 提取 | 使用 |
|------|------|------|
| `version` | XAPK manifest `version_name` | 登录 `appVersion` 等 |
| `sign` | base 分包 MD5 | 拼进 `sm` |
| `libcount` | `config.arm64_v8a` 内 `lib/arm64-v8a/*` 数量 | 拼进 `sm` |

`sm` = `d{sign}o{libcount}1E88A0177575728C9A399A9BD1F43A11D4100065n`（见归档 `version.py`）。  
专题：[tech/VERSION_FINGERPRINT.md](./tech/VERSION_FINGERPRINT.md)

**云制品最小集：每服一份 fingerprint JSON。不要发完整 XAPK。**

HTTP **428** → 版本线失败，需更新三元组。

## 2. 登录后（不来自 XAPK 文件）

| 数据 | 典型来源 |
|------|----------|
| Master 表（角色/词条定义等） | `GetResourceMasterData` + 各 `MstApi*`，内存缓存 + revision |
| 账号技能石/词条 | `SelectionAbilityApiGetSelectionAbilityDataList` 等 |
| 清日常各玩法 | 各业务 Request（见 MODULES） |

专题：[tech/DATA_AND_MST.md](./tech/DATA_AND_MST.md) · [tech/SDK_AND_LOGIN.md](./tech/SDK_AND_LOGIN.md)

## 3. 洗词条（工具示例）

- UI 角色列表：登录后 mst → `db.style_list` / character / figure。  
- 是否已洗好：账号 selection 数据 API。  
- 仍需要合法指纹才能登录。

## 4. 对照路径

- `archive/pre-rust-2026-08/autopcr/core/version.py`  
- `archive/pre-rust-2026-08/autopcr/model/modelbase.py`（`prepare` → sm）  
- `archive/pre-rust-2026-08/autopcr/core/sessionmgr.py`（登录后 `db.update`）  
- `archive/pre-rust-2026-08/autopcr/module/modules/wash.py`  
- `archive/pre-rust-2026-08/android/.../xapk_android.py`  
- 探针：`archive/pre-rust-2026-08/autopcr/util/probe_capture.py` · [tech/PROBE_CAPTURE.md](./tech/PROBE_CAPTURE.md)
