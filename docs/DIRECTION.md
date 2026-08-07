# 产品方向（一页）

**主人 PC** 解析游戏安装包 → **小体积指纹** → **GitHub（优先）+ Cloudflare Worker + 剪贴板/自备链接** 等多保险 →  
好友使用 **薄客户端 / 严格超集兜底**，**不必**导 XAPK；本机 Web 使用；可热更。

## 五条交付线（可行性已评估）

| 线 | 定位 |
|----|------|
| Win 魔改 Python | 验证床 / 过渡；0.5 已实跑 |
| Win **严格超集 + 启动器** | 不改上游文件、合原作者更新的**兜底** |
| Win **Rust 单 exe** | 中长期主产品；优先功能还原，再加新功能 |
| Android 严格超集 | 过渡（探路已证壳；体积大） |
| Android Rust 薄端 | 终局之一；共享 Rust core |

**当前工程主线：** **Android 移植**（WebView + 系统浏览器双开）；Win 骨架已可用。  
**纪律：** 双端起后 **每次**同步构建/验收 Win11 + Android。  
**Android 之后：** 系统性研究 + 新功能双端推进 — [PLAN_ANDROID…](./PLAN_ANDROID_AND_DUAL_PLATFORM.md) · [TASKBOARD](./TASKBOARD.md)。  
**R4 托管/会话池等：** 排在 Android 基础（B1–B10）之后（可推翻）。

**详情：** [PLAN_ANDROID…](./PLAN_ANDROID_AND_DUAL_PLATFORM.md) · [tech/ANDROID_DUAL_PLATFORM.md](./tech/ANDROID_DUAL_PLATFORM.md) · [tech/UPSTREAM_FOR_LLM_CONTRIBUTORS.md](./tech/UPSTREAM_FOR_LLM_CONTRIBUTORS.md) · [HANDOFF.md](./HANDOFF.md)
