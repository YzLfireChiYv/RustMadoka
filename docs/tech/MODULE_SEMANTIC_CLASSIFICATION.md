# 现有自动化功能 · 语义分类（审核整理版）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-07（重新审核整理） |
| **地位** | 系统性重构用的**分类真源之一**；与 [GAME_NAMING_GLOSSARY.md](./GAME_NAMING_GLOSSARY.md) 功能身份表配套 |
| **分类框架（主人）** | ① 纯领取　② 养成　③ 战斗（自动选队 / 手动指定队） |
| **证据** | 原版 `archive/.../modules/*` · Rust `daily.rs` / `wash.rs` · 主人对话（机制、三类、纠正项） |
| **MAY CONTAIN ERRORS** | Yes — 混合模块边界可再议；协议 path 以源码为准（G7） |

---

## 0. 重构前提（已对齐）

| 点 | 口径 |
|----|------|
| **系统性重构** | 目标是全面重做认知与实现质量，**不是**把原版 Python 再抄一遍 |
| **照搬风险** | 路径写错、Skip/错误语义糊、硬编码、静默默认队伍、未懂游戏却 1:1 异常壳（见 C20/L13、gather 曾用错 path） |
| **正确做法** | **功能身份** + **读哪些 id** + **发哪些请求** + **已知结局**（成功/跳过/中止/错误等，能写则写，未知标【未证实】）；原版=原理对照。展示标签**不是**游戏结局全集（见 ERROR_DIAGNOSTICS §模块结果） |
| **名字** | 工具中文/玩家黑话只是别名；**优先 key + 协议**（命名表） |
| **客户端中文** | **仅繁体** |

---

## 1. 三类定义

| 类 | 主人定义 | 代码工作判据 |
|----|----------|--------------|
| **① 纯领取** | 游戏怎么领/怎么 Skip 扫荡，工具就怎么做；含总力扫荡、塔扫荡、经验本/材料（「石头」）本等 | 主路径为 `*skip*`、`receive`、`claim`、学习类非战斗 API 等；**不**为通关而 `initialize_stage` 真开打 |
| **② 养成** | 晶花等刷小词条；商店买东西 | 改词条/库存；**洗词条无战斗** |
| **③ 战斗** | 真正进战；**自动选队** 或 **手动/配置指定队** | `quest_battle`/`multi_raid`/`pvp`/`exploration` 等 initialize→finalize（或 raid 伤害链） |

| 附属 | 含义 |
|------|------|
| **①′** | 资源兑换（买体力），服务 ①/③ |
| **混合** | 同一次执行里既有 ③ 又有 ①（先写清阶段） |
| **—** | 只读/账号工具，不进玩法三类 |

---

## 2. 主人已确认的对照

| 说法 | 模块 key | 类 |
|------|----------|-----|
| 活动 | `event` | **混合 ③→①**（自动主队补通关，再 skip） |
| 活动的打分 | `high_score` | **①**（`score_attack` skip） |
| 经验本 / 材料「石头」本 | `basic` 为主 | **① 仅 skip**（キオク/魔力解放已通关后可扫；**非**真战斗；详见 BASIC_SUPER_SWEEP） |
| 指定关卡硬刷 | `super_sweep` | **③ 手动队真战斗**（原版设计就是战斗，不是 skip） |
| 扫荡总力战 | `solo_raid`（原版即有） | **①** |
| 扫荡塔（工具名露娜塔） | `tower` | **①**（常须已到顶） |
| 洗词条 | `super_wash` | **②**，**无战斗** |
| 商店 | `*_shop` | **②** |
| Raid 一体四切片 | `self_raid` / `support_raid` / `raid_reward` / `like_raid` | 战斗 / 领取见下表 |

---

## 3. 日常 26 全表（审核后）

顺序 = 原版 `daily_modules` / Rust `daily_catalog`。

| # | key | 工具中文 | 类 | 队伍 | 主协议/行为（摘要） |
|---|-----|----------|----|------|---------------------|
| 1 | loginbonus | 领取登陆奖励 | **①** | — | 领登录奖励 |
| 2 | stamina_buy | 购买体力 | **①′** | — | `user/set_stamina_recover` |
| 3 | super_sweep | 快速刷图 | **③ 手** | 配置必填 | **真战斗** `init+info+finalize` 循环（原版 tool.py；非 skip） |
| 4 | raid_reward | 魔女舔盒 | **①** | — | `multi_raid/receive_reward` |
| 5 | self_raid | 魔女召唤 | **③ 自** | 代码规则+配置 | multi_raid **自己开房**打；默认本期最高难度 |
| 6 | support_raid | 魔女援助 | **③ 自/配** | 规则+配置等级 | multi_raid **指定等级**；可限工会 |
| 7 | like_raid | 魔女点赞 | **①** | — | multi_raid 点赞（非开打） |
| 8 | solo_raid | 扫荡总力战 | **①** | — | `solo_raid/skip_quest_battle`；灯塔危机相关 |
| 9 | high_score | 扫荡打分 | **①** | — | `score_attack/skip_quest_battle`（=活动打分） |
| 10 | arena | 自动PVP投降 | **③′** | 自动 PVP 队 | pvp 进房后投降；周常 |
| 11 | basic | 智能体力扫荡 | **①** | —（skip 不要求 party） | **只** `quest_battle/skip`；优先 101/魔力解放组；能力晶花进度关通常不可 skip → 诚实 Skip。规格：BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS |
| 12 | event | 扫荡活动 | **混合** | **自动** partyType=1 | 未通关 init+finalize → 剩余 skip |
| 13 | archive | 扫荡档案活动 | **混合** | 自动主队类 | 与 event 同构（档案） |
| 14 | event_shop | 清空活动兑换币 | **②** | — | shop 购买 |
| 15 | raid_shop | 清空raid兑换币 | **②** | — | shop 购买 |
| 16 | arena_shop | 清空jjc兑换币 | **②** | — | PVP 币商店（jjc=习惯说法） |
| 17 | tower | 扫荡露娜塔 | **①** | skip 可带队 id | `tower/skip`；层称镜/鏡；工具名非官方 |
| 18 | heart | 扫荡心之器 | **①** | 可配/自动 | 次数本扫荡向 |
| 19 | gather | 收集宝箱 | **①** | — | `gathering/get_gathering_top` 等（path 已对齐原版） |
| 20 | freegacha | 免费扭蛋 | **①** | — | `gacha/get_gacha_top` + exec |
| 21 | eventscenario | 阅读活动剧情 | **①** | — | 已读/adv |
| 22 | collection | 阅读光之间 | **①** | — | collection 已读 |
| 23 | battle_mission | 完成战斗任务 | **③ 自** | 自动主队 | 探索/任务点真战斗 |
| 24 | mission | 领取任务 | **①** | — | mission receive |
| 25 | present | 领取礼物 | **①** | — | present receive |
| 26 | info | 玩家信息 | **—** | — | 只读展示 |

