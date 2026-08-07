# 技术说明：Android + Win 双端架构与代码复用

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-06（壳 v0.2：去顶栏 + 手机 SPA） |
| **任务书** | [PLAN_ANDROID_AND_DUAL_PLATFORM.md](../PLAN_ANDROID_AND_DUAL_PLATFORM.md) |
| **读者** | 实现者 · 任意 LLM |
| **MAY CONTAIN ERRORS** | Yes — 以源码与真机为准 |

---

## 1. 为什么这样切

| 层 | 语言 | 职责 | 是否双端共享 |
|----|------|------|----------------|
| **core** | Rust | Gree、AES、msgpack、指纹、账号加密、日常模块、任务日志、TaskGate | **必须共享** |
| **app-shell Win** | Rust | axum、Owner 锁、命名管道 IPC、CLI、单 exe | Win only |
| **app-shell Android** | Kotlin + 少量 Rust glue | 进程生命周期、**全屏 WebView**、前台服务、存储路径、通知、FAB 菜单 | Android only |
| **UI** | HTML/JS（`static/`） | 路由、设置、日志、二次确认；**手机触控放大 CSS** | **逻辑共享，触控样式分叉** |

协议已在 Rust 踩坑修好（见 LESSONS_RUST_PORT）。**不要**为 Android 用 Kotlin/Java 重写协议。

**为何不整仓拉取第三方 WebView 浏览器当壳：** 业务入口固定为「JNI 启 Rust Owner → loopback HTTP → 同一 SPA」；通用浏览器壳会带多余导航/下载/多标签，与 Owner 单实例冲突。本仓壳按常见全屏 WebView 模式**自研精简**（无顶栏、FAB 次级操作），对照归档 `archive/pre-rust-2026-08/android` 仅学「前台服务 + WebView」形状。

---

## 2. 仓库布局（当前）

```text
crates/
  rustmadoka-core/          # 唯一协议+业务实现
  rustmadoka-app/           # Win 壳 + 嵌入式 serve + static/index.html
  rustmadoka-mobile/        # cdylib：JNI nativeStart / nativeStatus / nativeBuildStamp
apps/
  android/                  # Gradle + Kotlin 全屏 WebView 壳
    app/src/main/java/com/rustmadoka/android/
      MainActivity.kt       # 全屏 WebView + FAB + JS 桥
      BackendService.kt     # 前台服务启 Rust
      NativeBridge.kt       # loadLibrary + external
    app/src/main/jniLibs/{arm64-v8a,x86_64}/libautomadoka_mobile.so
scripts/
  build-android-native.ps1  # cargo-ndk → jniLibs
  build-android-apk.ps1     # assembleDebug + 可选 install
```

---

## 3. Android 运行时（已落地）

### 3.1 本机 HTTP + 全屏 WebView（与 Win 同构）

```text
MainActivity / BackendService
  → NativeBridge.nativeStart(filesDir/RustMadoka_data, 14103)
  → automadoka_app::run_embedded_serve
  → WebView.loadUrl("http://127.0.0.1:14103/")
  → onPageFinished: 注入 html.platform-android
```

| 优点 | 注意 |
|------|------|
| UI 与 Win 同一 SPA / 同一 `/api/*` | 改 `static/index.html` 须进 mobile `.so` 或 Win exe 再装 |
| 无原生顶栏，内容区全给 SPA | D04 走 FAB / 通知 / `RustMadokaShell.openExternalBrowser()` |
| 包名 `com.rustmadoka.android` | ≠ 主人日常包 `com.automadoka.app` |

### 3.2 壳 UI 约定（2026-08-06 v0.2）

| 项 | 定稿 |
|----|------|
| 顶栏 | **不要**（Python 探路遗留 status/XAPK 栏已删） |
| 全屏 | WebView match_parent；冷启动仅 boot 遮罩 |
| 系统浏览器 | FAB 菜单 + 通知 Action + JS 桥（D04） |
| 外链 | `shouldOverrideUrlLoading`：非 127.0.0.1 交系统浏览器 |
| 手势 | **禁缩放**、**禁横向过滑**、竖屏锁定（2026-08-07）；系统浏览器路径仍可能受机型限制 |

