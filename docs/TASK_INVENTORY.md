# 任务规划全景清点

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-07 07:50 |
| **用途** | 短表 [TASKBOARD.md](./TASKBOARD.md) 的展开：E1 定义、分池、日志归并、默认不做 |
| **入口** | [HANDOFF.md](./HANDOFF.md) |
| **MAY CONTAIN ERRORS** | Yes — 以主人当轮口令与源码为准 |

图例：`DONE` · `SMOKE` · `CODE` · `PLAN` · `WAIT` · `TODO` · `LATER`

---

## 0. 读法

| 池 | 驱动方 | 说明 |
|----|--------|------|
| **实现队列** | 主人点名或沿用默认序 | 可写产品代码 |
| **验收池** | 主人点测 | AI 不代写 FIXED |
| **文档已定 · 代码未做** | 点名后开工 | PLAN 齐、实现零 |
| **后置 / 默认不做** | — | 见 §7 |

**两条工作线（可交错，勿混优先级）：**

| 线 | 目标 | 当前建议头 |
|----|------|------------|
| **数据与入口** | 登录后数据可落盘可调用；CLI 与 HTTP 对称 | **E1** → AUD-COMMS → CLI-M |
| **产品化 UX** | 底层已有字段做成无脑控件 | **C10** 等（PLAN 在；未点名不写） |

---

## 1. 阶段

| 阶段 | 状态 |
|------|------|
| Z0 Win 骨架 | DONE 骨架 |
| Z1 Android 壳 | OWNER 可用；B 验收未齐 |
| Z1.5 研究+文档+规范 | 文档 DONE（含明文 token 07:28） |
| Z2 系统改进+新功能 | 门：点名；建议见 TASKBOARD §2 |

---

## 2. E1 会话快照导出（完整定义）

权威：[tech/RUST_CODEBASE_AUDIT_AND_ROADMAP.md](./tech/RUST_CODEBASE_AUDIT_AND_ROADMAP.md) §2.3、§4.2；登录串 [INIT_AND_RESPONSE_PAYLOADS.md](./tech/INIT_AND_RESPONSE_PAYLOADS.md)。

### 2.1 是什么

对某个游戏账号执行 **`full_login`**（完整登录串，不用轻量 `login_for_info`）之后，把当时已经在 `GameClient` 内存里的**游戏服业务数据**整理成 JSON，通过 **CLI** 与 **HTTP** 写到本机数据目录，供主人与 AI **二次打开分析**。

当前缺口：服务器已经给过队伍、道具、角色、配置、部分 mst 等，但重构版几乎只落账号开关与 task_log 文本；**没有**「把登录后资产整包留下来」的管道。

### 2.2 导出内容（E1 档）

| 块 | 来源 | 用途举例 |
|----|------|----------|
| `init_data` | `/api/user/get_init_data_list` 及合并字段 | `partyDataList`、持有 style/角色/卡/道具、`userParamData`… |
| `game_config` | `/api/config/get_config` | 规则对照 |
| 已缓存 mst | `bootstrap_mst` 等已在进程内的表 | 洗词条、关卡/角色定义对照 |
| 元数据 | 导出时刻 | 别名、用户组、渠道、`userId`、时间、build stamp 等 |

建议路径：

```text
automadoka_data/exports/{alias}/{timestamp}/
  manifest.json
  init_data.json
  game_config.json
  mst_….json
```

### 2.3 入口

| 入口 | 草案 |
|------|------|
| CLI | `automadoka export session -g <组> -a <别名> [--out dir]` |
| HTTP | `POST …/export/session` · `GET …/export/session/latest`（走现有鉴权） |
| IPC | 可后补；E1 最小不必 |

三条入口应调用**同一套 core 写出逻辑**。

### 2.4 与明文 / 协作卫生的关系（P8 / P9）

| 点 | 口径 |
|----|------|
| 本机落盘 | **允许明文**（P9）。exports 在 data 下，**gitignore**。 |
| 必须防的 | 导出目录与 token **不进 git**（P8）；**不打进**分发 exe（P8b）。 |
| Gree 私钥 / 引继 / 游戏密码 | **默认不必写入** E1 包：分析队伍与 mst 通常不需要；若写入会增加「整夹拷贝 exports 时误带设备材料」的协作风险。需要整机调试时可用**显式开关**再附带（本机明文仍合法）。 |
| 与 E0 | E0 = 本工具设置/开关导出（已有 `export_settings` 类）。E1 = **游戏服会话业务数据**。两套分开。 |

### 2.5 E1 与相邻档

| 档 | 内容 | 状态 |
|----|------|------|
| E0 | 账号配置/开关 | 已有 HTTP 类能力 |
| **E1** | full_login 内存快照 | **CODE · CLI 已验证**（2026-08-07）；HTTP 有路由未强制点测 |
| E2 | 登录后再按白名单 request（探索 collection 等） | 未做；建议 E1 后 |
| E3 | 每次 request 的 url/耗时摘要 | 可与 AUD-COMMS C 重叠 |
| 494 API 全扫 | 不做默认 | — |

### 2.6 验收口径

- CLI 与 HTTP 写出可读 JSON。  
- 走 full_login；轻量登录不得冒充完整快照。  
- 控制台与 log **不打印**引继/密码。  
- 写「CLI 已验证」或「HTTP 已验证」；**不**因此写日常 FIXED。

