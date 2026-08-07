# Rust 重写经验教训全录（L*）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-06；字段统一 2026-08-07 |
| **读者** | 下一会话实现者 |
| **字段** | 触犯记录 · 内容 · 损失 · 新规则 · 理由 · 源码/log |
| **总索引** | [LESSONS.md](../LESSONS.md) |
| **Inbound** | [HANDOFF.md](../HANDOFF.md) · [NORMS.md](../NORMS.md) · [RUST_REWRITE_DEPENDENCY_FEASIBILITY.md](./RUST_REWRITE_DEPENDENCY_FEASIBILITY.md) |
| **Outbound** | `crates/rustmadoka-core/src/gree.rs` · `crypto.rs` · `client.rs` · `modules/*` · `archive/pre-rust-2026-08/autopcr/` |

---

## L1. Gree `Invalid Signature`（403）

| 字段 | 内容 |
|------|------|
| **触犯记录** | 2026-08-06 |
| **内容** | 引继密码正确仍 `gree 403 Invalid Signature`。initialize 前挂 RSA（应为 HMAC）；RSA 摘要格式错；JSON 键序影响 body hash；坏 token 缓存反复失败 |
| **损失** | 登录全链路不可用；长时间排障 |
| **新规则** | initialize = HMAC；之后 RSA Prehashed SHA1；失败删 token 重注册；`serde_json` preserve_order |
| **理由** | 与 Python greeclient 阶段机一致；签名串必须字节级对齐 |
| **源码/log** | `gree.rs` · `archive/.../greeclient.py` · log `2026-08-06-gree-invalid-signature-fix` |

---

## L2. 游戏 API `crypto: Unpad Error`

| 字段 | 内容 |
|------|------|
| **触犯记录** | 2026-08-06 |
| **内容** | 登录/msgpack 请求 Unpad Error。PKLB 密钥派生输错；`rmp_serde::to_vec_named` 形状与 Python `packb` 不一致 |
| **损失** | 游戏 API 全挂 |
| **新规则** | 密钥结果对齐 `/TZh+1VxrtkNiDEH`；用 rmpv；黄金向量单测 |
| **理由** | AES 解不开服务端密文则上层全失败 |
| **源码/log** | `crypto.rs` · log `2026-08-06-unpad-error-fix` |

---

## L3. 洗词条角色列表从哪来

| 字段 | 内容 |
|------|------|
| **触犯记录** | 2026-08-06（文档钉死；Python 持有空 style 崩溃史） |
| **内容** | UI 下拉 = 登录后 mst 全表（style ∩ character ∩ figure），不是 XAPK，也不是账号持有列表。未持有 style 若仍提交会空指针类失败 |
| **损失** | 选错数据源导致功能错误；原版可崩溃 |
| **新规则** | 列表走 mst；未持有 style 应 Abort；见 WASH_CHARACTER_LIST |
| **理由** | 游戏 API 以 master 为准；持有列表与可选列表语义不同 |
| **源码/log** | `modules/wash.rs` · `mst.rs` · [WASH_CHARACTER_LIST.md](./WASH_CHARACTER_LIST.md) |

---

## L4. 用户不需要 XAPK；默认指纹源

| 字段 | 内容 |
|------|------|
| **触犯记录** | 2026-08-06（产品钉死；曾默认错仓） |
| **内容** | 协议最小资产 = version/sign/libcount（+sm）。默认远程曾指错仓。缺指纹时一键日常卡在登录前 |
| **损失** | 好友无法用；误导用户导完整包 |
| **新规则** | NORMS **P15**；默认 rules raw；exe 旁 publish 保底；tw 指纹可收录但 Sonet 登录未就绪须标 UI |
| **理由** | 用户不导 XAPK；代码仓 ≠ 指纹仓（C1） |
| **源码/log** | `fingerprint.rs` · `fp_slots.rs` · publish/automadoka.json |

---

## L5. 纯净树 vs 原版「不会踩的坑」

| 字段 | 内容 |
|------|------|
| **触犯记录** | 2026-08 探路/归档对照期 |
| **内容** | 纯 archive 无 `raid_config.json` → raidrunner import 炸；PyJWT 未写全 requirements；APKPure 验证码不能作主路径 |
| **损失** | 添加账号 500；启动失败；主路径不可用 |
| **新规则** | 依赖以本地实跑清单为准（NORMS 依赖条）；指纹主路径 = 本地 XAPK 解析 + 云 JSON，非 APKPure |
| **理由** | 超集有 stub/补丁掩盖的坑会在纯净树暴露 |
| **源码/log** | archive raid · requirements · PROBE |

---

## L6. UX / 产品文案与假死感

