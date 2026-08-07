# Upstream official repository file map (cc004/automadoka)

| Item | Content |
|------|---------|
| **Wall clock** | 2026-08-07 06:04 |
| **Authority** | git `origin/main` @ `9826135` (`9826135ca7e87b226486c2bc84821610b9f1d996`) |
| **Remote** | https://github.com/cc004/automadoka |
| **Local clones** | `ref-legacy-superset/upstream-ref/cc004-automadoka/` · frozen `archive/pre-rust-2026-08/` |
| **Path count** | **81** (excluding `.gitkeep`) |
| **Inbound** | [DOC_COVERAGE_AUDIT.md](./DOC_COVERAGE_AUDIT.md) · [UPSTREAM_SOURCE_AND_WIRE.md](./UPSTREAM_SOURCE_AND_WIRE.md) · [README.md](./README.md) |
| **MAY CONTAIN ERRORS** | Yes — re-run `git ls-tree origin/main` if hash moves |

## 0. 如何阅读 / How to read

本表穷尽官方 GitHub 树每一个路径（对照 `git ls-tree -r --name-only origin/main`）。

| Status | Meaning (EN) | 含义（中文） |
|--------|--------------|--------------|
| **FULL** | Construction-grade doc exists | 有可施工对照文档 |
| **PARTIAL** | Mentioned, not exhaustive | 有提及、未写尽 |
| **NONE** | Business-related, no dedicated chapter | 业务相关但无专章 |
| **N/A** | Engineering/license; no protocol chapter needed | 工程/许可证等无需协议专章 |

阅读顺序：本表 → [UPSTREAM_SOURCE_AND_WIRE.md](./UPSTREAM_SOURCE_AND_WIRE.md) → [API_INVENTORY.md](./API_INVENTORY.md) → [INIT_AND_RESPONSE_PAYLOADS.md](./INIT_AND_RESPONSE_PAYLOADS.md) → [PROTOCOL_STACK.md](./PROTOCOL_STACK.md) / [SDK_AND_LOGIN.md](./SDK_AND_LOGIN.md)。

## 1. Complete path table / 完整路径表

