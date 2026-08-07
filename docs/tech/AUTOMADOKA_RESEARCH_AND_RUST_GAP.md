# 原版 automadoka（autopcr）原理研究 · 与 Rust 对照缺口

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-07 05:06 |
| **任务** | R01 · [PLAN_RESEARCH_AUTOMADOKA.md](../PLAN_RESEARCH_AUTOMADOKA.md) |
| **性质** | 静态完整分析（归档树 + 当前 crates）；**非**真机逐模块 FIXED |
| **Outbound 原版** | `archive/pre-rust-2026-08/autopcr/` · `archive/pre-rust-2026-08/raid/` |
| **Outbound Rust** | `crates/rustmadoka-core/` · `crates/rustmadoka-app/` · `crates/rustmadoka-mobile/` |
| **Inbound** | [HANDOFF.md](../HANDOFF.md) · [TASKBOARD.md](../TASKBOARD.md) · [TASK_INVENTORY.md](../TASK_INVENTORY.md) · [MODULES.md](../MODULES.md) · [tech/README.md](./README.md) |
| **MAY CONTAIN ERRORS** | Yes — 以源码为准；【实测预留】须真机 |

**本文件是 R01 主产出之一。** 读完可决定系统性改进优先级；禁止未读原理就堆 R4 全量。

### 研究意图（2026-08-07 主人对齐 · 必读）

| 点 | 说明 |
|----|------|
| **不是** | 只做「模块有/无」对照表、只解释 API 而不谈产品 |
| **是** | 原版 **底层（协议 + 登录态 + 业务）能力面 ≫ 前端暴露面**；Rust 重构应把已有数据 **产品化**（预存、列表选择、防呆），而不只是复刻手填表单 |
| **样例** | 队伍：init 已有完整 `partyDataList`，用户仍因忘名「找不到队伍」→ 规划 **C10** 列表选择/自行输入二选一（圆点 UI）· [PLAN_PARTY_SELECT_UX.md](../PLAN_PARTY_SELECT_UX.md) · **暂不写代码** |
| **对照与通讯** | 原版 GitHub 完整性与「包/发/收」→ [UPSTREAM_SOURCE_AND_WIRE.md](./UPSTREAM_SOURCE_AND_WIRE.md) |

---

## 0. 一页结论

| 结论 | 说明 |
|------|------|
| **原版是什么** | 本机 Quart Web + 多用户「工具用户 / QQ 文件夹」+ 游戏角色卡片 + 模块化日常/工具 + Gree/Sonet SDK + msgpack+AES 游戏 API |
| **Rust 已复刻** | 协议主路径（Gree 日/国 + AES + sm）、用户组/加密组、日常 26 代码、洗词条流式、Owner/IPC、任务日志、指纹云、SPA、Android WebView 壳 |
| **有意差异（产品钉死）** | 日常默认**全关**（原版多数默认开）；商店优先级默认 0；不云存；单 Owner；默认指纹 rules 仓；端口 14103；数据 `RustMadoka_data/` |
| **最大缺口（能力移植）** | 定时 cron、台服 Sonet、工具 4 项、raidrunner、会话池/托管（R4）等 |
| **最大缺口（产品化 · 主人强调）** | **底层已有、前端未暴露**：如队伍列表仅服务解析、未做选择器；同类横切数据（体力相关、mst 下拉、商店类别…）仍多手填/隐式 |
| **最大缺口（工程质量）** | 日常 26 真机未逐条 FIXED；模块细分支可能偏差 |
| **原版已知坑** | 队伍 int()/名称混用（L11）；洗词条未持有 style；默认开易误耗；单跑改 enabled；APKPure 刷指纹；token 明文；PANIC 中断整日；Sonet migrate 未实现 |
| **下步改进门** | 对齐「可产品化」清单 + 真机点测 + 点名开工；非默认堆 R4 |

---

## 1. 原版架构全景

### 1.1 分层（与 OVERVIEW 对齐 · 本处钉死职责）

