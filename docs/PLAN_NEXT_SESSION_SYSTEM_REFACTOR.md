# 任务书：下一会话主线 — 系统重构 · 数据目录更名 · 多组 UI · 程序运行监视

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-07 |
| **状态** | **PLAN · 下一对话主入口** |
| **命名权** | 本文件 ID/产品名由 AI 拟定；允许后续改名，改名时同步 TASKBOARD/HANDOFF |
| **规范** | NORMS **G9**（全称）· **P7c**（整批）· **P26**（不因性能砍体验）· **P28**（提问门槛）· P5 · P16 · P22 |
| **Inbound** | [TASKBOARD.md](./TASKBOARD.md) · [HANDOFF.md](./HANDOFF.md) · [TASK_INVENTORY.md](./TASK_INVENTORY.md) · 主人当轮口令 |
| **Outbound** | `crates/*` · `static/index.html` · `scripts/*` · 本文件修订 |
| **MAY CONTAIN ERRORS** | Yes — 开工以源码与当轮点名为准 |
| **完整规格正文（禁止以本任务书表格代替）** | **[tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md](./tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md)** — 多用户组、监视分层、主页/设置分流、EN/JP/TW、数据文件夹更名意图等**全文** |
| **历史任务检索汇报** | [logs/2026-08-07-historical-tasks-found.md](./logs/2026-08-07-historical-tasks-found.md) |
| **记录纪律** | NORMS **G10**：禁止压缩主人原意；只许更详更完整 |
| **任务清理** | [logs/2026-08-07-handoff-prune.md](./logs/2026-08-07-handoff-prune.md) — 已剔除完成项与被上位替代项 |

---

## 0.−1 已从本任务书「待办展开」中剔除的内容（完整说明）

下列内容**曾经**出现在历史任务表或本文件早期草稿中，按主人要求在全面交接前**剔除出默认待办**，以免下一会话重复开工或与上位规格打架。

### 已完成（勿再当未做）

W1 wire 与 EN 采样；W2 分析文档；W3 可落地语义批次（**不是** C01 日常 FIXED）；双 exe 构建链；E1 会话导出；模块「部分完成」独立状态；空操作改跳过一批；占用心跳与端口中文确认的**代码实现**（真机手测仍可做，见 TASKBOARD PORT-SMOKE/HEART-SMOKE）；禁止自动结束普通版进程；CLI 写入 task_logs 与 run_config_snapshot；多份规范（G7/G8/G9/G10、P9 明文等）；DOC 双向链接一批补洞。

### 被上位替代（勿再按旧窄规格单独实现）

| 旧项 | 上位完整规格 |
|------|----------------|
| **C07 / PLAN_RUN_PANEL_AND_THEME §1**「仅把浏览器网页前端旧运行条改成是否开启」 | **PROC-MONITOR** + [MULTI_GROUP_UI_AND_MONITOR_SPEC.md](./tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md) 全文 §3–§4：程序运行面板终端完整只读监视、本体专有与面向用户组账号分层、主页精简可选监视、设置页进度条、非异常不可控制、叉掉关闭、跨组主页不串流等。旧 PLAN 文件**保留只读史**，实现时以 MULTI_GROUP 文为准。 |

### 本任务书仍然有效的开放包

见下文 §2（SYS-AUDIT-REFACTOR · DATA-DIR-RENAME · WEB-SYNC-CORE · PROC-MONITOR · CARD-CROSS-GROUP · CHANNEL-BADGE · PARTY-SELECT）与 §1.1 FIX-*。WAIT/LATER 见 TASKBOARD 清理后正文。

---

## 0. 产品用词（本任务书固定 · 禁止再混）

> **重要：** 下列表格仅为导航。主人当轮关于程序运行面板终端、浏览器网页前端、多用户组卡片、设置进度条、数据文件夹更名等**完整条件句**，以 [tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md](./tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md) **全文各节**为准。实现与验收不得只读本表。

| 全称 | 含义 | 禁止简写为 |
|------|------|------------|
| **程序运行面板终端** | 双击 exe 后的黑色窗口（本体进程 UI） | 控制台 / 运行面板（歧义） |
| **浏览器网页前端** | 浏览器打开的 loopback SPA | SPA / Web（单独使用时易混） |
| **数据文件夹** | 旁路运行时目录（更名后见 RENAME） | data（单独） |
| **用户组** | 工具内账号分组（加密/明文组） | 组（可，但首次写全） |
| **游戏账号卡片** | 组内某一游戏号条目 | 卡 / 账号（单独易混） |
| **游戏身份** | 渠道 + 引继（跨组同一真实号） | — |
| **本体专有提示** | 仅程序运行面板终端：端口、占用、Owner、双版本 | — |
| **账号监视提示** | 面向某用户组+游戏账号卡片的任务流；可精简到主页 | — |

---

