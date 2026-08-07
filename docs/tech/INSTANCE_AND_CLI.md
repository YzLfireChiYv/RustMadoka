# 技术说明：Owner/Client 实例 · 本机 IPC · 游戏账号任务门闩 · 端口

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-08（心跳场景钉死 + 数据夹布局链） |
| **任务书** | [PLAN_INSTANCE_CLI_PORT.md](../PLAN_INSTANCE_CLI_PORT.md) · [PLAN_AUDIT…§4](../PLAN_AUDIT_COMMS_OCCUPANCY_PROGRESS.md) |
| **数据夹布局** | [DATA_FOLDER_LAYOUT.md](./DATA_FOLDER_LAYOUT.md) · NORMS **P32** · `data_layout.rs` |
| **源码** | `owner_lock.rs` · `occupancy.rs` · `data_layout.rs` · `task_gate.rs` · `ipc.rs` · `lib.rs` · `http_server.rs` · `main.rs` |
| **审核/落地 log** | [owner-port-audit](../logs/2026-08-07-owner-port-audit.md) · [batch-owner-port-partial](../logs/2026-08-07-batch-owner-port-partial.md) · [session-autonomous-batch](../logs/2026-08-07-session-autonomous-batch.md) |
| **MAY CONTAIN ERRORS** | Yes — 以当前源码为准 |

---

## 1. 两层约束（**已实现** · 同机）

```text
层1  数据目录 Owner 锁（跨进程，owner.lock 文件独占锁 fs2）
     → 同一 RustMadoka_data 同时仅一个主人进程
     → 进程正常退出 / 崩溃：OS 释放锁（无需心跳）

层2  游戏账号任务表（进程内内存，channel + 引继码）
     → 同一真实游戏号同时仅一个「打官方服」任务
```

层 2 **不**用 data 落盘忙闲（免延迟与脏锁）；进程退出表自然空。

### 1.1 同机 vs 云盘（主人口径 · 2026-08-07 复核）

| 场景 | 机制 | 实现状态 |
|------|------|----------|
| **同一台电脑、同一 data 路径** | `owner.lock` **独占文件锁**；第二进程无条件退出 | **CODE**（`owner_lock.rs`） |
| **同一台电脑、不同 data 目录** | 允许多 Owner；各写各 `app.json` 端口 | **CODE** |
| **跨路径 / 云盘同步夹** | 独立文件 `occupancy_heartbeat.json`（**不写进 owner.lock**）；时间 + **数据文件夹路径**；约 1 分钟刷新；不同路径且 30 分钟内 active → 默认拒绝；程序运行面板终端输入 **`我已知晓`** 可强制 | **CODE**（`occupancy.rs`） |

心跳是无奈二次保险，**不得**替代同机 `owner.lock`。

### 1.2 「心跳」用于什么场景（检测理解 · 完整条件）

| 问 | 答 |
|----|-----|
| **用于什么** | **同一份数据文件夹**被放在**云同步盘 / 复制到另一绝对路径 / 另一台电脑**上时，防止你不知情地双开写坏账号配置。文件：`occupancy_heartbeat.json`。 |
| **不用于什么** | **不是**游戏服会话 keep-alive；**不是**同机双开的主防护（同机同路径用 `owner.lock` 文件锁）；**不是**浏览器网页前端的 UI 心跳。 |
| **为什么存在** | 主人钉死：同机可用高效文件锁；心跳复杂有漏洞，**只针对跨设备/跨路径同夹**，是无奈之举（见 OWNER 历史 AUD-HEART · PLAN_AUDIT §4）。 |
| **行为摘要** | 运行中约每 1 分钟刷新「时间 + 数据路径 + exe 路径」；正常退出写 idle；**同路径**再开立即允许（断电重启自愈）；**不同路径**且约 30 分钟内 active → 默认拒绝；在**程序运行面板终端**输入完整 **`我已知晓`** 可强制（单独 y 无效）。 |
| **交互位置** | 强制确认在黑色**程序运行面板终端**，不在浏览器网页前端（G9）。 |

---

## 2. 运行态

| 态 | 持 Owner 锁 | 行为 |
|----|-------------|------|
| **Owner** | 是 | Web（HTTP 127.0.0.1）+ 运行面板 + IPC 服务 + TaskGate |
| **Client** | 否 | 读 data 路径 → 连本机命名管道 → 投递 JSON 命令 → 打印结果 → 退出 |

