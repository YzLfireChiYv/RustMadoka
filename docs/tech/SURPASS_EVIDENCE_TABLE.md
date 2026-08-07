# 可代码验证的「全面超越」证据表

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-07 18:05（本机） |
| **标识** | **SURPASS-EVIDENCE-TABLE** |
| **失真声明** | **整份由 AI 维护，有可能出错和失真。** 证据优先级：当前源码与真机 > `docs/logs/` > 本表。禁止仅凭本表口号宣布「已全面超越」（NORMS **P30b · P31 · P5**）。 |
| **规范** | NORMS **P30 · P30b · G11** · 教训 **C27** |
| **Inbound** | [TASK_REMAINING_FULL.md](../TASK_REMAINING_FULL.md) §10A · [PLAN_RUSTMADOKA_FULL_REWRITE.md](../PLAN_RUSTMADOKA_FULL_REWRITE.md) · [AUTOMADOKA_RESEARCH_AND_RUST_GAP.md](./AUTOMADOKA_RESEARCH_AND_RUST_GAP.md) |
| **Outbound** | `crates/rustmadoka-core` · `crates/rustmadoka-app` · `archive/pre-rust-2026-08/autopcr/` |

---

## 0. 如何使用

1. 每条含：**能力**、**对照（原版 Python / 重构前）**、**代码侧如何证明**、**主人白话如何确认**、**状态**。  
2. 状态：`CODE` · `CLI` · `单测` · `HTTP` · `【未证】` · `WAIT 点测` · `LATER`。  
3. **CODE ≠ FIXED。** 主人点测通过前不得写「用户已可用」。  
4. 三维门槛（透明度 / 稳定性 / 功能性）在表末汇总；**AI 不得自行宣布已超越**。

---

## 1. 登录与指纹

| ID | 能力 | 对照 | 代码侧证明 | 主人白话确认 | 状态 |
|----|------|------|------------|--------------|------|
| S-LOGIN-EN | 国际服 Gree 登录 | archive greeclient + 历史 CLI | `gree.rs` · L1 黄金路径 · log en-debug-daily-wire | 浏览器或 CLI 能进号拿昵称 | CODE · CLI 样本；非主人 FIXED |
| S-LOGIN-JP | 日服登录 | 同左 | 同路径 channel=jp | 日服号能登录 | CODE；本会话未再跑 |
| S-LOGIN-TW | 台服 Sonet | Python 有 Sonet；Rust 拒绝 | `Channel::Tw` · `login_implemented` | 卡片见 TW 角标 + 未就绪提示 | **仅预留**（P30c） |
| S-FP-EMBED | 内嵌指纹保底 | 旧版易缺 | `fingerprint::EMBEDDED` · build.rs | 无网时仍可能有版本 | CODE |
| S-FP-RULES | rules 拉取 + 启用槽 | C1/P15/P29 | `fp_slots` · `/api/fp/*` · log ui-fp | 一键刷新后界面显示版本与时间 | CODE · HTTP；非主人 UI FIXED |
| S-FP-SILENT | 启动静默日检 | 产品要求 | serve 路径 silent refresh | 第二天自动检（可不感知） | CODE |

---

## 2. 模块语义（透明度）

| ID | 能力 | 对照 | 代码侧证明 | 主人白话确认 | 状态 |
|----|------|------|------------|--------------|------|
| S-OUT-LOGINBONUS | 无奖励→跳过，不假成功 | known-issues · Python Skip | `loginbonus_outcome_from_home` 单测 · wire 空列表 | 日志写「无可领」类跳过 | CODE · CLI+wire 空路径；非空【未证】 |
| S-OUT-EVENT0 | 活动次数 0→跳过 | known-issues | event_sweep · wire | 不写空成功「扫荡 队伍=」 | CODE · CLI+wire |
| S-OUT-BASIC-STAMINA | 训练扫荡按关卡 useStamina 算次数；失败 Skip/部分完成 | wire 214×10 假次数→500；mst 411102 耗体 15 | `daily::basic` · log basic-http500-fix | 日志见耗体与计划次数；不因过量 skip 整任务 error | **CODE** · 线上 skip 成功【未证·403】 |
| S-OUT-PARTIAL | 部分完成独立计数 | 原版有警告/部分语义 | `is_partial_success_log` · DailyReport.partial | 汇总能看见部分完成 | CODE |
| S-OUT-SKIP-ZH | 跳过中文、非 HTTP 长诊断 | C20/L13 | daily 多处 Skip · ERROR_DIAGNOSTICS | 日志可读 | 半成品持续 |
| S-SHOP-DEFAULT0 | 商店优先默认 0 | 原版 100 递减；P17b | `shop_priority_fields` default 0 | 新号设置里优先全 0 | CODE |

