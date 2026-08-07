# 交接文档（完整）— RustMadoka

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-08（本机；**最终清理：tech/PLAN 归档 · 大体积 7z · G12 交接规范 · 积压记录**） |
| **读者** | 下一会话人工智能协作者；主人 |
| **本文件角色** | **唯一完整现状入口**。请先完整阅读第 0 节、第 0.1 节开机检查、第 A 节「本会话变更全文」，再改会进入可执行文件的代码。 |
| **失真声明** | **AI 维护，有可能出错和失真。** 证据优先级：当前真机与当前源码与 `RustMadoka_data/wire/` 实盘 > 本文件 > 过时 PLAN/历史 log。矛盾时修订文档并写 log（G7 · P31）。 |
| **本交接过程 log** | [logs/2026-08-08-grok-history-quest-stage-mst.md](./logs/2026-08-08-grok-history-quest-stage-mst.md) · 更早全文交接 [logs/2026-08-08-session-handoff-complete.md](./logs/2026-08-08-session-handoff-complete.md) |
| **规范（交接书写）** | NORMS **G10**（任务与交接落盘禁止压缩主人原意；只许更详不许更少）· **G8**（禁止一句话/比喻收束）· **G1** · **G2** · **G9** · **G11** |

### 导航（先读谁）

| 优先级 | 路径 | 角色 |
|--------|------|------|
| 1 | **本文件** | 现状、本会话变更全文、开机检查、下一动作 |
| 2 | [NORMS.md](./NORMS.md) | G/P 纪律（**规范无「重点/非重点」分级**：每一条都至关重要；含 G2 禁止「不是……而是……」凭空对立） |
| 3 | [OWNER_REQUIREMENTS_AND_TASKS_FULL.md](./OWNER_REQUIREMENTS_AND_TASKS_FULL.md) | 主人需求主题 + **REQ-*** + 进度（G10 详文） |
| 4 | [logs/OWNER_INPUTS_RAW.md](./logs/OWNER_INPUTS_RAW.md) | 主人原文 **约 308** 条 · 时间升序 · **禁止用短表代替** |
| 5 | [TASK_REMAINING_FULL.md](./TASK_REMAINING_FULL.md) | 工程剩余全文（禁止用短表代替） |
| 6 | [TASKBOARD.md](./TASKBOARD.md) | 短表导航 only |
| 7 | [tech/BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS.md](./tech/BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS.md) | **智能体力扫荡 vs 快速刷图** · 强化本 skip 规则（本会话新增真源） |
| 8 | [research/magia-exedra/README.md](./research/magia-exedra/README.md) | 游戏 wiki/官网本地抓取索引 |
| 9 | [tech/CLI_WEB_PARITY.md](./tech/CLI_WEB_PARITY.md) · [tech/WIRE_AND_DEBUG_PROBES.md](./tech/WIRE_AND_DEBUG_PROBES.md) | P23 CLI 超集 · 开发版 wire |
| 10 | [PLAN_RUSTMADOKA_FULL_REWRITE.md](./PLAN_RUSTMADOKA_FULL_REWRITE.md) · [tech/GROUP_RAID_AND_DEVICE_IDENTITY.md](./tech/GROUP_RAID_AND_DEVICE_IDENTITY.md) | 全量重构 · 组队 |
| 11 | [logs/grok-build-history/](./logs/grok-build-history/) | Grok Build 历史副本（**20** 会话 + prompt_history） |
| 12 | [DOC_MAP.md](./DOC_MAP.md) · [LESSONS.md](./LESSONS.md) | 文档地图 · 教训索引 |
| 13 | [tech/HUMAN_FLOW_REPORT_FROM_PASSWORD_TO_FEATURES.md](./tech/HUMAN_FLOW_REPORT_FROM_PASSWORD_TO_FEATURES.md) | **人类向全流程报告**（密码→功能·对服·secret·源结构） |

**公开源码仓：** https://github.com/YzLfireChiYv/RustMadoka  

**工作区绝对路径：** `C:\GrokProject\automadoka`（文件夹名**永不更改**）

**产品正式名：** RustMadoka。交付：`RustMadoka.exe` / `RustMadoka_debug.exe` · 数据 `RustMadoka_data` · 端口 **14103**。

---

## 0. 摘要（完整条件 · 以 2026-08-08 03:20 为准）

### 0.1 产品是什么

