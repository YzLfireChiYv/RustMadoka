# 任务总表（短表 · 真源 · 已剔除完成项与被上位替代项）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-08（短表；积压记录 · 见 §0.6 · 全文 TASK_REMAINING） |
| **失真声明** | **本短表为 AI 维护导航，有可能出错和失真。** 全文条件见 [TASK_REMAINING_FULL.md](./TASK_REMAINING_FULL.md)。主人原意见 [logs/OWNER_INPUTS_RAW.md](./logs/OWNER_INPUTS_RAW.md)。需求对齐任务清单见 [OWNER_REQUIREMENTS_AND_TASKS_FULL.md](./OWNER_REQUIREMENTS_AND_TASKS_FULL.md)。 |
| **唯一完整现状入口** | [HANDOFF.md](./HANDOFF.md) |
| **剩余任务全文（禁止用本短表代替）** | **[TASK_REMAINING_FULL.md](./TASK_REMAINING_FULL.md)** |
| **全量重构主任务书** | [PLAN_RUSTMADOKA_FULL_REWRITE.md](./PLAN_RUSTMADOKA_FULL_REWRITE.md) |
| **多用户组等规格任务书（仍有效）** | [PLAN_NEXT_SESSION_SYSTEM_REFACTOR.md](./PLAN_NEXT_SESSION_SYSTEM_REFACTOR.md)（实现时以 MULTI_GROUP 全文为准） |
| **多用户组与监视完整规格（禁止用短表代替）** | [tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md](./tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md) |
| **语义假成功修复清单** | [logs/2026-08-07-known-issues-before-fix.md](./logs/2026-08-07-known-issues-before-fix.md) |
| **历史任务检索（含剔除说明）** | [logs/2026-08-07-historical-tasks-found.md](./logs/2026-08-07-historical-tasks-found.md) · [logs/2026-08-07-handoff-prune.md](./logs/2026-08-07-handoff-prune.md) |
| **规范** | [NORMS.md](./NORMS.md)（**P23 CLI⊇网页** · G8 · P5 · P29 · P31） |
| **组队 UI 任务书** | **[PLAN_GROUP_RAID_UI.md](./PLAN_GROUP_RAID_UI.md)**（卡片化；禁止添加下拉假平级） |

图例：`CODE`（代码已有，不等于 FIXED）· `PLAN` · `TODO` · `WAIT`（须主人点名或条件）· `LATER`（默认不抢主线）

**备忘 ≠ 开工（P7）。** 本表**只作导航**；每项完整条件、为什么要做、文档链见 **TASK_REMAINING_FULL**。已完成项与被上位方案替代的项见 §5 剔除表。

---

## 0. 本批已开工（2026-08-07 · 组队 Raid + device_id）

| ID | 内容 | 状态 |
|----|------|------|
| **GROUP-RAID** | 后端单号/降级 + **多配置卡片 UI/API/CLI** **CODE**；添加区只加号；待点测 FIXED | **CODE** 非 FIXED · [PLAN_GROUP_RAID_UI](./PLAN_GROUP_RAID_UI.md) · log cards-wire-audit |
| **DEVICE-ID-CARD** | device_id 按游戏账号卡片复用 | **CODE** · gree.rs |
| **STOP-OWNER-GROUP** | TaskGate 记录发起用户组；仅该组可停 | **CODE**（API 鉴权就绪；浏览器网页前端停钮可后接） |

---

## 0.5 本会话已推进（2026-08-08 · CODE 非 FIXED · 详见 HANDOFF §A）

| ID | 内容 | 状态 |
|----|------|------|
| **P23-CLI-BATCH** | `fp` · `task-log` · `control` · `notify` · system-toast | **CODE** |
| **ARENA-PATH** | `get_pvp_top` | **CODE** · JP 5 投降 wire |
| **BASIC-SKIP-POLICY** | 仅キオク/魔力优先；晶花-only 诚实 Skip | **CODE** · JP errors=0 |
| **RESEARCH-WIKI** | `docs/research/magia-exedra/` | 已抓 |
| **EN/JP-WIRE** | debug 全量 daily | CLI 验证 |
| **HANDOFF-0305** | 全面交接 | [HANDOFF](./HANDOFF.md) · [session-handoff-complete](./logs/2026-08-08-session-handoff-complete.md) |

