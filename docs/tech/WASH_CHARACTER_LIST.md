# 快速洗词条：角色列表从哪里来？

> **墙钟：** 2026-08-06  
> **证据等级：** 源码静态钉死 + **run-clean 探针实跑**（会话 `20260805T165817Z-2af2701e`）  
> **Outbound 源码：**  
> - Rust：`crates/rustmadoka-core/src/modules/wash.rs` · `mst.rs`  
> - 对照：`archive/pre-rust-2026-08/autopcr/module/modules/wash.py` · `db/database.py` · `core/sessionmgr.py` · `module/config.py`  
> **Inbound：** [RUNTIME_REPORT_2026-08-06.md](./RUNTIME_REPORT_2026-08-06.md) · [DATA_AND_MST.md](./DATA_AND_MST.md) · [EMPIRICAL_CHECKLIST.md](./EMPIRICAL_CHECKLIST.md) · [LESSONS_RUST_PORT.md](./LESSONS_RUST_PORT.md) L3

---

## 1. 一句话结论

**Web「目标角色」下拉框的数据，来自登录后游戏 Master API 拉回的「全服 style/character/figure 定义表」，存在进程内 `db.*_list`；不是 XAPK 文件，也不是「账号已拥有风格」列表。**

词条名称下拉同理，来自 **SelectionAbility master**（类型 2 的副词条）。  
真正洗练时，才用**账号维度**的 `SelectionAbilityApiGetSelectionAbilityDataList` 看当前技能石状态。

---

## 2. UI 绑定（前端看到的选项）

装饰器（`wash.py`）：

```text
@singlechoice('filter_style', '目标角色', '', get_style_list)
@singlechoice('filter_sub_selection_key_*', '目标词条*', NONE, get_sub_selection_list)
```

`Config.candidates` 在序列化给 Web 时若是 **callable 会当场调用**（`config.py`）：

```text
Web 打开工具配置 / generate_info
  → Config.candidates → get_style_list() / get_sub_selection_list()
  → 读全局 db 内存表
  → 返回字符串列表给 SPA 下拉
```

因此：**下拉刷新依赖「本进程是否已有成功登录过并执行过 db.update」**。  
从未登录成功时，`db.character_list` 等为空 → `get_style_list` 返回 `[]`（空列表）。

---

## 3. `get_style_list` 数据拼装（角色下拉）

源码逻辑（完整句）：

1. 若 `db.character_list` 或 `db.figure_list` 为空 → 返回 `[]`。  
2. `char_dict`：`characterMstId → name`（角色本名）。  
3. `figure_dict`：`styleFigureMstId → 角色名`（造型图关联到角色）。  
4. 遍历 **`db.style_list` 全部 style master**：  
   显示串 = `{styleMstId}:[{style.name}]{角色名}`  
   值 = `styleMstId`。

| 内存表 | Master API（首次网络） | 探针实测条数（EN 登录后） |
|--------|------------------------|---------------------------|
| `db.style_list` | `/api/mst/get_style_mst_list` | **119** |
| `db.character_list` | `/api/mst/get_character_mst_list` | **65** |
| `db.figure_list` | `/api/mst/get_style_figure_mst_list` | **94** |
| `db.selection_ability_list` | `/api/mst/get_selection_ability_mst_list` | **291** |

显示名示例（实跑结果 JSON）：  
`10050101:[盟神抉枪]佐倉杏子` — 即 styleMstId + 风格名 + 角色名。

### 3.1 重要边界：全表 ≠ 账号持有

| 数据 | API | 用途 |
|------|-----|------|
| **全 style master** | `MstApiGetStyleMstList` | **下拉列表**（几乎全部可点） |
| 账号风格数据 | `StyleApiGetStyleDataList`（登录串之一） | 持有/养成态，**不参与** `get_style_list` |
| 账号词条/技能石 | `SelectionAbilityApiGetSelectionAbilityDataList` | **执行洗练**时读当前槽位 |

因此：用户可在下拉中选到**账号尚未持有**的 style。  
执行时若 `selectionAbilityDataList` 里没有该 `styleMstId`，会走到空数据路径（见 §6 实跑 bug）。

---

## 4. Master 何时进内存？

`sessionmgr._login` 成功路径：

```text
SDK 登录 → LoginApi
→ db.update(next)     ← 这里拉 revision + 四类 mst 填 db.*_list
→ UserApiGetInitDataList …
→ StyleApiGetStyleDataList 等账号 API
→ _logged = True
```

`database.update`（探针 `mst_update`）：

```text
GetResourceMasterDataMstList  → revision 字典（实跑 revision_names=177）
强制 mst：
  style / selection_ability / character / style_figure
```

同进程后续登录：`mst_fetch` **cache_hit=true**，不再打四条 mst 网络（探针：仅首次各 1 次网络，之后 10 次 update 全缓存命中）。

**与 XAPK / 云指纹无关：** 指纹只保证能 `LoginApi`；角色名表在登录后 API。

---

## 5. 词条下拉 `get_sub_selection_list`

- 遍历 `db.selection_ability_list`  
- 仅 `selectionAbilityType == 2`（副词条）  
- 显示：`{id}:{name}`  

执行洗练时名称反查仍可再 `db.mst(MstApiGetSelectionAbilityMstListRequest())`（缓存命中）。

---

## 6. 执行洗练时的账号 API（实跑）

| API | 次数（探针会话） | 含义 |
|-----|------------------|------|
| `/api/selection_ability/get_selection_ability_data_list` | 3–4 | 读账号当前技能石/词条 |
| `/api/selection_ability/learn_sub_selection_ability` | **10** | 单次洗练（与 UI「重复次数」一致） |

国际服角色 `meimei` 洗词条结果：**成功**；配置目标角色  
`10050101:[盟神抉枪]佐倉杏子`，重复 10 次，日志汇总了各词条出现次数。

日服角色两次洗词条：**错误**

```text
'NoneType' object has no attribute 'subSelectionAbilityMstIds1'
```

对应源码在取得 `selection_ability_data_dict.get(style_id)` 为 **None** 时仍 `getattr(...)`（未先判空）。  
含义：选中的 style 在**账号 selection 数据列表里不存在**（未持有或未初始化该风格技能石），与「下拉来自全 mst」一致。  
后面虽有 `AbortError("没有找到角色")` 分支，但更早的 `init_sub_ids_str = getattr(...)` 已炸。

---

## 7. 对产品 / 云分发 / Rust 重写的含义

| 问题 | 结论 |
|------|------|
| 云端要不要发角色名表？ | **默认不必**；登录后 mst 即可（体积随 revision，首次四表足够 UI） |
| 指纹包要不要含角色？ | **不要**；指纹只服务 sm/version |
| 下拉是否应只显示持有角色？ | 产品可改进：用 `StyleApiGetStyleDataList` 或 selection 列表过滤 mst |
| 空数据崩溃 | 应判空再 Abort（上游小缺陷，重写时可修） |

---

## 8. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-06 | 结合探针与 wash 源码首版钉死 |