本产品是 **Magia Exedra**（魔法少女まどか☆マギカ Magia Exedra）的**本机**自动清日常与相关自动化工具。正式产品名称 **RustMadoka**。不是云端代打托管。不把引继码、游戏密码或登录令牌上传到公开仓库做云端账号存储。

运行形态以 **Windows 11** 为当前优先交付面：

1. 双击 **普通版** `RustMadoka.exe` 或 **开发版** `RustMadoka_debug.exe`（开发版 feature `wire_record`，无差别录通讯）。  
2. 数据文件夹旁路 **`RustMadoka_data`**（两版共用）。  
3. **程序运行面板终端**（黑色窗口，附着本体进程）。  
4. **浏览器网页前端** 默认 `http://127.0.0.1:14103/`。  
5. **CLI 与 Owner：** 无 Owner 时 `run *` 可拉起并保持；有 Owner 时 CLI 走 **IPC**（产品设计，非 Bug）。带 `--wire` 的 run 常走**本机独占**路径（抢 Owner 锁、执行、退出）。  
6. Android / 台服 **不是**当前验收主路径（台服仅预留）。  
7. 指纹：exe **内嵌** + rules raw **热更到数据夹**（P29）；默认源 rules 仓 `automadoka.json` URL，**禁止改 rules 仓内容**。

### 0.2 实现分层

| 路径 | 说明 |
|------|------|
| `crates/rustmadoka-core` | 协议与业务：登录 Gree、日常模块、`group_raid`、account、`device_id` 按卡、mst、wire |
| `crates/rustmadoka-app` | CLI / HTTP / Owner / RunHub / **session_pool** / IPC / TaskGate / fp_slots / system_toast |
| `crates/rustmadoka-app/static/index.html` | 浏览器网页前端 |
| `archive/pre-rust-2026-08/autopcr/` · `raid/` | 母项目 Python / raidworker **只读对照** |
| `docs/research/magia-exedra/` | 本会话抓取的 wiki/官网快照（可能过时） |

### 0.3 交付物与端口（本交接快照）

| 项 | 状态 |
|----|------|
| 普通版 / 开发版 | 根目录双 exe，本会话 dual 构建约 **2026-08-08 02:49 / 02:50**（约 19.2 MB / 19.5 MB） |
| 数据文件夹 | `RustMadoka_data` |
| 默认端口 | **14103** |
| 无旧兼容 | 不恢复 automadoka.exe / 13220 正式路径 |

### 0.4 本阶段已落地（可核验 · **≠ 主人点测 FIXED**）

下列均为 **CODE / CLI 验证** 等级，**禁止**写成 FIXED 或「用户已可用」除非主人点测。

#### 0.4.1 架构与规范（此前 + 本会话继承）

- R0–R1 大体完成；全量重构 R2–R7 **未**收口。  
- SessionPool：同一进程内同游戏身份复用 `GameClient`；空闲约 75 分钟丢弃。  
- CLI Owner 拉起 + IPC 附着为产品设计。  
- P9 本地明文永远允许；P8/P8b 不进 git、不打进分发。  
- 组队多配置卡片 / device_id 按卡 / 伤害规则等此前 CODE（待点测）。  

#### 0.4.2 本会话新增或实质推进（2026-08-08 约 接手～03:05）

**A. 协作与认知**

- 主人要求：先全面理解规范与历史及**为什么有规范**；规范**没有「重点」分级**，每条都至关重要（含 G2 禁止假对立句架）。  
- 主人怀疑「全面重构名实不符 / 命名未改净 / 多会话验证失真」——交接须诚实：R2–R7 未完；全量重构计划写明组队等为增量；文件夹名仍为 automadoka（主人钉死不改）。  
- 主人原文台账已存在（291 条 / 18 会话）；本会话继续以 HANDOFF + RAW + 源码为真源。  

**B. P23 CLI 补齐与系统通知**

| 能力 | CLI / 实现 | 说明 |
|------|------------|------|
| 指纹槽 | `fp slots` · `refresh` · `activate` · `reset` · `fill` | 列表/默认源刷新启用/切槽/回内嵌/自定义槽 |
| 任务日志 | `task-log list/show/progress/clear-older` | 读索引/全文/进度/按天清理 |
| 运行控制 | `control pause/resume/abort/status` | 须 Owner；IPC `RunPause` 等 |
| 通知历史 | `notify settings` | settings 功能通知 |
| 系统 toast | `notify system-get/set/test` · `system_toast.rs` | **默认 enabled=false**；任务定稿可弹 |
| HTTP | `/api/system-toast` GET/POST | 网页可配 |
| 设置通知 API | `/api/features/settings/notifications` | **修复**曾恒返回空列表 |