## 0.6 后续积压（零碎 · 仅记录 · 未开工 · 2026-08-08 主人点名）

| ID | 完整内容 | 状态 | 说明 |
|----|----------|------|------|
| **SECRET-MAINLINE** | **完整移植通关主线**（原版工具「神秘新功能」`secret`：探索篇章推进；原版硬编码 612001，产品应配置化 · L12/C19） | LATER / 记录 | 原理见 HUMAN_FLOW 报告 §7.3 · archive tool.py；**未**写代码 |
| **ANDROID-SHELL-UI** | **重构 Android 壳**，并**设计针对 Android 的 UI**（非仅 WebView 套桌面页） | LATER / 记录 | 现行 `rustmadoka-mobile` + apps/android 为壳级；须主人后续规格 |
| **WITCH-GANG-UP** | **优化魔女组队逻辑**；产品拟更名为 **「魔女群殴」**；**细节须主人补充后再实现** | WAIT 主人补充 | 现行规格 GROUP_RAID；改名与逻辑变更前禁止抢写 |

---

## 1. 下一会话主线（默认开工入口）

真源全文：[PLAN_NEXT_SESSION_SYSTEM_REFACTOR.md](./PLAN_NEXT_SESSION_SYSTEM_REFACTOR.md)  
交互与多用户组完整条件：[tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md](./tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md)（**G10：不得用下表压缩代替该文全文**）

| 包 ID | 完整名称 | 状态 | 说明 |
|-------|----------|------|------|
| **SYS-AUDIT-REFACTOR** | 系统性地检查并重构程序代码（含模块语义、错误归类、IPC、Owner、日志；并入下方 FIX-*） | PLAN | 主人下轮主任务之一 |
| **DATA-DIR-RENAME** | 运行时默认已是 **`RustMadoka_data`** / 14103；**剩余**主要是 tech/文案旧名清理（并入 DOC-CONSISTENCY） | **运行时 CODE** · 文档债 | 见 session-task-reorg；勿重复假迁移大包 |
| **WEB-SYNC-CORE** | 面向更详细的浏览器网页前端同步与多用户组：更安全、更高效地重构底层状态与 API | PLAN / 半成品 | 规格 §2；run status 过滤与 game_id 投影已有增量 |
| **PROC-MONITOR** | 程序运行面板终端：完整只读监视；非异常不可输入不可控制；叉掉关闭；本体专有 vs 面向用户组账号；主页精简可选监视；设置页进度条服务非高阶用户并跨组同步 | **CODE 骨架** | 启动说明 + `[监视]` 摘要；完整高阶流未齐；上位替代 C07 |
| **CARD-CROSS-GROUP** | 多用户组同一游戏身份：A 组主页显示清日常或某功能运行中；B 组主页显示在用户组 A 中正在运行；设置进度跨组同步 | **CODE** · 非 FIXED | paintRunsToCards + accounts.run_label；WAIT 点测 |
| **CHANNEL-BADGE** | 游戏账号卡片上显眼 **EN / JP / TW** | **CODE** · 非 FIXED | ch-badge CSS |
| **PARTY-SELECT** | 需设队伍的功能：列表选择队伍与自行输入二选一（圆点 UI）；底层 `partyDataList` 产品化 | **CODE** · 非 FIXED | parties API + 设置圆点；需刷新拉表；WAIT 点测 |

建议实施顺序见主任务书 §3（更名 → FIX → 同步底层 → 卡片/角标 → 监视 → 队伍选择 → 审计收口）。

---

## 2. 短线语义假成功（可并入 SYS-AUDIT，亦可先做）

真源：[logs/2026-08-07-known-issues-before-fix.md](./logs/2026-08-07-known-issues-before-fix.md)（含 wire 证据：`loginBonusDataList` 空仍成功；活动次数 0 仍成功等）

