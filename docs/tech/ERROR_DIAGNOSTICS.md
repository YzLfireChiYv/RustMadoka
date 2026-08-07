# 错误诊断手册（用户中文 · 机读码 · 供其它 AI）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-07（§模块结果：标签非完备） |
| **性质** | 错误分类与模块结果标签说明；**换模型/跨会话**分析失败时以本文 + 日志中的 `code` 为准 |
| **实现** | `crates/rustmadoka-core/src/diag.rs` · `error.rs`（含 `from_game_api_errors`）· `client.rs` · `modules/mod.rs` |
| **Inbound** | `modules/mod.rs` · app `lib.rs` · [W2_WIRE…](./W2_WIRE_ANALYSIS_AND_REWRITE_LIST.md) · NORMS P25 · L13 |
| **调用** | `CoreError::diagnose()` / `user_block_zh()` / `module_log_zh()` · app `err_zh()` |
| **产品** | [PLAN_NEXT_FOOLPROOF_AND_DIAG.md](../PLAN_NEXT_FOOLPROOF_AND_DIAG.md) |
| **MAY CONTAIN ERRORS** | Yes — 启发式原因非医学鉴定；业务码表未穷尽 |

---

## 模块结果标签（当前 · 非完备）

任务日志里模块 `status` 与一键汇总用的是**产品展示标签**，用于「今天这轮跑完观感如何」，**不是**游戏服务器结局的完备枚举，也不是 HTTP 状态码的 1:1 投影。

### 当前 Rust 调度层（`run_daily`）

| 标签 | 来源 | 计入 `ok=false`？ | 说明 |
|------|------|-------------------|------|
| **成功** | `Ok(log)` | 否 | 模块返回 Ok；`log` 内可能写「部分轮次」「空操作说明」等，**未**单独拆「部分成功」标签 |
| **跳过** | `CoreError::Skip` | 否 | 有意不执行、条件不满足、无可领等（各模块自行约定文案） |
| **中止** | `CoreError::Abort` | 是（`aborted`） | 常见：队伍解析失败、缺必填配置；**配置问题**，与协议挂掉不同 |
| **错误** | 其它 `CoreError` | 是（`errors`） | 网络/HTTP/登录/未映射业务码等；展示常走 `module_log_zh` 诊断块 |
| （整次）用户放弃 | `RunControlFlags` | 是（任务 `aborted`） | 后续模块不跑；与单模块「中止」不同 |

汇总文案应分写 **成功 / 跳过 / 中止 / 错误**（`DailyReport::summary_counts_zh`）。兼容字段 `failed = aborted + errors` 仍保留，**勿**在 UI 上只显示「失败」掩盖中止。

### 原版 Python 对照（archive）

`eResultStatus`：`成功` · `跳过` · `警告` · `中止` · `错误` · `致命`（六种）。  
整次任务还可因 WARNING/ABORT 抬 WARNING、因 ERROR/PANIC 抬 ERROR。  
**Rust 当前未实现「警告」「致命」独立标签**；未声称与原版一一等价。

### 已知缝隙（文档禁止写成「已覆盖全部」）

| 缝隙 | 现状 |
|------|------|
| **部分成功** | 如 `super_sweep` 计划 3 轮只完成 1 轮仍可能 `成功`（`ok_rounds>0`）；细节在 `log` |
| **空领取 vs 跳过** | **OUT-EMPTY 口径（2026-08-07）：** 无可领/无操作/无新内容 → `Skip`。已统一：present、raid_reward、shop 无兑换、eventscenario/collection/battle_mission 无内容、like/support 0 次、event 本轮空、freegacha 无免费等。以 `daily.rs` 为准。 |
| **部分完成** | **OUT-PARTIAL：** 仍标模块「成功」，log **首行**含 `【部分完成】` 与计数（例：super_sweep 完成 1/3 轮；gather 已加速未领取）。无独立展示标签。 |
| **业务码表** | `from_game_api_errors` 仅映射已采样码（18027/18044/18054/19001 等）；**OUT-BIZCODE：本批无新 wire 证据，未扩表** |
| **警告类** | 原版有 WARNING；Rust 无独立档，常被挤进成功 log 或错误 |
| **同码多义** | 同一 `code` 在不同 API 可能含义不同；表项须绑步骤证据 |

**证据优先级：** 当前真机 wire / task_log → 当前源码 → 本文 → 旧 HANDOFF 叙述。

---

## 0. 其它 AI 如何用本文分析一份失败日志