源码：`crates/rustmadoka-app/src/lib.rs` · `ipc.rs` · `http_server.rs` · `system_toast.rs` · `task_log.rs`（finalize 钩 toast）。  
规格：`docs/tech/CLI_WEB_PARITY.md` · `docs/tech/WINDOWS_SYSTEM_NOTIFY.md`。  
log：`docs/logs/2026-08-08-autonomous-p23-toast-tools.md`。

**C. 工具模块 clear_dungeon_event**

- `run module --key clear_dungeon_event`：对照 Python `tool.py` 迷宫隐藏事件。  
- raid_support：产品上位为 **group-raid** + 可选 `leave_after_support`。  
- secret / auto_register：仍 LATER。  

**D. arena 路径修复**

- 错误：`/api/pvp/get_top` → JP **HTTP 404**。  
- 正确（Python `PvpApiGetPvpTopRequest`）：**`/api/pvp/get_pvp_top`**。  
- 修复后 JP 单模块：免费 **5** 次 PVP 投降 **success**；wire `jp_w1/20260807T183052-fe936f1a`。  

**E. 智能体力扫荡 basic 口径与代码**

主人纠正与资料：

1. **basic = 只做扫荡 skip**（素材/经验强化本）；关键词是扫荡。  
2. **没有 skip 的内容要真战斗**；**快速刷图 super_sweep 原版就是真战斗**（init+finalize）。  
3. **411102 本来就不允许 skip**；对该关 skip 的 500 **不是**「协议整体坏了」的充分证明。  
4. 测试号训练进度 wire 常为 **仅组 403（能力晶花）+ 411101/411102**；wiki：可 skip 主写 **Kioku/Magic 已通关本**。  

代码（`daily.rs::basic`）：

- 选关优先 101 / 201–299；能力晶花 401–499 降权。  
- **仅有能力晶花进度时直接 Skip**（中文说明），**不**再对 41110x 硬 skip 打 500。  
- 规格：`docs/tech/BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS.md`。  
- 单测：`training_skip_prefers_kioku_and_magic_over_crystalis`。  
- core 单测 **21 passed**（本会话末）。  

**F. 国际服 / 日服 debug 清日常与 wire（测试号）**

| 跑次 | 会话目录 | 整次 ok | 备注 |
|------|----------|---------|------|
| EN 全量 | `wire/en_w1/20260807T181936-dc6521fc` | true · errors=0 | success=3；basic 对 411102 skip **500**（后知不可 skip） |
| JP 全量（arena 修前） | `wire/jp_w1/20260807T182637-b4723a95` | false · errors=1 | success=8；**真领登录奖励、15 场活动战**；arena 404 |
| JP arena 单测 | `…183052-fe936f1a` | arena success | 5 投降 |
| JP 全量（basic 修后） | 多趟含 `…185445-0c46cd0e` | **true · errors=0** | basic **诚实跳过**；HTTP **全 200**（该趟） |

测试账号（**禁止进公开仓**）：组 `123456` 密码见本地；别名 **`en_w1`**（en）、**`jp_w1`**（jp，本会话添加）；凭证 `plan/local-test-accounts.md`。

**G. 一趟清日常登录次数（主人问 · wire 钉死）**

- **单次 `run daily` 内：`/api/login` = 1，Gree authorize = 1，然后同一客户端跑完所有模块。**  
- 不是每模块登录一次。  
- 若用 `--wire` 独占进程：每趟 daily 各 1 次登录；Owner 长驻 + IPC 可复用池（约 75 分钟空闲丢弃）。  
- 设计意图：像真客户端一趟在线做完任务；CLI 测完退进程则不会全天只登一次。  

**H. 副本数字 ID 与名称对应表（主人问 · 原版 · **本会话已产品化 CODE**）**

