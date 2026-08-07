# Master data (mst) cache and datamgr — source-backed

| Item | Content |
|------|---------|
| **Wall clock** | 2026-08-07 06:04 |
| **Outbound** | `archive/pre-rust-2026-08/autopcr/db/database.py` · `core/datamgr.py` · `model/modelbase.py` · `model/handlers.py` · Rust `crates/rustmadoka-core/src/mst.rs` · `client.rs` |
| **Authority git** | `origin/main` @ `9826135` |
| **Inbound** | [INIT_AND_RESPONSE_PAYLOADS.md](./INIT_AND_RESPONSE_PAYLOADS.md) · [SDK_AND_LOGIN.md](./SDK_AND_LOGIN.md) · [WASH_CHARACTER_LIST.md](./WASH_CHARACTER_LIST.md) · [PROTOCOL_STACK.md](./PROTOCOL_STACK.md) · [UPSTREAM_FILE_MAP.md](./UPSTREAM_FILE_MAP.md) |
| **MAY CONTAIN ERRORS** | Yes |

---

## 1. database.update (login)

Source `db/database.py`:

1. Request resource master revision list → map `name → revision`.  
2. Assign `_client` for subsequent mst fetches.  
3. Preload four lists into attributes:
   - `style_list`
   - `selection_ability_list`
   - `character_list`
   - `figure_list`

## 2. database.mst(request)

| Step | Behavior |
|------|----------|
| Key | Last URL segment snake→camel (`_get_mst_key`) |
| Cache hit | If cached and revision equals `_current_revision[name]`, return cache |
| Miss | `await client.request(request)` → store `mstList` |

## 3. datamgr

| Role | Detail |
|------|--------|
| On every response | `await resp.update(self, request)` if payload implements update |
| Holds | `resp` (init bag), `config`, `collection` map, `user_name` |
| battle_log | `generate_battle_log(units)` → JSON string for finalize APIs |

Handlers of interest (`handlers.py`):

| Response | update effect |
|----------|---------------|
| UserApiGetInitDataListResponse | `mgr.resp = self` |
| ConfigApiGetConfigResponse | `mgr.config = self` |
| CollectionApiGetCollectionDataListResponse | rebuild collection map |
| UserApiSetStaminaRecoverResponse | patch userParamData |
| LikeApiExecLikeResponse | adjust friend medal counts from config |

## 4. Wash list data source

UI style choices come from **mst tables**, not only owned `styleDataList`. See [WASH_CHARACTER_LIST.md](./WASH_CHARACTER_LIST.md) and lesson L3.

## 5. Rust

| Python | Rust |
|--------|------|
| database.update preload | `bootstrap_mst` |
| mst() cache | `MstCache` |
| datamgr.resp | `init_data` |
| datamgr.config | `game_config` |
| quest stage 全表 `get_quest_stage_mst_list` | `GameClient::mst_list` + `filter_quest_stages` / `format_quest_stage_label` |

## 5.1 关卡 ID ↔ 名称（产品化 · 2026-08-08）

原版通过 Master：`/api/mst/get_quest_stage_mst_list` → `questStageMstId` + `name`（约四千行级）。

| 面 | 入口 |
|----|------|
| CLI | `RustMadoka.exe mst quest-stages` · `mst quest-lookup --id …` |
| 缓存 | `RustMadoka_data/cache/mst/{en\|jp}/quest_stage.json`（`--from-cache` / 无账号可读） |
| HTTP | `GET /api/accounts/:alias/mst/quest-stages` · `GET /api/mst/quest-stages?channel=` |
| 网页 | 设置 → 快速刷图 → 关卡 ID「查名称 / 按名搜索」 |
| 模块日志 | `basic` / `super_sweep` 日志带 `关卡=ID（名称）` |

**Outbound 源码：** `crates/rustmadoka-core/src/mst.rs` · `run_ops::query_quest_stages*` · `lib.rs` `MstCmd` · `http_server` 路由 · `static/index.html`。  
**对照：** [BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS.md](./BASIC_SUPER_SWEEP_AND_UPGRADE_QUESTS.md)。

## 6. Revision

| Date | Content |
|------|---------|
| 2026-08-06 | First version |
| 2026-08-07 06:04 | DOC-FULL-01: handlers + cache rules from source |
| 2026-08-08 | 关卡 ID↔名称 CLI/HTTP/缓存/日志产品化 |
