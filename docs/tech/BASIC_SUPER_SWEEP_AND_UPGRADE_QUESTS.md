# 技术规格：智能体力扫荡 · 快速刷图 · 强化クエスト（对照原版 + 游戏资料）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-08（关卡 ID↔名称产品化批） |
| **失真声明** | **AI 整理，有可能出错和失真。** 证据：原版 Python + wire + 本地 wiki 摘录。主人补充优先。 |
| **Inbound** | 主人 2026-08-08：扫荡=skip；无 skip 则真战斗；快速刷图本就是战斗；411102 不可 skip；原版有关卡 ID–名称表 |
| **Outbound 源码** | `archive/.../stamina.py` · `tool.py` · `crates/rustmadoka-core/src/modules/daily.rs` · `mst.rs` · app `mst` CLI / `run_ops` / HTTP |
| **本地资料** | `docs/research/magia-exedra/`（exedra.wiki / madodra.wikiru / game8 / 官网摘录） |

---

## 1. 原版 automadoka 职责（必须分清）

### 1.1 `basic` — 智能体力扫荡（日常 · ① 扫荡）

| 项 | 内容 |
|----|------|
| 文件 | `archive/pre-rust-2026-08/autopcr/module/modules/stamina.py` |
| 显示名 | `智能体力扫荡` |
| **description 原文** | `根据角色缺口扫荡最高等级素材本，如果材料溢出则扫荡经验本` |
| **协议** | **仅** `QuestBattleApiSkipQuestBattleRequest` → `/api/quest_battle/skip_quest_battle` |
| **没有** | `initialize_stage` / `finalize` 真开打 |
| 进度源 | `QuestOutGameApiGetUserTrainingQuestDataList` → `userQuestTrainingDataList`：`questGroupMstId` + `clearedQuestStageMstId`（另有 `rankUpEffectedQuestStageMstId`） |
| 选关 | 按角色魔力突破缺口算素材效率；溢出则 rate=-1 仍可选「经验向」本 |
| 次数 | 原版 `to_repeat = stamina // 10`（`ONCE_STAMINA_COST=10`）；Rust 改为按关卡 `useStamina` 更合理 |
| 产品关键词 | **扫荡 = skip**。不能 skip 的关**不是** basic 的目标解法 |

### 1.2 `super_sweep` — 快速刷图（日常列表挂载 · ③ 真战斗）

| 项 | 内容 |
|----|------|
| 文件 | `archive/.../tool.py` |
| 协议 | 循环 `initialize_stage` → `get_quest_info` → `finalize_stage_for_user` |
| 配置 | 关卡 ID、队伍、次数、autoMode |
| 耗体 | mst `useStamina // 2`（与 skip 训练本不同） |
| 产品关键词 | **原版就是真战斗**（技能石等依赖结算），不是 skip |

### 1.3 其它 ① 扫荡（对照）

| key | skip path（摘要） |
|-----|-------------------|
| solo_raid | `/api/solo_raid/skip_quest_battle` |
| high_score | `/api/score_attack/skip_quest_battle` |
| tower | `/api/tower/skip_quest_battle` |
| heart | 心之器/Heartphial 类，规则与强化本不同 |

---

## 2. 游戏侧：强化クエスト（Upgrade Quests）

