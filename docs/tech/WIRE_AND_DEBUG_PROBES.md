# 技术规格：通讯录制（wire）与 Debug 探针

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-07 19:00（本机） |
| **Outbound 源码** | `crates/rustmadoka-core/src/wire.rs` · `client.rs` · `gree.rs` · `modules/daily.rs`（super_sweep 探针）· `crates/rustmadoka-app/src/wire_scope.rs` · `run_ops.rs` · `http_server.rs`（health.debug）· `static/index.html`（长超时） |
| **Inbound** | HANDOFF · NORMS P3 · PLAN 双 exe · W2_WIRE_ANALYSIS |
| **MAY CONTAIN ERRORS** | Yes — 以 debug 构建与 `RustMadoka_data/wire/` 实盘为准 |

---

## 1. 产品规则（完整条件）

1. **仅开发版**（`RustMadoka_debug.exe`，feature `wire_record`）写入通讯录制与探针。  
2. **普通版**不写 wire、不暴露敏感探针细节以外的 health 字段可为空。  
3. **无差别记录**对游戏服务器与 Gree SDK 的 HTTP：成功与失败路径均写入 `events.jsonl`（明文 payload、envelope、密文 base64、状态码、错误、`duration_ms`）。  
4. 会话由 `wire::ensure_started` 在 **full_login / login_for_info / WireScope** 时打开；已打开则复用。  
5. 任务结束（单模块 / 日常 / 流式日常）调用 `wire::stop` 写 `session_end.json`。  
6. wire 落在 **数据文件夹**，**不进 git**、**不打进分发普通包**（P8/P8b）。

---

## 2. 磁盘布局

```text
RustMadoka_data/wire/{alias_sanitized}/{session_id}/
  meta.json          # schema 2、alias、channel、purpose、doc 链
  events.jsonl       # 每行一个事件：game_api | sdk_http | note/probe
  session_end.json   # stop 时事件数与结束时间
```

### 2.1 事件 kind

| kind | 含义 |
|------|------|
| `game_api` | 游戏 msgpack API（`client::request_raw`） |
| `sdk_http` | Gree OAuth/HTTP（`gree`） |
| `note` / probe 字段 | 测试探针：`login_begin`、`module_begin`、`super_sweep_enter` 等 |

---

## 3. 超时与「本机无响应」（已修）

| 层 | 旧问题 | 现口径 |
|----|--------|--------|
| 浏览器 `api()` | 默认 **20s** Abort → 快速刷图仍在跑却显示「本机无响应」 | 单模块 `super_sweep` 等 **900s**；一般模块 **300s**；日常流 **900s** |
| 游戏 HTTP 客户端 | 单请求 **25s** | **120s**（单 API） |
| Gree HTTP | **25s** | **90s** |

超时文案须说明「可能仍在打游戏服」，禁止把长任务一律说成 exe 死了。

---

## 4. Health 探针字段

`GET /api/health` 增加：

```json
"debug": {
  "wire_built": true,
  "wire_active": false,
  "wire_dir": null,
  "doc": "docs/tech/WIRE_AND_DEBUG_PROBES.md"
}
```

---

## 5. 与规范

| 规范 | 本文件如何满足 |
|------|----------------|
| P3 双 exe | wire 仅 debug feature |
| P6 log | 变更写 `docs/logs/` |
| P21/P22 | 代码头链本文；本文 Outbound 链源码 |
| P8 | wire 含通讯材料不进公开仓 |

---

## 6. 修订

| 墙钟 | 内容 |
|------|------|
| 2026-08-07 19:00 | 首版：全量 wire ensure、探针、超时修复说明 |
