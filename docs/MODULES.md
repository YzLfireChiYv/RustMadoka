# 功能清单（日常 / 工具）

> **权威注册表：** `archive/pre-rust-2026-08/autopcr/module/modules/__init__.py`  
> **R2 台账：** [tech/PHASE_R2_MODULE_PARITY.md](./tech/PHASE_R2_MODULE_PARITY.md)  
> **原版原理 + Rust 缺口：** [tech/AUTOMADOKA_RESEARCH_AND_RUST_GAP.md](./tech/AUTOMADOKA_RESEARCH_AND_RUST_GAP.md)  
> **实现：** `crates/rustmadoka-core/src/modules/`

## 日常（daily）— 执行顺序

| # | key | 名称 | 产品默认 | Rust（2026-08-06） |
|---|-----|------|----------|---------------------|
| 1 | loginbonus | 领取登陆奖励 | **关** | 已实现 |
| 2 | stamina_buy | 购买体力 | **关** | 已实现 |
| 3 | super_sweep | 快速刷图 | **关** | **真战斗** init+finalize（原版设计）；非 skip |
| 4 | raid_reward | 魔女舔盒 | **关** | 已实现 |
| 5 | self_raid | 魔女召唤 | **关** | 已实现；队伍默认空 |
| 6 | support_raid | 魔女援助 | **关** | 已实现；队伍默认空 |
| 7 | like_raid | 魔女点赞 | **关** | 已实现 |
| 8 | solo_raid | 扫荡总力战 | **关** | 已实现 |
| 9 | high_score | 扫荡打分 | **关** | 已实现 |
| 10 | arena | 自动PVP投降 | **关** | 已实现 |
| 11 | basic | 智能体力扫荡 | **关** | **仅 skip** キオク/魔力解放已通关本；能力晶花进度关（411102 等）不可 skip → 诚实跳过。规格 BASIC_SUPER_SWEEP |
| 12 | event | 扫荡活动 | **关** | 已实现 |
| 13 | archive | 扫荡档案活动 | **关** | 已实现 |
| 14 | event_shop | 清空活动兑换币 | **关** | 已实现；优先级默认全 0；**耗资源** |
| 15 | raid_shop | 清空 raid 兑换币 | **关** | 已实现；优先级默认全 0；**耗资源** |
| 16 | arena_shop | 清空 jjc 兑换币 | **关** | 已实现；优先级默认全 0；**耗资源** |
| 17 | tower | 扫荡露娜塔 | **关** | 已实现 |
| 18 | heart | 扫荡心之器 | **关** | 已实现 |
| 19 | gather | 收集宝箱 | **关** | 已实现 |
| 20 | freegacha | 免费扭蛋 | **关** | 已实现 |
| 21 | eventscenario | 阅读活动剧情 | **关** | 已实现 |
| 22 | collection | 阅读光之间 | **关** | 已实现 |
| 23 | battle_mission | 完成战斗任务 | **关** | 已实现 |
| 24 | mission | 领取任务 | **关** | 已实现 |
| 25 | present | 领取礼物 | **关** | 已实现 |
| 26 | info | 玩家信息 | **关** | 已实现（R1 曾点测） |

**产品默认说明（2026-08-06 主人口令）：** 全部模块默认关；兑换商店优先级全 0；魔女召唤/援助队伍空。  
**真机点测：** 除 info 外均待主人验证 → 不写 FIXED。

## 工具（tool）

| # | key | 名称 | 状态 |
|---|-----|------|------|
| 1 | super_wash | 快速洗词条 | **可跑 + NDJSON 流式** |
| 2 | raid_support | 魔女救世 | 原版小号援助+打完退出；**产品上位：组队 group-raid**（`leave_after_support` 可选） |
| 3 | secret | 神秘新功能 / 探索篇章 | **LATER**（C19 主线配置化；禁止硬编码 FIELD_CLEAR） |
| 4 | auto_register | 注册十个号 | **OPEN/LATER**（测试号用 CLI `account add` 更可控） |
| 5 | clear_dungeon_event | 完成迷宫隐藏事件 | **CODE** · `run module --key clear_dungeon_event` · 非 FIXED |

## 组队 Raid（用户组级 · 非日常 key）

| 项 | 内容 |
|----|------|
| **规格** | [tech/GROUP_RAID_AND_DEVICE_IDENTITY.md](./tech/GROUP_RAID_AND_DEVICE_IDENTITY.md)（**§8 UI：添加卡片心智 · 面板设置 · 单号打满日次数 · 删卡降级**） |
| **实现** | `crates/rustmadoka-core/src/modules/group_raid.rs`（编排）；网页面板 **待按 §8 落地** |
| **CLI** | `run group-raid -g <组> --aliases a,b --room-open guild\|friend\|all …`；**单号** aliases 仅一人 = 打满今日次数（规格；实现须对齐） |
| **默认** | 援助后**不**退出；`leave_after_support` 可选 |
| **状态** | 后端单号/降级 + **多配置卡片 UI/API/CLI CODE**（添加区只加号）· [PLAN_GROUP_RAID_UI](./PLAN_GROUP_RAID_UI.md) · log `group-raid-cards-wire-audit`；待点测 FIXED；**台服不管** |

## 调度与 UX

- 一键清日常：`run_daily_with_progress`；**只跑已启用**；请求可覆盖 `enabled` + `config`  
- 门禁：`ALLOW_DAILY_RUN=true`（工具 `ALLOW_TOOL_RUN` 仍 false）  
- 设置 schema：`modules/config_catalog.rs`  
- 持久化：`GET/POST /api/accounts/:alias/config`（扁平字典）  
- 流式：`POST .../daily/stream`（NDJSON）；`.../wash/stream` 仍门禁  
- 目录 API：`GET /api/daily_modules`  

**关键设置示例（无默认/错值会导致 Skip 或 Abort）：**

| 模块 | 关键配置 |
|------|----------|
| super_sweep | force_battle_quest_id, force_battle_team, force_battle_repeat_times |
| self_raid | start_raid_party, start_raid_damage_*, start_raid_result |
| support_raid | support_raid_party, support_raid_id, support_search_times… |
| heart | heart_team, heart_force_sweep |
| stamina_buy | stamina_buy_count, stamina_retain_count |
| basic | basic_stamina_5/4/3star |
| *_shop | `{prefix}_shop_priority_{中文类别}`（0=不买） |

## 重写注意

- 复刻以 Python `do_task` 为准；API 字段对照 `model/requests.py`。  
- 耗资源模块默认关。  
- 长任务必须可视化进度。
