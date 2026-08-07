# 文档与工作区地图

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-08（tech/PLAN 历史归档后） |
| **用途** | 读什么、改什么、只读什么；规范/教训/技术如何分层 |
| **MAY CONTAIN ERRORS** | Yes — 以目录实存为准 |

---

## 0. 分层（防混乱）

```text
现状与任务     HANDOFF.md · TASKBOARD.md · TASK_REMAINING_FULL.md · TASK_INVENTORY.md · PLAN_*.md · logs/
规则（短）     NORMS.md          ← 规则 + 索引 + 易触犯案例
教训（长）     LESSONS.md        ← 索引
               tech/LESSONS_*.md ← 正文（触犯时间/损失/新规则/原理）
技术专题       tech/*.md         ← 机制与源码双向链接（TECH_DOC_CONVENTION）
数据夹布局     tech/DATA_FOLDER_LAYOUT.md ← 正式自用 · layout_schema · 心跳≠owner.lock
人类全流程报告 tech/HUMAN_FLOW_REPORT_FROM_PASSWORD_TO_FEATURES.md ← 密码→功能·对服·源结构
游戏外援抓取   research/magia-exedra/ ← wiki/官网快照（可能过时）
多组/监视规格  tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md ← 完整交互（禁止压缩）
游戏功能框架   tech/GAME_FEATURE_FRAMEWORK.md ← 内容×协议×自动化三层
语义分类       tech/MODULE_SEMANTIC_CLASSIFICATION.md ← 领取/养成/战斗（重构用）
命名与功能身份 tech/GAME_NAMING_GLOSSARY.md
下轮主任务书   PLAN_NEXT_SESSION_SYSTEM_REFACTOR.md
可移植认知     AI_PROJECT_NORMS_PORTABLE.md
归档           archive/pre-rust · docs-tech-historical · docs-plan-historical · bundles/*.7z
```

**过时 tech：** 不删 → `archive/docs-tech-historical-2026-08/`。  
**现行 tech 索引：** [tech/README.md](./tech/README.md)。

**禁止：** 在 NORMS 里堆全部教训全文；在教训里写任务 NOW；多文件互称「唯一入口」。  
**禁止：** 用简写或压缩表代替主人讨论细节（NORMS **G10**）；完整条件写在 tech 正文。

---

## 1. 开机顺序

| 序 | 路径 | 角色 |
|----|------|------|
| 1 | [HANDOFF.md](./HANDOFF.md) | 唯一完整现状入口 |
| 2 | [NORMS.md](./NORMS.md) | 规则 G/P + 案例 + 索引 |
| 3 | [LESSONS.md](./LESSONS.md) | 教训总索引（触雷时打开正文） |
| 4 | [TASKBOARD.md](./TASKBOARD.md) · **[TASK_REMAINING_FULL.md](./TASK_REMAINING_FULL.md)** · [TASK_INVENTORY.md](./TASK_INVENTORY.md) | 待办短表 · **剩余任务全文（禁止用短表代替）** · 全景清点 |
| 5 | **[PLAN_NEXT_SESSION_SYSTEM_REFACTOR.md](./PLAN_NEXT_SESSION_SYSTEM_REFACTOR.md)** | **下轮主线任务书** |
| 6 | 当轮其它 PLAN | 点名任务 |
| 6 | [tech/README.md](./tech/README.md) | 技术索引 |

---

## 2. 规范与教训关系

| 文件 | 写什么 | 不写什么 |
|------|--------|----------|
| NORMS.md | 必须/禁止、简短为什么、易触犯短例、指向 LESSONS | 长故事、协议细节 |
| LESSONS.md | 全 ID 表、字段规范 | 任务进度 |
| LESSONS_RUST_PORT | L* 全文 | 协作分发 |
| LESSONS_SESSION_COLLAB | C* 全文 | Gree 字节级细节 |
| AI_PROJECT_NORMS_PORTABLE | 跨项目认知规则全文 | 本仓路径 |

---

## 3. 设备身份文档

| 材料 | 状态 |
|------|------|
| tech/SDK_AND_LOGIN.md §5 | 源码级齐全 |
| LESSONS L10 · C4 | token 语义 |
| 服务端弹窗实测 | 【实测预留】 |

---

## 4. 任务规划