- 原版通过 Master：**`/api/mst/get_quest_stage_mst_list`** → **`questStageMstId` + `name`**（wire 约 4153 行）。  
- 取法：`db.mst(MstApiGetQuestStageMstListRequest())`（`archive/.../db/database.py`）。  
- **产品入口（2026-08-08 本会话）：**  
  - CLI：`mst quest-stages` · `mst quest-lookup --id`（`--from-cache` 不登录 / `-g -a --refresh` 对服）  
  - 缓存：`RustMadoka_data/cache/mst/{en|jp}/quest_stage.json`  
  - HTTP：`/api/accounts/:alias/mst/quest-stages` · `/api/mst/quest-stages?channel=`  
  - 网页：设置 → 快速刷图 → 关卡 ID「查名称 / 按名搜索」  
  - 日志：`basic` / `super_sweep` 带 `关卡=ID（名称）`  
- CLI 已验证（缓存）：`411102`→能力晶花[木]Normal；`401101`→キオク Rank1；`filter 魔力`→魔力解放火 Rank1+。  
- 规格：`docs/tech/DATA_AND_MST.md` §5.1 · `BASIC_SUPER_SWEEP` §6 · log `2026-08-08-grok-history-quest-stage-mst.md`。  
- **非**主人点测 FIXED。  

**I. 游戏外援资料**

- `docs/research/magia-exedra/`：exedra.wiki Upgrade Quests、Help、madodra、game8、官网等 HTML/摘录。  

**J. 本会话（03:20）Grok 历史同步**

- 已复制 **20** 会话至 `docs/logs/grok-build-history/`（含上一轮 `019fdd5d-…` 全文 chat/events/updates）。  
- `build_owner_requirements_full.py` 已重跑 → OWNER_INPUTS_RAW **约 308** 条。  

### 0.5 明确尚未完成（完整条件）

1. **主人点测 FIXED**：组队真人、日常 C01、端口/心跳手测等（P5）。主人曾表示不敢用真号测——**默认仍不建议真号当验收主路径**（P30），除非门槛达标。  
2. **basic 真扫成功**：日服 `jp_w1` 已于 2026-08-08 **先 super_sweep 通关 403101（魔力解放火 Rank1）再 basic skip ×329 success**（CLI/wire 证据 · 见 log `2026-08-08-jp-stone-basic-skip-success`）。**非**主人点测 FIXED；新数据夹重建后须重新加号测。  
3. **组队**：无多号端到端成功 wire；rescueType【未证实】；入房 id_search+initialize 风险已注释收紧但非 FIXED。  
4. **P23 仍弱**：主页完整监视流仅网页；wash 全参数盘点；部分交互仅网页。关卡 ID 表已补 CLI/网页。  
5. **TOOL-PORT 剩余**：secret、auto_register LATER。  
6. **WEB-SYNC · R2–R7 物理分区 · PLAN 根旧名扫尾**。  
7. **发布 · Android B 验收 · 台服真机 · 多平台**。  
8. **真号门槛后**：非空登录奖励/有次数活动写 FIXED 等。  
9. Grok 历史本会话已同步 20 会话；之后新对话仍须重拷 + `build_owner_requirements_full.py`。  

### 0.6 协作授权

1. 可结束两 exe、改删重建 `RustMadoka_data`（P1）。  
2. 可用 CLI 对测试号冒烟与完整日常（P2；本会话 EN/JP 均已用）。  
3. 高风险（force-push、向 origin 推、系统破坏）须先问。  
4. 本地明文永远允许（P9）；不进 git、不打进分发（P8/P8b）。  

### 0.7 测试账号提示（禁止写入公开仓）

| 项 | 内容 |
|----|------|
| 用户组 | 常见 `123456`（加密，密码本地 `123456`） |
| EN | 别名 `en_w1` |
| JP | 别名 **`jp_w1`**（本会话 `account add`） |
| 凭证真源 | `plan/local-test-accounts.md`（gitignore） |

### 0.8 本机产物快照（交接时）

| 路径 | 说明 |
|------|------|
| `RustMadoka.exe` | 约 2026-08-08 **03:18** |
| `RustMadoka_debug.exe` | 约 2026-08-08 **03:19** · wire |
| `RustMadoka_data/` | **已移走**；归档见 `archive/runtime-RustMadoka_data-20260808-0325/`（主人将重建新夹） |
| `docs/tech/BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS.md` | 扫荡/刷图规格 + 关卡查询入口 |
| `docs/tech/DATA_AND_MST.md` | mst + 关卡对照 §5.1 |
| `docs/research/magia-exedra/` | wiki 抓取 |
| `docs/logs/grok-build-history/` | **20** 会话副本 |
| 本会话关键 log | 见 §8 |