| ID | 完整内容 | 状态 |
|----|----------|------|
| **FIX-LOGINBONUS** | 根据 `get_home_info` 回包中的 `loginBonusDataList` 等字段区分：无可领则跳过并中文说明；有可领则成功并尽量摘要。禁止无条件 `Ok("已领取登录奖励")`。 | **CODE · CLI+wire 已验证空列表→跳过**（`2026-08-07-en-debug-daily-wire`）；非空真领路径未采样；**非 FIXED** |
| **FIX-EVENT-EMPTY** | 当各活动 `todayPlayableCount` 为 0 且本模块未发出战斗或 skip 类请求时，结果必须是跳过，禁止空成功「活动扫荡 队伍=…」。 | **CODE · CLI+wire 已验证次数 0→跳过且无开战**；有次数成功路径未采样；**非 FIXED** |
| **FIX-EVENT-LOG** | 活动模块成功时日志必须写清活动名称与/或 id、本轮做了战斗还是 skip、次数变化；使后继能回答「扫了哪几个活动」。 | CODE（成功分支文案已写）；**有次数真跑未验证** |
| **FIX-SCENARIO-DEDUP** | 活动剧情：已读或不该重复请求的段落不得再次标为首次成功；过滤后再 request。主人已确认「ひとりの時間」两段在新开档期阅读为正确场景。 | CODE 既有 clear 过滤；本轮 EN 为「暂无新内容」跳过 |

---

## 3. 须主人点名或外部条件的项（不默认开写代码）

| ID | 完整内容 | 状态 | 规格或条件 |
|----|----------|------|------------|
| **19001-V** | 在游戏内具备链接 Raid 用编成（`isMultiRaid`）的账号上，再验证 self_raid 发车与业务码 19001 处理是否正确 | WAIT | 需有编成的号；W2 §3.1 |
| **C01** | 日常模块逐条真机点测后，再按证据写 FIXED（禁止仅凭 CLI failed=0） | WAIT | P5 |
| **W4-REL** | 正式发布说明、版本策略、上传普通版/开发版 exe 与 apk | WAIT | PLAN_RELEASE |
| **AUD-COMMS-A** | 对游戏服务器通讯节奏/像真客户端行为的系统审计与补强（与「本地明文」无关） | WAIT | PLAN_AUDIT 包 A |
| **Android-B** | Android B1–B10 正式验收勾选 | WAIT | PLAN_ANDROID |
| **CLI-M-wash** | 洗词条等能力与 HTTP 面是否仍有缺口的对齐核查与补齐 | WAIT | 点名再查 |
| **PORT-SMOKE / HEART-SMOKE** | 端口中文确认换号、跨路径心跳「我已知晓」的**真机手测**（实现已 CODE，验收未齐） | WAIT 手测 | INSTANCE_AND_CLI · occupancy.rs |

---

## 4. 默认后置（LATER · 不抢下一会话主线）

| ID | 完整内容 | 依据 |
|----|----------|------|
| **STORY-P** | 探索/主线推进配置化产品（禁止硬搬原版 secret 常量当唯一真理） | C19 · LATER |
| **W5** | macOS · Arch Linux · OpenWrt 等交付 | PLAN_MULTI · 总路线 W5 |
| **THEME-LIGHT** | 浏览器网页前端亮色模式 | PLAN_RUN_PANEL §2 · 远期 |
| **DOC-FULL-02** | 百科式文档主线 | LATER |
| **Sonet** | 台服登录与业务 | 未实现 · 点名 |
| **R4 托管/会话池等** | 原 R4 大项 | 门禁与点名 |
| **token 再加密等本机军备** | 禁止默认主线 | P9b |

---

## 5. 已从「待办」剔除的项目（完成 · 或被上位替代）

下列**不再**作为下一会话默认待办推销。史实与证据仍在 log/PLAN 中可查。

### 5.1 已完成（代码或文档已落地 · 不等于日常 FIXED）

