# 计划书：RustMadoka 全量重构（从零产品口径 · 分阶段落地）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-08（R0–R7 代码收口批回写） |
| **状态** | **PLAN · 主任务真源 · 代码侧部分收口**（见 §4 状态列；点测 FIXED 仍 WAIT） |
| **产品** | **RustMadoka**（全新产品口径；不是「给 automadoka 打补丁」） |
| **工作区路径** | 保持 `C:\GrokProject\automadoka` 文件夹名不变（主人钉死） |
| **规范** | NORMS G1/G7/G8/G9/G10 · P5/P7c/P8/P8b/P9/P16/P21/P22/P25/P26/P27 · C20 |
| **Inbound** | 主人当轮口令 · HANDOFF · 本文 |
| **Outbound** | `crates/*` · `apps/*` · `scripts/*` · `docs/tech/*` · `docs/NORMS.md` · HANDOFF/TASKBOARD |
| **MAY CONTAIN ERRORS** | Yes — 以源码与真机为准；计划可修订但须写修订表 |

---

## 0. 一句话目标（完整条件）

在 **Windows 11 优先** 的前提下，把产品建成可维护的 **RustMadoka**：  
协议与业务在 **平台无关 core**；桌面壳 / 将来 Android·Arch·macOS·OpenWrt·iOS 只做 **薄平台层**；  
**浏览器网页前端** 允许大量复用现有 SPA 交互与布局；  
**不考虑** 与旧 `automadoka_data` / `automadoka.exe` / 13220 的向前兼容；  
代码质量、完整注释、技术文档与 **双向链接** 为验收的一部分，不是事后补丁。

**当前诚实状态：** 组队 Raid 与 device_id 按卡片仅为 **增量**；**全量重构未完成**。本计划是此后唯一主线任务书。

---

## 1. 主人已钉死口径（汇总，禁止再问）

| 主题 | 口径 |
|------|------|
| 产品名 | RustMadoka |
| 可执行文件 | **普通版** `RustMadoka.exe` + **开发版** `RustMadoka_debug.exe`（wire） |
| 数据文件夹 | 仅 **`RustMadoka_data`**；全新安装；无旧目录迁移 |
| 默认端口 | **14103**（与旧 automadoka 无关） |
| Crate | `rustmadoka-core` / `rustmadoka-app` / `rustmadoka-mobile` |
| rules 仓 | **不改**；指纹仍可拉现有 `automadoka.json` URL |
| 本地工作区目录名 | **永不改** |
| 明文 token | 永远允许（P9）；防护仅不进 git、不打进分发物 |
| 测试 | AI 可对测试号跑完整清日常与全部功能；优先 **EN** |
| 组队 Raid | 要；援助后退出 **默认关**；device_id **按游戏账号卡片**；不刻意多账号 sleep |
| 旧 data | **归档备份不删**，集中子目录或高价值提取 |
| Android | 需完全重构，**本主线 Win 先**；架构预留 |
| 多平台预留 | Arch / macOS / OpenWrt / iOS：接口与模块边界预留，实现后置 |
| UI 复用 | **仅浏览器网页前端**允许大量复用原 SPA；Rust 业务禁止裸复刻（C20） |

### 1.1 组队伤害（AI 自定 · 写入规格）

原版：`random(min,max)` 绝对值；救世路径可对 `raid.hp` 一次打满。无「百分比拆分」精密公式。

**本产品算法（保证「能动手的都打完后 boss 必死」）：**

1. 本房参与伤害人数 **n** = 组队名单人数（配置人数，2≤n≤10）。  
2. 开房回包读 **boss 总血量** `H`。  
3. 每人伤害整数 ∈ \[[⌊0.10H⌋, ⌊(1.10−0.10n)H⌋]\]，且下限至少 1。  
4. **目标和** `T = H`（对满血；整数上用 `T = H` 且最后一人吃残差，使 **Σ ≥ H**）。  
5. 前 n−1 人在合法区间内随机；第 n 人 = `T − Σ前`，若越界则 **拒绝采样重试**（有限次数）；仍失败则均匀压到区间内并 **把差额补给最后一人再 clamp 后若仍 Σ&lt;H 则把不足部分加到上限未满的任意人**（保证 Σ≥H）。  
6. 体力不足：**完全不考虑** 为失败条件——能打则打，不能打的跳过；若跳过后剩余动手者伤害按 **实际动手人数 k** 重算上下限与 Σ≥H（避免「支援完了 boss 还活着」）。  
7. 援助后退出默认关。

