# 任务书：Android 端 · 与 Win 双版本同步 · 代码复用

| 项 | 内容 |
|----|------|
| **状态** | **规划定稿 · 下一会话主线开工（D01）** |
| **墙钟** | 2026-08-06（交接钉：首要 Android） |
| **读者** | 主人 · 下一会话 AI（含低价 API 模型） |
| **前置** | Win Rust 主路径已具备：登录/账号/清日常/设置/日志/指纹槽/AM2/洗词条/诊断（见 HANDOFF） |
| **原则** | **先双端对齐「当前基础功能」→ 再加新功能且双端同步推进** |
| **UI** | WebView + **「用系统浏览器打开」** 双保险 |
| **构建** | 起双端后 **每次**尽量同步编 Win11 + Android |
| **MAY CONTAIN ERRORS** | Yes |

### Outbound（必读）

| 文档 | 为何 |
|------|------|
| [tech/ANDROID_DUAL_PLATFORM.md](./tech/ANDROID_DUAL_PLATFORM.md) | 架构、crate 切分、Android 壳、同步纪律 |
| [tech/UPSTREAM_FOR_LLM_CONTRIBUTORS.md](./tech/UPSTREAM_FOR_LLM_CONTRIBUTORS.md) | **给其它 LLM 的跟进手册**（入口/证据/禁止项） |
| [tech/INSTANCE_AND_CLI.md](./tech/INSTANCE_AND_CLI.md) | Owner/IPC/端口（Win；Android 对照表） |
| [tech/UI_ROUTING_AND_TASK_LOGS.md](./tech/UI_ROUTING_AND_TASK_LOGS.md) | 路由与任务日志（Web 可共享） |
| [tech/LESSONS_RUST_PORT.md](./tech/LESSONS_RUST_PORT.md) | 协议坑（Gree/AES） |
| [tech/PROTOCOL_STACK.md](./tech/PROTOCOL_STACK.md) · [SDK_AND_LOGIN.md](./SDK_AND_LOGIN.md) | 协议真相 |
| [PLAN_RELEASE_AND_SELF_UPDATE.md](./PLAN_RELEASE_AND_SELF_UPDATE.md) | 版本 stamp、发行仓 |
| [NORMS.md](./NORMS.md) | R0 纪律 |
| 源码 | `crates/rustmadoka-core` · `crates/rustmadoka-app` · `static/` |

### Inbound

| 入口 | 关系 |
|------|------|
| [HANDOFF.md](./HANDOFF.md) | 下一阶段入口 |
| [DIRECTION.md](./DIRECTION.md) | 产品五线中的 A-Rust |

---

## 1. 目标与非目标

### 1.1 目标

1. **Android 客户端**提供与 Win **同一套基础能力**（见 §2 清单），体验接近：用户组、游戏账号、设置/一键清日常、日志、指纹拉取、安全门禁。  
2. **业务与协议代码尽量只写一份**：落在 **`rustmadoka-core`（Rust）**；两端只做壳（Win exe / Android APK）。  
3. **Web UI 尽量共用**（HTML/JS 同一棵 `static/` 或发布为热更包）；Android 用 WebView 打开本机服务或嵌入静态页 + 本地 API。  
4. **新功能必须双端同步推进**：禁止只做 Win 堆功能导致 Android 永久落后（见 §4 同步纪律）。  
5. **文档完整到**：换便宜 LLM 也能按 HANDOFF → 任务书 → tech → 源码路径跟进，不靠聊天记忆。

### 1.2 非目标（本阶段）

- 不为「更快写 UI」整仓换语言重写协议。  
- 不做 Google Play 上架与商店合规全套（可后置）。  
- 不恢复 Python 为主工程。  
- 不把完整 XAPK 打进 APK。

---

## 2. 「基础功能」对齐清单（Android 首期必须项）

以下与当前 Win 产品对齐；**全部完成前不加 R4 托管等大新功能**。