## 1. 余下任务总表（收集 · 与 TASKBOARD 同步）

### 1.1 语义假成功（短线 · 可先于大重构或并入）

| ID | 内容 | 状态 | 规格 |
|----|------|------|------|
| **FIX-LOGINBONUS** | `loginBonusDataList` 空→跳过；非空→成功 | TODO | [known-issues-before-fix](./logs/2026-08-07-known-issues-before-fix.md) |
| **FIX-EVENT-EMPTY** | 活动次数 0 且无战斗/skip→跳过 | TODO | 同上 |
| **FIX-EVENT-LOG** | 活动日志写活动名/id/操作 | TODO | 同上 |
| **FIX-SCENARIO-DEDUP** | 已读剧情不重复成功 | TODO | 同上 |

### 1.2 历史已立项、易被漏掉（须进下一会话队列）

| ID | 内容 | 状态 | 规格真源 |
|----|------|------|----------|
| **PARTY-SELECT**（原 C10） | 需设队伍的功能：列表选择与自行输入二选一；底层已有 `partyDataList` | **PLAN 未实现** · 已挂主队列 | [PLAN_PARTY_SELECT_UX.md](./PLAN_PARTY_SELECT_UX.md) 全文 |
| **19001-V · C01 · W4-REL · AUD-COMMS-A · Android-B · CLI-M-wash** | 见 TASKBOARD §3 | WAIT | 各 PLAN |
| **PORT-SMOKE / HEART-SMOKE** | 端口与心跳**真机手测**（实现已有） | WAIT 手测 | INSTANCE |
| **STORY-P · W5 · THEME-LIGHT · DOC-FULL-02 · Sonet · R4** | 见 TASKBOARD §4 | LATER | 各 PLAN |

### 1.3 仍指导本主线的既有产品立场（非独立待办票）

底层能力产品化（支撑 PARTY-SELECT）；明文 token 与协作卫生（P8–P10，更名不改）；同数据文件夹单 Owner + 跨路径心跳文件（路径随 RENAME）；对服不乱发包（P27）；secret 配置化后置（C19，不抢本主线）；指纹 rules（P15）；双 exe 形态（DUAL，已 CODE）。完整设想检索见 [historical-tasks-found.md](./logs/2026-08-07-historical-tasks-found.md)。

---

## 2. 下一会话六大工作包（主人当轮主任务）

### 包 A — **SYS-AUDIT-REFACTOR**（系统检查与重构）

| 项 | 内容 |
|----|------|
| **范围** | `rustmadoka-core` / `rustmadoka-app` / `static` / mobile 入口；模块语义、错误三态、IPC、Owner、日志 |
| **动作** | 全面阅读关键路径；列债；按功能身份+I/O+成功跳过失败重构；并入 FIX-* |
| **纪律** | C20 禁止未理解照搬；P25；改完双向链接 |
| **完成定义** | 债表落盘 + 高优先级假成功已修 + 编译通过；不写 FIXED |

### 包 B — **DATA-DIR-RENAME**（数据文件夹更名）

| 项 | 内容 |
|----|------|
| **旧名** | `automadoka_data`（过渡名，全面放弃） |
| **新名** | **`RustMadoka_data`**（旁路 exe；全平台口径一致） |
| **动作** | 默认路径常量、创建、诊断文案、文档、脚本、gitignore、迁移策略（首次启动：若仅有旧目录可提示迁移或自动复制一次——实现时定，须写清） |
| **禁止** | 把用户数据打进 exe；推送 data 到 git |
| **完成定义** | 新装默认只写 `RustMadoka_data`；双 exe 共用；文档与注释无「正式路径仍叫 automadoka_data」 |

### 包 C — **WEB-SYNC-CORE**（多用户组 · 网页同步 · 底层更安全高效）

| 项 | 内容 |
|----|------|
| **问题** | 多标签/多组共用 Owner；设置通知、进度、会话键混用易串 |
| **方向** | 统一**游戏身份键**（公开可用 `game_id_hash`，内部仍 channel+引继）；用户组维度过滤通知与主页流；设置以数据文件夹为真源；任务进度按身份广播、主页按组裁剪 |
| **安全** | 列表 API 不回传明文引继；跨组只暴露 hash + 展示态 |
| **高效** | 订阅/轮询策略分层：主页可选监视 vs 设置页进度条；避免无意义全量刷 |
| **完成定义** | 技术说明专节 + 核心 API 行为可测；与包 D/E 接口对齐 |

### 包 D — **PROC-MONITOR**（程序运行面板终端 · 高阶监视）

