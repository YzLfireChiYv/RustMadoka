# 原版对照完整性 · 通讯线（安装包 / 发出 / 收回）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-07 05:35 |
| **用途** | 回答：本机是否有完整原版 GitHub 对照；游戏相关通讯能取什么、发什么、拿什么 |
| **Outbound 原版** | Git remote `origin` = https://github.com/cc004/automadoka · `ref-legacy-superset/upstream-ref/cc004-automadoka/` · `archive/pre-rust-2026-08/` |
| **Outbound Rust** | `crates/rustmadoka-core`（gree / client / fingerprint / crypto / **wire**）· app feature `wire_record` |
| **Inbound** | [HANDOFF.md](../HANDOFF.md) · [W2_WIRE_ANALYSIS…](./W2_WIRE_ANALYSIS_AND_REWRITE_LIST.md) · [AUTOMADOKA_RESEARCH_AND_RUST_GAP.md](./AUTOMADOKA_RESEARCH_AND_RUST_GAP.md) · [PROTOCOL_STACK.md](./PROTOCOL_STACK.md) · [SDK_AND_LOGIN.md](./SDK_AND_LOGIN.md) · [VERSION_FINGERPRINT.md](./VERSION_FINGERPRINT.md) · [API_INVENTORY.md](./API_INVENTORY.md) |
| **MAY CONTAIN ERRORS** | Yes — 以 `git ls-remote` 与源码为准；线上字段随游戏版本可变 |

---

## 0. 一页结论

| 问题 | 结论 |
|------|------|
| **官方 GitHub 是否在本机可用？** | **是。** remote `origin` → `cc004/automadoka`；本机 `origin/main` = **`9826135`**（`fix wash`，2026-06-03），与 `git ls-remote origin HEAD` **一致**（2026-08-07 核对）。 |
| **是否完整历史克隆？** | **是（两处）。** (1) 本仓库 git 对象含 `origin/main` 树；(2) `ref-legacy-superset/upstream-ref/cc004-automadoka/` 为完整 clone（约 1040 commits，push 禁用）。 |
| **`archive/pre-rust-2026-08/` 是否 = 官方仓？** | **业务源码齐，布局是归档快照。** 含完整 `autopcr/` + `raid/` + 根文件副本 `upstream-tree/`；另含探路 Android、collab 文档、**本地探针** `probe_capture.py`（官方仓无）。**不是** git 历史本身。 |
| **官方仓有没有前端 SPA 源？** | **没有 ClientApp 构建产物进 git。** `origin/main` 仅有 `httpserver.py` 等；本机 `archive/.../ClientApp/` 与超集内 assets 是**打包进运行树的静态前端**，不能当作「上游 React 源码仓」。 |
| **通讯上本工具靠什么？** | **不跑游戏安装包进程。** 安装包/XAPK 只用于抽 **指纹三元组**；身份走 **Gree/Sonet SDK HTTP**；业务走 **游戏 API（msgpack+AES）**。 |

---

## 1. 本机原版材料地图（对照用）

```text
官方 GitHub  cc004/automadoka
    │
    ├─ 本仓 git remote「origin」     ← 可 git show origin/main:path
    ├─ ref-legacy-superset/upstream-ref/cc004-automadoka/  ← 完整 clone，只读
    ├─ ref-legacy-superset/upstream-ref/static-snapshot/automadoka-main/  ← zip 快照
    ├─ ref-legacy-superset/app/autopcr/  ← 超集运行树内嵌的上游业务（可能被本地 patch）
    └─ archive/pre-rust-2026-08/
           ├─ autopcr/ · raid/ · upstream-tree/  ← 冻结对照（Rust 文档默认 Outbound）
           ├─ android/ 探路
           └─ docs-collab-python/
```

| 路径 | 角色 | 与 origin/main 关系 |
|------|------|---------------------|
| `git` `origin/main` @ `9826135` | **权威当前官方树** | 自身 |
| `upstream-ref/cc004-automadoka` | 完整历史 + 工作树 | HEAD 同 `9826135` |
| `archive/.../autopcr` | 文档/Rust 默认对照 | 业务 py 齐全；多 `probe_capture`；**无 git 历史** |
| `ref-legacy-superset/app` | 可跑超集 | 上游 + win 壳/补丁；**可能与纯净 origin 有 diff** |
| 根 `crates/` | 本产品 | 非上游 |

