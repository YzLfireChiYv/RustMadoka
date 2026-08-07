# 日志中文 ↔ 内部字段对照表

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-06 |
| **用途** | 用户日志偏中文；AI/维护对照内部字段 |
| **Outbound** | `crates/rustmadoka-app/src/task_log.rs` · `static/index.html` · `crates/rustmadoka-core/src/diag.rs` |
| **Inbound** | [ERROR_DIAGNOSTICS.md](./ERROR_DIAGNOSTICS.md) · [UI_ROUTING_AND_TASK_LOGS.md](./UI_ROUTING_AND_TASK_LOGS.md) |

## 任务会话

| 中文/界面 | 字段 |
|-----------|------|
| 一键清日常 | `trigger=one_click_daily` |
| 单独运行 | `trigger=single_module` |
| 失败 | `status=error` |
| 成功 | `status=success` |
| 说明/诊断块 | `message` |
| 模块结果空 | `modules=[]` → 登录前失败 |

## 指纹槽

| 中文 | id / 文件 |
|------|-----------|
| 默认·程序内置 | `default_embedded` · 编译 `EMBEDDED_COMBINED_JSON` |
| 默认·已更新缓存 | `default_pulled` · `fp_slots.json` + cache |
| 自定义槽 1/2 | `custom_0` / `custom_1` |
| 启用 | `active_slot_id` |

## 默认源刷新 status

| 中文 | status |
|------|--------|
| 拉取不到 | `unreachable` |
| 格式错误 | `bad_format` |
| 换上了更新的 | `updated` |
| 已是最新 | `already_latest` |

## 配置短码 AM2

| 中文 | 字段 |
|------|------|
| 整卡配置 | `AM2.CFG.u1_7.s2.{payload}.{crc}` |
| 三店 | `AM2.SHOP3…` |
| 活动/raid/jjc 单店 | `SHOPe` / `SHOPr` / `SHOPa` |
| 上游兼容 | `UPSTREAM_COMPAT` = `1.7`（手维） |
| 我方 schema | `CONFIG_PACK_SCHEMA` = 2 |
| 旧 AM1 | **作废** |

## 版本展示

| 中文 | 来源 |
|------|------|
| 构建时间到分钟 | `build_stamp` / `AUTOMADOKA_BUILD_STAMP` |
| 上游兼容 | `upstream_compat` |

## 修订

| 日期 | 内容 |
|------|------|
| 2026-08-06 | 随槽位 UI 首版 |
| 2026-08-06 | AM2 短码 · 版本字段 |