| 项 | 内容 |
|----|------|
| **形态** | 程序运行面板终端：**完整**日志/状态流；**非异常时不可输入、不可控制**（只查看监视）；关闭 = 叉掉窗口即可（与「关进程」一致） |
| **提示分层** | **本体专有**（端口、占用、Owner、双版本提示等）· **账号监视**（某用户组+卡片任务流，可含模块级状态） |
| **与浏览器关系** | 账号监视类经精简后，可出现在**该用户组主页**（可点开/关闭）；旧「给小白看的进度条」保留在设置/卡片区；新完整流给高阶用户 |
| **非目标** | 在终端里做完整 SPA；异常确认（端口/心跳）仍可输入（属异常路径） |
| **完成定义** | 正常运行终端只读；异常路径仍可中文确认；文档写清分层 |

### 包 E — **CARD-CROSS-GROUP**（多用户组 · 同一游戏身份 · 卡片 UI）

| 项 | 内容 |
|----|------|
| **场景** | 用户组 A、B 各有「游戏账号甲」卡片（同一游戏身份） |
| **A 组主页卡片** | 显示本机任务态：**「清日常运行中」** 或 **「某功能运行中」**（模块级，不限清日常一种文案） |
| **B 组主页卡片** | **不**显示 A 的流式明细；显示 **「在用户组 A 中运行中」**（或等价完整句）；**不**串 A 的流式数据到 B 主页 |
| **设置页进度条** | **跨组同步**同一游戏身份的进度（与主页分流：主页按组隔离流，设置条跟身份） |
| **完成定义** | A/B 对照可测；文案可改名但语义不变 |

### 包 F — **CHANNEL-BADGE**（服务器角标）

| 项 | 内容 |
|----|------|
| **显示** | 游戏账号卡片上用显眼 **EN / JP / TW**（及未知渠道完整展示） |
| **完成定义** | 列表/卡片一眼可辨；与 channel 字段一致 |

### 包 G — **PARTY-SELECT**（找回 · 队伍列表 UX）

| 项 | 内容 |
|----|------|
| **规格** | 见 PLAN_PARTY_SELECT_UX：圆点二选一 · 列表点选 · 保留手输 |
| **排期** | 与 WEB-SYNC / 设置页重构同批或紧随；**禁止再从主队列消失** |
| **完成定义** | 设置页队伍项可列表选；手输仍可用；需 full_login/缓存策略写清 |

---

## 3. 建议实施顺序（可同会话多包）

```text
1) DATA-DIR-RENAME（触面广，尽早定常量，避免后续双名）
2) FIX-* 语义假成功（小、有 wire 证据）
3) WEB-SYNC-CORE 键与 API 契约
4) CARD-CROSS-GROUP + CHANNEL-BADGE（前端+状态聚合）
5) PROC-MONITOR（终端只读 + 主页精简监视）
6) PARTY-SELECT（设置页产品化）
7) SYS-AUDIT 收口债表与双向链接
```

---

## 4. 双向链接检查（本轮抽样 · 非穷尽）

### 4.1 较好

| 代码 | 文档 |
|------|------|
| `gree.rs` | SDK_AND_LOGIN · L1 |
| `owner_lock.rs` / `occupancy.rs` | INSTANCE_AND_CLI · PLAN_AUDIT §4 |
| `wire.rs` / `error.rs` | W2 · L13 |
| `diag.rs` | ERROR_DIAGNOSTICS |
| `session_export.rs` | E1 审计 |

### 4.2 缺口 / 过时（下轮补）

| 问题 | 建议 |
|------|------|
| 全库 `automadoka_data` 字符串散落 core/app/docs/scripts | RENAME 时统一 + 注释双向 |
| `PLAN_PARTY_SELECT` 未挂 TASKBOARD 近表 | 已收入本文件 §1.2 · §2G |
| `TASKBOARD` §5 仍写「部分完成仅 log」等过时句 | 下轮改写 |
| `TASK_INVENTORY` 仍写 AUD-COMMS 建议下一、C10 未点名 | 同步本主线 |
| `INSTANCE` / 多窗进度 与 CARD-CROSS-GROUP 新规格 | 新开 tech 专节 Outbound↔run_control/static |
| `config_catalog` 队伍字段仍写「须手填」 | PARTY-SELECT 后改注释+链 PLAN |
| 历史 log 单 exe 路径 | 可保留为史；正文章节改 RustMadoka_data |

**约定：** 下轮每包收口跑一轮「改了的文件头 ↔ tech/PLAN」检查；不声称全库穷尽除非扫完。

---

## 5. 验收总则

- 无点测不写 FIXED（P5）。  
- 数据文件夹更名后：新路径可跑登录+info+设置；迁移路径有说明。  
- 多组同身份：A/B 主页文案不同；设置进度同步。  
- 程序运行面板终端：正常态只读；叉掉即退出。  
- 卡片 EN/JP/TW 显眼。  
- PARTY-SELECT：列表+手输。  
- FIX-*：第二次日常 loginbonus/event 不空成功。

---

## 6. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-07 | 首版：余下任务收集 · 六大包 · C10 找回 · 双向链接抽样 · 历史设想收集 |
