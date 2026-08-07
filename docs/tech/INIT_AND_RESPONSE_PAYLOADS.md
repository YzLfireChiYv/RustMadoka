# Login chain and response payloads (sessionmgr + models)

| Item | Content |
|------|---------|
| **Wall clock** | 2026-08-07 06:04 |
| **Outbound (authority)** | `archive/pre-rust-2026-08/autopcr/core/sessionmgr.py` · `db/database.py` · `model/requests.py` · `model/responses.py` · `model/handlers.py` · `model/common.py` · `core/datamgr.py` · `core/bootstrap.py` |
| **Authority git** | `origin/main` @ `9826135` (cc004/automadoka) |
| **Rust** | `crates/rustmadoka-core/src/client.rs` (`full_login` / `login_for_info` / `bootstrap_mst`) |
| **Inbound** | [SDK_AND_LOGIN.md](./SDK_AND_LOGIN.md) · [PROTOCOL_STACK.md](./PROTOCOL_STACK.md) · [API_INVENTORY.md](./API_INVENTORY.md) · [UPSTREAM_FILE_MAP.md](./UPSTREAM_FILE_MAP.md) · [PARTY_TEAM_RESOLVE.md](./PARTY_TEAM_RESOLVE.md) · [DOC_COVERAGE_AUDIT.md](./DOC_COVERAGE_AUDIT.md) |
| **MAY CONTAIN ERRORS** | Yes — field lists from pydantic models; live server may add fields |

---

## 0. 中文摘要（给主人）

登录成功后，工具不是「只拿到一个 token」就完事。它会按固定顺序向**游戏服**连发一串 API。其中最重要的一包是 **`/api/user/get_init_data_list`**：里面已经带有**全部队伍**（名称、序号、服务器 id）、角色/风格/卡/道具、昵称体力等级等。  
因此产品可以做「队伍列表点选」（任务 C10），而不必让用户死记队伍名。  
Rust 的 `full_login` 对齐同一串；`login_for_info` 是产品优化的**轻量**路径，可能**没有**完整 `partyDataList`。

---

## 1. When the chain runs / 何时触发

Source: `sessionmgr.request` / `_login`.

| Condition | Behavior |
|-----------|----------|
| `_logged == False` | Before any game request, run full `_login` |
| After `_login` success | `_logged = True` |
| Later `ApiException` on business request | `_logged = False` (next request re-logins) |
| `VersionUpdatedException` (HTTP 428) | Loop `continue` and retry full login after fingerprint update |
| Login rate limit | `@FreqLimiter(LOGIN_LIMIT_TIMES=5, LOGIN_LIMIT_INTERVAL=30)` on `_ensure_token` |

Constants: `autopcr/constants.py` — `LOGIN_LIMIT_TIMES`, `LOGIN_LIMIT_INTERVAL`.

---

## 2. Ordered init chain / 初始化串顺序（源码钉死）

Source: `sessionmgr._login` lines calling `next.request(...)` after `_ensure_token` and `db.update`.

| Step | Phase | Request class | HTTP path | Primary response fields (from `responses.py`) |
|-----:|-------|---------------|-----------|-----------------------------------------------|
| 0a | SDK | `sdk.login()` (not game HTTP) | Gree/Sonet hosts | **uuid** (and private key cache for Gree) |
| 0b | Game login | `LoginApiLoginRequest` | `/api/login` (property on class; body fields in requests.py) | `sessionId`, `userId`, `status`, `banType` |
| 1 | Master meta | via `db.update` → `GetResourceMasterDataMstListRequest` | `/api/mst/get_resource_master_data_mst_list` (name in database.py comment path) | revision list for mst names |
| 2 | Master preload | `MstApiGetStyleMstListRequest` etc. | four mst list APIs | full definition tables for style / selection_ability / character / figure |
| 3 | **Init bag** | `UserApiGetInitDataListRequest` | **`/api/user/get_init_data_list`** | see §3 |
| 4 | Builds | `PartyApiGetCharacterBuildDataListRequest` | `/api/party/get_character_build_data_list` | character build list |
| 5 | Characters | `CharacterApiGetCharacterListRequest` | `/api/character/get_character_list` | character list |
| 6 | Collection param | `CollectionApiGetCollectionParamUpAchieveDataListRequest` | `/api/collection/get_collection_param_up_achieve_data_list` | `collectionParamUpAchieveDataList` |
| 7 | Collection data | `CollectionApiGetCollectionDataListRequest` | `/api/collection/get_collection_data_list` | `collectionDataList`, illust, field stage collection, magia link flag |
| 8 | Styles held | `StyleApiGetStyleDataListRequest` | `/api/style/get_style_data_list` | `styleDataList` |
| 9 | User param | `UserApiGetUserParamDataRequest` | `/api/user/get_user_param_data` | `userParamData` |
| 10 | Game config | `ConfigApiGetConfigRequest` | `/api/config/get_config` | large nested config (§5) |
| 11 | Options | `UserApiLoadOptionRequest` | `/api/user/load_option` | client options including default party ids |
| 12 | Web pay | `WebPayApiCancelLatestRequest` | `/api/web_pay/cancel_latest` | `result` |
| 13 | Terms | `TermsApiGetUpdatedTermsRequest(storeType=2)` | `/api/terms/get_updated_terms` | `needAgree`, `termsList` |

