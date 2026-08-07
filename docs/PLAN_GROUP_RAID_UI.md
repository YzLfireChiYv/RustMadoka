# 任务书：组队 Raid UI（借用「游戏账号卡片」· 多配置）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-08 01:55（本机） |
| **状态** | **CODE · 多配置卡片已落地 · 待主人点测 FIXED · 已写入全面交接** |
| **规格真源** | [tech/GROUP_RAID_AND_DEVICE_IDENTITY.md](./tech/GROUP_RAID_AND_DEVICE_IDENTITY.md) **§8**（以本节任务书入口口径为准） |
| **规范** | NORMS P5 · P16 · P23 · P7（备忘≠开工）· P30c 台服不管 |
| **失真声明** | **AI 维护，有可能出错和失真。** |

---

## 0. 主人纠正（2026-08-08 · 完整条件 · 白话）

主人原意（摘要，完整句）：

1. **添加游戏账号那里，只负责添加卡片**；具体设置在**卡片那里**做。  
2. 不同人需要不同数量的组队功能，**不能单一入口**，所以**借用「游戏角色卡片」的设计**（多套配置 = 多张卡，而不是服下拉里塞一个假平级类型，也不是全站一个全局开始钮写死）。  
3. **接入任务规划**；**别抢跑**；**优先准备交接**。

落地口径：

1. **「添加游戏账号」只负责添加游戏账号卡片**（别名 + 日服/国际服 + 引继 + 密码）。**不要**在「服」下拉里塞「组队 Raid」与日服/国际服平级。  
2. 组队功能 **不能单一全局入口**：不同人需要**不同数量**的组队配置。  
3. **借用「游戏角色/游戏账号卡片」的设计**：组队配置以**卡片形态**存在（或在卡片上设置/打开面板），需要几套组队就有几张「组队相关卡片/配置卡」。  
4. 实现时：先定卡片数据模型与列表 UI（**GR-MODEL-MULTI → GR-CARD-UI**），再接已有 `group_raid` 编排（**GR-WIRE**；单号/删卡降级/API/CLI **已有 CODE 可复用**）。  
5. **AI 勿抢跑**：未按本任务书点名开工前，不要再往 `static/index.html` 塞组队半成品面板。回退见 log `2026-08-08-group-raid-ui-revert-handoff`。

---

## 1. 与错误实现的区分

| 错误（已回退） | 正确 |
|----------------|------|
| 添加折叠里类型=日服/国际服/**组队 Raid** | 添加折叠**仅**日服/国际服加号 |
| 全组一个组队折叠面板 | **多张组队配置**，卡片化 |
| 单一「开始组队」全局 | 每张组队卡/卡片入口各自设置与开始 |

---

## 2. 建议数据与 UI（设计草案 · 实现时再细化）

| 项 | 建议 |
|----|------|
| **游戏账号卡片** | 现有 `GameAccount` 列表不变 |
| **组队配置实体** | 用户组内 `Vec`：id、显示名、参与别名列表、room_open、party、leave_after_support（可从现 `group_raid` 单例升级为列表） |
| **主页** | 账号卡片区旁或同网格：**组队配置卡片**（看起来像卡，不是添加账号里的下拉项） |
| **操作** | 新建组队卡 / 打开设置面板 / 开始 / 删除该配置 |
| **逻辑** | 仍：单号打满日次数；名单与现账号取交删卡降级（§8.2–8.3） |
| **CLI** | 按配置 id 或 aliases 启动（P23） |

---

## 3. 任务拆分（下一实现会话）

| ID | 内容 | 状态 |
|----|------|------|
| **GR-UI-REVERT** | 回退添加账号下拉里的 group_raid 选项 | **CODE** |
| **GR-MODEL-MULTI** | 组队配置从单例改为**多份**列表落盘 | **CODE** |
| **GR-CARD-UI** | 主页卡片区展示组队配置卡 + 设置面板 | **CODE** |
| **GR-WIRE** | 接现有 POST group-raid / 删卡降级 / 单号 · config_id | **CODE** |
| **GR-CLI-MULTI** | CLI `--config-id` 或 `--aliases` | **CODE** |
| **GR-POINT** | 主人点测 FIXED | **WAIT** |

---

## 4. 源码与 API（落地后 · Inbound）

| 路径 | 职责 |
|------|------|
| `crates/rustmadoka-core/src/account.rs` | `GroupRaidConfigEntry` · `GroupRaidPanelConfig.entries` · 旧单例迁移 |
| `crates/rustmadoka-core/src/modules/group_raid.rs` | 编排 · 伤害拆分 · 对服 multi_raid |
| `crates/rustmadoka-app/src/run_ops.rs` | resolve / exec / load / upsert / delete / by config_id |
| `crates/rustmadoka-app/src/http_server.rs` | `GET/PUT /api/group-raid/config` · `POST/DELETE /api/group-raid/entry` · `POST /api/group-raid` |
| `crates/rustmadoka-app/src/lib.rs` | CLI `run group-raid --config-id` |
| `crates/rustmadoka-app/static/index.html` | 主页 `#grCards` 组队配置卡 UI |
| 规格 | [tech/GROUP_RAID_AND_DEVICE_IDENTITY.md](./tech/GROUP_RAID_AND_DEVICE_IDENTITY.md) §8 |
| 对服审核 log | [logs/2026-08-08-group-raid-cards-wire-audit.md](./logs/2026-08-08-group-raid-cards-wire-audit.md) |

---

## 5. 修订

| 墙钟 | 内容 |
|------|------|
| 2026-08-08 | 初版任务书（错误平级入口） |
| **2026-08-08 00:40** | **主人纠正：添加只加号；组队借用卡片多配置；禁止抢跑** |
| **2026-08-08 00:45** | 接入 TASK_REMAINING_FULL GR-*；交接强化；仍不实现卡片 UI |
| **2026-08-08 00:52** | 主人授权全部已知任务：多配置卡片 + wire 审核落地 |