---

## 3. 多用户组与监视

| ID | 能力 | 对照 | 代码侧证明 | 主人白话确认 | 状态 |
|----|------|------|------------|--------------|------|
| S-GID-HASH | 列表不回传引继，用 game_id_hash | MULTI_GROUP §2.2 | `TaskGate::game_id_hash` · accounts API | 网络面板无明文引继 | CODE |
| S-CROSS-CARD | 跨组卡片「在用户组 A 中运行」 | MULTI_GROUP §5.3 | `paintRunsToCards` · `run_label` · runs_all | 两组同号时 B 组卡片见文案 | CODE；**WAIT 点测** |
| S-HOME-FILTER | 主页流只看本组 | MULTI_GROUP §4.1 | `?group=` 过滤 runs | 主页监视无他组明细 | CODE；WAIT 点测 |
| S-SET-PROGRESS | 设置页进度跟游戏身份 | MULTI_GROUP §4.2 | run poll 按 game_id_hash | 从 B 组开设置仍见进度 | CODE；WAIT 点测 |
| S-STOP-OWNER | 仅发起组可停 | MULTI_GROUP · TaskGate | `may_stop` · run pause body.group | 他组点放弃应失败 | CODE；WAIT 点测 |
| S-BADGE | EN/JP/TW 角标 | §5.5 | `channelBadgeHtml` · CSS | 扫一眼能分服 | CODE |
| S-PROC-MON | 程序运行面板终端只读监视 | §3 | serve 启动说明 + `[监视]` 行 | 黑窗见忙线摘要、叉掉即关 | CODE 骨架；完整高阶流【半成品】 |
| S-PARTY-UI | 队伍列表/手输二选一 | PLAN_PARTY_SELECT | config_type party · `/api/.../parties` | 设置里圆点选队 | CODE；需刷新拉表；WAIT 点测 |

---

## 4. 宿主与交付

| ID | 能力 | 对照 | 代码侧证明 | 主人白话确认 | 状态 |
|----|------|------|------------|--------------|------|
| S-DUAL-EXE | 普通/开发双 exe | 产品钉死 | `build-win-dual.ps1` · 根目录两文件 | 能双击两个文件 | CODE |
| S-DATA-DIR | RustMadoka_data · 14103 | 无旧兼容 | `paths.rs` · DEFAULT_LISTEN_PORT | 旁路文件夹与端口 | CODE |
| S-CLI-ZH | CLI 中文 help | R3 | clap about/help | `RustMadoka.exe --help` 中文 | CODE |
| S-OWNER | 同 data 单 Owner | P16 | owner_lock | 第二进程退出 | CODE |
| S-ROUTE-AUTH | 错 URL/加密验密 | C23 · PLAN_UI | applyRoute · login API | 错地址不串加密组 | 主人曾验 URL；改密 UI WAIT |
| S-GROUP-RAID | 组队编排 + 伤害拆分 | GROUP_RAID 规格 | group_raid.rs 单测 · CLI · HTTP POST | 多号互援 | CODE；真人点测 WAIT |
| S-R2-NS | core 语义分区命名空间 | FULL_REWRITE R2 | `protocol.rs` · `domain.rs` | （开发者项） | CODE 演进式；物理迁目录【未完】 |

---

## 5. 三维汇总（诚实 · 非宣布超越）

| 维 | 当前诚实判断（AI · 可能失真） | 主要缺口 |
|----|------------------------------|----------|
| **透明度** | 空奖励/次数 0 已显著改善；非空真领路径未采样 | 活动有次数 wire；部分模块英文 Skip 文案 |
| **稳定性** | 登录主路径 CLI 可用；宿主可编译双包 | 主人浏览器长稳点测；端口/心跳手测残项 |
| **功能性** | 日常/商店/洗词条/组队代码面大体在；多组 UI 本批补了一截 | MULTI_GROUP 全文未齐；PARTY 需真数据刷新；TW/Android/发布后置 |

**结论句（完整条件，非口号）：** 截至本表墙钟，重构版在多项契约与语义上相对早期假成功状态已有可核验改进，但 **尚未** 达到可自行宣布「相对原版 Python 与重构前 Rust 全面超越」的证据完备度；真号验收仍后置（P30）。

---

## 6. 修订

| 墙钟 | 内容 |
|------|------|
| 2026-08-07 18:05 | 首版。**AI 完成，有可能出错和失真。** |