| 字段 | 内容 |
|------|------|
| **触犯记录** | 2026-08-06 |
| **内容** | 测试期只有 info 却写「清日常全部」；洗词条默认 10 次被当成卡死；长请求无进度 |
| **损失** | 用户不敢点或以为坏了；误耗恐慌 |
| **新规则** | 文案与真实能力一致；长任务必须逐轮/流式反馈；数据目录名 `RustMadoka_data/` |
| **理由** | 信任与误操作成本高于文案省事 |
| **源码/log** | static · wash 流式 · safety 门禁 |

---

## L7. 工程与环境

| 字段 | 内容 |
|------|------|
| **触犯记录** | 2026-08-06 及环境文档期 |
| **内容** | 旧 Python 污染 PATH；release exe 占用拒绝访问；PS5.1 无 BOM 中文脚本失败；做事不写 log |
| **损失** | 本机其它 Python 用途被踩；无法覆盖 exe；脚本不可运行；交接丢记忆 |
| **新规则** | NORMS **P11 P13 P4 P6** |
| **理由** | 环境与交付单一口径；落盘跨会话 |
| **源码/log** | DEV_ENV · POWERSHELL_WIN11 · NORMS |

---

## L8. 协议实现对照清单（每改必测）

| 字段 | 内容 |
|------|------|
| **触犯记录** | 2026-08-06（L1/L2 后固化为清单） |
| **内容** | 协议字段细微偏差即可全挂；需黄金标准可对照 |
| **损失** | 回归登录/请求失败 |
| **新规则** | 改协议栈必对下表与单测；登录串见 SDK_AND_LOGIN |
| **理由** | 服务端不容错密钥/键序/sm |
| **源码/log** | crypto · client · PROTOCOL_STACK |

| 黄金项 | 标准 |
|--------|------|
| sm | `d{sign}o{libcount}1E88A0177575728C9A399A9BD1F43A11D4100065n` |
| AES key | `/TZh+1VxrtkNiDEH`（当前派生结果） |
| IV | `8846515530616782552cab5e1d7c850f` |
| envelope 序 | payload, uuid, userId, sessionId, actionToken, ctag, actionTime |
| actionTime | Windows FILETIME 风格 |

---

## L9. 产品路线（倾向 · 已拍板）

| 字段 | 内容 |
|------|------|
| **触犯记录** | 2026-08 换道讨论（非单次 bug） |
| **内容** | 多产品线并行易只堆 Win 或只堆 Python |
| **损失** | Android 永久落后；精力分散 |
| **新规则** | NORMS **P24**：Win Rust 主路径 → Android 共享 core；新功能进 Rust；Python 超集兜底不挡 |
| **理由** | 协议一份维护成本最低 |
| **源码/log** | DIRECTION · PLAN_ANDROID · PRODUCT_LINES |

---

## L10. Gree token/私钥缓存

| 字段 | 内容 |
|------|------|
| **触犯记录** | 2026-08-06（审计误解；非功能缺失） |
| **内容** | `cache/token/{引继}_{密码MD5}.json` 存 privateKey+uuid（及 device_id）；主人以为「多写的安全功能」 |
| **损失** | 信任摩擦；可能被误删导致重迁 |
| **新规则** | 文档讲清（SDK §5 · C4）；删卡清匹配 token；勿进 git |
| **理由** | 对照 Python；二次登录与设备稳定；非银行级「再加密产品」 |
| **源码/log** | `gree.rs` save_cache/load_cache · device_profile |

---

## L11. 队伍码「名称」难用

| 字段 | 内容 |
|------|------|
| **触犯记录** | 2026-08-06（母项目遗留 + 移植放大） |
| **内容** | 快速刷图等填队伍名/「20」类数字失败或行为诡异。原版先 int() 当 partyDataId；默认 "20"；Rust 曾对未知 id 仍提交 |
| **损失** | 刷图失败/错队；用户困惑 |
| **新规则** | `resolve_party`：名 → 序号 → id，失败列清单；见 PARTY_TEAM_RESOLVE |
| **理由** | 三套标识混用必须统一解析失败策略 |
| **源码/log** | `modules/daily.rs` · tool.py/raid.py/sweep.py · log party-team-resolve |

---

## L12. 探索主线是两层图；原版 secret 只走 prev1 链到硬编码篇章

