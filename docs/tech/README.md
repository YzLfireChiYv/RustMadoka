# 技术文档索引（现行 · RustMadoka）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-08（过时文档归档后） |
| **失真声明** | **AI 维护，有可能出错和失真。** 以源码与 `RustMadoka_data` 真机为准。 |
| **范围** | 现行 `crates/rustmadoka-*` · 原版对照 `archive/pre-rust-2026-08/autopcr` |
| **历史文档** | **不删** · 已移入 [archive/docs-tech-historical-2026-08/](../../archive/docs-tech-historical-2026-08/) |
| **规范** | [NORMS.md](../NORMS.md) · [TECH_DOC_CONVENTION.md](./TECH_DOC_CONVENTION.md) |
| **交接入口** | [HANDOFF.md](../HANDOFF.md) |
| **人类全流程** | [HUMAN_FLOW_REPORT_FROM_PASSWORD_TO_FEATURES.md](./HUMAN_FLOW_REPORT_FROM_PASSWORD_TO_FEATURES.md) |

---

## 1. 现行必读（按主题）

| 主题 | 文档 |
|------|------|
| 从密码到功能（人话+对服） | [HUMAN_FLOW_REPORT…](./HUMAN_FLOW_REPORT_FROM_PASSWORD_TO_FEATURES.md) |
| 数据夹布局 / 家用安全 | [DATA_FOLDER_LAYOUT.md](./DATA_FOLDER_LAYOUT.md) |
| Owner / CLI / 会话池 / 心跳 | [INSTANCE_AND_CLI.md](./INSTANCE_AND_CLI.md) |
| 协议加密管道 | [PROTOCOL_STACK.md](./PROTOCOL_STACK.md) |
| 登录 Gree / 初始化串 | [SDK_AND_LOGIN.md](./SDK_AND_LOGIN.md) · [INIT_AND_RESPONSE_PAYLOADS.md](./INIT_AND_RESPONSE_PAYLOADS.md) |
| 指纹 | [VERSION_FINGERPRINT.md](./VERSION_FINGERPRINT.md) |
| 日常语义 / 扫荡 | [MODULE_SEMANTIC_CLASSIFICATION.md](./MODULE_SEMANTIC_CLASSIFICATION.md) · [BASIC_SUPER_SWEEP…](./BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS.md) |
| 组队（将改名「魔女群殴」见 TASK 积压） | [GROUP_RAID_AND_DEVICE_IDENTITY.md](./GROUP_RAID_AND_DEVICE_IDENTITY.md) |
| 多组监视 UI | [MULTI_GROUP_UI_AND_MONITOR_SPEC.md](./MULTI_GROUP_UI_AND_MONITOR_SPEC.md) |
| 错误诊断 | [ERROR_DIAGNOSTICS.md](./ERROR_DIAGNOSTICS.md) |
| CLI ⊇ 网页 | [CLI_WEB_PARITY.md](./CLI_WEB_PARITY.md) |
| wire 开发版 | [WIRE_AND_DEBUG_PROBES.md](./WIRE_AND_DEBUG_PROBES.md) |
| 原版研究缺口 | [AUTOMADOKA_RESEARCH_AND_RUST_GAP.md](./AUTOMADOKA_RESEARCH_AND_RUST_GAP.md) |
| 教训 L* / C* | [LESSONS_RUST_PORT.md](./LESSONS_RUST_PORT.md) · [LESSONS_SESSION_COLLAB.md](./LESSONS_SESSION_COLLAB.md) |
| 数据 / mst | [DATA_AND_MST.md](./DATA_AND_MST.md) |
| 渠道 / API 表 | [CHANNELS.md](./CHANNELS.md) · [API_INVENTORY.md](./API_INVENTORY.md) |
| UI 路由日志 | [UI_ROUTING_AND_TASK_LOGS.md](./UI_ROUTING_AND_TASK_LOGS.md) |
| Android（待重构） | [ANDROID_DUAL_PLATFORM.md](./ANDROID_DUAL_PLATFORM.md) |
| 上游对照 | [UPSTREAM_SOURCE_AND_WIRE.md](./UPSTREAM_SOURCE_AND_WIRE.md) · [UPSTREAM_FILE_MAP.md](./UPSTREAM_FILE_MAP.md) · [UPSTREAM_FOR_LLM…](./UPSTREAM_FOR_LLM_CONTRIBUTORS.md) |
| 洗词条 / 队伍解析 | [WASH_CHARACTER_LIST.md](./WASH_CHARACTER_LIST.md) · [PARTY_TEAM_RESOLVE.md](./PARTY_TEAM_RESOLVE.md) |
| 游戏命名 / 框架 | [GAME_NAMING_GLOSSARY.md](./GAME_NAMING_GLOSSARY.md) · [GAME_FEATURE_FRAMEWORK.md](./GAME_FEATURE_FRAMEWORK.md) |
| 指纹通知 / toast | [WINDOWS_SYSTEM_NOTIFY.md](./WINDOWS_SYSTEM_NOTIFY.md) |
| 安全审计（协作卫生为主；家用见 NORMS P9c） | [SECURITY_AND_PRIVACY_AUDIT.md](./SECURITY_AND_PRIVACY_AUDIT.md) |
| 超越证据表 | [SURPASS_EVIDENCE_TABLE.md](./SURPASS_EVIDENCE_TABLE.md) |
| 日志中文 | [LOG_ZH_MAP.md](./LOG_ZH_MAP.md) |

---

## 2. 已归档（勿当现行完成定义）

路径：`archive/docs-tech-historical-2026-08/` · 见该处 README。  
历史 PLAN：`archive/docs-plan-historical-2026-08/`。

---

## 3. 修订

| 墙钟 | 内容 |
|------|------|
| 2026-08-07 | 旧索引（含 Python 时代全表） |
| **2026-08-08** | **现行/历史拆分；17 份 tech + 10 份 PLAN 归档** |