| ID | 能力 | Win 锚点 | Android 验收 |
|----|------|----------|--------------|
| B1 | 指纹拉取/本地缓存 | `fingerprint` · fetch-fp | 同 JSON，无用户导 XAPK |
| B2 | 用户组 + 可选加密 | `account.rs` | 同数据语义；路径用 app files |
| B3 | 游戏账号卡片 CRUD | Web 卡片 | 同：增删、复制设置、剪贴板导入导出 |
| B4 | 获取账号信息 / 轻量登录 | `login_for_info` | 同协议 |
| B5 | 日常 26 + 默认全关 + 一键 | `modules/*` · safety | 同默认与门禁 |
| B6 | 单模块运行 · 勾选独立 | settings UI | 同 |
| B7 | 任务日志 + 进度 + 暂停 | task_log · RunControl | 同语义 |
| B8 | 二次确认（主页/设置独立） | config 键 | 同键名 |
| B9 | 版本 build_stamp 展示 | `/api/version` | 同 |
| B10 | 与主人日常包隔离 | 端口/包名 | **包名 ≠ com.automadoka.app**；端口/前台服务自定 |

**工具洗词条等：** 可与 Win 同门禁；非 B 清单阻断项。

---

## 3. 推荐实现形态（摘要）

详见 [tech/ANDROID_DUAL_PLATFORM.md](./tech/ANDROID_DUAL_PLATFORM.md)。

```text
                    ┌─────────────────────────┐
                    │  rustmadoka-core (Rust)   │
                    │  协议·账号·模块·日志·指纹  │
                    └───────────┬─────────────┘
              ┌─────────────────┼─────────────────┐
              ▼                                   ▼
   automadoka-app (Win)                  automadoka-android
   · axum + static Web                   · Kotlin 壳 + WebView
   · Owner 锁 / 命名管道 IPC             · 本机 loopback HTTP 或
   · 单 exe + automadoka_data              JNI/uniffi 薄封装
              │                                   │
              └──────── 同一套 static UI ─────────┘
```

**默认策略：** Android 上 **复用本机 HTTP + 同一套 SPA**（与 Win 一致），Kotlin 只负责：启动 Rust 服务/库、前台保活、通知、选目录、返回键。  
**双保险（主人钉死）：** 壳内 WebView 使用 **且** 提供菜单/按钮 **「用系统浏览器打开」** → `Intent.ACTION_VIEW` 打开同一 `http://127.0.0.1:PORT/`（用户可在 Chrome 等里操作，WebView 异常时仍可用）。  
**备选：** `uniffi`/`jni` 暴露 core API、纯原生 UI——工作量大，**不作为首期**。

---

## 4. 双端同步纪律（硬）

| 规则 | 说明 |
|------|------|
| **S1 功能开关同源** | 新能力默认进 `rustmadoka-core` 或 `static/`；禁止只写在 `main.rs` Win 专用分支且无 Android 路径 |
| **S2 任务书双端验收** | 每个新功能任务书写「Win 验收 + Android 验收」；暂缓 Android 须主人书面点名 |
| **S3 API 契约稳定** | Web 调用的 `/api/*` 视为公共契约；破坏性变更写 VERSION / tech 文档 |
| **S4 配置键名统一** | `confirm_*`、`log_*`、模块 key 与 Win 相同，便于导入导出 |
| **S5 文档先于堆码** | 跨会话/跨模型：改公共行为先改 PLAN/tech 三行 |
| **S6 阶段门** | Android B1–B10 未齐：**禁止**开 R4 托管/新玩法大项（主人可推翻） |

---

## 5. 阶段路线

| 阶段 | 内容 | 退出标准 |
|------|------|----------|
| **A0** | 本文 + ANDROID_DUAL_PLATFORM + LLM 手册；Win 功能冻结范围说明 | 主人批准 |
| **A1** | core 与 app 解耦检查：无 Win-only 协议逻辑 | core 可被第二 binary 链接 |
| **A2** | Android 工程：启动 core HTTP 或等价 + WebView 加载 UI | 本机模拟器打开登录页 |
| **A3** | 数据目录、加密、info 点测 | 日服 info 通 |
| **A4** | 清日常/日志/设置对齐 B5–B8 | 清单勾完 |
| **A5** | 双端 CI/发版：同一 build_stamp 策略 | 文档可复现 |
| **A6+** | 新功能双端同步（会话池、托管等） | 每项双验收 |

---

## 6. 发行与仓

- 发行仓目标：`YzLfireChiYv/automadoka`（指纹 + Releases + 可选 APK）。  
- **删仓重建**：须主人本机 `gh auth login` 后执行（本环境无 GitHub 登录，**AI 不能代删**）。命令见 HANDOFF/下文。  
- Win/Android 资产命名带 `build_stamp`。

---

## 7. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-06 | 首版：基础清单、复用架构、同步纪律、阶段门 |
| 2026-08-06 | 交接：列为 NOW；系统浏览器双保险；双端同步构建 |
