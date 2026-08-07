# 原游戏功能框架（Magia Exedra × automadoka 对照）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-07 |
| **性质** | **游戏内容与系统分区的工作框架（草案级可演进）**；对接原版工具能力与 Rust 现状。主产出是认知与排期底座；随新 mst/真机/源码应改 |
| **方法** | 用 **已落盘数据 + 原版/协议代码** 搭骨架，再用 **攻略站名词** 对照玩家可见名。证据冲突时优先：当次对服回包/mst → 原版源码 → 攻略站（攻略站可错、可过时） |
| **Outbound 数据** | `RustMadoka_data/exports/**`（E1）· `exports/_analysis/field_stage_mst.json` 等（**单环境样本**） |
| **Outbound 代码** | `archive/.../autopcr` · `docs/tech/API_INVENTORY.md` · `crates/rustmadoka-core` |
| **外援名词** | Game8 等公开攻略（Main/Nightmare/Chaos 等叫法） |
| **Inbound** | HANDOFF · LESSONS C18–C20 · L12–L13 · NORMS **G7** · TECH_DOC_CONVENTION §1.10 |
| **MAY CONTAIN ERRORS** | **Yes。** 后继 AI 若发现与源码/真机/新导出矛盾，**应当质疑并修订本文**，勿因「文档已写」而压抑证伪。 |

---

## 0. 本文尝试回答什么（可随证据改答案）

1. 游戏侧大致有哪些大块内容（玩家向分区 + 协议向分区）——**开放集合，会随版本增删**。  
2. 已观察到的块在协议/数据上**常见**挂靠位置（API 前缀、mst、init）；未观察的不表示不存在。  
3. 原版 automadoka 自动化**已登记**覆盖与缺口。  
4. 现行 Rust **当前树**上的覆盖与诚实边界。  
5. 后续深挖与产品化（含可配置主线推进）**建议**挂靠哪一层。

**纪律（C20 / P25 / G7）：** 描述自动化步骤时，尽量写清：游戏里做什么、成功、有意跳过、真失败（能写多少写多少，未知则标【未证实】）。本文先搭框架；模块「原理表」逐步填，填错可改。

---

## 1. 总览：三层模型

```text
┌─────────────────────────────────────────────────────────────┐
│ L0  玩家可见内容（wiki/UI 名词）                              │
│     记忆之窗/主线难度 · 养成 · 扭蛋 · 商店 · 活动 · 社交…      │
└───────────────────────────┬─────────────────────────────────┘
                            │ 名词对照
┌───────────────────────────▼─────────────────────────────────┐
│ L1  游戏系统 / 协议域（API 前缀 · mst 表 · 会话资产）          │
│     exploration · quest_battle · multi_raid · gacha · …     │
└───────────────────────────┬─────────────────────────────────┘
                            │ 自动化映射
┌───────────────────────────▼─────────────────────────────────┐
│ L2  工具能力（原版模块 · Rust 模块 · 导出数据）                │
│     daily 26 · tool · cron · E1/E2 快照 · STORY-P（后置）   │
└─────────────────────────────────────────────────────────────┘
```

分析顺序说明：本轮采用 **「数据与协议搭骨架 → wiki 补名词 → 模块表填能力」**。仅 wiki 或仅函数名都容易偏；交叉核对更稳，仍不保证穷尽。

---

## 2. L0 玩家可见内容分区（wiki 对照）

下列分区来自公开攻略站常见导航与解锁说明（Game8 等），用于**命名对照**；id、解锁条件、是否仍存在某玩法，以当次 mst/回包为准。表**不声称**穷尽全部玩法。

| 分区 | 玩家侧含义（摘要） | 备注 |
|------|-------------------|------|
| **主线 / 记忆之窗** | 按故事篇（魔女·记忆）推进；含地图点、迷宫、Boss | 攻略常分 Main / Nightmare / Chaos 等模式 |
| **强化·养成** | 角色（キオク/style 等）、等级、素材本 | 序盘解锁与主线进度挂钩（wiki 有解锁表） |
| **心之器等养成本** | 专项扫荡类内容 | 工具侧有 heart 模块 |
| **活动** | 期间限定关卡、剧情、兑换 | event / event_shop / eventscenario |
| **档案活动** | 复刻/档案向扫荡 | archive 模块 |
| **限时 Raid（リンクレイド）** | 20 级限时；开房/加入；工具拆召唤·援助·舔盒·点赞 | multi_raid · 见 GAME_NAMING §1.1 |
| **灯塔危机·多队接力** | 自己多队接力（对比 raid 单队可好友帮） | 原版模块 **扫荡总力战** `solo_raid` · §1.2 |
| **打分 / 竞技** | 分数挑战、PVP（jjc=习惯说法） | score_attack · pvp |
| **塔 / 镜层** | 爬塔；工具曾称露娜塔 | tower · §1.4 |
| **收集·光之间** | 图鉴/剧情阅读与红点 | collection |
| **扭蛋** | 抽卡；含免费池 | gacha |
| **商店 / 兑换** | 各币种兑换 | shop 系 API + 工具商店模块 |
| **任务 / 礼物 / 登录奖励** | 日常领取 | mission · present · loginbonus |
| **宝箱 / 挂机领取** | 收集系定时收益 | gathering（工具名：收集宝箱） |
| **邀请** | 邀请绑定 | Invitation API；secret 段 A |
| **社交 / 公会 / 聊天** | 好友、guild、gvg 等 | 协议面很大；原版日常几乎不碰 |
| **设置 / 教程** | 选项、教程步 | userParam.tutorialStep 等 |