| 字段 | 内容 |
|------|------|
| **触犯记录** | 2026-08-07（导出分析 + 对服拉 field_stage/point mst；首次完整钉死） |
| **内容** | 探索「主线」在协议里至少两层：（1）**篇章** `fieldStage`：边为 `prevFieldStageMstId` 与 `prevFieldStageMstId2`；（2）**篇内点** `fieldPoint`：边为 `prevFieldPointMstId`，`pointType` 区分迷宫(1)与战斗(2/3/4)。原版 `secret`：邀请绑定后 `clear_field(612001)`，递归先清 prev1，再按层/点到达并打迷宫或战斗 API；战斗 `partyDataId` 写死 1、结果写死胜。`clear_dungeon_event` 只处理已通关篇隐藏事件。E1 登录包**不含** field mst 与 collection，须另拉。本号 `clearedFieldStageMstId=612001` 且 collection 已 clear 该目标。 |
| **损失 / 负面影响** | 混用 secret / clear_dungeon_event / battle_mission 会导致错误工期与错误风险预期。只信攻略站顺序而不拉 mst，会与服 id 脱节。把 612001 当「全游戏主线终点」会低估后续记忆之窗。 |
| **新规则** | 实现或文档描述探索推进时：必须区分篇章图与点图；必须写明走 prev1 还是 prev2；点类型分支对照原版 API。产品配置化见协作教训 **C19**。黄金对照：分析 log 中 600001 篇内 1-1…Boss 序与 600001→…→612001 篇章序。 |
| **理由 / 原理** | 服务端进度与解锁以 mst id 与 collection/csv 为准。两层图 + 点类型决定请求序列与体力/战斗风险。原版硬编码给出**可运行的默认策略样本**，产品要参数化才能跟版本。 |
| **源码/log** | `tool.py` secret · `common.py` FieldStage/Point 记录 · `exports/_analysis/` · `docs/logs/2026-08-07-export-data-analysis.md` |

---

## L13. 模块结果须按游戏语义映射到**当前标签**；HTTP/业务失败不可默认吞成「用户级错误叙事」

| 字段 | 内容 |
|------|------|
| **触犯记录** | 2026-08-07 读本机清日常 task_log；与协作教训 C20 同时登记；**2026-08-07 审核：三标签非完备** |
| **内容** | 调度层（`run_daily`）当前把 `Result` 收成 **成功 / 跳过 / 中止 / 错误** 四标签（另：用户整次放弃）。问题在**模块实现与请求层**：许多「今天无内容」路径应 `Skip`，样例中 **gather / freegacha** 曾在 request 阶段以 **HTTP 404** 进错误分支，整次日常 `失败2`。跳过文案曾大量英文；错误文案为完整诊断模板。原版 Python `eResultStatus` 含 **成功/跳过/警告/中止/错误/致命** 六种，Rust **未**完整复刻「警告/致命」，也**未**单独表达模块内部分成功。业务码→Skip 表（`from_game_api_errors`）仅覆盖已采样码，**不是**官方全表。 |
| **损失 / 负面影响** | 日志与 UI 把可预期空转显示为故障；点测无法用 `status=error` 判断是否回归；文档若写「三态覆盖全部场景」会误导后继把标签当完备本体。 |
| **新规则** | （1）每个模块注释或 tech 表尽量列出：入口 API、成功条件、Skip 条件、Abort 条件、真失败条件；**未知标【未证实】**，禁止假装已穷尽。（2）对服业务「无次数/无奖励/未开放/数量不足」等：在模块内译为 `CoreError::Skip(中文)`，禁止直接把原始 HTTP 诊断当模块结果。（3）仅当签名、会话、指纹、未识别协议损坏等无法解释为业务跳过时，才用错误态 + `diagnose`。（4）改模块时对照原版 Skip **与**游戏内条件，双源验证。（5）汇总：仅 Skip 不得抬 `ok=false`；中止与错误分计展示，勿并成笼统「失败」二字掩盖配置问题。（6）**禁止**在文档中写「成功/跳过/失败已覆盖全部游戏结局」。 |
| **理由 / 原理** | 展示标签是给人与自动化看的**粗粒度产品语义**，不是 HTTP 状态码镜子，也不是游戏状态机的完备枚举。404/业务码在不同步骤含义不同。不结合步骤映射，diag 越完整越误导；把标签写成绝对全集，后继不敢承认部分成功、警告、未映射码等缝隙。 |
| **源码/log** | `modules/mod.rs` · `error.rs` `from_game_api_errors` · `daily.rs` · `diag.rs` · ERROR_DIAGNOSTICS §模块结果 · C20 · log `2026-08-07-outcome-audit` |

---

## 修订

| 日期 | 内容 |
|------|------|
| 2026-08-06 | 首版全录 |
| 2026-08-06 | L4 指纹源；L10 token；L11 队伍 |
| 2026-08-07 | L1/L2 标准字段 |
| 2026-08-07 | **L3–L11 全部标准字段**；顺序 L1–L11；Outbound 头 |
| 2026-08-07 08:00 | L12 探索两层图与 secret 边界 |
| **2026-08-07** | **L13 模块三态与游戏语义映射** |