```text
┌──────────────────────────────────────────────────────────────┐
│ L4  人机：ClientApp SPA（打包静态） / 浏览器                    │
└───────────────────────────────┬──────────────────────────────┘
                                │ HTTP /daily/*  + 登录 cookie
┌───────────────────────────────▼──────────────────────────────┐
│ L3  HttpServer (Quart) + AccountManager + ModuleManager        │
│     工具用户目录 · 游戏角色 JSON · 模块 config · result 落盘    │
│     crons 后台：每分钟扫描 → do_daily                          │
└───────────────────────────────┬──────────────────────────────┘
                                │ Module.do_from(pcrclient)
┌───────────────────────────────▼──────────────────────────────┐
│ L2  pcrclient = Container 洋葱：                                │
│     mutex → sessionmgr → datamgr → errorhandler → apiclient    │
└───────┬───────────────────┬───────────────────┬──────────────┘
        ▼                   ▼                   ▼
   SDK 登录(uuid)     游戏 API root        cache/version.json
   Gree / Sonet       msgpack+AES+sign     sm 指纹
```

| 层 | 原版路径 | 职责摘要 |
|----|----------|----------|
| 常量 | `constants.py` | 端口 13200、限频、渠道名、路径、默认 Unity 头 |
| 协议 | `core/apiclient.py` | 加密封包、actionTime、428 版本、401、串行锁 |
| 会话 | `core/sessionmgr.py` | 未登录 → SDK+LoginApi+mst+初始化串 |
| 状态 | `core/datamgr.py` | 响应 `update` 写内存；`generate_battle_log` |
| 客户端 | `core/pcrclient.py` | 组装组件；体力/cron 运行时键；教程/辅助 API |
| 加密 | `core/crypto.py` | PKLB AES 密钥派生、PackHelper、ApiCrypto.sign |
| 指纹 | `core/version.py` | version/sign/libcount → sm；默认 APKPure 更新 |
| SDK | `sdk/greeclient.py` · `sdkclients.py` · `sonetclient.py` | 渠道登录与签名 |
| 模型 | `model/requests.py` 等 | pydantic 请求/响应；`url` 属性 = API 路径 |
| DB | `db/database.py` | mst revision 缓存；登录预拉 style/character/… |
| 模块 | `module/*` · `modules/*` | 注册表、执行、配置装饰器、结果 |
| Web | `http_server/httpserver.py` | 鉴权、账号 CRUD、do_daily/do_single、结果查询 |
| 定时 | `module/crons.py` · `modules/cron.py` | 6+6 槽点钟跑日常 |
| 团战辅 | `raid/raidworker.py` · `raidrunner.py` | 小号秒伤；农场队列（可 stub） |
| 工具 | `util/*` | 限频、探针、绘图、ILP 等 |

### 1.2 磁盘布局（原版）

| 常量 | 默认 | 内容 |
|------|------|------|
| `CACHE_DIR` | `./cache/` | version.json、token、http_server 用户、modules cache |
| `CONFIG_PATH` | `cache/http_server/` | 工具用户目录（qid 文件夹） |
| `RESULT_DIR` | `./result/` | 日常/单模块结果 JSON（保留约 4 份） |
| `LOG_PATH` | `./log/` | 运行日志 |
| token | `cache/token/{引继}_{pwdMD5}.json` | Gree 私钥+uuid 或 Sonet device/uuid/token |

### 1.3 运行时数据模型

| 概念 | 类型 | 字段要点 |
|------|------|----------|
| 工具用户 | `UserData` | password、default_account、clan、admin、disabled |
| 游戏角色 | `AccountData` | username(引继)、password、channel、game_name、level、config 扁平字典、daily_result、single_result、batch_accounts |
| 任务结果 | `TaskResult` | order[] + result{key→ModuleResult} |
| 模块结果 | `ModuleResult` | name、config 摘要、log、status 枚举 |