Client **知道** data 路径，但**不绑定**为目录主人。

### 2.1 CLI `run` 与 Owner 拉起（**主人产品设计 · 真源**）

| 入口 | 无 Owner 时行为 | 有 Owner 时行为 |
|------|-----------------|-----------------|
| **`serve`** | 本进程成为 Owner：程序运行面板终端 + HTTP，长期运行 | 第二进程应被 Owner 锁拒绝 |
| **`run info` / `run daily` / `run module`…** | **本进程升级为 Owner**：执行本次命令后 **继续 serve 保持运行**（CLI 拉起程序） | **命名管道 IPC** 投递到已有 Owner；Client 打印结果后退出 |
| **`run` + `--wire`（开发版录制）** | 本机独占执行后退出（不抢长期 serve 形态） | 若需录制在本机 CLI 侧，仍可 `--wire` 独占 |

**说明：**

1. 「`command done; panel serves`」**不是故障**：表示本次 CLI 命令已完成，程序本体继续开着，供浏览器与后续 CLI IPC。  
2. 2026-08-07 AI 曾误把该行为改成「无 Owner 则执行后退出」——**违反本设计，已恢复**。  
3. 进程内 **SessionPool**（游戏 Login 复用）建在 Owner 上，与「拉起后保持」一致；多次 CLI 经 IPC 打同一 Owner 才共享游戏会话。  
4. AI 冒烟若要「命令结束进程也结束」：先 `serve` 再 IPC Client，或使用测试专用路径，**不要**再改掉 CLI 拉起。

Outbound：`crates/rustmadoka-app/src/lib.rs` → `cmd_run` · `ipc.rs`（含 `RunModule`）。

---

## 3. 端口（程序运行面板终端）

| 优先级 | 来源 |
|--------|------|
| 1 | CLI 显式 `--port` |
| 2 | 数据文件夹 `app.json` → `listen_port` |
| 3 | 默认 `14103`（文件夹里可以没有端口字段） |

| 步骤 | 行为 |
|------|------|
| 绑定成功 | 写入 `app.json`；非默认端口时在程序运行面板终端醒目提示；浏览器网页前端只读显示 |
| 绑定失败 | 说明占用 → 要求完整句 **`我知道端口被占用`** → 再让用户**自己输入**端口数字（程序不代填）→ 成功则持久保存 |
| 同数据文件夹第二进程 | **Owner 锁**处退出，到不了换端口 |

## 3.1 普通版 / 开发版 CLI

| 规则 | 实现 |
|------|------|
| 禁止自动 taskkill 已运行实例 | **CODE** |
| 开发版独占能力（wire 等）抢锁失败 | 中文说明：请手动关程序运行面板终端，再开开发版 |
| 开发版普通命令走 IPC | 提示命令已交给已运行实例 |

---

## 4. IPC 协议（Windows 命名管道）

- 管道名：`\\.\pipe\automadoka_<data目录路径短哈希>`  
- 一行请求 JSON + 一行响应 JSON（UTF-8）  
- 不走系统 HTTP 代理；与 Web 的 HTTP 分离  

请求示例：

```json
{"cmd":"run_info","group":"明文组","alias":"日服主号","group_password":null}
```

---

## 5. TaskGate

- 键：`{channel_lower}:{migration_code}`  
- `try_begin` / `try_begin_owned` / `Drop` 释放  
- `try_begin_many`：组队 Raid 一次占用多号  
- 登记字段：`task` + **`owner_group`（发起用户组）**  
- **停止权：** 仅 `owner_group` 可停止该账号当前任务（`may_stop`）；其它用户组只读占用态  
- 冲突：返回错误文案  

打服任务：info、daily、module、wash、**组队 Raid**。  
不占锁：group/account 本地列表、通知历史、配置自动保存（纯本地）。  

规格：`docs/tech/GROUP_RAID_AND_DEVICE_IDENTITY.md` §1.2 · 源码 `task_gate.rs`

---

## 6. Inbound / Outbound

- Outbound：PLAN_INSTANCE_CLI_PORT · NORMS · HANDOFF  
- Inbound：`lib.rs` / `owner_lock.rs` 头注释 · crates/README · HANDOFF P16  
- 双版本：两 exe 共用 data 时仍同一 Owner 锁；与是否 `wire_record` 无关