### 1.1 完整性核对（2026-08-07）

| 检查 | 结果 |
|------|------|
| `git ls-remote origin HEAD` | `9826135…` |
| `git log origin/main -1` | 同 hash，`fix wash`，2026-06-03 |
| origin 树 `*.py` 数量 | **68** |
| `upstream-ref/cc004` `*.py` | **68** |
| archive `autopcr` `*.py` | **56**（仅 autopcr 子树；raid/根测试在旁路；+本地 probe） |
| origin 是否含 `ClientApp/index.html` | **否** |
| 公开仓 `rustmadoka` | 本产品干净历史；**不是**母项目镜像 |

**结论：** 做「原版能力/通讯」对照时：

1. **优先** `origin/main` 或 `ref-legacy-superset/upstream-ref/cc004-automadoka`（与官方一致）。  
2. **日常文档链接**仍可写 `archive/pre-rust-2026-08/autopcr`（冻结、带探针注释）；若与 origin 有 diff，以 origin 为准并记 log。  
3. **不要**把 `ref-legacy-superset/app` 默认当「纯官方」（可能有超集补丁）。  
4. 刷新官方：在 `upstream-ref/cc004-automadoka` 内 `git fetch` + `git pull --ff-only`（pushurl=no_push）。

### 1.2 官方仓能力边界（避免误解）

| 有 | 无 / 不在仓内 |
|----|----------------|
| 完整 Python 协议与模块、model 请求表（约 **494** 条 `/api/...` url）、Gree/Sonet、Quart 服务端、raid 脚本、Docker | 游戏安装包本体；ClientApp **源码工程**；用户 token/引继；完整 mst 数据文件（运行时向服务器拉） |

---

## 2. 通讯总览（三层）

```text
┌─────────────────────────────────────────────────────────────────┐
│ A. 安装包 / XAPK（离线解析，一般不上游戏服）                      │
│    → version + sign(MD5) + libcount → sm 指纹                     │
└───────────────────────────────┬─────────────────────────────────┘
                                │ sm / appVersion 注入后续请求
┌───────────────────────────────▼─────────────────────────────────┐
│ B. 渠道 SDK 服（明文 JSON + OAuth/JWT）                           │
│    日/国：Gree payment 域名  ·  台：Sonet mme-sdk                  │
│    → uuid、设备登记、引继迁移、（Gree）RSA 私钥会话材料             │
└───────────────────────────────┬─────────────────────────────────┘
                                │ uuid + 私钥/JWT
┌───────────────────────────────▼─────────────────────────────────┐
│ C. 游戏 API 服（HTTP POST body = AES(msgpack(envelope))）         │
│    日：api.mmme.pokelabo.jp  ·  国：api-gl…  ·  台：app-mme.so-net │
│    → sessionId/userId、账号数据、mst、战斗结果…                    │
└─────────────────────────────────────────────────────────────────┘
```

本工具 **模拟官方客户端协议**，不嵌入完整 Unity 游戏包。

---

## 3. A 层：从安装包能获取什么

### 3.1 产品实际用到的（指纹）

| 字段 | 如何从 XAPK 得到 | 之后干什么 |
|------|------------------|------------|
| **version** | `manifest.json` → `version_name` | Login / Gree payload 的 `appVersion` |
| **sign** | split `id=base` 的 APK **整文件 MD5** hex | 拼进 `sm` |
| **libcount** | split `id=config.arm64_v8a` 内 `lib/arm64-v8a/*` **文件个数** | 拼进 `sm` |
| **package_id**（可选元数据） | `manifest.json` → `package_name` | 判断 jp/en/tw；发布 JSON |

```text
sm = "d" + sign + "o" + libcount + "1E88A0177575728C9A399A9BD1F43A11D4100065n"
```

每个游戏业务请求的 payload 在 `prepare()` 时写入 **`sm`**（见 `modelbase.RequestBase` / Rust 等价）。

源码：`version.py` · `fingerprint.rs::extract_from_xapk` · [VERSION_FINGERPRINT.md](./VERSION_FINGERPRINT.md)。

### 3.2 明确不从安装包当主路径拿的

