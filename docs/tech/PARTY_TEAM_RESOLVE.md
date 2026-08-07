# 队伍码 / 队伍名解析（快速刷图 · 魔女 · 心之器）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-06 |
| **现象** | 填「队伍名」或「看起来像序号的数字」时经常失败 |
| **源码** | `crates/rustmadoka-core/src/modules/daily.rs` · `resolve_party` |
| **对照** | `archive/pre-rust-2026-08/autopcr/module/modules/tool.py` · `raid.py` · `sweep.py` |

---

## 1. 字段含义（游戏侧）

| 字段 | 含义 | 用户常见误解 |
|------|------|----------------|
| **name** | 编成自定义名称（可中文） | — |
| **partyIndex** | 编成**槽位序号**（界面上「第几队」类） | 和服务器 id 不是一回事 |
| **partyDataId** | 服务器队伍主键（请求里真正用的） | 默认配置里的 `20` 易被当成序号 |

战斗/扫荡 API 的 `partyDataId` 必须是 **partyDataId**，不是序号、不是名称字符串。

---

## 2. 母项目原逻辑（遗留）

### 2.1 快速刷图 `super_sweep`（tool.py）

```text
team = 配置字符串
try: team = int(team)          # 成功 → 直接当 partyDataId 用
except: 在 partyDataList 里找 name == team 得到 partyDataId
if team is None: Abort
```

问题：

1. **先 int**：配置为 `"20"` 时**永远不会**按名称查；也**不会**按 `partyIndex==20` 查。  
2. **默认 `'20'`**：作者环境里可能刚好存在 `partyDataId=20`；对多数账号无效。  
3. **名称全等**：`party.name == team`，首尾空格即失败。  
4. **找不到时 Abort 文案**用的是已变成 `None` 的 `team`，原配置丢失。

### 2.2 心之器 `heart`（sweep.py）

数字会 **再校验** 是否在 `partyDataList` 中存在，比 `super_sweep` 稳一点。

### 2.3 魔女 raid.py

与 super_sweep 类似：先 int 再 name。

---

## 3. 本仓库旧实现问题

Rust 旧 `resolve_party`：数字解析成功后，若列表中无此 `partyDataId`，仍 `Ok(id)` **原样提交** → 服务端失败，且无队伍列表提示。

---

## 4. 现解析顺序（产品改进）

1. trim；空 / `0` → 第一支有人的 `partyType=1`  
2. 名称 trim 后全等  
3. 纯数字 → **先 `partyIndex`，再 `partyDataId`**（均须在列表中）  
4. 名称包含匹配（**仅唯一命中**）  
5. 失败：Abort + 列出可用「名称 / 序号 / id」  

配置文案：`队伍：名称 / 编成序号(partyIndex) / 服务器id(partyDataId)`。  
快速刷图默认队伍改为 **空**（未填则 Skip）。

---

## 5. 使用建议（给用户）

| 推荐 | 写法 |
|------|------|
| 最稳 | 游戏内编成**名称**完整粘贴 |
| 数字 | 优先理解为**编成序号**；若不对再试是否为服务器 id |
| 排错 | 看模块日志里的「可用队伍示例」 |

---

## 修订

| 日期 | 内容 |
|------|------|
| 2026-08-06 | 首版：母项目 int 优先坑 + Rust 改进解析 |