规格真源仍扩写：`docs/tech/GROUP_RAID_AND_DEVICE_IDENTITY.md`。

---

## 2. 架构原则（模块化 · 平台分离）

```text
┌─────────────────────────────────────────────────────────────┐
│  浏览器网页前端 static/（可大量复用原 SPA 交互/布局）          │
│  仅经 HTTP JSON；无业务加密逻辑                               │
└───────────────────────────┬─────────────────────────────────┘
                            │ loopback HTTP
┌───────────────────────────▼─────────────────────────────────┐
│  rustmadoka-app（桌面宿主 · 当前 Win 实现）                    │
│  · CLI · 程序运行面板终端 · Owner/IPC · TaskGate · 静态托管   │
│  · feature wire_record → 开发版                               │
│  平台专属：Win 控制台、路径、打开浏览器、双 exe 构建脚本         │
└───────────────────────────┬─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│  rustmadoka-core（平台无关）                                  │
│  · Gree/登录 · 加密 · 指纹 sm · GameClient · 模块 · 组队编排  │
│  · 账号存储模型 · 错误/诊断 · wire 可选 feature               │
│  禁止：Win API、Android JNI、直接读环境变量当产品配置           │
└─────────────────────────────────────────────────────────────┘

将来：
  rustmadoka-desktop-linux / macos  → 复用 core + 薄壳
  rustmadoka-mobile (Android/iOS)   → JNI/FFI 调 core 的 serve API
  OpenWrt                           → headless CLI + core
```

| 层 | 放什么 | 不放什么 |
|----|--------|----------|
| **core** | 协议、模块语义、组队、存储格式、诊断 | UI、进程锁文件名可注入 trait |
| **app** | HTTP、CLI、Owner、静态页、Win 体验 | 游戏 AES/Gree 细节复制一份 |
| **static** | 页面与交互 | 私钥、引继明文逻辑 |
| **scripts** | 双 exe、Android 构建 | 业务规则 |

**依赖方向：** static → app → core；禁止 core 依赖 app。

---

## 3. 质量门禁（每阶段必过）

1. **注释（P21）：** 新/改公共 API 与模块：职责、输入输出、不变量、成功/跳过/失败、边界。  
2. **双向链接（P22）：** 代码头注释写 `docs/tech/….md`；文档写 Outbound 源码路径。  
3. **禁止裸复刻（C20/P25）：** 对照原版只作假说；游戏语义写清。  
4. **编译：** `cargo build -p rustmadoka-app --release` 与 debug feature 双通过（更名后）。  
5. **静态 JS：** 改 `static/index.html` 后 `scripts/check-static-js.ps1`。  
6. **证据（P5）：** 无点测不写 FIXED；CLI 只写 CLI 已验证。  
7. **log：** 每实质阶段 `docs/logs/` + HANDOFF ≥3 行。  
8. **无向前兼容：** 不保留双读 `automadoka_data`；不产出 `automadoka.exe`。

---

## 4. 阶段划分（建议顺序 · 可同会话多阶段）

### 阶段 R0 — 计划与规范落盘（本文件 + NORMS）

| 交付 | 完成定义 |
|------|----------|
| 本文为唯一全量重构真源 | HANDOFF/TASKBOARD 链到本文 |
| NORMS：产品命名、端口、数据目录、双 exe、协作授权（非用户环境变量） | 无「正式路径仍 automadoka_data」 |
| 组队伤害算法写入 GROUP_RAID 规格 | 含 Σ≥H 与体力跳过重算 |

### 阶段 R1 — 仓库与产物身份

| 交付 | 完成定义 |
|------|----------|
| Workspace members → `rustmadoka-core/app/mobile` | `cargo build` 通过 |
| bin 名 `RustMadoka`；构建脚本只产出两 exe | 无 automadoka.exe 目标 |
| 默认数据目录 `RustMadoka_data`；默认端口 14103 | 常量单一真源 |
| `.gitignore` 覆盖新数据目录与 exe | 不提交 secret |
| 旧 `automadoka_data` **移动到归档子目录**（不删） | 如 `archive/runtime-data-2026-08/` 或 `automadoka_data_archived/` |

### 阶段 R2 — core 模块边界清理

