# 技术规格：CLI ⊇ 浏览器网页前端（能力超集）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-08（P23 CLI 补齐批） |
| **规范** | NORMS **P23**（2026-08-07 升级）· **G11** · **P5** · **P31** |
| **失真声明** | **AI 维护，有可能出错和失真。** 以当前 clap 子命令与 `http_server` 路由为准。 |
| **Inbound** | 主人 2026-08-07：网页全部功能 CLI 须能做；CLI 只许更多不许更少 |
| **Outbound** | `crates/rustmadoka-app/src/lib.rs`（CLI）· `http_server.rs`（HTTP）· `run_ops.rs` · `ipc.rs` · `session_pool.rs` |

---

## 1. 规则（完整条件）

1. **浏览器网页前端**能完成的产品操作，必须有 **CLI** 对等路径（打印结果 / 退出码 / JSON 可脚本化）。  
2. CLI **可以**额外提供网页没有的能力（`--wire`、批量、导出格式、静默开关等）。  
3. CLI **不可以**长期少于网页：新网页功能 = 同批 CLI，或 TASK 写明同迭代必补。  
4. 实现优先复用 `run_ops` / Owner IPC / 同一 `GameClient` 会话池，**禁止**为 CLI 另写一套协议。  
5. 「有 HTTP API」≠「有 CLI」：必须落到 `RustMadoka.exe <subcommand>` 可调用。

---

## 2. 架构难度（诚实）

| 判断 | 说明 |
|------|------|
| **总体** | **中等偏易，不是架构级重写** |
| 为何不难 | 业务在 `rustmadoka-core` + `run_ops`；网页只是 HTTP + SPA；Owner/IPC 已是 CLI 附着点 |
| 主要工作量 | （1）盘点 HTTP/SPA 功能表；（2）为缺口补 clap 子命令；（3）需要时扩 IPC；（4）完成定义与 log |
| 较难点 | 强交互 UI（多步向导、拖拽监视、加密组验密体验）→ CLI 用参数/子命令/二次确认句表达，不是 1:1 复制控件 |
| 与拉起 Owner | CLI 无 Owner 拉起并保持、有 Owner 走 IPC — **产品设计**；会话池在 Owner 内复用登录 |

---

## 3. 现状对照（抽样 · 非穷尽 · 2026-08-08）

### 3.1 已有较完整 CLI 路径（可脚本）

| 能力 | CLI 大致入口 |
|------|----------------|
| 启动 Owner / 附着 | `serve` · `run *` 拉起或 IPC |
| 用户组 list/create | `group …` |
| 账号增删改查配置 | `account …` |
| 登录信息 | `run info` |
| 清日常 | `run daily` |
| 单模块 | `run module --key …`（含 `clear_dungeon_event`） |
| 组队 | `run group-raid …`（IPC 面仍可扩） |
| 会话导出 | `export session` |
| 拉指纹 | `fetch-fp` |
| 指纹槽 | `fp slots` · `fp refresh` · `fp activate` · `fp reset` |
| 任务日志 | `task-log list` · `show` · `progress` · `clear-older` |
| 运行控制 | `control pause` · `resume` · `abort` · `status`（须 Owner） |
| 设置通知历史 | `notify settings` |
| 系统 toast 配置 | `notify system-get` · `system-set` · `system-test` |
| 关卡 ID↔名称 | `mst quest-stages` · `mst quest-lookup`（可 `--from-cache`） |

### 3.2 网页有、CLI 仍弱或仅网页体验的（诚实）

| 能力（网页侧） | 缺口说明 |
|----------------|----------|
| 多用户组主页监视流 / 跨组文案 | 主页完整流为网页产品面；CLI 有 `control status` 摘要 |
| 路由/加密组验密体验 | 仅网页（CLI 用 `--group-password`） |
| 洗词条完整参数面 | 继续盘点 HTTP wash 与 CLI 参数是否全对等 |
| 指纹自定义槽填入 | `fp fill` 已有（自定义槽 JSON） |

**完成定义（本规格）：** 缺口项有 CLI + `--help` 中文；无点测不写 FIXED。

---

## 4. 相关产品能力状态（主人点名）

### 4.1 exe 内嵌指纹 + GitHub 热更 + 可随 exe 拷贝

| 条件（P29） | 代码侧状态（诚实） |
|-------------|-------------------|
| 编译期内嵌 `publish/automadoka.json` | **有**（`EMBEDDED_COMBINED_JSON` · `fp_slots` 内置槽） |
| 运行时 rules raw 拉取，写入**数据文件夹**槽，不改写 exe | **有**（`refresh_default_source` · `default_pulled`） |
| 刷新后登录**实际用**新槽 | **有方向 CODE**（自动启用拉取槽 · `fp_load` 槽优先） |
| 静默日检（Owner 启动每天最多一次） | **有** |
| UI 展示日/国版本与上次刷新 | **网页/API 有**；**CLI `fp slots/refresh` CODE**（非 FIXED） |
| 「新版指纹随 exe 拷走」 | **内嵌随 exe**；**拉取槽在 data 旁路**，只拷 exe 不拷 data 时只有内嵌保底——**符合「exe 可单独拷」+ data 可另拷** 模型 |

**不得**写「主人已点测 FIXED」除非点测过。细节见 [VERSION_FINGERPRINT.md](./VERSION_FINGERPRINT.md)。

### 4.2 Windows 系统通知（右下角系统 toast）

| 项 | 状态 |
|----|------|
| 产品意图 | 允许 exe 弹 **Windows 系统通知**；在**数据文件夹**配置；**默认关** |
| 当前实现 | **CODE**：`system_toast.rs` + `notifications/system_toast.json` 默认 `enabled=false`；任务定稿可弹；CLI `notify system-*` |
| 与「网页 toast」 | 不同产品面；系统 toast 默认关 |

---

## 5. 修订

| 墙钟 | 内容 |
|------|------|
| 2026-08-07 23:53 | 首版：P23 升级、难度、缺口表、指纹与系统通知状态 |