After step 0b, `sessionmgr` assigns:

```text
self._container.sessionId = resp.sessionId
self._container.userId = resp.userId
self._container.uuid = self.uuid  # from SDK
```

`finally` of `_ensure_token`: `await self.sdk.invoke_post_login()`.

### 2.1 LoginApiLoginRequest body fields (source)

From `requests.py` `LoginApiLoginRequest` (plus `RequestBase`: `lastHomeAccessTime`, `sm` via `prepare()`):

| Field | Typical tool value |
|-------|-------------------|
| appVersion | `version_info.version` |
| urlParam | `None` |
| deviceModel | `"Asus ASUS_I003DD"` |
| osType | `2` |
| osVersion | Android 9 string |
| storeType | `2` |
| graphicsDeviceId / VendorId | `0` |
| processorCount | `4` |
| processorType | x86-64 SSE string |
| supportedRenderTargetCount | `8` |
| supports* flags | as in sessionmgr |
| supportsStencil | `1` |
| uuid | `None` in request body (container uuid set separately) |
| xuid | `0` |
| sm | injected by `RequestBase.prepare()` from fingerprint |

### 2.2 LoginApiLoginResponse fields

| Field | Type (model) | Use |
|-------|--------------|-----|
| sessionId | str | All subsequent envelopes |
| userId | int | Envelope userId |
| status | int | Account status |
| banType | int | Ban info |

---

## 3. UserApiGetInitDataListResponse (the rich bag)

Source: `responses.py` class `UserApiGetInitDataListResponse`.

| Field | Meaning (EN) | 含义（中文） | Product note |
|-------|--------------|--------------|--------------|
| **partyDataList** | All party formations | **全部编成队伍** | **C10 list source**; name / partyIndex / partyDataId |
| **styleDataList** | Owned styles | 持有风格/造型 | Wash / combat |
| **characterDataList** | Character progress | 角色数据 | |
| **cardDataList** | Cards | 卡牌 | |
| **itemDataList** | Items inventory | 道具 | |
| **characterBuildDataList** | Builds | 构筑数据 | Also refreshed at step 4 |
| **userParamData** | Nickname, level, stamina, … | 昵称等级体力等 | Info module / UI cards |
| **userData** | User record | 用户数据 | |
| **miniTutorialData** | Tutorial state | 教程状态 | |
| **styleRentalBorrowingDataList** | Rental borrows | 租借相关 | |

### 3.1 PartyPartyDataRecord fields (common.py)

| Field | Role |
|-------|------|
| userId | Owner user |
| **partyDataId** | Server primary key used in battle APIs |
| **name** | User-visible formation name |
| **partyType** | Formation purpose type (e.g. main quest type 1) |
| isQuest / isPvp / isExploration / isMapGve / isScoreAttack / isMultiRaid / isSoloRaid | Flags |
| member1..5 | Style mst ids |
| cardMstId1..5 | Card mst ids |
| subStyleMstIds1..5 | Sub style strings |
| leaderStyleMstId | Leader |
| partyPower | Power score |
| **partyIndex** | Slot index (“which team number”) |

Handlers: `handlers.py` `UserApiGetInitDataListResponse.update` sets `datamgr.resp = self` so modules read `client.data.resp.partyDataList`.

### 3.2 UserUserParamDataRecord (high-value subset)

| Field | 中文 |
|-------|------|
| name | 昵称 |
| level / exp / totalExp | 等级经验 |
| stamina / staminaUpdatedTime | 体力 |
| money / totalMoney | 金币 |
| recoveryCount / gemRecoveryCount | 恢复次数 |
| pvpWin / pvpLose / pvpWinRate | PVP |
| tutorialStep | 教程步 |
| maxPartyPower | 最高队伍战力 |
| todayFriendMedalCount | 友情勋章 |

---

## 4. db.update master preload

Source: `db/database.py` `update`.