### 3.3 备选：uniffi + 原生 UI

工作量大，**不作为当前主线**。仍禁止在 Kotlin 重写 AES/Gree。

---

## 4. SPA 手机端样式（共享文件内分叉）

源码：`crates/rustmadoka-app/static/index.html`

| 机制 | 作用 |
|------|------|
| `@media (max-width: 820px), (pointer: coarse)` | 大字体、min-height≈48px 按钮、单列布局、细边框色 |
| `html.platform-android` | WebView 注入；为 FAB 留 `margin-bottom: 72px` |
| 桌面宽屏 | 保持原密度，不盲目放大 |

**原则：** 逻辑/API 不双份；**触控命中区与字号**允许手机加码。见 [UI_ROUTING_AND_TASK_LOGS.md](./UI_ROUTING_AND_TASK_LOGS.md) §SPA 纪律。

---

## 5. 数据目录

| 平台 | 路径 |
|------|------|
| Win | `{exe_dir}/RustMadoka_data/` |
| Android | `context.filesDir/RustMadoka_data/` |

结构一致：`users/` · `cache/token/` · `task_logs/` · `app.json` · …

加密组语义见 `rustmadoka-core` account.rs。

---

## 6. 与 Win 差异对照

| 能力 | Win | Android 首期 |
|------|-----|----------------|
| Owner 单实例 | owner.lock | 单进程 Activity/Service + 同锁文件语义 |
| CLI / IPC | 有 | **可不做**；用 Web |
| 自更新 | GitHub Releases exe | APK 旁路（后置） |
| 壳装饰 | 系统浏览器 | 全屏 WebView + FAB |
| 隐式后台 | 允许 | 前台服务 + SPECIAL_USE |

---

## 7. 构建

| 步骤 | 命令 |
|------|------|
| native `.so` | `scripts/build-android-native.ps1`（`cargo ndk -t x86_64 -t arm64-v8a`） |
| APK | `scripts/build-android-apk.ps1` 或 `apps/android` 下 `gradlew assembleDebug` |
| 真机 ABI | 当前设备常见 `arm64-v8a`；模拟器 `x86_64` |
| Win 交付 | `scripts/build-win-dual.ps1` → `RustMadoka.exe` + `RustMadoka_debug.exe`（SPA 同步；见 HANDOFF P3） |

`build_stamp` 与 Win 同一 `build.rs` 规则（PLAN_RELEASE）。

---

## 8. 安全

- 不云存引继码。  
- cleartext 本机 HTTP：仅 loopback；`network_security_config` 允许 127.0.0.1。  
- JS 桥仅暴露 `openExternalBrowser` / `getBuildStamp`，无任意 URL 反射。

---

## 9. 协议禁止事项（Android 同样适用）

见 [LESSONS_RUST_PORT.md](./LESSONS_RUST_PORT.md)：Gree initialize HMAC vs RSA；AES key；msgpack 形状；428 处理。

---

## 10. Inbound / Outbound

| 方向 | 文档/源码 |
|------|-----------|
| Outbound | `crates/rustmadoka-core` · `rustmadoka-app` · `rustmadoka-mobile` · `apps/android` · `static/index.html` · PLAN_ANDROID |
| Inbound | [HANDOFF.md](../HANDOFF.md) · [TASKBOARD.md](../TASKBOARD.md) · [UPSTREAM_FOR_LLM_CONTRIBUTORS.md](./UPSTREAM_FOR_LLM_CONTRIBUTORS.md) |

### 修订

| 日期 | 内容 |
|------|------|
| 2026-08-06 | 首版架构 |
| 2026-08-06 | v0.2：去原生顶栏、FAB/通知 D04、SPA 触控放大、布局路径钉死 |
