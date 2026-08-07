# 工作区

**完整目录与规范关系：** [docs/DOC_MAP.md](./docs/DOC_MAP.md)（优先读）。

| 路径 | 角色 |
|------|------|
| `docs/HANDOFF.md` | **完整交接入口** |
| `docs/NORMS.md` | 规则 G/P · 案例 · 索引 |
| `docs/LESSONS.md` | 教训总索引 |
| `docs/PLAN_RUSTMADOKA_FULL_REWRITE.md` | 全量重构阶段真源 |
| `docs/TASKBOARD.md` | 开放任务短表 |
| `docs/tech/` | 技术文档 |
| `docs/logs/` | 过程日志 |
| `crates/rustmadoka-core` | 平台无关协议与业务 |
| `crates/rustmadoka-app` | 桌面宿主（CLI / HTTP / Owner） |
| `crates/rustmadoka-mobile` | Android 动态库壳 |
| `apps/android/` | Android WebView 壳 |
| `scripts/` | 双 exe、Android、静态 JS 检查 |
| `archive/` | 只读对照（Python、旧运行时数据） |
| `RustMadoka.exe` / `RustMadoka_debug.exe` | 根目录交付物（gitignore） |
| `RustMadoka_data/` | 运行时数据（gitignore） |

**本机工具链：** 见 [docs/DEV_ENV.md](./docs/DEV_ENV.md)。  
**公开远程：** `rustmadoka` → https://github.com/YzLfireChiYv/RustMadoka （禁止把 origin=cc004 全史 force 过去）。
