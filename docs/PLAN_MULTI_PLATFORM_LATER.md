# 远期规划：macOS · Arch Linux · OpenWrt（局域网 Web）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-07 06:56 |
| **状态** | **LATER / 远期** — 仅规划，不挡当前 Win/Android 与通讯/心跳任务 |
| **原则** | 协议与业务在 `rustmadoka-core`；UI 尽量 `static` + 本机 HTTP；壳按平台换 |
| **Inbound** | [PLAN_AUDIT_COMMS_OCCUPANCY_PROGRESS.md](./PLAN_AUDIT_COMMS_OCCUPANCY_PROGRESS.md) · [DIRECTION.md](./DIRECTION.md) · [ANDROID_DUAL_PLATFORM](./tech/ANDROID_DUAL_PLATFORM.md) |
| **MAY CONTAIN ERRORS** | Yes |

---

## 0. 为何现在就记一笔

主人已明确：**还要做** macOS、Arch Linux、OpenWrt。  
当前实现（占用路径、控制台中文确认、Owner 锁）设计时 **避免写死「仅 Windows」**，减少远期撕裂。

---

## 1. 平台表

| 平台 | 定位 | 形态倾向 | 备注 |
|------|------|----------|------|
| **Windows** | 当前主路径 | 单 exe + loopback Web + 运行面板 | 已有 |
| **Android** | 当前双端之一 | WebView + 嵌入 serve | 壳可用；B 清单未齐 |
| **macOS** | 远期 | 单二进制或 .app；loopback Web；无 Win 命名管道则 IPC 换 Unix socket | 代码签名/公证另议 |
| **Arch Linux** | 远期桌面/服务器 | 单二进制；systemd 用户服务可选；Web 本机或 LAN 策略另定 | 依赖少、贴 core |
| **OpenWrt** | 远期网关/常驻 | **允许局域网内手机/电脑用网页访问**；绑定与鉴权必须另设计 | 见 §3 |

---

## 2. 共享与分叉

| 层 | 共享 | 可能分叉 |
|----|------|----------|
| 游戏协议 / 模块 | `rustmadoka-core` | 无 |
| SPA | `static/index.html` | 主题/触控可调 |
| HTTP API | 契约稳定 | OpenWrt 可能需监听非 loopback + 鉴权 |
| 实例锁 / 心跳 | 语义统一 | 路径、文件锁实现（flock 等） |
| IPC | 语义统一 | Win pipe vs Unix socket |
| 运行面板 | 可选 | 无 GUI 平台以 Web/日志为主 |

---

## 3. OpenWrt 专项（仅远期要点）

| 点 | 倾向（未钉死） |
|----|----------------|
| 用途 | 路由/旁路机 7×24；手机浏览器打开局域网地址清日常 |
| 监听 | 可能 `0.0.0.0:端口` 或桥接接口；**默认不能像 Win 一样只 127.0.0.1** |
| 鉴权 | 局域网也不等于安全：至少口令 / token；禁止默认无密暴露到公网 |
| 资源 | 体积与内存敏感；musl 交叉编译；少依赖 |
| 与心跳 | data 可在 USB/外置存储；多客户端抢同一 data 仍用心跳+锁语义 |
| 不做（远期第一刀也不做） | 公网暴露、复杂反向代理全家桶 |

**具体方案开工前再开专章任务书。**

---

## 4. 排期关系

```text
现在：Win 通讯像客户端 + 心跳 + 进度报错（PLAN_AUDIT…）
     Android 继续 B 清单（并行、不挡）
远期：macOS → Arch 桌面 → OpenWrt（顺序可推翻；OpenWrt 依赖 API 稳定与鉴权设计）
```

---

## 5. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-07 06:56 | 首版远期三端 + OpenWrt 局域网 Web 要点 |