| 文件 | 角色 |
|------|------|
| [TASKBOARD.md](./TASKBOARD.md) | 短表真源 · NOW |
| [TASK_INVENTORY.md](./TASK_INVENTORY.md) | **全景清点**（PLAN+审计+logs） |
| PLAN_ANDROID · INSTANCE · UI · R3 · R4 · RELEASE · NEXT · PLAN.md | 分主题任务书 |
| PLAN_RESEARCH_AUTOMADOKA.md | R01；§1.0 意图：底层≫前端 |
| PLAN_PARTY_SELECT_UX.md | **C10** 队伍列表/手输二选一（暂不写代码） |
| **[PLAN_GROUP_RAID_UI.md](./PLAN_GROUP_RAID_UI.md)** | **组队 UI：添加只加号 · 借用卡片多配置 · 禁止抢跑**（GR-* 在 TASK_REMAINING_FULL） |
| tech/GROUP_RAID_AND_DEVICE_IDENTITY.md | 组队规格全文 · §8.1 入口口径 |
| tech/AUTOMADOKA_RESEARCH_AND_RUST_GAP.md | 原版原理 + 缺口；§研究意图 · §8.0 产品化 |
| tech/UPSTREAM_SOURCE_AND_WIRE.md | 原版对照完整性 · 通讯（包/发/收） |
| tech/DOC_COVERAGE_AUDIT.md | 文档覆盖审计 |
| tech/UPSTREAM_FILE_MAP.md | 官方 81 路径总表 |
| tech/INIT_AND_RESPONSE_PAYLOADS.md | 登录串与收回字段 |
| tech/API_INVENTORY.md | 494 API 全表 |
| PLAN_AUDIT_COMMS_OCCUPANCY_PROGRESS.md | **最终对齐**：像客户端/心跳/进度 |
| PLAN_MULTI_PLATFORM_LATER.md | 远期 macOS·Arch·OpenWrt |
| PLAN_RUN_PANEL_AND_THEME.md | 运行面板开关化 · 亮色远期 |

---

## 5. 技术文档

| 入口 | 内容 |
|------|------|
| [tech/README.md](./tech/README.md) | 全表 |
| [tech/TECH_DOC_CONVENTION.md](./tech/TECH_DOC_CONVENTION.md) | 双向链接与模板 |
| [tech/SURPASS_EVIDENCE_TABLE.md](./tech/SURPASS_EVIDENCE_TABLE.md) | 可代码验证的「全面超越」证据表（P30b） |
| [tech/WIRE_AND_DEBUG_PROBES.md](./tech/WIRE_AND_DEBUG_PROBES.md) | 开发版全量通讯录制与测试探针 |
| [tech/SESSION_TECH_REPORT_2026-08-07.md](./tech/SESSION_TECH_REPORT_2026-08-07.md) | 2026-08-07 技术报告（规范检查+剩余） |
| [logs/OWNER_INPUTS_RAW.md](./logs/OWNER_INPUTS_RAW.md) | 主人原始输入完整摘录（Grok prompt_history · 时间升序） |
| [OWNER_REQUIREMENTS_AND_TASKS_FULL.md](./OWNER_REQUIREMENTS_AND_TASKS_FULL.md) | 主人需求主题整理 + REQ 任务清单 + 当前进度（G10 详文） |
| [logs/grok-build-history/](./logs/grok-build-history/) | Grok Build 多会话历史副本（本机） |
| 协议 | PROTOCOL_STACK · SDK_AND_LOGIN · VERSION_FINGERPRINT · LESSONS_RUST_PORT |
| 双端 | ANDROID_DUAL_PLATFORM · UI_ROUTING · INSTANCE_AND_CLI |
| 安全 | SECURITY_AND_PRIVACY_AUDIT |

---

## 6. 仓库目录（摘要）

| 路径 | 角色 | 可写？ |
|------|------|--------|
| `crates/*` | 正式实现 | 是 |
| `apps/android/` | Android 壳 | 是 |
| `docs/` | 交接与技术 | 是 |
| `scripts/` · `publish/` | 构建/指纹源 | 是 |
| `RustMadoka.exe` · `RustMadoka_debug.exe` · `RustMadoka_data/` | 正式交付与运行时 | 本地；勿推 secret |
| `archive/` · `ref-legacy-superset/` | 只读对照 | 默认否 |
| `target/` · `run-clean/` · `run-remote/` | 构建/旧沙箱 | 本地 |

---

## 7. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-07 | 首版清单 |
| 2026-08-07 | 对齐 NORMS/LESSONS 分层；TECH_DOC_CONVENTION |
| 2026-08-07 | TASK_INVENTORY 全景清点 |
| 2026-08-07 | 全面交接 HANDOFF 定稿 |
| 2026-08-07 | R01 研究文 AUTOMADOKA_RESEARCH_AND_RUST_GAP |
| 2026-08-07 07:19 | 日志复核：TASKBOARD/TASK_INVENTORY 重排（log task-plan-reorg） |
| 2026-08-07 07:35 | 任务规划再整理；E1 专章；对齐明文 P9（log task-plan-reorg-2） |
| 2026-08-07 | GAME_FEATURE_FRAMEWORK 原游戏功能框架首版 |
| 2026-08-07 | GAME_FEATURE_FRAMEWORK：原游戏功能框架首版 |