**③ 自** = 代码自动选队　**③ 手** = 用户配置队伍　**③′** = 特殊战斗（投降）

---

## 4. 工具区

| key | 工具中文 | 类 | 说明 |
|-----|----------|----|------|
| super_wash | 快速洗词条 | **②** | `learn_sub_selection_ability` 循环；**无战斗** |
| secret | 神秘新功能 | **③** | 探索推进；**原理重写**，不复用源码（C19） |
| clear_dungeon_event | 完成迷宫隐藏事件 | **③** | 已通关篇迷宫事件 |
| raid_support | 小号救援 | **③** | multi_raid 辅号 |
| auto_register | 注册号 | **—** | 账号工具 |

---

## 5. 按类汇总

### ① 纯领取

loginbonus · raid_reward · like_raid · **solo_raid** · **high_score** · **basic** · **tower** · heart · gather · freegacha · eventscenario · collection · mission · present  

①′：stamina_buy  

### ② 养成

event_shop · raid_shop · arena_shop · **super_wash**  

### ③ 战斗

| 自动选队 | 手动/配置队 | 特殊 |
|----------|-------------|------|
| event / archive 的补通关段 | super_sweep | arena 投降 |
| self_raid · support_raid | heart 可配队 | |
| battle_mission | | |
| secret · clear_dungeon_event · raid_support（工具） | | |

### 混合（重构必拆阶段）

| 模块 | 阶段 |
|------|------|
| **event** / **archive** | ① 若仍有未通关可打次数：③ 自动队真打 → ② 剩余：① skip |

---

## 6. multi_raid 四切片（同一功能）

| 工具名 | key | 类 | 行为（主人+代码） |
|--------|-----|----|-------------------|
| 魔女召唤 | self_raid | ③ | 自己开房；默认本期最高难度；不填等级 |
| 魔女援助 | support_raid | ③ | 打配置等级；可只打工会房 |
| 魔女舔盒 | raid_reward | ① | 领奖 |
| 魔女点赞 | like_raid | ① | 点赞 |

玩法骨架：限时、约 20 级、新期 id 可新可旧、旧期继承进度、通关后自选难度开房、他人可加房（含仅自己/工会等）；一人一队，朋友可帮。

---

## 7. 已知代码事故（分类审核时记下）

| 项 | 状态 |
|----|------|
| gather/freegacha 曾用错 path（`get_top`）导致 404 记成「错误」 | 已改为 `get_gathering_top` / `get_gacha_top`；**EN：gather 可领时 R1 全链成功，冷却后 Skip；freegacha 无免费三轮 Skip，exec 未采到**（W2 三轮） |
| Skip 文案大量英文 | 仍有残留；重构 ① 类时应中文业务句 |
| secret 硬编码 612001 | 产品须配置化，且实现宜重写 |
| **super_sweep 全轮失败仍 Ok** | W3 **R1 CODE**：0 轮成功 → Skip |
| **solo_raid 18054 记错误** | W3 **R2/R3 CODE** → Skip |
| **self_raid 19001** | W3：isMultiRaid=false 明确 Skip + 码表；有团战队时再证根因 |

---

## 8. 文档关系

| 文档 | 角色 |
|------|------|
| **本文** | ①②③ 分类 + 模块全表 |
| [W2_WIRE_ANALYSIS_AND_REWRITE_LIST](./W2_WIRE_ANALYSIS_AND_REWRITE_LIST.md) | wire 证据 + W3 变更清单 R*（§8 落地） |
| 实现 | `crates/rustmadoka-core/src/modules/{mod,daily}.rs` |
| GAME_NAMING_GLOSSARY | 功能身份、别名、协议优先 |
| GAME_FEATURE_FRAMEWORK | 游戏内容大图 |
| ERROR_DIAGNOSTICS / C20 | 业务 Skip 与诊断 |

---

## 9. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-07 | 首版分类 |
| 2026-08-07 | 洗词条无战斗；活动打分=high_score |
| **2026-08-07** | **重新审核整理：统一前提、全表、混合阶段、raid 四切片、已确认项、事故栏** |
| **2026-08-07** | W2：链 wire 分析；补充 super_sweep/solo_raid/self_raid 事故 |