本地摘录：`docs/research/magia-exedra/wiki-exedra/Upgrade_Quests.clean.md`（来源 [exedra.wiki Upgrade Quests](https://exedra.wiki/wiki/Upgrade_Quests)）。

| 类型（英/日习惯） | 作用 | Rank | 可 skip（wiki） |
|-------------------|------|------|-----------------|
| **Kioku Training**（キオク强化素材） | エンハンスグロウ等经验素材 | 1–20 | **已通关后**可 skip；每 skip 消耗 **10 QP** 量级 |
| **Magic Unlock**（魔力解放素材，分属性） | プリズムストーン/属性石等 | 1–20 | **同上，可 skip** |
| **Crystalis**（能力晶花等） | 晶花解锁素材 | Easy–Extra | wiki **未**写与 Kioku/Magic 相同的 skip 句；主人钉死 **411102 不可 skip** |

mst 对照（JP wire 中 `get_quest_stage_mst_list` 样本）：

| questGroupMstId | 角色（从 name 模式） | 例 stage |
|-----------------|----------------------|----------|
| **101** | キオク强化素材 | 401101 Rank1（useStamina=10）… |
| **201+** | 魔力解放素材[属性] | … |
| **301** | （能力晶花总类等） | … |
| **401–405** | 能力晶花[属性] | **411101** Easy、**411102** Normal、411105 Extra（useStamina 15/20） |

wire 训练进度样本（测试号）：

| 号 | questGroupMstId | clearedQuestStageMstId | rankUpEffected |
|----|-----------------|------------------------|----------------|
| en_w1 | **403** | **411102** | 0 |
| jp_w1 | **403** | **411101** | 0 |

即：测试号强化进度**只有能力晶花组**，没有 101/魔力解放组的 cleared 记录 → basic 只能选到 41110x → skip 失败与主人「411102 不可 skip」及 wiki「Kioku/Magic 才强调 skip」一致。

---

## 3. 语义分类（与主人三类对齐）

见 [MODULE_SEMANTIC_CLASSIFICATION.md](./MODULE_SEMANTIC_CLASSIFICATION.md)：

| key | 类 | 说明 |
|-----|----|------|
| basic | **① 扫荡** | 只 skip 可扫训练本 |
| super_sweep | **③ 手** | 指定关真战斗 |

禁止：把 basic 改成 skip 失败就自动 super_sweep（除非主人点名改产品定义）。

---

## 4. Rust 实现要求（2026-08-08 修订）

1. 保持 **只 skip**；失败不得假成功。  
2. **选关优先级**：优先 `questGroupMstId` 属于 **キオク(101) / 魔力解放(201–299 一带)** 的已通关记录；**能力晶花组(401–405 等)** 降权或仅在没有可 skip 组时尝试并写清风险。  
3. 若进度**仅有**能力晶花类关：默认 **Skip** 并中文说明「当前仅有能力晶花进度关，游戏侧通常不可 skip；请先通关キオク/魔力解放强化本，或用快速刷图真战斗」。  
4. skip HTTP 5xx：文案写 **关卡可能不允许扫荡 / 未满足 skip 条件**，避免只甩加密失败。  
5. 次数：按关卡 `useStamina`；分批 ≤20。  
6. 注释与本文、MODULE_SEMANTIC 双向链接。

---

## 5. 本地研究目录

```text
docs/research/magia-exedra/
  README.md
  wiki-exedra/          # exedra.wiki 抓取
  wiki-madodra/         # madodra.wikiru 抓取
  wiki-game8/           # game8 / appmedia
  official-news.html
```

抓取时间：约 2026-08-08；内容可能过时，以游戏内与官方为准。

---

## 6. 关卡 ID ↔ 名称怎么查（产品入口）

原版：`db.mst(MstApiGetQuestStageMstListRequest())` → `questStageMstId` + `name`。

```bat
REM 只读本地缓存（数据文件夹 cache/mst/{channel}/quest_stage.json）
RustMadoka.exe mst quest-stages --from-cache --channel jp --id 411102
RustMadoka.exe mst quest-stages --from-cache --channel jp --filter キオク --limit 20
RustMadoka.exe mst quest-lookup --id 401101 --from-cache --channel jp

REM 登录账号对服刷新缓存
RustMadoka.exe mst quest-stages -g 123456 -a jp_w1 --group-password *** --refresh --filter 魔力
```

HTTP：`GET /api/mst/quest-stages?channel=jp&filter=…` · `GET /api/accounts/:alias/mst/quest-stages?id=…`  
网页：设置 → 展开「快速刷图」→ 关卡 ID 旁「查名称 / 按名搜索」。  
模块日志：`basic` / `super_sweep` 打印 `关卡=411102（能力晶花…）`。

wire 样本：`411102` = 能力晶花クエスト[木]RankNormal；`401101` = キオク強化素材獲得クエストRank1。

---

## 7. 修订

| 墙钟 | 内容 |
|------|------|
| 2026-08-08 03:00 | 首版：原版对照 + wiki 强化本 skip 规则 + 测试号 403/411102 解释 + Rust 实现要求 |
| 2026-08-08 | 关卡 ID↔名称 CLI/HTTP/网页/日志；mst 缓存路径 |
