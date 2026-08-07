# 技术规格：数据文件夹 `RustMadoka_data` 布局与向后兼容

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-08（主人布局设想对照 + 目标 schema 2） |
| **失真声明** | **AI 维护，有可能出错和失真。** 以 `crates/rustmadoka-app/src/data_layout.rs` 与当前 Store/占用源码为准。 |
| **规范** | NORMS **P32** · **P1b** · **P8/P8b/P9** · **P16** · **P30d** |
| **Inbound** | 主人 2026-08-08：正式用真号；夹不轻易变；**账号卡单独存 / 组下设置与日志 / 纯设置与含别名设置分开** |
| **Outbound 源码** | `data_layout.rs` · `settings_files.rs` · `paths.rs` · `account.rs` · `session_pool.rs` · `occupancy.rs` · … |

---

## 0. 产品定位（正式自用起）

从 **2026-08-08** 起，主人声明：**正式使用本工具登录自己的游戏账号**。  
因此 **`RustMadoka_data` 视为长期资产目录**（账号、配置、日志、指纹槽、token 缓存），**不是**可随意整夹丢弃的测试沙箱。

| 原则 | 完整条件 |
|------|----------|
| **稳定根名** | 旁路 exe 的文件夹名固定为 **`RustMadoka_data`**（`paths::DATA_DIR_NAME`）。可用 CLI `--data-dir` 覆盖路径，但布局语义相同。 |
| **向后兼容** | 新程序版本必须能打开旧 `layout_schema` 与旧 `users/*.json`（组信封 schema）；只许**增加**字段/子目录，禁止静默改名导致读不到号。 |
| **向前拒绝** | 若磁盘 `layout_schema` **高于**本程序认识的版本 → **拒绝打开并提示升级程序**（避免半写坏夹）。 |
| **不删用户数据** | `ensure_data_layout` **只创建缺失目录与写 layout 清单**，永不自动清空整夹。 |
| **删用户组** | `Store::delete_group`：删 `users/{组}.json` + 该组 `groups/{组}/` 整树 + 该组 `task_logs/{组seg}/`。**不**因删一组就删 `accounts/{card_id}`（同引继可能还在别的组）。 |
| **同角色同登录** | 游戏会话池与 device_id 键 = **渠道+引继**，与该卡在「加密组还是明文组」无关。 |
| **家用安全 P9c** | 加密组：小孩打不开组、拿不到组内密码（信封 + 不写明文 identity 旁路）。明文组：可写 identity 方便对照。不为多余安全牺牲便利。 |
| **AI 协作** | 正式自用后，AI **不得**在未获主人当轮明确授权时清空、移走整夹（**P1b**）。结束进程仍可（P1）。 |

与旧口径对照：早期「测试数据可弃 / AI 可重建 data」（原 P1 全文）在**测试沙箱**仍可用；**含真号的正式夹**走 P1b。

---

## 0.1 主人布局设想 vs 当前实现（2026-08-08 对照）

| 主人设想 | 当前实现（layout 1 + Store） | 差距 |
|----------|------------------------------|------|
| **游戏账号卡片单独存**（同设备 id） | 引继+密码+整份 config **塞在** `users/{组}.json` 的 accounts 数组里；**device_id 已按引继**落在 `cache/device_by_account/` 与 token 文件（**不**绑用户组） | 设备 id 已对；**卡片身份未独立成文件**，同号进两组会**复制两份引继/密码** |
| **设置和日志按用户组存**；组下还有**卡片文件夹与数据** | 设置在组文件里嵌在账号对象上；日志在 `task_logs/{组哈希}/{别名哈希}/` | 组/别名有分，但**不是**「组目录 → 卡片子目录」的清晰树；设置与引继同文件 |
| **纯设置** vs **含别名信息的设置** 分开，方便**复制 JSON 同步配置** | 导出设置 API 可剪贴板拷 config（无引继）；磁盘上**没有**独立的 `settings.json` / `shared.json` 给人直接拷文件 | 能同步，但依赖网页/API；**不适合**「只拷一个 json 文件」的工作流 |

**结论：** 设备身份方向与主人一致；**落盘形态尚未按主人设想拆开**。正式自用前应把目标定为 **layout_schema 2**（见 §1.1），旧 `users/*.json` 继续可读（P32）。

---

## 1. 根目录文件一览（layout_schema = 1 · **现行读路径**）