| ID / 主题 | 说明 | 证据入口 |
|-----------|------|----------|
| **W1** | wire 录制 + EN 采样 | wire/en_w1 · w1 log |
| **W2** | 分析与 R1–R16 清单文档 | W2_WIRE_ANALYSIS… |
| **W3 可落地批次** | Skip/中文/假成功一批；en_w1 曾 failed=0 | w3 log · 非 C01 FIXED |
| **DUAL** | 普通版/开发版双 exe 与 build-win-dual | dual-exe-wire log |
| **E1** | 会话导出 CLI（及 HTTP 路由；Android 无入口） | e1 log |
| **OUT-PARTIAL** | 模块结果独立「部分完成」状态与计数 | mod.rs · batch-owner-port-partial |
| **OUT-EMPTY** | 多处空操作改为跳过 | outcome-batch · daily.rs |
| **OUT-BIZCODE** | 无新证据不扩业务码表（已按纪律收口） | error.rs 注释 |
| **OUT-DOC / G7 假绝对** | 技术文档语气与结果标签非完备 | ERROR_DIAGNOSTICS · NORMS |
| **C22 / P7b** | 禁止建议注水 | LESSONS · NORMS |
| **G8 G9 G10 等规范** | 禁止比喻简写、名称全称、禁止压缩主人原意 | NORMS |
| **明文 P9 口径** | 全平台永远允许明文 token 等 | NORMS · plaintext log |
| **占用心跳实现** | `occupancy_heartbeat.json` 独立于 owner.lock；「我已知晓」 | occupancy.rs · **手测未齐见 §3 PORT-SMOKE/HEART-SMOKE** |
| **端口换号实现** | 「我知道端口被占用」后用户自填端口并持久化 | bind_http_listener · **手测未齐同上** |
| **禁止自动 taskkill 普通版** | 开发版抢锁失败只提示手动关闭 | lib.rs |
| **CLI 任务日志落盘** | `run daily` 写 task_logs + run_config_snapshot | lib.rs · en-daily-cli-test |
| **DOC-LINK 一批** | 2026-08-07 双向链接补洞 | doc-bidirectional-link-audit |
| **R01 静态研究文** | AUTOMADOKA_RESEARCH_AND_RUST_GAP 等 | tech/ |
| **游戏语义分类 / 命名表 / 功能框架** | 文档已有 | MODULE_SEMANTIC · NAMING · FRAMEWORK |

### 5.2 被上位方案替代（不再单独排期）

| 旧 ID / 旧任务书 | 原内容摘要 | 上位替代 | 旧文如何处理 |
|------------------|------------|----------|--------------|
| **C07 / WATCH-TOGGLE** · [PLAN_RUN_PANEL_AND_THEME.md](./PLAN_RUN_PANEL_AND_THEME.md) §1 | 仅把浏览器网页前端旧运行条改成「是否开启」、默认关以省 800ms 轮询 | **PROC-MONITOR** + MULTI_GROUP 文 §3–§4：程序运行面板终端完整只读流；主页精简可选监视；设置页进度条；非异常不可控制；叉掉关闭 | 旧 PLAN **保留为史**；实现以 MULTI_GROUP 与 PLAN_NEXT §PROC-MONITOR 为准，**不要**再单独开 C07 窄实现 |
| 「部分完成是否升独立 status 须主人再定」 | 曾列为产品犹豫 | 已 **CODE** 独立 status「部分完成」 | 不再列为待决 |
| 任务书里重复罗列「双 exe 未做 / 心跳仅 PLAN / 端口 yes」等过时句 | 多份 HANDOFF/旧 § | 以当前 HANDOFF 与 INSTANCE 为准 | 旧 log 可保留时间戳史实 |

### 5.3 仍有效但已降为「手测残项」的 CODE

实现已在树内，**不**再当「从零开发」任务；仅保留 §3 **PORT-SMOKE / HEART-SMOKE** 作为真机验收残项。

---

## 6. 下一会话开机最小集

```text
1. docs/HANDOFF.md（本交接后全文）
2. docs/TASKBOARD.md（本文件 · 仅余下项）
3. docs/PLAN_NEXT_SESSION_SYSTEM_REFACTOR.md
4. docs/tech/MULTI_GROUP_UI_AND_MONITOR_SPEC.md（完整条件，禁止压缩读）
5. docs/logs/2026-08-07-known-issues-before-fix.md（若先做 FIX-*）
6. docs/NORMS.md：G2 G7 G8 G9 G10 · P5 P7b P7c P9 P16 P25 P26 P27 P28
7. 改代码后：build-win-dual.ps1；改浏览器网页前端：check-static-js.ps1
```

---

## 7. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-07 | 剔除已完成 OUT/W1–W3/DUAL/E1/心跳端口实现等；剔除被 PROC-MONITOR 上位的 C07；短表仅余主线+FIX+WAIT+LATER |
| 更早 | 见 git 与 historical logs |