| # | Path (origin/main) | Role (EN) | 职责（中文） | Status | Primary tech doc |
|---|--------------------|-----------|--------------|--------|------------------|
| 1 | `.dockerignore` | Docker ignore | Docker 忽略规则 | **N/A** | — |
| 2 | `.github/workflows/build_docker_image.yml` | CI docker build | CI 构建 Docker | **N/A** | — |
| 3 | `.gitignore` | Git ignore | Git 忽略规则 | **N/A** | — |
| 4 | `.pylintrc` | Pylint config | Pylint 配置 | **N/A** | — |
| 5 | `.vscode/launch.json` | VS Code launch | VS Code 启动配置 | **N/A** | — |
| 6 | `Dockerfile` | Docker image build | Docker 镜像构建 | **PARTIAL** | RUST_REWRITE_DEPENDENCY_FEASIBILITY |
| 7 | `LICENSE` | License | 许可证 | **N/A** | — |
| 8 | `README.md` | Project README | 官方项目说明 | **N/A** | — |
| 9 | `__init__.py` | Root package marker | 根包标记 | **N/A** | — |
| 10 | `_db_test.py` | DB/mst test entry | 数据库/mst 测试入口 | **NONE** | — |
| 11 | `_download_web.py` | Web asset download helper | Web 资源下载辅助 | **NONE** | — |
| 12 | `_httpserver_test.py` | HTTP server test | HTTP 服务测试 | **NONE** | — |
| 13 | `_raid_runner.py` | Raid runner entry | 团战 runner 入口 | **PARTIAL** | RESEARCH · raid |
| 14 | `_reward_test.py` | Reward test | 奖励测试 | **NONE** | — |
| 15 | `_start_both.py` | Start helper | 启动辅助 | **NONE** | — |
| 16 | `_tw_test.py` | Taiwan channel test | 台服渠道测试 | **PARTIAL** | SDK_AND_LOGIN |
| 17 | `_us_test.py` | Global channel test | 国际服测试 | **PARTIAL** | SDK_AND_LOGIN |
| 18 | `_version_update.py` | Version update test | 指纹更新测试 | **PARTIAL** | VERSION_FINGERPRINT |
| 19 | `autopcr/constants.py` | Ports, paths, channel names, default headers, rate limits | 端口、路径、渠道名、默认 HTTP 头、限频常量 | **PARTIAL** | CHANNELS · UPSTREAM_SOURCE_AND_WIRE |
| 20 | `autopcr/core/__init__.py` | core package | core 包 | **N/A** | — |
| 21 | `autopcr/core/apiclient.py` | Game API HTTP: pack, sign, status, lock | 游戏 API：加密封包、签名、状态码、串行锁 | **FULL** | PROTOCOL_STACK · UPSTREAM_SOURCE_AND_WIRE |
| 22 | `autopcr/core/base.py` | Container onion Component/RequestHandler | Container 洋葱组件模型 | **FULL** | PROTOCOL_STACK |
| 23 | `autopcr/core/bootstrap.py` | create_client / create_new | 创建客户端/注册新号 | **PARTIAL** | INIT_AND_RESPONSE_PAYLOADS · tool auto_register |
| 24 | `autopcr/core/crypto.py` | AES msgpack pack, PKLB key, ApiCrypto.sign | AES/msgpack、密钥派生、API 签名 | **FULL** | PROTOCOL_STACK · LESSONS L2 |
| 25 | `autopcr/core/datamgr.py` | In-memory state; battle_log; response.update | 内存态、battleLog、响应回写 | **FULL** | DATA_AND_MST · INIT_AND_RESPONSE_PAYLOADS |
| 26 | `autopcr/core/misc.py` | errorhandler retries; mutexhandler | 错误重试与互斥组件 | **PARTIAL** | PROTOCOL_STACK |
| 27 | `autopcr/core/pcrclient.py` | Assembles pipeline; config keys; tutorial helpers | 组装管道、配置键、教程辅助 | **PARTIAL** | OVERVIEW · MODULES_RUNTIME |
| 28 | `autopcr/core/sdkclient.py` | Abstract SDK interface | SDK 抽象接口 | **FULL** | SDK_AND_LOGIN · CHANNELS |
| 29 | `autopcr/core/sessionmgr.py` | SDK + LoginApi + init chain | SDK 登录与游戏初始化串 | **FULL** | SDK_AND_LOGIN · INIT_AND_RESPONSE_PAYLOADS |
| 30 | `autopcr/core/version.py` | version/sign/libcount/sm; APKPure update | 指纹三元组与 sm；默认 APKPure 更新 | **FULL** | VERSION_FINGERPRINT |
| 31 | `autopcr/db/database.py` | mst revision cache; login preload | mst revision 缓存与登录预拉 | **FULL** | DATA_AND_MST · INIT_AND_RESPONSE_PAYLOADS |
| 32 | `autopcr/http_server/httpserver.py` | Quart /daily Web + JSON API | Quart 本机 Web 与 API | **PARTIAL** | HTTP_SERVER |
| 33 | `autopcr/http_server/httpserver_test.py` | HTTP server tests | HTTP 服务测试 | **NONE** | — |
| 34 | `autopcr/model/.gitignore` | model gitignore | model 目录忽略 | **N/A** | — |
| 35 | `autopcr/model/common.py` | Shared record types (party, item, ...) | 共享记录类型（队伍、道具等） | **PARTIAL** | PARTY_TEAM_RESOLVE · INIT_AND_RESPONSE_PAYLOADS |
| 36 | `autopcr/model/enums.py` | Game enums | 游戏枚举 | **NONE** | DOC-FULL-02 |
| 37 | `autopcr/model/error.py` | Model error helpers | 模型错误辅助 | **NONE** | — |
| 38 | `autopcr/model/handlers.py` | Response.update hooks into datamgr | 响应 update 挂到 datamgr | **FULL** | INIT_AND_RESPONSE_PAYLOADS · DATA_AND_MST |
| 39 | `autopcr/model/modelbase.py` | RequestBase.prepare sm; envelope types | prepare 注入 sm；外层 envelope 类型 | **FULL** | PROTOCOL_STACK |
| 40 | `autopcr/model/models.py` | Model re-exports | 模型再导出 | **PARTIAL** | — |
| 41 | `autopcr/model/requests.py` | All Request classes + url (~494 paths) | 全部请求类与 url（约 494 路径） | **FULL** | API_INVENTORY |
| 42 | `autopcr/model/resourcemodels.py` | Resource-related models | 资源相关模型 | **NONE** | DOC-FULL-02 |
| 43 | `autopcr/model/responses.py` | All response payload field models | 全部响应 payload 字段模型 | **FULL** | INIT_AND_RESPONSE_PAYLOADS · API_INVENTORY |
| 44 | `autopcr/module/accountmgr.py` | Tool users + game accounts CRUD | 工具用户与游戏角色 CRUD | **PARTIAL** | HTTP_SERVER · RESEARCH |
| 45 | `autopcr/module/config.py` | Config field types / UI schema decorators | 配置字段类型与 UI schema 装饰器 | **PARTIAL** | MODULES_RUNTIME |
| 46 | `autopcr/module/crons.py` | Cron scheduler loop | 定时任务调度循环 | **PARTIAL** | RESEARCH |
| 47 | `autopcr/module/modulebase.py` | Module base, results, tags | 模块基类、结果、标签 | **PARTIAL** | MODULES_RUNTIME |
| 48 | `autopcr/module/modulelistmgr.py` | Module list manager tabs | 模块页签管理 | **PARTIAL** | MODULES_RUNTIME |
| 49 | `autopcr/module/modulemgr.py` | do_daily / do_task / do_from_key | 日常/单模块调度 | **PARTIAL** | MODULES_RUNTIME · PHASE_R2 |
| 50 | `autopcr/module/modules/__init__.py` | daily/tool/cron registration order | 日常/工具/定时注册表与顺序 | **FULL** | MODULES · PHASE_R2 |
| 51 | `autopcr/module/modules/collection.py` | Module file collection.py | 业务模块：活动剧情/光之间 | **PARTIAL** | PHASE_R2 · MODULES |
| 52 | `autopcr/module/modules/common.py` | Module file common.py | 业务模块：登录奖励/玩家信息 | **PARTIAL** | PHASE_R2 · MODULES |
| 53 | `autopcr/module/modules/cron.py` | Module file cron.py | 业务模块：定时槽 cron1-6 | **PARTIAL** | PHASE_R2 · MODULES |
| 54 | `autopcr/module/modules/gacha.py` | Module file gacha.py | 业务模块：免费扭蛋 | **PARTIAL** | PHASE_R2 · MODULES |
| 55 | `autopcr/module/modules/raid.py` | Module file raid.py | 业务模块：魔女系列与救世 | **PARTIAL** | PHASE_R2 · MODULES |
| 56 | `autopcr/module/modules/shop.py` | Module file shop.py | 业务模块：兑换商店 | **PARTIAL** | PHASE_R2 · MODULES |
| 57 | `autopcr/module/modules/stamina.py` | Module file stamina.py | 业务模块：买体力/智能扫荡 | **PARTIAL** | PHASE_R2 · MODULES |
| 58 | `autopcr/module/modules/sweep.py` | Module file sweep.py | 业务模块：各类扫荡/任务/礼物 | **PARTIAL** | PHASE_R2 · MODULES |
| 59 | `autopcr/module/modules/tool.py` | Module file tool.py | 业务模块：刷图/secret/注册/迷宫事件 | **PARTIAL** | PHASE_R2 · MODULES |
| 60 | `autopcr/module/modules/wash.py` | Module file wash.py | 业务模块：快速洗词条 | **PARTIAL** | PHASE_R2 · MODULES |
| 61 | `autopcr/sdk/greeclient.py` | Gree OAuth register/migrate/authorize | Gree 注册/引继/授权与 OAuth 签名 | **FULL** | SDK_AND_LOGIN · UPSTREAM_SOURCE_AND_WIRE · L1 |
| 62 | `autopcr/sdk/sdkclients.py` | Channel factory BSDK/QSDK/RSA/Sonet | 渠道工厂与 apiroot/签名绑定 | **FULL** | SDK_AND_LOGIN · CHANNELS |
| 63 | `autopcr/sdk/sonetclient.py` | Taiwan Sonet SDK | 台服 Sonet SDK | **FULL** | SDK_AND_LOGIN · UPSTREAM_SOURCE_AND_WIRE |
| 64 | `autopcr/util/aiorequests.py` | Async HTTP wrapper | 异步 HTTP 封装 | **PARTIAL** | PROTOCOL_STACK |
| 65 | `autopcr/util/calculator.py` | Calc helpers for modules | 模块计算辅助 | **NONE** | DOC-FULL-02 |
| 66 | `autopcr/util/draw.py` | Image draw helpers | 绘图辅助 | **NONE** | — |
| 67 | `autopcr/util/draw_table.py` | Table draw | 表格绘图 | **NONE** | — |
| 68 | `autopcr/util/freqlimiter.py` | Rate limiter decorator | 限频装饰器 | **PARTIAL** | PROTOCOL_STACK · constants |
| 69 | `autopcr/util/ilp_solver.py` | ILP solver | 整数规划求解 | **NONE** | DOC-FULL-02 |
| 70 | `autopcr/util/linq.py` | LINQ-like helpers | LINQ 风格辅助 | **NONE** | — |
| 71 | `autopcr/util/logger.py` | Logging singleton | 日志单例 | **NONE** | — |
| 72 | `autopcr/util/statistics.py` | Stats helpers | 统计辅助 | **NONE** | — |
| 73 | `autopcr/util/streamzip.py` | Streaming zip for remote XAPK | 远程 XAPK 流式 zip | **PARTIAL** | VERSION_FINGERPRINT |
| 74 | `autopcr/util/type_utils.py` | Type utilities for response parse | 响应解析类型工具 | **PARTIAL** | PROTOCOL_STACK |
| 75 | `"data/\345\276\256\350\275\257\351\233\205\351\273\221.ttf"` | Font asset | 字体资源 | **N/A** | — |
| 76 | `docker-compose.yaml` | Compose stack | Compose 编排 | **PARTIAL** | — |
| 77 | `login_test.py` | Manual login test entry | 手动登录测试入口 | **NONE** | DOC-FULL-02 |
| 78 | `raid/raidrunner.py` | Multi-account raid farm queue | 多开团战农场队列 | **PARTIAL** | RESEARCH |
| 79 | `raid/raidworker.py` | Secondary client for raid assist | 团战小号辅助客户端 | **PARTIAL** | RESEARCH · raid_support |
| 80 | `requirements.txt` | Python dependencies | Python 依赖清单 | **PARTIAL** | RUST_REWRITE_DEPENDENCY_FEASIBILITY |
| 81 | `start.bat` | Windows start helper | Windows 启动脚本 | **N/A** | — |

## 2. Status counts / 状态统计

- **FULL**: 16
- **PARTIAL**: 36
- **NONE**: 17
- **N/A**: 12

## 3. Archive vs origin / 归档差异

| Item | Note |
|------|------|
| `archive/.../util/probe_capture.py` | **Not** on origin; local analysis probe only |
| `ClientApp/` SPA assets | **Not** in origin git; present in some run trees only |
| Protocol truth | Prefer `origin/main` @ hash above or `upstream-ref/cc004-automadoka` |

## 4. Revision

| Date | Content |
|------|---------|
| 2026-08-07 06:04 | DOC-FULL-01: full 81-path map vs origin/main |