---

## 0.1 开机检查（必须按顺序）

1. 完整阅读本文件 **第 0 节** 与 **第 A 节**（本会话变更全文）。  
2. 读 [NORMS.md](./NORMS.md)：G1 G2 G7 G8 G9 G10 G11 · P5 P6 P7c P7d P7e P8 P9 P16 P21 P22 P23 P25 P27 P28 P29 P30 P31。**规范无重点分级**；G2 禁止「不是……而是……」假对立。  
3. 扫 OWNER_REQUIREMENTS §0–§3；冲突打开 OWNER_INPUTS_RAW。  
4. 读 TASK_REMAINING_FULL 与 PLAN_RUSTMADOKA_FULL_REWRITE。  
5. 若做 basic / 强化本：必读 **BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS** + research/magia-exedra README。  
6. 若做组队：GROUP_RAID + PLAN_GROUP_RAID_UI + group-raid logs。  
7. 若做 wire：WIRE_AND_DEBUG_PROBES。  
8. 核对 `crates/rustmadoka-*`、根目录双 exe（≥03:18）、`RustMadoka_data`（含 mst 缓存可选）。  
9. 改 exe：`scripts/build-win-dual.ps1`；改 static：`scripts/check-static-js.ps1`。  
10. 实质工作写 `docs/logs/`；更新本文件 ≥3 行；真墙钟到分钟。  
11. 公开推送仅 **rustmadoka**；禁止 origin 全史 force；推前扫 secret。  

---

## A. 本会话（接手～交接）工作全文与证据

本节字数与条件**只许更详**；后继不得用短表代替本节。

### A.0 续会话（2026-08-08 约 03:10～03:20 · 历史同步 + 关卡表）

主人任务：提取上一轮完整 Grok 聊天到仓库；在不需主人协助时完成能做的升级/优化；举例充分利用关卡 ID–名称表。

| 交付 | 证据 |
|------|------|
| Grok 历史 20 会话复制 | `docs/logs/grok-build-history/` · 新增 `019fdd5d-…` `019fdd9c-…` |
| 需求台账再生 | OWNER_INPUTS_RAW n≈308 · `build_owner_requirements_full.py` |
| `mst quest-stages` / `quest-lookup` CLI | dual 构建后本地 `--from-cache` 冒烟 |
| HTTP + 网页查名称 | `http_server` 路由 · `static/index.html` 快速刷图字段 |
| basic/super_sweep 日志带关卡名 | `daily.rs` |
| 缓存自动落盘 | daily/module 后若内存有表则写 `cache/mst/{channel}/` |
| core 单测 | 22 passed（含 `format_and_filter_quest_stages`） |
| 过程 log | `docs/logs/2026-08-08-grok-history-quest-stage-mst.md` |

硬阻塞未破：主人点测 FIXED、真号、basic 真 skip（号进度）、组队真人、R2–R7 全收口。

### A.1 会话任务脉络（主人输入顺序 · 完整条件 · 上一轮 019fdd5d）

1. **接手**：全面了解规范与历史及原因；指出 Grok 多会话验证失真；要求全面重构 RustMadoka；将进行多轮审核评估。  
2. **规范纠正**：规范中**不存在「重点」**；每一条都至关重要；举例禁止「不是……而是……」套话。要求梳理多轮对话，从「产品需要哪些功能」到「为什么需要」。  
3. **开始推进**：除非须补充信息或抉择，否则不停止，完成已知任务。  
4. **真号安全评估问**：发包错误是否系统性审核过——答复：**当时未做全产品对服审计**；不建议真号主路径（P30）。  
5. **国际服 VPN**：debug 全量清日常抓取校验。  
6. **日服 VPN**：添加 `jp_w1`、全量 wire、arena 修、再测。  
7. **basic 500 历史**：说明 wire 里 skip 全是 500；主人记成功多次 → 区分整次日常成功 vs basic skip。  
8. **主人：411102 本来就不允许 skip**。  
9. **主人：原版信息很详细；经验/石头本可 skip 才叫智能体力扫荡；无 skip 要真战斗；快速刷图原版就是战斗；怀疑缺少对原版 Python 研究**——接受并回写规格。  
10. **三项任务**：全面对照文档 + 互联网/wiki 落盘 + 改代码构建日服测。  
11. **日服 VPN 再测**。  
12. **问一趟清日常几次登录**。  
13. **问原版 ID–名称表**。  
14. **整理准备交接**（本文件）。  