### 2.1 主线难度（与 mst `difficulty` 对照 · 工作假说）

本机一次拉到的 `field_stage_mst` **108** 条。补充观察（`exports/_analysis/framework_continue.json`）：

| 观察 | 内容（单样本，可被新 mst 推翻） |
|------|--------------------------------|
| 条数 | difficulty **1/2/3 各 32**；**4 为 12** |
| id 末位 | 本样本中 **id % 10 == difficulty**（d1 全以 1 结尾，d2 以 2…）；**像**刻意编码，是否全版本成立待更多样本 |
| 同故事多难度 | 约 **32** 个 family 前缀同时含 d1+d2+d3（例：600001/600002/600003 皆「薔薇園…前編」系） |
| 命名 | d1 名称常带「某某的记忆」；d2/d3 常为短名；d4 样本里有独立 id 段（如 700004「薔薇園の魔女」） |
| prev2 | d1 样本 **prev2 全 0**；d2 **多数 prev2≠0**（例 600002：prev1=602001，prev2=600001）——像「要先推进度又要对本篇 Main」类条件，**完整规则未写清** |
| 本号通关 | collection 已 clear 的篇章，样本里 **difficulty 计数偏 d1**（与「先打 Main」玩法一致，仍是单号） |

| difficulty | 样本条数 | 与攻略名词（**假说**） |
|------------|----------|------------------------|
| **1** | 32 | 攻略 **Main**？ |
| **2** | 32 | 攻略 **Battle**？ |
| **3** | 32 | 攻略 **Nightmare**？ |
| **4** | 12 | 攻略 **Chaos** 或精简档？`clear_dungeon_event` 对 d4 **continue 跳过**（仅原版工具行为） |

**prev 边：** secret 递归只用 `prevFieldStageMstId`。`prevFieldStageMstId2` 在 d2 上很活跃，用途【未穷尽】。  

**diff=1 沿 prev1 到 612001 的链**：见 export-data-analysis log。  
secret **只**从硬编码 612001 入口递归；更新篇章需配置化（C19），本文不把 612001 写成「游戏主线终点」。

### 2.2 篇内结构（探索）

| 概念 | 协议 | 玩家感受 |
|------|------|----------|
| 篇章 fieldStage | fieldStageMstId · name | 一扇「记忆之窗」 |
| 层 stratum | fieldStratumMstId · stratumNum | 地图分层 |
| 点 point | fieldPointMstId · name（1-1…Boss） | 可到达节点 |
| 点类型 pointType | 1 迷宫；2/3/4 战斗（secret 分支） | 迷宫探索 vs 战斗 |
| 进度 | clearFieldPointMstIdCsv 等 | 已清点列表 |

例：600001 篇内 **13 点直线**（1-1…Boss）见分析 log。

---

## 3. L1 协议 / 系统域（API 前缀骨架）

来源：某次生成的 `API_INVENTORY` 路径前缀统计（文档称约 494 唯一路径量级，**以再生成为准**）+ 本机 E1 init 字段。前缀→内容为**工作对照**，单 API 可能跨玩法。

| 协议域（前缀） | 工作中的内容对照 | 登录/日常里常见触达（观察） |
|----------------|------------------|------------------------------|
| **mst** | 几乎所有静态定义表 | 登录 bootstrap 子集；推进需 field_* 等 |
| **user** | 账号、init、体力参数、选项 | full_login 核心 |
| **exploration** | 探索地图、点、迷宫事件 | secret / clear_dungeon_event / 部分任务 |
| **quest_battle** / **quest_out_game** | 关卡战斗与外围 | 扫荡、刷图、任务战 |
| **multi_raid** | 魔女多人战 | raid_* 模块 |
| **solo_raid** | 总力战 | solo_raid 模块 |
| **score_attack** | 打分 | high_score |
| **pvp** / **gvg** | 对战 | arena 等 |
| **tower** | 塔 | tower |
| **gacha** | 扭蛋 | freegacha |
| **collection** | 收集/光之间/adv 已读 | collection · eventscenario |
| **party** / **style** / **character** / **card** | 编成与养成资产 | init_data；resolve_party |
| **selection_ability** | 词条 | 洗词条 |
| **gathering** | 宝箱挂机 | gather |
| **present** / **mission** 等 | 礼物任务 | present · mission |
| **shop** 系 | 兑换 | event_shop 等 |
| **guild** / **friend** / **chat** | 社交 | 原版日常覆盖弱 |
| **home** / **notification** | 主页通知 | 部分 |
| **alternative_story** | 支线故事 | 待填 |
| **style_rental** | 租借 | init 有 rental 列表 |
| **debug** | 调试向 | 正式工具路径通常应避开 |