```text
RustMadoka_data/
  layout.json                 # 布局版本清单（本规格真源）
  app.json                    # 端口、信息源等应用设置
  app_runtime.json            # 运行时戳（信息性）
  owner.lock                  # 同机同路径 Owner 独占（OS 文件锁；正常退出/崩溃释放）
  occupancy_heartbeat.json    # 跨路径/云盘占用二次保险（≠ owner.lock）
  fp_slots.json               # 指纹槽（内嵌/拉取/自定义）
  users/                      # 【现行】用户组 JSON（每组一文件：账号+设置+组队混装）
  groups/                     # 【schema2 起】用户组目录树（见 §1.1）
  accounts/                   # 【schema2 起】游戏账号卡片身份（见 §1.1）
  cache/                      # 可重建缓存（token/mst/队伍/device_by_account）
  task_logs/                  # 任务日志按组·别名（现行路径；可与 groups/.../logs 镜像）
  notifications/              # 设置通知历史、系统 toast 配置等
  exports/                    # 会话导出等
  wire/                       # 仅开发版录制；可大；非正式功能依赖
```

| 路径 | 角色 | 丢了能否恢复 | 是否可进 git |
|------|------|--------------|--------------|
| `layout.json` | 布局 schema | 可再 ensure 生成 | 否（本机） |
| `app.json` | 端口、指纹源列表 | 可回默认 | 否 |
| `users/*.json` | **用户组 + 游戏账号 + 模块 config** | **难**（真号资产） | **禁止** |
| `cache/token/` | Gree 私钥材料等（P9 明文允许） | 可重登再生 | **禁止** |
| `cache/mst/` | 关卡 ID↔名称等产品缓存 | 可再拉 mst | 否 |
| `cache/parties/` | 队伍列表缓存 | 可再刷新 | 否 |
| `cache/device_by_account/` | 按卡 device_id | 丢了可能换设备感 | 否 |
| `fp_slots.json` | 指纹槽 | 可回内嵌 + 再拉 | 否 |
| `task_logs/` | 任务历史 | 可清空 | 否 |
| `occupancy_heartbeat.json` | 跨路径占用 | 可删（仅影响 30 分钟窗口） | 否 |
| `owner.lock` | 运行锁 | 进程死后由 OS 释放 | 否 |
| `wire/` | 通讯录制 | 可整夹删 | **禁止**（或含敏感） |

---

## 1.1 目标布局 layout_schema = 2（主人设想 · 产品真源）