**config 扁平字典约定：** 模块开关键 = 模块类名/key（如 `loginbonus: true`）；参数键独立（如 `stamina_buy_count`、`event_shop_priority_钻石`）。

---

## 2. 协议与登录原理（静态钉死）

### 2.1 游戏请求管道

1. `RequestBase.prepare()` 注入 `lastHomeAccessTime`、`sm`（来自 version_info）。  
2. 组装 envelope：`payload, actionTime, sessionId, userId, actionToken, uuid, ctag`（**键序影响 hash 时须与 Python dict 序一致**）。  
3. `crypto.PackHelper.pack`：msgpack → AES-CBC(PKLB key) → IV||密文。  
4. `sdk.post_sign(crypted)` → 头 `x-post-signature`。  
5. POST `apiroot + request.url`；200 解密解包；428 → `update_version` + 重登；401 会话失效。

**黄金向量与教训：** [LESSONS_RUST_PORT.md](./LESSONS_RUST_PORT.md) L1/L2/L8 · [PROTOCOL_STACK.md](./PROTOCOL_STACK.md)。

### 2.2 Gree（日/国）身份机

| 阶段 | 行为 | 签名 |
|------|------|------|
| initialize | 注册设备 + 公钥 → uuid | **HMAC-SHA1(APP_SECRET)**，无 RSA 字段 |
| migrate | 引继码 + B_encode(密码) | 之后 RSA |
| authorize | 拿会话 | RSA-SHA1 Prehashed |
| 游戏 API | 加密 body 签名 | `ApiCrypto.sign` RSA |

缓存：`privateKey` + `uuid`（Rust 另加稳定 `device_id` 于 device_profile）。详见 [SDK_AND_LOGIN.md](./SDK_AND_LOGIN.md) §5。

### 2.3 Sonet（台服）

| 点 | 原版事实 |
|----|----------|
| API root | `https://app-mme.so-net.tw` |
| SDK | `https://mme-sdk.so-net.tw` · gameid 2601 |
| 签名数据 | 排序 key=value& + 后缀 `sonet` → MD5 |
| 游戏签名 | JWT 作 post_sign；LoginApi 写 `jwttoken` |
| 加密 key | `SONET_HASH_KEY`（非 PKLB） |
| migrate_from | **`NotImplementedError`**（原版未完成引继迁入） |
| 注册 | register → handoverid 当引继 |

**Rust：** 指纹可收录 tw；`Channel::Tw.login_implemented() == false`。

### 2.4 登录后初始化串（sessionmgr）

与 SDK_AND_LOGIN §3 一致；Rust `GameClient::full_login` 对齐同一串。  
**额外产品路径：** `login_for_info` 轻量登录（跳过大量 mst/初始化）— 原版无对等「仅 info」优化。

### 2.5 指纹 sm

```text
sm = "d" + sign + "o" + libcount + "1E88A0177575728C9A399A9BD1F43A11D4100065n"
```

原版 `update_version` 默认流式下 APKPure XAPK（验证码/体积不适合用户主路径）。  
产品钉死：主人 PC 解析 → **rules 仓 raw** + 安装包 embed（P15 · L4 · C1）。

---

## 3. 模块系统原理

### 3.1 页签注册（`modules/__init__.py`）

| ModuleList | key | 内容 |
|------------|-----|------|
| daily_modules | daily | **26** 项有序（含挂在日常的 super_sweep） |
| tool_modules | tool | super_wash · raid_support · secret · auto_register · clear_dungeon_event |
| cron_modules | cron | cron1–6（另有注释掉的 support-only 槽） |
| planning / unit / clan / danger | — | **空占位** |

### 3.2 Module 生命周期

```text
装饰器(@name/@default/@config types)
  → Module.__init__(parent) 绑定 key=类名
  → do_from(client):
       do_check → (可选登录) do_task → ModuleResult
  → 状态: SUCCESS | SKIP | WARNING | ABORT | ERROR | PANIC
```