### A.2 代码变更清单（本会话 · 主要）

| 区域 | 变更摘要 |
|------|----------|
| `rustmadoka-app` CLI | `fp` / `task-log` / `control` / `notify` 子命令 |
| `ipc.rs` + `http_server` | RunPause/Resume/Abort/Status；RunHub 接入；system-toast API；settings notifications 修空列表 |
| `system_toast.rs` | Win toast 默认关；finalize 钩子 |
| `daily.rs` arena | `get_pvp_top` |
| `daily.rs` basic | skip 优先组；晶花-only 直接 Skip；失败文案；函数级文档 |
| `daily.rs` clear_dungeon_event | 工具模块可 `run module` |
| 文档/research | 见上节 |

### A.3 Wire / 任务会话索引（测试号 · 本会话关键）

| 用途 | 路径 |
|------|------|
| EN 全量 daily | `RustMadoka_data/wire/en_w1/20260807T181936-dc6521fc` |
| EN basic 探针 | `…/20260807T182254-4c0e5157` |
| JP 全量（有活动真打） | `wire/jp_w1/20260807T182637-b4723a95` |
| JP arena 修后 | `…/20260807T183052-fe936f1a` |
| JP 全量 errors=0 | `…/20260807T185445-0c46cd0e`（HTTP 全 200） |
| 登录次数证据 | 上述 daily 会话各 **1×** `/api/login` |

### A.4 过程 log 索引（本会话必须阅读的）

| log | 内容 |
|-----|------|
| [2026-08-08-autonomous-p23-toast-tools.md](./logs/2026-08-08-autonomous-p23-toast-tools.md) | P23 CLI · toast · 迷宫工具 |
| [2026-08-08-en-debug-daily-full-wire.md](./logs/2026-08-08-en-debug-daily-full-wire.md) | EN 全量 + 411102 不可 skip 补记 |
| [2026-08-08-jp-debug-daily-full-wire.md](./logs/2026-08-08-jp-debug-daily-full-wire.md) | JP 全量 + arena 修复 |
| [2026-08-08-basic-research-jp-retest.md](./logs/2026-08-08-basic-research-jp-retest.md) | wiki 落盘 + basic 改 + JP 复测 |
| [2026-08-08-jp-vpn-retest.md](./logs/2026-08-08-jp-vpn-retest.md) | 日服 VPN 再测 ok=true |
| [2026-08-08-session-handoff-complete.md](./logs/2026-08-08-session-handoff-complete.md) | **本交接过程** |
| 更早同日 | final-handoff-full · owner-history · group-raid-* · session-audit-fixpass |

### A.5 主人钉死的认知纠正（必须继承）

1. **规范没有重点/非重点**；G2 等每条都要遵守。  
2. **G10**：交接与任务记录只许更详不许压缩。  
3. **basic = 扫荡 skip**；**super_sweep = 真战斗**；不可混谈。  
4. **411102 不可 skip**；晶花进度 only 时应跳过而非硬 500。  
5. **P5 / P30**：无点测不 FIXED；真号验收后置。  
6. **P27**：对服像客户端；arena 写错 path 是真实发包错误并已修。  
7. **一趟 daily = 一登录**；多趟 CLI = 多次登录。  
8. **关卡 ID–名称** = mst `get_quest_stage_mst_list`。  

---

## 1. 远程与禁止提交

| 远程 | 地址 | 用途 |
|------|------|------|
| **rustmadoka** | https://github.com/YzLfireChiYv/RustMadoka.git | 公开主远程 |
| **origin** | https://github.com/cc004/automadoka.git | 母项目；禁止随意 push |

禁止进 git：数据文件夹、引继/密码/token、plan/、wire 含账号材料、完整游戏包、research 若含敏感则勿推。  
`docs/logs/grok-build-history/sessions/**/updates.jsonl` 等大体量已 gitignore。

---

## 2. 运行与构建

```bat
cd /d C:\GrokProject\automadoka
powershell -File scripts\build-win-dual.ps1
RustMadoka.exe
RustMadoka_debug.exe
```

| 项 | 说明 |
|----|------|
| 浏览器网页前端 | `http://127.0.0.1:14103/` |
| 数据文件夹 | `RustMadoka_data` |
| 开发版 wire | `RustMadoka_data/wire/{alias}/…` |

