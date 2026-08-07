# 技术说明：浏览器路由 · 任务进度/日志 · 暂停 · 二次确认

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-07 20:15（监视流章节增补） |
| **任务书** | [PLAN_UI_ROUTING_LOGS.md](../PLAN_UI_ROUTING_LOGS.md) · [MULTI_GROUP_UI_AND_MONITOR_SPEC.md](./MULTI_GROUP_UI_AND_MONITOR_SPEC.md) |
| **源码** | `crates/rustmadoka-app/static/index.html` · `http_server.rs` · `task_log.rs` · `run_control.rs` |
| **MAY CONTAIN ERRORS** | Yes — AI 维护；以源码为准 |

---

## 0. 运行监视三面（完整条件 · 2026-08-07）

| 产品面 | 数据 | 用途 |
|--------|------|------|
| **设置页进度条** | `RunStatusSnapshot`（round/total/message） | 不懂技术用户；按**游戏身份**跨组同步 |
| **浏览器网页前端 · 用户组主页运行面板** | `GET /api/run/status?group=` → `stream_lines` | 该用户组完整过程流；可关；**不是**进度条复读 |
| **程序运行面板终端** | 同源 `RunHub` 流，按 `stream_lines_after(seq)` 增量 `eprintln!("[流] …")` + `[监视·摘要]` | 高阶只读；非异常不可控制；叉掉关闭 |

实现要点：

- `RunHub::update_progress` / `begin` / `end_with_report` 同时写快照与流缓冲（上限 8000 行）。  
- 主页 `filter_group` 隔离他组流（MULTI_GROUP §4.1）。  
- 终端轮询约 400ms 拉新 seq；正常态不读 stdin 做任务控制。  

Outbound：`run_control.rs` · `http_server.rs` serve 启动块 · `static/index.html` `#homeConsole`。

---

## 1. 路由

- Axum：`/` 与 `/*path` 均返回同一 SPA `index.html`（`include_str!`，**改 HTML 必须 release 覆盖根目录 exe**）。  
- 前端 `pathname` 分段 `decodeURIComponent`：  
  - `[]` → login  
  - `[group]` → cards  
  - `[group, "检测"]` → diagnose  
  - `[group, alias]` → settings  
  - `[group, alias, "工具"|"日志"|…]` → tools / logs  

回退：组不存在→`/`；别名不存在→`/{group}`；加密未会话→login。

### 1.1 SPA 纪律（硬 · 2026-08-06 C13）

| 规则 | 原因 |
|------|------|
| 单文件 `static/index.html` 承载全部 UI/JS | 半截编辑极易 **语法错误 → 整页死**（界面永久「加载中」） |
| 改完必须：优先 `scripts/build-win-dual.ps1`（或 release 覆盖 `RustMadoka.exe` / `automadoka.exe`）→ **Ctrl+F5** | HTML 嵌进 exe；浏览器还缓存旧页 |
| 冒烟：**打开登录页** 须出现「暂无用户组」或组列表，**禁止**停在「加载中…」 | API 通 ≠ SPA 活 |
| `init` 分步 try；列表加载失败写红字，不空转 | 见 LESSONS_SESSION_COLLAB **C13** |
| 手机端：`@media (max-width:820px),(pointer:coarse)` + `html.platform-android` | 大字体/大点按/窄边框色；**逻辑仍同一 SPA**；Android 壳注入见 [ANDROID_DUAL_PLATFORM.md](./ANDROID_DUAL_PLATFORM.md) §4 |
| 改完 HTML 后 Android 须重编 `rustmadoka-mobile` 再装 APK | SPA 经 `include_str!` 打进 `.so`，不是 assets 热替换 |
| 模块行：上行标题、下行三键 | 防手机把中文挤成竖排（2026-08-07） |
| 折叠双态：`.btn-fold` / `.is-open` | 展开与收起视觉不同 |
| 日志筛选：`.log-filters` 两列 + `.check-line` | 勾选与文案水平对齐；可再改 |
| 完整日志可折 | `openLog` / `toggleLogDetail` |
| 多窗口设置同步 | `config/auto` 回完整 config；设置页 1.5s 轮询；本地 pending 时不覆盖 |
| 系统浏览器 | fetch 超时 + HTML no-store；须 App/exe 在运行 |

## 2. 任务日志落盘

```text
automadoka_data/task_logs/{safe_group}/{safe_alias}/
  index.json          # 会话摘要列表（新在前）
  {session_id}.json   # 定稿完整日志（仅 terminal 状态可对外「完整」）
  {session_id}.progress.json  # 运行中进度快照（实时写）
```

会话字段：`id, trigger, group, alias, status, started_at, finished_at, modules[], message`  
`trigger`: `one_click_daily` | `single_module` | `cli` | `scheduled`  

`status`: `running` | `paused` | `success` | `aborted` | `error`  
完整日志 API：仅 `status != running && != paused` 或显式 `finalized=true` 后返回全文；进行中只给 progress。

自动清理：`app.json` 或 per-account config `log_auto_clean` + `log_keep_one_click=100`。

## 3. 运行控制

`RunControl`（Owner 内）：

- 当前任务：`session_id, account_key, kind, pause_flag, abort_flag`  
- 进度回调写 `.progress.json`  
- 暂停：置位，调度循环协作检查  
- 放弃：abort + finalize 日志  

与 `TaskGate`（引继互斥）配合。

## 4. API（增量）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/run/status` | 当前进度/忙线（可带 group/alias 过滤） |
| POST | `/api/run/pause` | 紧急暂停 |
| POST | `/api/run/resume` | 继续 |
| POST | `/api/run/abort` | 放弃 |
| POST | `/api/accounts/:alias/module/:key/run` | 单模块（stream 可选） |
| GET | `/api/accounts/:alias/task_logs` | 列表（trigger 筛选） |
| GET | `/api/accounts/:alias/task_logs/:id` | 完整日志（已定稿） |
| GET | `/api/accounts/:alias/task_logs/:id/progress` | 进行中进度 |
| DELETE | `/api/accounts/:alias/task_logs` | 清理 |
| POST | `/api/groups/password` | 改密（body: name, old_password, new_password；成功废该组会话） |
| POST | `/api/logout` | 注销当前 token |

## 5. 二次确认默认

配置键建议：`confirm_{module_key}` bool，默认 = `!low_risk(module)`。  
一键：`confirm_one_click_daily` 默认 true（因可能含非低风险，或按「任一启用非低风险则确认」）。

## 6. Inbound

HANDOFF · PLAN_UI_ROUTING_LOGS · MODULES（resource_heavy / 低风险列表）