| 内容 | 说明 |
|------|------|
| 完整 so/资源/剧情资产 | 体积大；登录与清日常 **不需要** 解包进客户端 |
| 用户账号/引继 | 安装包没有；用户手填 |
| 实时 mst 平衡表 | 游戏服 `/api/mst/*` 下发（可 revision 缓存） |
| APKPure 在线刷包 | 原版 `update_version` 默认路径；**产品禁用**，改 rules/embed/本地 XAPK |

### 3.3 原版 vs 本产品指纹来源

| | 原版 | Rust 产品 |
|--|------|-----------|
| 主路径 | 常 APKPure 流式 XAPK 或本地 cache/version.json | **rules 仓 raw** + 安装包 **embed** + 可选本地 XAPK 提取 |
| 用户 | 可能被迫碰完整包 | **不必**自导 XAPK（主人预解析云分发） |

---

## 4. B 层：发给渠道服 / 从渠道服拿什么（Gree 日国 · 摘要）

**基址示例：**  
- 日：`https://gl-pkl-jp-payment.gree-apps.net/v1.0`  
- 国：`https://gl-pkl-us-payment.gree-apps.net/v1.0`  

| 方向 | 典型端点 | 发送（概念） | 收回（概念） |
|------|----------|--------------|--------------|
| 注册设备 | `POST /auth/initialize` | `device_id`、公钥 PEM、`payload` JSON（含 appVersion、设备画像、**sm**） | **uuid**；本地保存 **RSA 私钥** |
| 设引继密码 | `/migration/password/register` | `migration_password`=B_encode(密码)、device_id | OK |
| 取引继码 | `GET /migration/code` | OAuth | **migration_code** |
| 迁入账号 | `/migration/code/verify` → `/migration` | 引继码+密码 → token；再带 src/dst uuid、公钥 | 账号绑定到本机 uuid |
| 授权 | `POST /auth/authorize` | OAuth（RSA 阶段） | 会话可用；Inactive 则 `/linked/active/update` 再试 |

**签名阶段机（L1）：** initialize **无**私钥 → HMAC-SHA1(APP_SECRET)；之后 RSA-SHA1 Prehashed。  
**落盘：** `cache/token/{引继}_{密码MD5}.json` → `privateKey` + `uuid`（Rust 另有 `device_profile` 稳定 device_id）。

**台服 Sonet（摘要）：** `mme-sdk.so-net.tw` JSON+MD5 sign 后缀 `sonet`；拿 uuid/token/JWT；游戏 API 用 JWT 作签名材料；AES 密钥 `SONET_HASH_KEY`。原版 **migrate_from 未实现**；Rust **登录未移植**。

细节：[SDK_AND_LOGIN.md](./SDK_AND_LOGIN.md) · `greeclient.py` · `gree.rs`。

---

## 5. C 层：发给游戏服 / 从游戏服拿什么

### 5.1 传输形态（每条业务 API）

| 项 | 内容 |
|----|------|
| 方法 | **POST**（几乎全部） |
| URL | `{apiroot}{Request.url}`，如 `https://api.mmme.pokelabo.jp/api/user/get_init_data_list` |
| Content-Type | `application/x-msgpack` |
| 其它头 | `x-unity-version`、`x-region`、`user-agent`（模拟 Android Unity）、`x-post-signature` 等 |
| Body | **AES-CBC(PKCS7)** 加密后的 **msgpack** 字节；IV 固定策略见 crypto |
| 签名 | 对 **密文字节** `ApiCrypto.sign(RSA私钥)` → 头 `x-post-signature`（Gree 渠道） |

**明文逻辑 envelope（加密前）：**

```text
{
  payload: { ...业务字段, lastHomeAccessTime, sm },
  uuid, userId, sessionId, actionToken, ctag, actionTime
}
```

- `actionTime`：Windows FILETIME 风格  
- 登录前 `sessionId` 可空；登录后带上  

**响应：** 同密钥解密 →

```text
{ payload: TResponse, url, status, errors? }
```

`errors` 非空 → 业务失败；HTTP **428** → 指纹过期需刷新 version；**401** → 会话失效需重登。

权威实现：`apiclient.py` · `client.rs` · [PROTOCOL_STACK.md](./PROTOCOL_STACK.md)。

### 5.2 模型层「能调用的 API 全集」