CLI 示例（本会话扩充后）：

```bat
RustMadoka.exe run info -g 123456 -a jp_w1 --group-password *** --json
RustMadoka_debug.exe run daily -g 123456 -a jp_w1 --group-password *** --json --wire --all-modules --safe-raid-damage
RustMadoka.exe run module -g 123456 -a jp_w1 --group-password *** --key arena --json
RustMadoka.exe fp slots
RustMadoka.exe task-log list -g 123456 -a en_w1
RustMadoka.exe notify system-get
RustMadoka.exe control status
RustMadoka.exe run group-raid -g <组> --config-id <id> --json
RustMadoka.exe mst quest-stages --from-cache --channel jp --id 411102
RustMadoka.exe mst quest-stages --from-cache --channel jp --filter キオク --limit 20
RustMadoka.exe mst quest-lookup --id 401101 --from-cache --channel jp
RustMadoka.exe mst quest-stages -g 123456 -a jp_w1 --group-password *** --refresh --filter 魔力
```

再生主人需求台账：

```bat
python scripts\build_owner_requirements_full.py
```

---

## 3. 语义与模块立场

真源：

- [MODULE_SEMANTIC_CLASSIFICATION.md](./tech/MODULE_SEMANTIC_CLASSIFICATION.md)  
- [BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS.md](./tech/BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS.md)  
- [GAME_NAMING_GLOSSARY.md](./tech/GAME_NAMING_GLOSSARY.md)  
- [ERROR_DIAGNOSTICS.md](./tech/ERROR_DIAGNOSTICS.md)  
- 原版 `archive/.../stamina.py` · `tool.py`  

改游戏步骤必须写清成功/跳过/中止/失败/部分等（P25 · C20 · L13）。禁止以「已对照 Python」代替原理说明。禁止把 basic 改成「skip 失败就自动真战斗」除非主人点名改产品定义。

---

## 4. 安全与通讯

- 本地明文 P9；协作卫生 P8/P8b。  
- 对游戏服像正常客户端（P27）。  
- device_id 按游戏账号卡片（引继）。  
- 组队：无多号端到端成功 wire 铁证。  
- 真号：P30 后置；本会话仅测试号。  

---

## 5. 入口地图

```text
docs/HANDOFF.md                                 本文件
docs/logs/2026-08-08-session-handoff-complete.md 本交接过程 log
docs/tech/BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS.md 扫荡/刷图
docs/research/magia-exedra/                     wiki 抓取
docs/OWNER_REQUIREMENTS_AND_TASKS_FULL.md
docs/logs/OWNER_INPUTS_RAW.md
docs/logs/grok-build-history/
docs/TASK_REMAINING_FULL.md
docs/TASKBOARD.md
docs/PLAN_RUSTMADOKA_FULL_REWRITE.md
docs/tech/CLI_WEB_PARITY.md
docs/tech/WIRE_AND_DEBUG_PROBES.md
docs/tech/GROUP_RAID_AND_DEVICE_IDENTITY.md
docs/NORMS.md · docs/LESSONS.md · docs/DOC_MAP.md
crates/rustmadoka-core · rustmadoka-app · static/
archive/pre-rust-2026-08/
scripts/build-win-dual.ps1 · check-static-js.ps1 · build_owner_requirements_full.py
```

---

## 6. 下一会话默认动作

**完整队列：** OWNER_REQUIREMENTS §3 REQ · TASK_REMAINING_FULL · BASIC_SUPER_SWEEP（若碰体力扫荡）· CLI_WEB_PARITY · GROUP_RAID（若碰组队）。

**本交接边界：** 代码与文档停在可交接状态；**未**主人点测 FIXED；**未**授权真号主验收。

1. 完成 §0.1 开机检查。  
2. 若主人要 **キオク/魔力解放进度** 上测 basic 真 skip：先确认训练进度 wire 非仅 403。  
3. 若主人点测组队：主页组队卡 / CLI `--config-id` / debug wire。  
4. 若继续工程：WEB-SYNC、R2–R7、剩余 P23（主页流/wash 参数）、点测反馈修复。关卡 ID–名称已 CODE。  
5. 新主人输入：同步 grok-build-history + `build_owner_requirements_full.py`。  
6. 改代码 dual 构建；改 static 必 check-static-js。  
7. CLI 拉起 Owner 是设计；`--wire` 独占是另一条路径。  