| 机制 | 行为 |
|------|------|
| `@tag_stamina_consume/get` | 运行时键可短路「不执行」 |
| 模块 cache | `cache/modules/{key}/{account_md5}.json` |
| `do_daily` | 按 daily_modules 序；**PANIC 中断后续** |
| `do_from_key` | 强制 `config[key]=True` 后单跑（**会改语义上的「开启」写入当次 config**） |
| 会战禁 | `db.is_clan_battle_time()` 当前 **恒 False**；禁名单文件仍存在 |

### 3.3 日常 26 · 原版默认 vs 产品默认

| # | key | 中文 | 原版 @default | Rust 产品默认 | 源文件 |
|---|-----|------|---------------|---------------|--------|
| 1 | loginbonus | 领取登陆奖励 | 开 | **关** | common.py |
| 2 | stamina_buy | 购买体力 | **关** | 关 | stamina.py |
| 3 | super_sweep | 快速刷图 | **关** | 关 | tool.py |
| 4 | raid_reward | 魔女舔盒 | 开 | 关 | raid.py |
| 5 | self_raid | 魔女召唤 | **关** | 关 | raid.py |
| 6 | support_raid | 魔女援助 | **关** | 关 | raid.py |
| 7 | like_raid | 魔女点赞 | 开 | 关 | raid.py |
| 8 | solo_raid | 扫荡总力战 | 开 | 关 | sweep.py |
| 9 | high_score | 扫荡打分 | 开 | 关 | sweep.py |
| 10 | arena | 自动PVP投降 | **关** | 关 | sweep.py |
| 11 | basic | 智能体力扫荡 | 开 | 关 | stamina.py |
| 12 | event | 扫荡活动 | 开 | 关 | sweep.py |
| 13 | archive | 扫荡档案活动 | 开 | 关 | sweep.py |
| 14 | event_shop | 清空活动兑换币 | 开 | 关；优先级 0 | shop.py |
| 15 | raid_shop | 清空 raid 兑换币 | 开 | 关；优先级 0 | shop.py |
| 16 | arena_shop | 清空 jjc 兑换币 | 开 | 关；优先级 0 | shop.py |
| 17 | tower | 扫荡露娜塔 | 开 | 关 | sweep.py |
| 18 | heart | 扫荡心之器 | 开 | 关 | sweep.py |
| 19 | gather | 收集宝箱 | 开 | 关 | sweep.py |
| 20 | freegacha | 免费扭蛋 | 开 | 关 | gacha.py |
| 21 | eventscenario | 阅读活动剧情 | 开 | 关 | collection.py |
| 22 | collection | 阅读光之间 | 开 | 关 | collection.py |
| 23 | battle_mission | 完成战斗任务 | 开 | 关 | sweep.py |
| 24 | mission | 领取任务 | 开 | 关 | sweep.py |
| 25 | present | 领取礼物 | 开 | 关 | sweep.py |
| 26 | info | 玩家信息 | 开 | 关 | common.py |

**为什么产品全关：** 原版「打开就跑多数模块」易误耗体力/石/兑换币（C5 · P17）。

### 3.4 工具 5 项

| key | 中文 | 默认 | 原理摘要 | 依赖 |
|-----|------|------|----------|------|
| super_wash | 快速洗词条 | 关 | 刷图拿 selection ability；列表来自 **mst 全表** 非持有列表 | mst + quest battle |
| raid_support | 魔女救世 | 关 | 第二账号 `raidworker` 秒当前房间 | raidworker + 小号凭证 |
| secret | 神秘新功能 | 关 | 邀请绑定 + 探索篇章清点（FIELD_CLEAR=612001） | Exploration* API |
| auto_register | 注册十个号 | 关 | `bootstrap.create_new` 固定密码 `12345678` | Gree register |
| clear_dungeon_event | 完成迷宫隐藏事件 | **开** | 已通关篇章上 eventType=21 隐藏事件 | Exploration + mst |

### 3.5 定时 cron

