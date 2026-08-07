# RustMadoka

Magia Exedra 本机自动清日常工具（Windows 优先）。**不以云端代打、不以公开仓存号。**

| 文档 | 说明 |
|------|------|
| **[docs/HANDOFF.md](./docs/HANDOFF.md)** | **完整交接入口（从这里读）** |
| [docs/PLAN_RUSTMADOKA_FULL_REWRITE.md](./docs/PLAN_RUSTMADOKA_FULL_REWRITE.md) | 全量重构阶段 R0–R7 |
| [docs/NORMS.md](./docs/NORMS.md) | 项目纪律 |
| [docs/LESSONS.md](./docs/LESSONS.md) | 教训索引 |
| [docs/tech/README.md](./docs/tech/README.md) | 技术文档索引 |
| [docs/DEV_ENV.md](./docs/DEV_ENV.md) | 本机 Rust / MSVC |

旧 Python 业务只读：`archive/pre-rust-2026-08/`。

## 构建与运行（Win11）

```bat
cd /d C:\GrokProject\automadoka
powershell -File scripts\build-win-dual.ps1
RustMadoka.exe
RustMadoka_debug.exe
```

| 项 | 默认 |
|----|------|
| 普通版 | `RustMadoka.exe` |
| 开发版 | `RustMadoka_debug.exe`（`wire_record` 通讯录制） |
| 数据文件夹 | 旁路 **`RustMadoka_data`** |
| 浏览器网页前端 | `http://127.0.0.1:14103/` |

指纹默认源（只读）：`https://raw.githubusercontent.com/YzLfireChiYv/rules/main/automadoka.json`  
本地明文 token/引继 **永远允许**（NORMS P9）；禁止进 git、禁止打进分发 exe。

## CLI 示例

```bat
RustMadoka.exe group list
RustMadoka.exe run info -g <用户组> -a <别名> --json
RustMadoka.exe run daily -g <用户组> -a <别名> --json --all-modules
```

详见 [docs/HANDOFF.md](./docs/HANDOFF.md) · [docs/tech/INSTANCE_AND_CLI.md](./docs/tech/INSTANCE_AND_CLI.md)。