| 交付 | 完成定义 | 状态 2026-08-08 |
|------|----------|-----------------|
| 目录语义分区：`protocol/` · `domain/` · `diag/` | 编译通过；文档图更新 | **语义命名空间 CODE**（`protocol.rs`/`domain.rs` re-export）；物理 `src/protocol/` **演进式未盲搬** |
| Owner/路径注入 | 单测或注释 | **CODE**：`paths` + `data_layout` + Store.`data_dir` |
| 每文件头双向链接 | 抽查 | **触改文件已链**；全库抽查未宣称 100% |

*说明：允许「移动+改名+加注释」的演进式重构，不必删除已验证的 Gree/AES 实现再盲写一遍（那是重蹈 L1/L2）。*

### 阶段 R3 — app 宿主与 CLI

| 交付 | 完成定义 |
|------|----------|
| CLI 子命令稳定：serve / group / account / run / export / group-raid | `--help` 中文完整 |
| TaskGate + owner_group 停止权文档与 HTTP 雏形 | 与 INSTANCE 文一致 |
| 程序运行面板终端：正常只读方向对齐 MULTI_GROUP（可分迭代） | 规格对照表 |

### 阶段 R4 — 浏览器网页前端（复用 SPA）

| 交付 | 完成定义 |
|------|----------|
| 在复用布局上改品牌文案 RustMadoka、端口展示、数据路径提示 | check-static-js 通过 |
| EN/JP/TW 角标 · 组队入口（可简版） | 规格对照 |
| 跨组占用文案挂钩 WEB-SYNC 增量 | 可分迭代 |

### 阶段 R5 — 业务语义债

| 交付 | 完成定义 |
|------|----------|
| FIX-LOGINBONUS / EVENT-* / SCENARIO-DEDUP | wire 对照 + CLI EN |
| 组队伤害算法按 §1.1 替换当前拆分 | 单测纯函数 `split_group_raid_damages` |
| 模块结果标签与 ERROR_DIAGNOSTICS 一致 | 无假绝对 |

### 阶段 R6 — 监视与多组（规格已有）

按 `MULTI_GROUP_UI_AND_MONITOR_SPEC.md` 全文：WEB-SYNC-CORE → CARD-CROSS-GROUP → CHANNEL-BADGE → PROC-MONITOR → PARTY-SELECT。

### 阶段 R7 — 平台预留与收口

| 交付 | 完成定义 |
|------|----------|
| core 无 Win-only API | grep 约束 |
| mobile crate 可编译（功能可 stub） | CI 级本地 build |
| HANDOFF 能力矩阵更新；W4 发布仍 WAIT 点名 | 诚实 CODE/FIXED |

---

## 5. 明确不做（本主线）

- 旧路径/旧端口/旧 exe 兼容与静默迁移  
- 改 rules 仓内容  
- 默认 token 再加密  
- 台服 Sonet（未点名）  
- Android 完整验收 B 表（后置）  
- 把 archive Python 业务抄进 core 当完成  
- 建议注水与假课题（C22）

---

## 6. 测试策略

| 层级 | 内容 |
|------|------|
| 纯函数 | 伤害拆分、路径常量、channel 解析 |
| CLI EN | `run info` / 单模块 / 有条件 `group-raid`（多号） |
| 离线 | `archive` + 已归档 wire/exports 对照字段 |
| 主人 | 重构后全面点测；AI 不写 FIXED 代替点测 |

测试账号：`plan/local-test-accounts.md`（gitignore）；优先 EN。

---

## 7. 文档地图（重构期）

| 文件 | 角色 |
|------|------|
| **本文** | 全量重构唯一阶段真源 |
| `GROUP_RAID_AND_DEVICE_IDENTITY.md` | 组队 + device_id |
| `MULTI_GROUP_UI_AND_MONITOR_SPEC.md` | 多组/监视完整交互 |
| `TECH_DOC_CONVENTION.md` | 双向链接模板 |
| `SDK_AND_LOGIN.md` / `PROTOCOL_STACK.md` | 协议 |
| `HANDOFF.md` | 现状入口（每阶段回写） |
| `docs/logs/2026-08-07-*.md` | 过程 |

---

## 8. 建议实施节奏（开工顺序）

```text
R0 规范+本计划          ← 立即
R1 更名+路径+端口+归档  ← 紧接（触面广，先定）
R1.5 组队伤害算法修正   ← 小、可测
R2 core 分区            ← 质量骨架
R5 假成功 FIX（可穿插）
R3/R4 宿主与网页
R6 多组监视
R7 收口
```

---

## 9. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-07 15:06 | 首版；伤害算法 AI 自定；架构分层；R0–R7 |