| 机制 | 说明 |
|------|------|
| 槽 | cron1–6：默认时刻 05/09/13/17/21/01，可配置 `time_cronN` |
| 调度 | `crons.py` 每 30s 睡 → 补齐分钟 → 扫所有 qid/账号 |
| 条件 | `enable && 时刻匹配 && is_cron_condition` |
| 执行 | `pre_cron_run` 设 `cron_run` 标志 → `do_daily(cronModule)` |
| 日志 | `cache/http_server/cron_log.txt` 行 JSON |
| Support 专用槽 | 源码中 cron1_–cron6_ **注释未注册** |

### 3.6 团战辅进程

| 文件 | 角色 |
|------|------|
| `raid/raidworker.py` | 小号登录、体力、进房/结算（**真实协议客户端**） |
| `raid/raidrunner.py` | 多开农场队列（探路/部分环境 stub） |

Rust **未移植** 独立 raid 进程模型；日常内 self_raid/support_raid 在 `daily.rs` 用主号会话实现。

---

## 4. Web / 账号管理原理

### 4.1 原版 HTTP 能力地图（`httpserver.py`）

| 区域 | 代表路由 | 能力 |
|------|----------|------|
| 鉴权 | login/register/logout | 工具用户 cookie；可开放注册 |
| 用户管理 | admin 建删用户、禁用 | SUPERUSER |
| 账号 | CRUD、TSV 导入、跨账号 config 同步 | AccountManager |
| 配置 | modules config GET/PUT | 扁平 config |
| 执行 | do_daily、do_single | 阻塞式任务；结果进 result/ |
| 结果 | daily/single result list/get | 最近约 4 份 |
| 其它 | clan_forbid、running_status（占位）、app version | 管理向 |
| 静态 | ClientApp SPA | 与 Rust 单文件 SPA 不同（React 分包） |

### 4.2 与 Rust 产品模型对照

