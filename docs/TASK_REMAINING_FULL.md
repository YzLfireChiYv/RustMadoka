# 剩余任务完整记录（全文 · 禁止用短表代替）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-08（积压记录 SECRET/ANDROID/魔女群殴 · 文档归档清理） |
| **整理者** | AI |
| **失真声明** | **整份文档均为 AI 完成，有可能出错和失真。** 证据优先级：当前源码与真机 > [HANDOFF.md](./HANDOFF.md) 与 `docs/logs/` > 本文件。发现矛盾须修订并写 log（G7 · **P31**）。 |
| **主人原意完整摘录** | [logs/OWNER_INPUTS_RAW.md](./logs/OWNER_INPUTS_RAW.md)（291 条 · 时间升序） |
| **主人需求任务清单** | [OWNER_REQUIREMENTS_AND_TASKS_FULL.md](./OWNER_REQUIREMENTS_AND_TASKS_FULL.md)（主题+REQ+进度） |
| **技术报告** | [tech/SESSION_TECH_REPORT_2026-08-07.md](./tech/SESSION_TECH_REPORT_2026-08-07.md) |
| **导航短表** | [TASKBOARD.md](./TASKBOARD.md) 仅索引 |
| **规范** | NORMS **G1 · G8 · G10 · P5 · P6 · P7d · P21 · P22 · P30 · P31** |

---

## 0. 使用方式

1. 先读 HANDOFF 第 0 节与开机检查。  
2. 每项含：标识、完整内容、为什么、文档/源码、完成定义、诚实状态、依赖。  
3. **CODE ≠ FIXED ≠ 主人点测通过。**

---

## 1. 门槛

| 标识 | 完整内容 | 为什么 | 状态 |
|------|----------|--------|------|
| **GATE-REAL-ACCOUNT** | 真号验收后置 | 避免假成功阶段浪费真实资源 | WAIT · P30 |
| **GATE-SURPASS** | 透明度/稳定性/功能性相对原版与重构前基本达标或可代码验证超越 | 主人钉死 | **未达** · SURPASS 表见 tech/SURPASS_EVIDENCE_TABLE.md |

---

## 2. 全量重构 R0–R7

| 标识 | 内容 | 状态 |
|------|------|------|
| **R0** | 计划与规范 | 大体完成 |
| **R1** | 命名/路径/端口/双 exe | 大部分完成；tech 旧名未扫完 |
| **R2** | core 物理分区 protocol/domain/diag | **半成品**（仅命名空间） |
| **R3** | 宿主 CLI 中文 help、监视产品化 | 半成品；本批主页完整流推进 |
| **R4** | 浏览器网页前端产品化 | 半成品 |
| **R5** | 假成功语义 | 空路径 CODE；非空/有次数【未证】 |
| **R6** | MULTI_GROUP 全文 | 半成品（控制权/跨组文案/流有推进） |
| **R7** | 平台预留 | 未完成 |

---

## 3. 本批已 CODE（仍须主人点测）

| 标识 | 完整内容 | 状态 |
|------|----------|------|
| **HOME-STREAM** | 主页运行面板 = 完整监视流（stream_lines），非设置页进度条复读；深色滚动条；可拖高度 | **CODE** |
| **OWNER-CTRL** | 仅发起用户组 pause/resume/abort | **CODE** |
| **LOG-BY-DAY** | 日志按 7/30 天清理；筛选/清理分折叠；就地多开 | **CODE** |
| **SUPER-SWEEP-LIVE** | 快速刷图逐轮进度 + 单模块 NDJSON | **CODE** · CLI EN 3/3 成功 |
| **WIRE-PROBES** | debug 全量 wire + 探针 | **CODE** |
| **FP-PRODUCT** | 指纹内嵌+刷新启用+UI | CODE · HTTP |
| **ROUTE-AUTH** | 路由鉴权分层 | 主人曾验 URL |
| **PROC-MONITOR-TERMINAL** | 黑窗 `[流]` 镜像 stream_lines + 摘要 | **CODE** · 单测 |
| **DOC-CONSISTENCY tech** | tech 正式 crate/端口/data 名 | **大批 CODE**（32 文件；PLAN 根扫尾未完） |
| **CLI-RUN-NO-HANG** | 无 Owner 时 run 本地执行后退出 | **CODE** · info EXIT 0 |

---

## 4. 明确未完成