1. 打开任务会话 JSON（`task_logs/.../*.json`）或用户粘贴的完整日志。  
2. 看 **`message`**（整次失败时）或模块 **`log`**（单模块失败时）。  
3. 提取 **`【CODE】`** 或文中的 **`[CODE]`**（如 `NET_TIMEOUT`、`HTTP_428_VERSION`）。  
4. 在本文 §2 查该 code → 类别、含义、可能原因、建议操作、相关源码。  
5. 若 **`modules: []` 且 status=error**：登录前/登录中失败（指纹、Gree、网络），**不是**日常模块逻辑全挂。见 [PLAN_NEXT §2](../PLAN_NEXT_FOOLPROOF_AND_DIAG.md)。  
6. 若只有 HTTP 数字无中文：属旧版本日志；用 `diagnose_text(原始串)` 逻辑或本文 HTTP 表反推。  
7. **禁止**仅根据 status 数字下定论而不写可能原因。

### 日志字段约定

| 字段 | 含义 |
|------|------|
| `status` | `running` / `success` / `error` / `aborted` / … |
| `message` | **整次**任务定稿说明；失败时应为中文诊断块（含 code） |
| `modules[].log` | 单模块结果；错误时为短中文 + code |
| `modules: []` | 未进入模块调度（常见：指纹/登录失败） |

### 诊断块形状（用户可见）

```text
【NET_TIMEOUT】游戏API请求：连接或等待超时
在「游戏API请求」阶段与远程服务器通信失败。…

可能原因：
  1. …
建议操作：
  1. …
技术细节：…
```

---

## 1. 错误类别总览

| category | 说明 | 典型 code 前缀 |
|----------|------|----------------|
| `network` | TCP/TLS/超时/DNS/代理 | `NET_*` |
| `http` | HTTP 非 200（含 428/401/403/404/5xx） | `HTTP_*` |
| `login` | Gree / 游戏登录 / 会话 | `LOGIN_*` |
| `fingerprint` | 缺指纹、坏 JSON、渠道无条目 | `FP_*` |
| `crypto` | AES/msgpack/Unpad | `CRYPTO_*` |
| `api` | 业务 payload 错误 | `API_*` |
| `module` | 跳过/中止（非系统崩溃） | `MODULE_*` |
| `io` / `json` / `config` | 本地文件、解析、门禁 | `IO` `JSON` `GATE_*` |
| `other` | 未归类 | `OTHER` |

---

## 2. 错误码表（稳定 code）

### 2.1 网络 `NET_*`

| code | 含义 | 可能情况 | 建议 | 源码锚点 |
|------|------|----------|------|----------|
| **NET_TIMEOUT** | 连接或读超时 | 加速卡顿；地区不对链路挂起；防火墙 | 换节点；核对日/美 IP；重试 | `diag::classify_reqwest` · client/gree 超时 12s/25s |
| **NET_CONNECT** | 连不上主机 | 无网；DNS；域名被拦；错代理 | 查系统网络；调加速分流 | 同上 |
| **NET_DNS** | 域名解析失败 | DNS 污染；加速 DNS 模式 | 换 DNS/节点 | 同上 |
| **NET_TLS** | TLS/证书失败 | 系统时间错；HTTPS 解密代理 | 对时；关中间人代理 | 同上 |
| **NET_PROXY** | 代理错误 | 系统代理指向死端口 | 查代理设置 | 同上 |
| **NET_OTHER** | 其它网络 | 限流、重置、未知 | 重试；复制报告 | 同上 |

**地区启发式（产品层，非 code）：**  
日服账号 → 期望日本出口；国际服 → 常期望美国。检测为「一键排查」能力，**默认警告不硬拦**。

### 2.2 HTTP `HTTP_*`

| code | HTTP | 含义 | 可能情况 | 建议 |
|------|------|------|----------|------|
| **HTTP_428_VERSION** | 428 | 版本指纹不匹配 | 指纹旧；日/国服串用；缓存脏 | 更新指纹；确认渠道；升级 exe |
| **HTTP_401_SESSION** | 401 | 会话失效 | 过期；多端挤号；token 坏 | 重登；清 `cache/token` |
| **HTTP_403** | 403 | 拒绝访问 | 地区；鉴权；WAF | 换符合地区的 IP；核密码 |
| **LOGIN_GREE_SIGNATURE** | 常 403 + Invalid Signature | Gree 签名失败 | 密码错；token 坏；签名实现回归 | 核引继密码；删 token；见 LESSONS L1 |
| **HTTP_404** | 404 | 路径不存在 **或** 解密失败体 | API 变更；Unpad 后误读；错服 | 更指纹；对渠道；大量 404 报维护者 |
| **HTTP_5XX** | 5xx | 官方服务端错误 | 维护、过载 | 等待重试 |
| **HTTP_OTHER** | 其它 | 非 200 | 网关异常等 | 看技术细节；重试 |