| 统计 | 值 |
|------|-----|
| `requests.py` 中 `/api/...` **唯一路径** | **约 494** |
| 含义 | 官方客户端能力的 **静态上界**（pydantic 模型生成/维护） |
| 原版/日常实际命中 | **远小于** 494；登录串 + 各模块 `do_task` 子集 |
| 清单入口 | [API_INVENTORY.md](./API_INVENTORY.md)（分类）；完整枚举以 `requests.py` 为准 |

前缀包括：login、user、party、character、style、collection、mst、quest、mission、present、gacha、shop、multi_raid、solo_raid、pvp、selection_ability、tower、exploration、tutorial、web_pay、chat…（大量玩法与 **社交/支付** 模型也在表里，原版模块未必全用）。

### 5.3 登录后「收回」的核心数据（能力富矿）

**会话：**

| 字段 | 来源 | 用途 |
|------|------|------|
| sessionId | LoginApi 响应 | 后续 envelope |
| userId | LoginApi | 同上 |

**初始化串（sessionmgr，有序）：** 在 SDK uuid 就绪后：

1. LoginApi  
2. `db.update` → resource master **revision** + 预拉 style/selection/character/figure **mst 定义表**  
3. `UserApiGetInitDataList` → **账号持有物大包**（见下）  
4. party build / character list / collection×2 / style list / userParam / config / load_option / web_pay cancel / terms  

**`UserApiGetInitDataList` 的 payload 字段（模型静态钉死）：**

| 字段 | 含义（产品向） |
|------|----------------|
| **partyDataList** | 全部编成：名称、序号、id、类型、成员… |
| **styleDataList** | 持有风格/造型进度 |
| **characterDataList** | 角色数据 |
| **cardDataList** | 卡 |
| **itemDataList** | 道具 |
| **characterBuildDataList** | 构筑 |
| **userParamData** | 昵称、等级、体力等 |
| **userData** | 用户数据 |
| **miniTutorialData** | 教程状态 |
| **styleRentalBorrowingDataList** | 租借相关 |

→ 这就是「底层远超前端」的数据源之一：**一次 init 已有队伍全表**，前端却常只给文本框。

**mst（定义表，非账号私有）：** 角色名、词条名、关卡消耗、商店目录等；按 revision 缓存。洗词条下拉 = mst 拼表，不是「仅持有列表」（L3）。

**单次模块：** 再按玩法 POST 对应 `/api/...`，响应里常带回更新后的 list 片段；原版 `datamgr` 用 `response.update` 写回内存（Rust 多用 `Value` 合并关键路径）。

### 5.4 本工具「发送」给游戏服的内容类型（概念）

| 类别 | 例子 | 敏感度 |
|------|------|--------|
| 身份与会话 | uuid、userId、sessionId、sm、appVersion | 高 |
| 设备画像（模拟） | deviceModel、osVersion 等固定串 | 中（指纹类） |
| 玩法指令 | 开战 questStageMstId、partyDataId、扫荡、领奖 id 列表 | 中高（消耗资源） |
| 战斗结果摘要 | battleLog JSON（可简化空 Commands） | 中 |
| **不发送** | 工具用户组密码；本机文件路径；他人无关隐私（设计上） | — |

**不云存** 引继/游戏密码（P8）；落盘仅在用户本机 `automadoka_data`。

---

## 6. 与「完整原版能力」的关系（给后续产品化）

| 层 | 原版模型/代码已具备 | 当前前端/产品暴露 | 备注 |
|----|---------------------|-------------------|------|
| API 模型 ~494 | 是 | 仅模块用到的子集 | 表在 = 可扩展上界 |
| init 大包 | 是 | 昵称等级/部分设置；**队伍未做选择器** | → C10 |
| mst 全表 | 是（登录拉） | 洗词条下拉等 | 可继续产品化 |
| cron / 工具 4 / Sonet | 代码有（Sonet 半成品） | 未或未齐 | 移植类 |
| 聊天/公会/支付等 API | 模型有 | 基本不用 | 非主线 |

研究/改进应继续问：

> **这条数据登录后是不是已经在内存或一次请求就能拿到？用户是否还在手填、盲填？**

而不是只问「有没有同名菜单模块」。

---

## 7. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-07 05:35 | 首版：对照完整性 + 通讯 A/B/C 层 |