| 标识 | 完整内容 | 为什么 | 文档/源码 | 完成定义 | 状态 |
|------|----------|--------|-----------|----------|------|
| **HOME-STREAM-POLISH** | 主页流与终端粒度/手感的产品 polish | 体验 | MULTI_GROUP · static | 可感一致 | 可选 polish |
| **BASIC-HTTP500** | 训练扫荡：按关卡 **useStamina** 算次数 + 在已通关组内正确选倍率；omit partyDataId=0 | 曾用固定 10 算次数（411102 实为 15）导致过量 skip→500 | daily.rs · [basic-http500-fix](./logs/2026-08-07-basic-http500-fix.md) | ① CODE 已落地；② EN CLI skip 成功或诚实 Skip | **CODE** · 线上 FIXED 待 403 解除后复测 |
| **TOOL-PORT** | 工具四件：`clear_dungeon_event` **CODE**；raid_support 由 **group-raid** 上位；secret/auto_register **LATER** | 原版范围 | daily.rs · MODULES | 单模块 CLI/网页可跑；非 FIXED | **半成品** |
| **WEB-SYNC-CORE** | 多组状态更安全高效底层（非仅轮询） | 主人规格 | MULTI_GROUP §2 | 规格对照 | PLAN |
| **CARD-CROSS-GROUP** | 跨组文案与设置进度 | 已 CODE 一批 | static · RunHub | 点测 FIXED | CODE 非 FIXED |
| **PARTY-SELECT** | 队伍列表/手输 | C10 | PLAN_PARTY_SELECT | 点测 | CODE 非 FIXED |
| **FIX-LOGINBONUS-NONEMPTY** | 非空登录奖励真领 | 缺真实数据 | daily · wire | wire+点测 | 后置 |
| **FIX-EVENT-PLAYABLE** | 有次数活动成功 | 同上 | daily | 同上 | 后置 |
| **C01** | 日常逐条主人点测 FIXED | P5 | MODULES | 点测表 | WAIT |
| **GR-UI-REVERT** | 回退「添加账号」服下拉 `group_raid` 与添加区内半成品面板 | 主人纠正：添加只加号 | static · PLAN_GROUP_RAID_UI | 添加折叠仅加号 | **CODE** |
| **GR-MODEL-MULTI** | 用户组内组队配置**多份列表**落盘 | 不同人不同数量配置 | account.rs entries · 单测迁移 | 可增删多份重启保留 | **CODE** |
| **GR-CARD-UI** | 主页组队配置卡片 + 设置面板 | 借用账号卡片设计 | static grCards | 新建/设置/开始/删除 | **CODE** 非 FIXED |
| **GR-WIRE** | config_id → 编排；删卡降级；单号 | 接线 | run_ops · API · group_raid | 按卡开跑 | **CODE** |
| **GR-CLI-MULTI** | CLI `--config-id` 或 `--aliases` | P23 | lib.rs GroupRaid | 与网页对等 | **CODE** |
| **WIRE-RAID-ALIGN** | multi_raid 发包对齐 Python + 过伤 win + add_damage 不吞错 | 对服可靠 | daily/group_raid · log cards-wire-audit | 静态对照完成 | **CODE** 非线上 FIXED |
| **GROUP-RAID-LIVE** | 真人多号组队点测 FIXED | 产品验收 | GROUP_RAID | 点测表 | **WAIT** |
| **QUEST-STAGE-NAME** | 关卡 ID↔名称：CLI `mst quest-stages` / lookup、缓存、HTTP、网页查名称、basic/super_sweep 日志带名 | 主人点名原版有表；产品化底层 mst | mst.rs · run_ops · lib.rs · http_server · static · DATA_AND_MST · BASIC_SUPER_SWEEP §6 · log grok-history-quest-stage-mst | CLI 缓存冒烟已过；无点测 FIXED | **CODE** |
| **DOC-CONSISTENCY** | tech/PLAN 旧名全库 | 后继误读 | tech/* | 无 automadoka-core/13220 正式口径 | 未扫完 |
| **COMMENT-ENCODING** | 乱码注释清理 | 可读性 | task_log 等 | 无乱码 | 未全库 |
| **P21-FULL** | 函数级注释标准 | 跨会话 | NORMS | 抽查通过 | 未全库 |
| **SURPASS-EVIDENCE** | 可验证超越表维护 | P30b | SURPASS_EVIDENCE_TABLE | 条目可勾 | 进行中 |
| **Android-B** | Android 验收 | 双端 | PLAN_ANDROID | B 表 | WAIT/后置 |
| **W4-REL** | 正式发布上传 | 分发 | PLAN_RELEASE | 点名 | WAIT |
| **Sonet** | 台服 | 无号无包 | P30c | 预留即可 | 预留 |
| **W5** | macOS/Arch/OpenWrt | 远期 | PLAN_MULTI | LATER | LATER |
| **STORY-P** | 主线探索配置化 | C19 | L12 | LATER | LATER |
| **AUD-COMMS-A** | 对服节奏审计 | P27 | PLAN_AUDIT | 点名 | WAIT |
| **P9b** | token 再加密等 | **禁止默认** | NORMS | 不做 | 默认不做 |
| **SECRET-MAINLINE** | 完整移植通关主线（原版 secret / 神秘新功能）；配置化终点篇，禁止永久写死 612001 当唯一产品 | 主人 2026-08-08 积压 | HUMAN_FLOW §7.3 · tool.py · L12/C19 | 可 CLI/网页跑通探索推进；非 FIXED | **LATER 记录** |
| **ANDROID-SHELL-UI** | 重构 Android 壳 + 针对 Android 的 UI | 主人积压 | ANDROID_DUAL · mobile | 规格后实现 | **LATER 记录** |
| **WITCH-GANG-UP** | 魔女组队逻辑优化；改名「魔女群殴」；**细节待主人补充** | 主人积压 | GROUP_RAID | 主人补充后改规格再写码 | **WAIT 主人** |

---

## 5. 后续工作序列（2026-08-07 22:58 中断后重排 · AI 默认队列）

**失真声明：** 本序列为 AI 重排，有可能出错和失真。完整检查见 [logs/2026-08-07-post-crash-task-reaudit.md](./logs/2026-08-07-post-crash-task-reaudit.md)。更早波次论证见 [session-task-reorg](./logs/2026-08-07-session-task-reorg.md)（部分项已完成，勿重复开工）。

**总目的：** 沿透明度 / 稳定性 / 功能性 / 可维护性推进，服务 **GATE-SURPASS**；**GATE-REAL-ACCOUNT** 之前不用真号做验收主路径。

### 5.1 目的轴与主要剩余

| 目的轴 | 主要剩余 |
|--------|----------|
| 透明度 | **BASIC 真 skip 根因**；Skip 中文扫尾；非空/有次数后置；C01 点测 |
| 稳定性 | 双 exe 已有（约 22:03）；编译默认参数 OK（主人 override 电压）；PORT/HEART 手测可选 |
| 功能性 | 组队卡片 **CODE**（待点测）；TOOL-PORT；WEB-SYNC；R2–R7；组队真人 WAIT |
| 可维护性 | PLAN 根旧名；P21 触改；SURPASS 表 |

### 5.2 已 CODE（不阻塞 · 不重复当「从零」）

HOME-STREAM · OWNER-CTRL · LOG-BY-DAY · SUPER-SWEEP-LIVE · PROC-MONITOR-TERMINAL · DOC tech 正式名 · CLI-RUN-NO-HANG · loginbonus/event 空路径 Skip · basic **分批+Skip 语义**（真 skip 成功仍 OPEN）· CARD-CROSS / CHANNEL-BADGE / PARTY / FP / ROUTE（点测 FIXED 另论）。

### 5.3 AI 默认开干顺序（完整句 · 2026-08-08 更新）

**纪律：** 添加账号区**永远只加游戏账号卡片**。组队多配置卡片 **已 CODE**（2026-08-08 00:52）。

1. **GROUP-RAID-LIVE / C01：** 主人点测后写 FIXED。  
2. **TOOL-PORT 剩余：** secret（探索/主线，C19 后置）· auto_register（后置）；clear_dungeon_event 已 CODE。  
3. **P23 主缺口表已补 CLI**（fp / task-log / control / notify）；仍弱：主页流、wash 全参数、fp 自定义填槽。  
4. **WEB-SYNC / R2–R7：** 见全量重构书（架构债）。  
5. **A2 EN/JP 日常线上复测：** EN `…dc6521fc` 全量 ok；JP `…b4723a95` 真领+活动战成功；**arena URL 已修**（`get_pvp_top`）；**basic skip 500 仍 OPEN**（EN 411102 / JP 411101）。  
6. **E 门槛项：** 真号/发布/Android — 不默认阻塞。  
7. **REQ-WIN-TOAST：** **CODE** 默认关；CLI `notify system-*`。

### 5.4 明确不抢主线

Sonet 真机 · W5 多平台 · STORY-P · THEME-LIGHT · 百科 DOC-FULL-02 · R4 托管 · P9b 本地加密军备 · **CPU 压测目录与无关系统日志**。

### 5.5 与运行时路径的说明

**DATA-DIR-RENAME：** 源码默认已是 `RustMadoka_data` 与端口 14103。tech 正式口径已扫；PLAN 根旧名扫尾并入波次 D。

### 5.6 编译

主人 2026-08-07 22:58：override 电压高强度编译已通过 → **后续默认 cargo / build-win-dual 参数即可**，不必再把限核当硬门槛。

---

## 6. 修订

| 墙钟 | 内容 |
|------|------|
| 2026-08-07 17:36 | 首版全文 |
| 2026-08-07 17:40 | 四问答复 |
| 2026-08-07 18:05～19:20 | 多组/wire/超时/刷图进度等推进回写 |
| 2026-08-07 19:50 | 主页完整流；主人输入摘录链；全文重写剩余表与技术报告链 |
| 2026-08-07 19:55 | §5 后续工作序列重排（目的轴 + 波次 0–5）；log session-task-reorg |
| 2026-08-07 20:33 | 自主批：PROC-MONITOR · DOC tech · CLI hang · EN daily |
| **2026-08-07 22:58** | **中断后系统检查 + §5 波次 A–E 重排**；log post-crash-task-reaudit |
| 2026-08-08 03:20 | QUEST-STAGE-NAME CODE；Grok 历史 20 会话 |
| **2026-08-08** | **积压记录：** SECRET-MAINLINE · ANDROID-SHELL-UI · WITCH-GANG-UP（魔女群殴·待主人细节）；tech/PLAN 历史归档；大体积 7z |