| 维度 | 原版 | Rust |
|------|------|------|
| 用户边界 | 多 qid 文件夹 + 可选密码 | **用户组**（可加密 vault） |
| 默认绑定 | 127.0.0.1 倾向但可部署更宽 | **钉死 loopback**（P9/P10） |
| 实例 | 单进程 asyncio；账号级 Lock | **同 data 单 Owner** + IPC Client |
| 任务 | result 文件 + 内存列表 | `task_logs/` + progress 文件 + 暂停门 |
| 流式 | 无 NDJSON 日常流 | **daily/stream · wash/stream** |
| 端口 | 13200 | **14103** |
| 数据目录 | cache/ 混放 | **RustMadoka_data/** 旁路 exe |

---

## 5. Rust 重构映射表

### 5.1 目录 ↔ 职责

| 原版 | Rust | 状态 |
|------|------|------|
| `core/crypto.py` | `crypto.rs` | **对齐**（L2 已踩） |
| `sdk/greeclient.py` | `gree.rs` | **对齐**（L1 已踩） |
| `sdk/sonetclient.py` | — | **未实现** |
| `core/version.py` | `fingerprint.rs` + app fp_slots | **产品化**（rules/embed/槽） |
| `core/apiclient+sessionmgr+pcrclient` | `client.rs` | **主路径对齐**；无 Container 洋葱；有 light_login |
| `core/datamgr.py` | client 内 init_data + battle_log | **部分**（无通用 response.update） |
| `db/database.py` | `mst.rs` | **部分**（revision + 四类预拉 + 按需） |
| `module/accountmgr` | `account.rs` + app Store | **重设计**（用户组） |
| `module/modules/*` daily | `modules/daily.rs` | **代码齐 / 未 FIXED** |
| `modules/wash.py` | `modules/wash.rs` | **可跑+流式** |
| tool 其余 4 | — | **未做**（有意） |
| `modules/cron` + crons | — | **未做** |
| `http_server` | `rustmadoka-app` axum | **能力超集 UI**；路由不兼容原 API |
| ClientApp | `static/index.html` | **重写 SPA** |
| raidworker/runner | — | **未做** |
| — | `safety.rs` / `diag.rs` / owner_lock / task_gate | **新产品** |
| — | `rustmadoka-mobile` + Android 壳 | **新产品** |

### 5.2 行为差异（实现时必须知道）

| 点 | 原版 | Rust | 备注 |
|----|------|------|------|
| 日常默认 | 多数 `@default(True)` | **全 false** | 产品安全 |
| 单模块失败 | PANIC 才 break；ERROR 继续 | Skip/Error/Abort **均继续** | 故意更宽容 |
| 单跑与开关 | do_from_key 写 config[key]=True | **单跑不改 enabled**（P17/C6） | |
| 登录 | 全量 sessionmgr | full_login + **login_for_info** | |
| 指纹更新 | APKPure | rules/embed/本地 XAPK | |
| 设备 id | 常随机 | **device_profile 稳定** | |
| RSA 捷径渠道 | BSDKRSA/QSDKRSA | 未作独立渠道 | 一般用户不需要 |
| 会战禁跑 | 名单+时间（时间恒 false） | 未移植 | 低优先级 |
| 限频 | FreqLimiter 装饰器 | 依赖超时与串行锁 | 【可改进】 |

### 5.3 能力矩阵（诚实 · 对齐 HANDOFF）

| 能力 | 原版 | Rust | 证据档 |
|------|------|------|--------|
| Gree 日/国登录 | 有 | 有 | 主人测通 / L1 |
| Sonet 登录 | 半成品（migrate 缺） | 无 | 静态 |
| AES 游戏 API | 有 | 有 | L2 |
| 日常 26 | 有 | 代码有 | **未逐条 FIXED** |
| 洗词条 | 有 | 有+流式 | CODE |
| 工具 4 项 | 有 | 无 | 有意 |
| 定时 | 有 | 无 | |
| 团战多开 | raidrunner | 无 | 后置 R4 |
| 多用户托管 Web | 有 | 用户组本地 | 产品不同 |
| Android | 探路 Python 大包 | 薄 WebView+Rust | 主人可用壳 |
| 指纹云分发 | 弱/自备 | 强（rules） | P15 |
| 中文诊断 | 弱 | diag.rs | 产品 |

---

## 6. 原版已知问题与协议债

| ID | 问题 | 影响 | Rust 是否已规避 |
|----|------|------|-----------------|
| O1 | 队伍「名称 / 序号 / partyDataId」混用，先 int() | 刷图失败 | **部分**：`resolve_party`（L11） |
| O2 | 洗词条未持有 style 可崩 | 工具崩溃 | **倾向 Abort** |
| O3 | 默认模块多开 | 误耗 | **默认全关** |
| O4 | 单跑打开一键开关 | 勾选污染 | **独立** |
| O5 | APKPure 主路径 | 不可用/验证码 | **rules** |
| O6 | Sonet migrate 未实现 | 台服引继不可用 | 同缺 |
| O7 | token 明文落盘 | 本机读盘 | 同（P9 不默认再加密） |
| O8 | 日志/结果可含敏感 | 隐私 | 过程 log 规范禁真实引继 |
| O9 | model 巨型 pydantic | 维护成本 | Rust 用 Value 路径字符串 |
| O10 | 纯净树缺 raid_config/依赖 | 启动炸 | L5；Rust 无此依赖 |
| O11 | do_from_key 副作用 | UX 坑 | 已规避 |
| O12 | 会战时间检测空实现 | 禁跑无效 | 同未做真实日历 |

**协议债：** 初始化串是否随游戏版本增减 **【实测预留】**；428 后指纹刷新策略依赖云 JSON 更新及时性。

---

## 7. 文档债与注释债（本轮处理）

| 债 | 处理 |
|----|------|
| 无单一「原版原理 + Rust 缺口」入口 | **本文** |
| MODULES_RUNTIME 仍写「不要求对齐 26」 | 过时 → 修订指向 PHASE_R2 + 本文 |
| tech/README 无 R01 链 | 加入索引 |
| core 模块头注释未链研究文 | 本轮补双向链接 |
| 日常实现细分支 vs Python | **残留**：逐模块 diff 属 Z2/点测驱动，不在 R01 伪称完成 |

---

## 8. 系统性改进 backlog（可点名 · 非自动开工）

优先级倾向（**可推翻**；真机与主人体验优先）：

### 8.0 产品化（底层已有 → 前端暴露 · 优先理解方向）

| 序 | 项 | 类型 | 状态 / 门禁 |
|----|-----|------|-------------|
| **P0** | **队伍：列表选择 \| 自行输入（圆点二选一）**；预存/刷新 `partyDataList` | UX · 防呆 | **C10 PLAN** · [PLAN_PARTY_SELECT_UX](../PLAN_PARTY_SELECT_UX.md) · **暂不写代码** |
| P1 | 继续扫 init/mst/模块配置：可预填、可选、可缓存的字段清单 | 研究 | R01 延续 · 未开工 |

### 8.1 其它

| 序 | 项 | 类型 | 门禁 |
|----|-----|------|------|
| 1 | 日常 26 / 关键路径真机 FIXED | 验收 | 测试号 · C01 |
| 2 | 逐模块 Python↔Rust 行为 diff | 质量 | 点测失败驱动 |
| 3 | 运行面板「是否开启」 | UX | C07 PLAN |
| 4 | Android B1–B10 验收 | 双端 | D06 |
| 5 | 定时 cron 移植 | 功能 | 点名 |
| 6 | clear_dungeon_event / secret 按需 | 工具 | ALLOW_TOOL |
| 7 | Sonet 登录（含 migrate） | 渠道 | 点名 |
| 8 | raid_support（小号） | 工具 | 安全文案 |
| 9 | 会话复用 / 托管刷图 | R4 | B 清单或点名 |
| 10 | auto_register / 农场 runner | 低/危险 | 默认不做 |

**明确不做默认主线：** 本地安全军备（P9）、云存凭证、只堆 Win。

---

## 9. 源码导航速查

### 9.1 原版（只读）

| 主题 | 路径 |
|------|------|
| 模块注册 | `…/module/modules/__init__.py` |
| 日常实现 | `…/modules/{common,stamina,tool,raid,sweep,shop,gacha,collection}.py` |
| 模块基类 | `…/module/modulebase.py` · `modulemgr.py` |
| 账号 | `…/module/accountmgr.py` |
| Web | `…/http_server/httpserver.py` |
| Gree | `…/sdk/greeclient.py` · `sdkclients.py` |
| Sonet | `…/sdk/sonetclient.py` |
| 协议 | `…/core/{apiclient,sessionmgr,crypto,pcrclient,datamgr,version}.py` |
| 模型 URL | `…/model/requests.py`（极大） |
| 团战 | `archive/…/raid/raidworker.py` |

### 9.2 Rust

| 主题 | 路径 |
|------|------|
| 协议客户端 | `crates/rustmadoka-core/src/client.rs` |
| Gree | `…/gree.rs` |
| 加密 | `…/crypto.rs` |
| 指纹 | `…/fingerprint.rs` |
| 日常 | `…/modules/daily.rs` · `mod.rs` · `config_catalog.rs` |
| 洗词条 | `…/modules/wash.rs` |
| 账号 | `…/account.rs` |
| 门禁 | `…/safety.rs` |
| 诊断 | `…/diag.rs` |
| App/HTTP | `crates/rustmadoka-app/src/lib.rs` |
| SPA | `…/static/index.html` |
| Android | `crates/rustmadoka-mobile` · `apps/android/` |

---

## 10. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-07 05:06 | R01 首版：原版全景、模块/协议/Web/cron/Sonet、Rust 映射、缺口与 backlog |
| 2026-08-07 05:19 | 意图对齐：底层≫前端；§8.0 C10 队伍选择；研究意图节 |