### 3.1 登录后常见进入内存、且 E1 会带上的资产（子集，随 full_login 实现而变）

| 块 | 内容 | 本仓库 E1 样本 |
|----|------|----------------|
| partyDataList | 队伍名/序号/id | 有 |
| style/character/card/item | 持有 | 有 |
| userParamData | 等级体力、clearedFieldStageMstId、tutorialStep… | 有 |
| game_config | 配置嵌套 | 有 |
| mst 若干表 | 当前实现含 style/character/figure/selection_ability 等 | 有（偏洗词条） |
| field_stage / point / collection | 探索图与通关 | 当前 E1 **未**写入；分析时另拉过 |

**单号快照示例**（群友日服、一次导出）：level 53；clearedFieldStageMstId 612001；队伍 2；style 9；item 种类约 30。其它账号/版本会不同。

---

## 4. L2 自动化框架（原版 automadoka）

### 4.1 页签

| 页签 | key | 实质 |
|------|-----|------|
| 日常 | daily | **26** 模块有序 |
| 工具 | tool | 洗词条、raid 辅、secret、注册、迷宫隐藏事件 |
| 定时 | cron | 6 槽钟点跑日常 |
| 规划/角色/公会/危险 | — | **空占位** |

### 4.2 日常 26 ↔ 游戏分区（能力映射）

| 模块 key | 中文 | 主要挂靠 L0/L1 | 语义注意（P25） |
|----------|------|----------------|-----------------|
| loginbonus | 登陆奖励 | 登录奖励 | 已领→应跳过 |
| stamina_buy | 购买体力 | 体力 | 达上限/无石→跳过 |
| super_sweep | 快速刷图 | quest_battle | 队伍解析；失败≠跳过 |
| raid_reward | 魔女舔盒 | multi_raid | 无可领→成功或跳过须统一 |
| self_raid / support_raid / like_raid | 召唤/援助/点赞 | multi_raid | 次数/房间条件 |
| solo_raid | 总力战扫荡 | solo_raid | 无活动→跳过 |
| high_score | 扫荡打分 | score_attack | 无→跳过 |
| arena | PVP 投降 | pvp | 默认关 |
| basic | 智能体力扫荡 | quest + 体力 | 复杂分支 |
| event / archive | 活动/档案扫荡 | 活动 | 无→跳过 |
| event_shop / raid_shop / arena_shop | 兑币 | shop | 优先级配置 |
| tower | 露娜塔 | tower | 关闭→跳过 |
| heart | 心之器 | 养成本 | |
| gather | 收集宝箱 | gathering | **业务空转易被报成错误**（见 C20 样本） |
| freegacha | 免费扭蛋 | gacha | 同上样本 HTTP_404→错误 |
| eventscenario / collection | 剧情/光之间 | collection | 无红点→成功/跳过 |
| battle_mission | 战斗任务 | 任务+探索/战斗 | 已通关点再打 |
| mission / present | 任务/礼物 | mission/present | 无可领→跳过 |
| info | 玩家信息 | userParam | 只读 |

### 4.3 工具 5 项

| 模块 | 游戏意义 | 产品注意 |
|------|----------|----------|
| super_wash | 刷词条 | mst 全表列表 |
| raid_support | 小号救魔女 | 独立进程模型 |
| **secret** | 邀请 + **探索 Main 链清到硬编码篇** | STORY-P 样例；须配置化（C19） |
| auto_register | 批量注册 | 危险 |
| clear_dungeon_event | 已通关篇隐藏迷宫事件 | 不推进未通关主线 |

### 4.4 横切能力（非模块表）

| 能力 | 说明 |
|------|------|
| 会话 full_login | 拿齐 init + 部分 mst |
| 通用 request | 任意 `/api/...` |
| 账号/用户组 | 多号、加密组可选 |
| 定时 cron | 钟点日常 |
| Web/CLI | 人机入口 |

---

## 5. Rust 重构覆盖（诚实矩阵）