> **实现状态（2026-08-08 审核后）：**  
> - **写**：`Store::save_group` 权威写 `users/*.json`，并 **`mirror_layout2`** 写 `accounts/*/identity.json`（仅明文组）、`groups/*/meta.json`、`cards/*/settings.json`+`link.json`、`settings/shared.json`。  
> - **读**：仍 **`load_group` ← users/**（旁路不作为登录权威，防双源分叉）。  
> - **加密组**：不写明文 identity（A2）。  
> - **单测**：`layout2_mirror_plain_group_writes_identity_and_settings`。  
> - 完整「只读 groups+accounts、可删 users」迁移工具：**未做**（需点测与迁移脚本）。

```text
RustMadoka_data/
  layout.json
  app.json · owner.lock · occupancy_heartbeat.json · fp_slots.json

  accounts/                              # 游戏账号卡片（身份真源 · 绑 device_id）
    {card_id}/                           # 稳定 id（非别名；别名只在组内）
      identity.json                      # channel + 引继 + 密码（敏感；P9 明文允许）
      # device_id 仍以 cache/device_by_account/{引继安全名}.json 为准（按卡复用）

  groups/                                # 用户组
    {group_name}/
      meta.json                          # 组密码开关、成员表 alias→card_id、组队多配置
      settings/
        shared.json                      # 【纯设置】无别名、无引继；可整文件复制到另一组同步
      cards/
        {alias}/
          link.json                      # { "card_id": "..." } 指向 accounts/
          settings.json                  # 【本卡片设置】模块开关/队伍名等；可单独复制到另一别名
      logs/                              # 可选：与 task_logs 镜像或迁入
        {alias}/

  cache/
    token/ · device_by_account/ · mst/ · parties/ · version.json
  task_logs/                             # 兼容路径（组哈希/别名哈希）在迁完前保留
  notifications/ · exports/ · wire/
```

| 文件 | 可否直接复制同步 | 含什么 |
|------|------------------|--------|
| `groups/…/settings/shared.json` | **是**（推荐组间默认同步） | 模块默认、商店优先等**不含别名**的键 |
| `groups/…/cards/{alias}/settings.json` | **是**（卡间同步） | 该别名的模块 config（仍**无**引继/密码） |
| `accounts/…/identity.json` | **否**（敏感；不要发群） | 引继+密码+渠道 |
| `users/{组}.json` | 旧格式整包；**不推荐**当同步真源 | 混装 |

**device_id 规则（不变）：** 键 = 引继码（游戏账号卡片），不是用户组别名。同一引继无论挂在几个组，**同一 device_id**。

---

## 2. `layout.json` 字段

| 字段 | 含义 |
|------|------|
| `layout_schema` | **整数**；程序常量 `LAYOUT_SCHEMA`（当前 **2**：已建 groups/accounts 目录；Store 权威仍兼容 schema1 的 users/） |
| `product` | `"RustMadoka"` |
| `ensured_at` | 最近一次 ensure 的 UTC RFC3339 |
| `app_version` | 写入时的程序版本（信息性） |
| `note` | 人类备注（升级说明等） |

启动路径：`cli_main` 与 `run_owner_serve` 均调用 `data_layout::ensure_data_layout`。

---

## 3. 子树约定

### 3.1 `users/{组名}.json`

- 信封 **schema: 2**（`account::StoredGroup`）：明文 `accounts` 或 vault 密文 + `public_aliases` + 可选 `group_raid` 多配置卡片。  
- 新字段必须 `#[serde(default)]`，旧文件缺字段仍能打开。  
- **禁止**把引继/密码写入本文件以外的「备份到公开仓」路径（P8）。

### 3.2 `cache/`

| 子路径 | 内容 |
|--------|------|
| `token/` | Gree 等登录材料（文件名与引继相关） |
| `mst/{channel}/quest_stage.json` | 关卡 ID↔名称产品缓存 |
| `parties/{game_id_hash}.json` | 队伍列表 |
| `device_by_account/` | 按游戏账号卡片的 device 档案 |
| `version.json` / `automadoka*.json` | 当前选用指纹旁路缓存 |

### 3.3 `task_logs/`

按用户组与别名分目录存任务会话（索引 + 全文）；清理策略见 UI 设置 / CLI `task-log clear-older`。

### 3.4 `notifications/`

设置变更通知、系统 toast 配置等；默认 toast **关**。

### 3.5 `wire/`（开发版）

仅 `wire_record` 构建写入；**正式日常使用不必依赖**；体积大，备份整夹时注意。

---

## 4. 占用：两层机制（与「心跳」分工）

详见 [INSTANCE_AND_CLI.md](./INSTANCE_AND_CLI.md) · [PLAN_AUDIT_COMMS_OCCUPANCY_PROGRESS.md](../PLAN_AUDIT_COMMS_OCCUPANCY_PROGRESS.md) §4 · 源码 `occupancy.rs`。

| 层 | 文件 | 场景 | 行为 |
|----|------|------|------|
| **同机同路径硬互斥** | `owner.lock` | 同一台电脑、同一数据文件夹路径，两个 exe | OS **独占文件锁**；第二进程直接起不来。**不靠心跳。** |
| **跨路径 / 云盘二次保险** | `occupancy_heartbeat.json` | 同一份数据经 OneDrive 等同步到**另一路径/另一台机**，或本机复制夹到新路径 | 约 **1 分钟**刷新时间 + 路径 + exe 路径；**不同路径**且约 **30 分钟**内仍 active → 默认拒绝；程序运行面板终端输入完整短语 **`我已知晓`** 可强制。同路径（断电重启）→ **立即允许**。 |

**心跳不是什么：**

- **不是**游戏服务器心跳 / keep-alive。  
- **不是**同机双开的主防护（主防护是 `owner.lock`）。  
- **不是**浏览器网页前端上的 UI 动画。  
- 主人原意：心跳复杂且有漏洞，**只作跨设备同夹（云盘）的无奈二次保险**。

交互位置：强制确认在 **程序运行面板终端**（黑色窗口），不在浏览器网页前端（G9）。

---

## 5. 向后兼容规则（写代码时必须遵守）

1. **增字段**：JSON 一律 `default`；缺省有合理默认。  
2. **增目录**：只 `create_dir_all`，不搬迁旧路径。  
3. **改文件名**：须同时读旧路径至少一个版本周期，或提供一次性迁移并写 log；默认**禁止**。  
4. **升 `layout_schema`**：本程序 `LAYOUT_SCHEMA` +1；ensure 时把旧夹标到新版本；读逻辑仍接受旧文件内容。  
5. **降级程序**：若夹的 schema 更高 → 拒绝并提示升级（已实现）。  
6. **禁止**「为省事重建空 data」作为修 bug 的默认手段（正式自用夹）。  
7. 与旧 **`automadoka_data` / 13220**：**不追求**兼容（产品已钉死 RustMadoka 口径）。

---

## 6. 主人重建新夹时的检查清单（白话）

1. 双击 `RustMadoka.exe` → 旁路出现 `RustMadoka_data`，内含 `layout.json` 与空 `users/` 等。  
2. 在浏览器网页前端建用户组、加自己的游戏账号（引继+密码仅本机）。  
3. 需要指纹时用设置页刷新（rules 仓）或依赖内嵌。  
4. **不要**把整个 `RustMadoka_data` 推到 GitHub。  
5. 若数据夹放在云盘同步目录：注意跨设备 30 分钟占用窗口与「我已知晓」。

---

## 7. 登录复用：清日常后再点单个功能还要不要登录？

真源：`session_pool.rs`（空闲 **TTL = 75 分钟**）· `CLIENT_SESSION_SIMULATION_FEASIBILITY.md`。

| 场景 | 是否还要再登录游戏（LoginApi） |
|------|--------------------------------|
| **同一 Owner 进程未关**，几分钟内再跑同一游戏号的单模块 / 工具 | **不需要**再完整登录：复用进程内 `GameClient`（池键 = channel+引继） |
| 同一 Owner 内刚跑完一趟 `run daily`，马上再 `run module`（IPC 进同一 Owner） | **不需要**额外登录（池里已有 Full 会话） |
| 关掉 exe / 进程退出后再开 | **需要**重新登录（内存池清空；Gree token 文件仍在，会快一些，但仍走登录串） |
| 空闲 **超过约 75 分钟** 未用该号 | 池丢弃 → **需要**再登录 |
| 服务端 **401** 会话失效 | 丢会话 → **需要**再登录 |
| CLI **`run … --wire`** 独占进程（抢锁执行完就退出） | **每一趟 CLI 进程各登录一次**（不经过长驻 Owner 池） |
| 无 Owner 时 `run` 拉起 Owner 并保持 | 第一次登录在本进程；之后 IPC 命令复用 |

**白话：** 程序一直开着、几分钟内同一账号连点功能 → **像真客户端一次登录连玩**；关了程序或隔很久 → 再登。

**一趟清日常内部：** 模块之间**不会**每个模块登一次；wire 已钉死单次 daily 内 `/api/login` = 1。

---

## 8. 组队 Raid 再审核（摘要 · 2026-08-08）

完整规格：`GROUP_RAID_AND_DEVICE_IDENTITY.md` · 实现 `group_raid.rs` · `run_ops::exec_group_raid*`。

| 项 | 状态（诚实） |
|----|----------------|
| 产品形态 | 用户组级任务；主页**多配置卡片** + CLI `--config-id` / `--aliases` |
| 流程 | 召唤 → 互援 → 舔盒 → 多轮；援助后退出**默认关** |
| 伤害 | 按人数拆分使 Σ≥H；体力不足跳过该号后按 k 重算 |
| device_id | **按游戏账号卡片（引继）**；非全夹共用 |
| 互斥 | TaskGate 多号占用；仅发起用户组可停 |
| rescueType 与 工会/好友/全体 映射 | 代码有表；**【未证实】** 对服语义 |
| 多号真人端到端 | **无成功 FIXED wire**；CODE ≠ 点测 FIXED（P5） |
| 入房 id 精准 | 规格优先房间 id；风险注释见 HANDOFF 已知对服风险 |
| 单号 | 允许 1 人打满日次数（开放「仅自己」） |

**结论：** 架构与设备身份对齐主人要求；**正式真号组队前**建议先用测试号双开 wire 跑通一轮，勿默认 FIXED。

---

## 9. 修订

| 墙钟 | 内容 |
|------|------|
| 2026-08-08 | 首版：正式自用、layout_schema=1、目录表、心跳与 owner.lock 分工、兼容规则 |
| 2026-08-08 | 主人布局设想对照；目标 schema2；登录复用 §7；组队再审 §8 |