| Call | Purpose |
|------|---------|
| Resource master revision list | Build `_current_revision` map name→revision |
| Style mst list | `db.style_list` |
| Selection ability mst list | `db.selection_ability_list` (wash UI names) |
| Character mst list | `db.character_list` |
| Style figure mst list | `db.figure_list` |

Wash character dropdown = intersection of mst tables, **not** only owned styles (see WASH_CHARACTER_LIST / L3).

---

## 5. ConfigApiGetConfigResponse (nested configs)

Source: `responses.py` `ConfigApiGetConfigResponse` field names:

| Field | Domain |
|-------|--------|
| loginBonusConfig | Login bonus rules |
| characterConfig | Character |
| cardConfig | Card |
| collectionConfig | Collection |
| styleConfig | Style |
| pvpConfig | PVP |
| talismanConfig | Talisman |
| missionConfig | Mission |
| guildConfig | Guild |
| gveConfig | GVE |
| presentBoxConfig | Present box |
| userConfig | User / stamina limits etc. |
| questConfig | Quest |
| tutorialConfig | Tutorial |
| towerConfig | Tower |
| partyConfig | Party |
| subscriptionConfig | Shop subscription |
| storyEventConfig | Story event |
| chatConfig | Chat |
| firestoreConfig | Firestore |
| termsConfig | Terms |
| gatheringConfig | Gathering |
| gvgConfig | GVG |
| scoreAttackConfig | Score attack |
| isPreRelease | Flag |
| (+ multiRaidConfig appears on live objects used by raid modules — confirm in full response class if extended) | Raid LP limits |

Handlers: `ConfigApiGetConfigResponse.update` → `datamgr.config = self`.

Rust: stored as `GameClient.game_config` JSON `Value`.

---

## 6. UserApiLoadOptionResponse (options that encode default parties)

Selected fields from model (product-relevant):

| Field | Note |
|-------|------|
| questPartyDataId | Default quest party id |
| pvpPartyDataId | Default PVP party |
| characterHeartPartyDataId | Heart content party |
| battleAuto / battleSpeedType | Battle UX |
| sameCharaOnParty | Party rule |
| dungeonIsFast / dungeonIsManual | Dungeon UX |
| … | Full list in responses.py `UserApiLoadOptionResponse` |

---

## 7. Collection / style / character step responses

| Step | Response class | Main lists |
|------|----------------|------------|
| Collection param | `CollectionApiGetCollectionParamUpAchieveDataListResponse` | collectionParamUpAchieveDataList |
| Collection data | `CollectionApiGetCollectionDataListResponse` | collectionDataList, collectionIllustAchieveDataList, fieldStageCollectionInfoList |
| Style | `StyleApiGetStyleDataListResponse` | styleDataList |
| Character | (CharacterApiGetCharacterListResponse — see responses.py) | character list |

Collection handler builds `datamgr.collection` map keyed by `(objectType, objectId)`.

---

## 8. datamgr.generate_battle_log

Source: `datamgr.py`.

Produces JSON string:

```json
{
  "Commands": [],
  "ResultBattleUnits": [ { "serializeBattleParameter": {...}, "Id": ..., "SkillSet": {...} } ],
  "ResultRound": 1
}
```

Used when finalizing stages (sweep / super_sweep). Rust: `GameClient::battle_log_from_units`.

---

## 9. bootstrap.create_new (tool auto_register)

Source: `bootstrap.py`.

```text
sdk = client_type(None)
await sdk.register('12345678')   # fixed password in create_new path used by module
client = pcrclient(sdk)
await client.login()
await client.clear_tutorial()
return client
```

Not part of normal sessionmgr chain; used by `auto_register` module.

---

## 10. Rust mapping

| Python | Rust |
|--------|------|
| sessionmgr full chain | `GameClient::full_login` |
| light path (product) | `login_for_info` — may skip full mst/init |
| datamgr.resp | `init_data` Value |
| datamgr.config | `game_config` Value |
| db mst lists | `MstCache` / `bootstrap_mst` |
| partyDataList | `init_data["partyDataList"]` · `resolve_party` |

---

## 11. Product implications (底层 ≫ 前端)

| Data already on wire after full login | Typical UI today | Opportunity |
|--------------------------------------|------------------|-------------|
| partyDataList names/indices/ids | Free-text team fields | **C10** select list |
| userParam stamina/level/name | Partial on cards | Always-fresh header |
| styleDataList / mst | Wash dropdown only | More pickers |
| load_option default party ids | Unused in SPA | Prefill team mode |
| config nested limits | Hardcoded assumptions | Show remaining recoveries |

---

## 12. Revision

| Date | Content |
|------|---------|
| 2026-08-07 06:04 | DOC-FULL-01: full login chain + payload field tables from source |
