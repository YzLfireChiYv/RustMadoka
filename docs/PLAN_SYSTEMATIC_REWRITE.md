# 任务书：系统性重构与多平台交付路线

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-07 |
| **状态** | **路线已定 · 阶段 W1 为下一开工门** |
| **真源短表** | [TASKBOARD.md](./TASKBOARD.md) |
| **规范** | [NORMS.md](./NORMS.md) **G8** · P5 · P25 · C20 · 明文 P9 |
| **语义分类** | [tech/MODULE_SEMANTIC_CLASSIFICATION.md](./tech/MODULE_SEMANTIC_CLASSIFICATION.md) |
| **功能身份** | [tech/GAME_NAMING_GLOSSARY.md](./tech/GAME_NAMING_GLOSSARY.md) |
| **MAY CONTAIN ERRORS** | Yes — 各阶段完成以真机与源码为准 |

---

## 0. 主人钉死的总路线（按序）

1. **完成 CLI 控制**，并 **完整记录** 所有发往游戏服务器的内容与服务器返还的内容。允许在 **原版 Python** 上改代码做采样；Rust 侧同等能力优先作为产品路径。  
2. **整理并分析** 上一步采集的数据，用于 **重构出新的程序版本**（按功能身份与 ①领取 / ②养成 / ③战斗 语义，禁止未理解就照搬）。  
3. 在新版本中 **拓展更多功能**，并 **发布新的 `.exe` 与 `.apk`**。  
4. **持续迭代**，并交付 **macOS、Arch Linux、OpenWrt** 等版本（与 [PLAN_MULTI_PLATFORM_LATER.md](./PLAN_MULTI_PLATFORM_LATER.md) 对齐，实现时升为当期阶段而非永久口号）。

**名称工作：** 不再作为主线消耗；显示名后补。优先 **key + 协议 I/O**。

---

## 1. 阶段划分

| 阶段 | ID | 内容 | 门禁 / 完成定义（草案） |
|------|-----|------|-------------------------|
| **W1** | **WIRE** | CLI 可驱动关键路径；**全量 wire 录制**（请求 URL、方法、明文业务 body/关键字段、回包状态、解密后业务 JSON 或约定摘要；时间戳；账号别名；禁止把私钥写进默认可分享日志——本机 data 明文仍合法 P9） | CLI 能开/关录制、能指定组/号跑 info 或单模块/日常；落盘目录固定且 gitignore；至少一种路径（Rust 优先，Python 可并行）跑通登录+1 领取模块+1 战斗或活动相关采样 |
| **W2** | **ANALYZE** | 整理采样：按模块/按 API 建成功·跳过·失败表；对照 ①②③ 分类；列出路径错误与语义错误清单；输出「重构变更清单」 | 文档落盘 tech 或 logs；与 MODULE_SEMANTIC 双向链接；主人可审 |
| **W3** | **REWRITE** | 按清单重构 Rust：优先 ① 领取反馈语义与 path 正确性；混合模块状态机；③ 队伍与开房；② 商店/洗词条门禁 | 根目录 release exe；关键路径 CLI 可验；P5 不写无点测 FIXED |
| **W4** | **EXPAND+SHIP** | 拓展功能（配置化探索等按点名）；**发布新 exe + apk** | 版本号/stamp；发布说明；双端构建纪律 |
| **W5** | **MULTI** | macOS · Arch · OpenWrt 持续迭代交付 | 见 PLAN_MULTI_PLATFORM_LATER；core 共享；IPC/绑定按平台 |

---

## 2. W1 规格要点（下一轮主任务）

### 2.1 CLI

- 保持并可扩展：`group` / `account` / `run info|daily` / `export session`  
- 增加或完善：按模块运行、wire 录制开关与输出目录（具体子命令实现时定名）  
- 与 Owner/IPC 共存规则沿用现有实例模型  

### 2.2 Wire 完整记录

| 应记录 | 说明 |
|--------|------|
| 出站 | 完整 URL 或 path+host 策略、业务 payload（解密前/后策略在实现里写清并固定一种默认可分析形态） |
| 入站 | HTTP 状态、解密后业务 JSON（或与出站对称的完整形态） |
| 元数据 | 墙钟、模块 key、别名、channel、是否 skip/error |

落盘建议：`automadoka_data/wire/{alias}/{timestamp}/`（gitignore）。

### 2.3 Python 采样

- 允许改 `archive` **外**的可运行 Python 树或明确标注的采样分支；**禁止**把采样脏改合回「只读 archive 真理」而不说明  
- 若改 archive 对照树：只读原则下优先复制到 `scripts/` 或临时 probe，避免污染只读史  

（实现时在 log 写清改了哪棵树。）

---

## 3. 与旧队列关系

| 旧项 | 关系 |
|------|------|
| E1 | 已 CODE；**不替代** W1 全量 wire（E1 是登录后内存快照） |
| AUD-COMMS | 并入 W3/W4 的反馈与像客户端，不抢 W1 |
| STORY-P / C10 / C07 | W4 点名；配置化探索在原理重写后 |
| 多平台 | **W5**；设计时继续避免仅 Win 死绑 |

---

## 4. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-07 | 主人总路线四步；W1–W5；W1 wire+CLI 规格草案 |