### 2.7 实现记录（2026-08-07）

| 项 | 内容 |
|----|------|
| 源码 | `crates/automadoka-core/src/session_export.rs` · app CLI `export session` · HTTP `POST/GET …/export/session` |
| CLI 验证 | 组 `123456` / 别名 `群友日服` / JP · `user_id=749807808230` · party_count=2 · mst style=119 |
| 目录例 | `automadoka_data/exports/群友日服/<timestamp>/` |
| Android | **不提供**此功能入口（主人钉死） |
| HTTP | 路由已挂；本轮以 CLI 为主验证 |
| log | `docs/logs/2026-08-07-e1-session-export.md` |

---

## 3. 实现队列（与 TASKBOARD §2 同步）

| 序 | ID | 状态 | 解锁 |
|----|-----|------|------|
| 1 | E1 | **CODE · CLI 验证** | 真实服 JSON 可读 |
| 2 | AUD-COMMS A→B→C | PLAN 文齐 · **建议下一** | 像客户端 + 云 data 占用 + 可观测 |
| 3 | CLI-M | PLAN | AI/脚本与 HTTP 同能 |
| 4 | TOOL-S secret 等 | PLAN 后置 | 探索推进（默认关） |
| 5 | C07 | PLAN | 面板开关化 |
| 6 | C10 | PLAN 未点名 | 队伍列表 UX |

---

## 4. 验收池

| ID | 状态 | 备注 |
|----|------|------|
| C01 日常 26 | WAIT | 失败驱动 diff |
| C03 暂停体感 | TODO | |
| C04 Release exe | TODO | |
| C06 URL 回退 | TODO | |
| C09 运行条 UI | CODE | |
| UI-07 批 | 请点测 | 指纹文案、模块布局、日志、多窗 |
| D04 系统浏览器 | SMOKE | 根因未证 |
| D05/D06 Android | TODO | info · B1–B10 |

### 4.1 Android B 表（摘要）

B1–B10 中壳与 SPA 路径多已 CODE/OWNER；**正式勾选验收**仍是 D06。纪律：B 未齐默认不抢 R4 大项。

---

## 5. 文档 / 研究

| ID | 状态 |
|----|------|
| R01 静态研究 | DONE |
| R03 横切产品化清单 | PLAN |
| DOC-FULL-01 | DONE |
| DOC-FULL-02 | LATER |
| AUD-ARCH 文 | DONE |
| AUD-COMMS 对齐文 | DONE；实现 §3 |
| 明文 token 规范 | DONE 07:28 |
| G2 对仗句修正 | 已改挂钩文 |

---

## 6. 日志时间线（压缩）

| 时段 | 归并 |
|------|------|
| 08-06 | 协议 L1/L2 · R2 日常 · R3 账号 · Owner · UI · 指纹 rules · 防呆 · 公开仓 · Android 壳 |
| 08-07 早 | UI/多窗 · WebView 可用 · D04 SMOKE · 规范分层 · 任务清点 |
| 08-07 中 | R01 · 意图 C10 · 通讯线 · DOC-FULL-01 · AUD-COMMS 对齐 · 审核路线 · 全面交接 |
| 08-07 07:19 | 任务初排 |
| 08-07 07:28 | 明文 token 规范 |
| **08-07 07:35** | **本表再整理** |

过程 log 目录：`docs/logs/`（非任务入口）。

---

## 7. 默认后置 / 禁止当主线

| 项 | 依据 |
|----|------|
| token/账号再加密、S* 本机加固 | P9b |
| R4 全量托管 | 门禁 / 点名 |
| Sonet 抢期 | 点名 |
| DOC-FULL-02 百科主线 | LATER |
| 产品内 OneDrive/灰区教程 | PLAN_AUDIT 文档受众 |
| 农场 / auto_register | 默认不做 |
| 无证据 FIXED | P5 |
| 未点名开工 C10/C07 | P7 备忘≠开工 |

---

## 8. 相对 07:19 表的变更

| 点 | 变更 |
|----|------|
| E1 措辞 | 去掉与 P9 打架的硬口号「导出永不带 token」；改为：本机明文合法；默认可不写 Gree 私钥/引继；协作靠 gitignore 与 P8b |
| 结构 | 短表更短；本文件专章写清 E1 |
| 规范挂钩 | P8/P8b/P9 写进阶段与后置 |
| 队列 | 仍建议 E1 → AUD-COMMS → CLI-M |

---

## 9. 文件索引

| 文件 | 角色 |
|------|------|
| [TASKBOARD.md](./TASKBOARD.md) | **短表真源** |
| 本文件 | 全景 + E1 定义 |
| [HANDOFF.md](./HANDOFF.md) | 完整现状 |
| [PLAN_AUDIT…](./PLAN_AUDIT_COMMS_OCCUPANCY_PROGRESS.md) | AUD-COMMS |
| [RUST_CODEBASE_AUDIT…](./tech/RUST_CODEBASE_AUDIT_AND_ROADMAP.md) | E1/CLI/secret 路线 |
| [AUTOMADOKA_RESEARCH…](./tech/AUTOMADOKA_RESEARCH_AND_RUST_GAP.md) | 原版缺口 §8 |

---

## 10. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-07 05:19 | 首版全景 |
| 2026-08-07 07:19 | 日志复核 |
| **2026-08-07 07:35** | **再整理；E1 专章；对齐明文规范** |