### 6.1 明确后置

台服真机、百科主线、token 再加密、R4 托管、主线探索配置化、macOS/Arch/OpenWrt/iOS、亮色主题、真号门槛后写 FIXED 等。

### 6.2 已知对服风险（非已修清单）

- 组队：入房 id_search + initialize；add_damage 分片半截；rescueType 未证实；无多号成功 wire。  
- basic：仅在可 skip 的キオク/魔力解放进度上才可能真扫成功。  
- 部分模块依赖号进度（编成、活动次数、塔等）会 Skip——属预期。  

---

## 7. 规范速查

| ID | 要点 |
|----|------|
| G1 G8 G10 | 完整；禁一句话收束；不压缩主人原意（交接只许更详） |
| G2 | 禁止「不是……而是……」假对立；**规范无重点分级** |
| G9 G11 | 全称；对主人白话 |
| P5 | 无点测不 FIXED |
| P6 | log + HANDOFF + 真墙钟 |
| P7c P7d | 整批做完；汇报不当停工 |
| P7e | 修 bug 先查 PLAN/tech/log |
| P8 P9 | 秘密不进仓/分发；本地明文永远允许 |
| P16 | 单 Owner |
| P21 P22 | 注释与文档双向链接 |
| P23 | CLI⊇网页 |
| P25 | 模块结果按游戏语义 |
| P27 | 对服不乱发包 |
| P30 P30b | 真号后置；超越可代码验证 |
| P31 | AI 落盘标失真 |

---

## 8. 近期优先日志

| 日志 | 方向 |
|------|------|
| **`2026-08-08-final-cleanup-archive-handoff-norms.md`** | **文档归档 · 7z · G12 · 积压** |
| `2026-08-08-data-layout-audit-r2r7-batch.md` | 数据夹审核 + layout2 + R2–R7 矩阵 |
| `2026-08-08-jp-stone-basic-skip-success.md` | JP 魔力解放先打后 skip + 数据夹归档 |
| `2026-08-08-grok-history-quest-stage-mst.md` | Grok 历史同步 + 关卡 ID↔名称产品化 |
| `2026-08-08-session-handoff-complete.md` | 上一轮全面交接过程 |
| `2026-08-08-jp-vpn-retest.md` | 日服再测 ok=true |
| `2026-08-08-basic-research-jp-retest.md` | wiki + basic 改 |
| `2026-08-08-jp-debug-daily-full-wire.md` | JP 全量 + arena |
| `2026-08-08-en-debug-daily-full-wire.md` | EN 全量 + 411102 |
| `2026-08-08-autonomous-p23-toast-tools.md` | P23 CLI · toast |
| `2026-08-08-session-audit-fixpass.md` | 组队排查 |
| `OWNER_INPUTS_RAW.md` | 主人原文约 308 条 |

---

## 9. 修订

| 墙钟 | 内容 |
|------|------|
| 2026-08-08 00:45 | 组队 UI 口径纠正 |
| 2026-08-08 00:52 | 组队多配置卡片 CODE |
| 2026-08-08 01:32 | 组队排查修复批 |
| 2026-08-08 01:50 | Grok 历史 · 需求清单 |
| 2026-08-08 01:55 | 全面交接（01:55 版） |
| 2026-08-08 02:15 | P23/toast 推进回写 |
| 2026-08-08 03:05 | 上一轮全文交接：basic/arena/wire/登录次数/ID 表/research |
| 2026-08-08 03:20 | Grok 20 会话同步；mst quest-stages CLI/HTTP/网页/日志；dual 03:18/03:19 |
| 2026-08-08 03:25 | JP 403101 先打后 basic ×329 success；数据夹归档 `archive/runtime-RustMadoka_data-20260808-0325` |
| 2026-08-08 | P30d 正式自用；P1b/P32；DATA_FOLDER_LAYOUT；心跳=跨路径二次保险 |
| 2026-08-08 | 布局设想对照 schema2；settings 双写；登录池；组队再审 |
| 2026-08-08 | 数据夹审核 A1–A8；mirror_layout2；R0–R7 矩阵 |
| **2026-08-08** | **最终清理：17 tech+10 PLAN 归档；bundles 7z；G12；积压 SECRET/ANDROID/魔女群殴；log final-cleanup-archive-handoff-norms** |