| 域 | 状态 |
|----|------|
| 协议登录 Gree + AES + 日/国 | 可用 |
| 日常 26 代码路径 | 有实现路径；语义与点测未齐（C20/L13）；单模块可能与 Python 有偏差 |
| 洗词条 | 有实现 + 门禁 |
| secret / clear_dungeon_event / cron / Sonet | 当前树中**未见**对等移植（以源码为准） |
| E1 会话导出 | Win CLI 至少一次成功（见 e1 log）；HTTP 有路由、点测范围另记 |
| 探索 mst 整包导出 | 分析用 example 拉过；产品形态未定 |
| 报错诊断壳 | 有；业务跳过与错误在样本日志中曾混淆 |
| Android | WebView 壳主人侧曾可用；E1 **不提供** Android 入口（产品钉死） |

---

## 6. 框架使用方式（给后续 AI / 人）

| 目的 | 打开 |
|------|------|
| 玩家/官方/工具名词 | **[GAME_NAMING_GLOSSARY.md](./GAME_NAMING_GLOSSARY.md)**（分 A–F 层，禁混用） |
| 协议落点 | 本文 §3 + API_INVENTORY + INIT 文档 |
| 自动化已有 | 本文 §4 + MODULES_RUNTIME + RESEARCH 缺口 |
| 探索图细节 | export-data-analysis log · L12 · field_stage_mst.json |
| 改模块语义 | P25：写成功/跳过/失败表再改代码 · C20 |
| 主线推进产品 | C19 STORY-P：后置、默认可配置 |

**建议避免：** 仅以「原版有同名模块」宣布理解完成；仅以 wiki 清单代替 mst id；把本文表格当成不可修订的教条（G7）。

---

## 7. 建议的下一分析切片（非实现队列 · 可改序）

1. ~~gather/gacha 路径与 Skip 文案~~ → **本轮已推进**（见附录 A/B；路径已改正，**待真机再跑日常验证**）。  
2. **difficulty 假说**：用更多账号 mst / 攻略解锁条件交叉验证 d1–d4 与 Main/Battle/Nightmare/Chaos。  
3. **multi_raid** 状态机草图。  
4. **E2 导出草案**（field_stage + collection 白名单）。  
5. **STORY-P 配置项清单**（后置实现）。  
6. 其它日常模块 API 路径 **对照 requests.py 抽检**（防止同类 get_top 笔误）。

产品实现顺序仍以 TASKBOARD / 主人点名为准。

---

## 8. 附录 A · gather（收集宝箱）原理表（修订）

| 项 | 当前认识（可修正） |
|----|-------------------|
| 游戏侧 | 首页挂机宝箱类领取；原版描述「收集首页宝箱」 |
| 原版 API | `get_gathering_top` → 可选 `shortcut_gathering` → 满时 `receive_reward` |
| **路径事故** | Rust 曾写 `/api/gathering/get_top`；原版为 **`/api/gathering/get_gathering_top`**。与样本日志 **HTTP_404 → 记成错误** **高度吻合**（仍建议真机确认） |
| 原版 Skip | `SkipError("宝箱时间未超过10小时，不收取")`——主人举的「时间/量不够就不收」类语义在此 |
| 当前 Rust（修订后） | 路径对齐原版；未满 10h → `Skip("宝箱时间未超过约10小时，不收取")`；成功日志中文化 |
| 成功 | 加速（若 shortcutCount==0）+ 或领取奖励 |
| 真失败（候选） | 签名/会话坏、路径再变、非预期业务码【需样例】 |

## 9. 附录 B · freegacha（免费扭蛋）原理表（修订）

| 项 | 当前认识（可修正） |
|----|-------------------|
| 原版 API | **`/api/gacha/get_gacha_top`**（非 `get_top`）→ `gacha_exec` |
| **路径事故** | Rust 曾用 `get_top`；与同次 404 样本 **同样高度吻合** |
| 原版无免费池 | 循环不跑，**常不抛 Skip**（空成功） |
| 原版次数用尽 | `_log` 跳过，continue |
| 当前 Rust（修订后） | 路径对齐；无任何免费可抽 → `Skip("没有可抽的免费扭蛋")` |
| 待真机 | 有免费池时 exec 字段、结果解析是否够用 |

## 10. 附录 C · 路径对照教训（给实现）

| 模块 | 错误路径（曾） | 原版 requests.py |
|------|----------------|------------------|
| gather | `/api/gathering/get_top` | `/api/gathering/get_gathering_top` |
| freegacha | `/api/gacha/get_top` | `/api/gacha/get_gacha_top` |

说明：1:1 移植时若**凭名字猜路径**而未打开 `requests.py` 的 `url` 属性，会把协议 404 送进「错误」通道，看起来像业务失败（C20）。

---

## 11. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-07 | 首版：L0/L1/L2 三层 |
| 2026-08-07 | G7 语气软化；附录 A/B 草案 |
| 2026-08-07 | **difficulty 样本规律；gather/gacha 路径根因与代码修正；附录 C** |