**禁止：** 日志只写 `404` 不写中文与可能情况（实现上经 `classify_http`）。

### 2.3 登录 `LOGIN_*`

| code | 含义 | 可能情况 | 建议 | 文档 |
|------|------|----------|------|------|
| **LOGIN_GREE_SIGNATURE** | 见上 | 见上 | 见上 | LESSONS L1 · SDK_AND_LOGIN |
| **LOGIN_GREE_INACTIVE** | 账号未激活类 | 需 active/update | 程序会尝试；仍失败核账号 | gree.rs |
| **LOGIN_CREDENTIALS** | 凭证问题 | 引继/密码错 | 重填 | — |
| **LOGIN_CHANNEL_UNSUPPORTED** | 台服等未实现 | Sonet 未移植 | 用 jp/en | CHANNELS · HANDOFF |
| **LOGIN_SESSION_EXPIRED** | 会话过期 | 同 401 | 重登 | — |
| **LOGIN_OTHER** | 其它登录失败 | 综合 | 看细节 | — |

### 2.4 指纹 `FP_*`

| code | 含义 | 可能情况 | 建议 |
|------|------|----------|------|
| **FP_MISSING** | 无可用指纹 | 只发 exe；GitHub 不通；无 cache/内置 | 一键排查；publish 旁路；内置指纹版 |
| **FP_INVALID** | JSON/字段坏 | 粘贴损坏 | 重新拉取 |
| **FP_CHANNEL** | 当前服无条目 | 合集缺 jp/en/tw | 换源或补渠道 |

加载顺序（实现演进中）：cache → 内置 → 旁路 publish → 远程 → version.json。见 PLAN_NEXT。

### 2.5 加密 `CRYPTO_*`

| code | 含义 | 可能情况 | 建议 | 文档 |
|------|------|----------|------|------|
| **CRYPTO_UNPAD** | AES Unpad 失败 | 密钥错；响应非密文 | 更指纹；查渠道；开发回归 | LESSONS L2 |
| **CRYPTO_OTHER** | 其它加解密 | msgpack 形状等 | 报告维护者 | PROTOCOL_STACK |

### 2.6 模块 / 门禁

| code | 含义 | 说明 |
|------|------|------|
| **MODULE_SKIP** | 跳过 | 正常：无事可做、条件不足 |
| **MODULE_ABORT** | 中止 | 缺队伍/关卡等配置；防呆 |
| **GATE_DENIED** | 门禁 | `ALLOW_DAILY_RUN` / 工具门禁 |
| **API_BUSINESS** | 业务错误 | 看服务器消息 |
| **IO** / **JSON** / **CONFIG** / **OTHER** | 本地与杂项 | 权限、坏文件、未归类 |

---

## 3. 实现对照（Outbound）

| 路径 | 职责 |
|------|------|
| `crates/rustmadoka-core/src/diag.rs` | 分类、净化 body、中文块、单测 |
| `crates/rustmadoka-core/src/error.rs` | `CoreError`；`http_status`；**`from_game_api_errors`（W3 业务码→Skip）** |
| `crates/rustmadoka-core/src/client.rs` | 游戏 HTTP → `network_from_reqwest` / `http_status`；errors[] 走 `from_game_api_errors` |
| `crates/rustmadoka-core/src/gree.rs` | Gree HTTP；Login 串含 status+净化 body |
| `crates/rustmadoka-core/src/modules/mod.rs` | 模块错误 → `module_log_zh()`；Skip 不进 failed |
| `crates/rustmadoka-app/src/lib.rs` | CLI/HTTP 任务中文；`run module` 的 status 字段 |
| `crates/rustmadoka-app/static/index.html` | 日志列表突出 message；完整日志人话+JSON |
| 业务码表与验收 | `docs/tech/W2_WIRE_ANALYSIS_AND_REWRITE_LIST.md` §2 · §8 |

---

## 4. 兼容与稳定性注意（代码审核摘要）

| 点 | 做法 |
|----|------|
| 超时 | Gree/游戏 HTTP connect 12s、total 25s，避免无限挂起 |
| 乱码 body | `sanitize_body`：控制字符与高二进制比例 → 中文说明而非 `\u0001` 倾倒 |
| 网络错误存储 | `NET_*|阶段|reqwest原文` 便于 diagnose 还原 |
| Display vs 用户文案 | `Display` 短机读；用户/日志用 `user_block_zh` |
| modules 空 | 产品与文档双写，避免误判「日常全坏」 |

---

## 5. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-06 | 首版：与 diag.rs 同步；供跨 AI 分析 |
