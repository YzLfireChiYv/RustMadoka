# 本机开发环境（Win11）

> **墙钟：** 2026-08-06 00:19  
> **用途：** 记录已装什么、怎么验证、与产品 Python 纪律如何区分。  
> **任务入口仍是** [HANDOFF.md](./HANDOFF.md)。

---

## 1. 原则（主人已拍板）

| 类别 | 纪律 | 原因 |
|------|------|------|
| **旧版 / 为本项目污染系统的 Python** | **禁止**装进 Program Files、勾选系统 PATH、改注册表 | 本机其它地方需要**新版**系统 Python；为本项目再灌旧版会互相踩 PATH |
| **本机已有的新版系统 Python** | **不要卸载、不要改**；本项目业务运行**不依赖**它 | 留给主人其它用途 |
| **本项目跑母项目/探路 Python** | 只用 **embed/便携**（`ref-legacy-superset/app/win/runtime/python`）或项目内 venv | 与系统 Python 隔离 |
| **Rust 与独立开发组件** | **允许**系统级安装（rustup、MSVC Build Tools、Windows SDK 等） | 现代工具链；与 Python 禁令无关 |

**当前开发策略（口语确认）：**  
先用现有母项目/便携 Python **实跑功能、拿数据**（阶段 0.5），再决定 Rust 制品与是否逐步/完全重写协议代码。Rust 与 Python 的长期配合方式（混跑 / 全重写）**尚未钉死**。

---

## 2. 已安装与验证（2026-08-06）

### 2.1 Rust

| 项 | 值 |
|----|-----|
| 安装方式 | `winget install Rustlang.Rustup` |
| rustc / cargo | **1.97.1**（stable） |
| 默认 target | `x86_64-pc-windows-msvc` |
| 用户目录 | `%USERPROFILE%\.cargo` · `%USERPROFILE%\.rustup` |
| PATH | 用户 PATH 含 `%USERPROFILE%\.cargo\bin`（**新开**终端生效；旧会话需刷新 PATH） |

### 2.2 MSVC 链接器（Rust Windows 默认需要）

| 项 | 值 |
|----|-----|
| 产品 | Visual Studio 2022 **Build Tools** 17.14.37 |
| 路径 | `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools` |
| 组件 | VC Tools + Windows SDK **10.0.26100**（`Lib`/`Include` 已存在） |
| vcvars | `...\VC\Auxiliary\Build\vcvars64.bat` |
| vswhere | `isComplete=true` · `isLaunchable=true` · `canceled=0` |

说明：首次 winget 安装曾因网络中断呈 incomplete；补装后 SDK 与实例状态已完整。若某会话 `cargo build` 报找不到 `link.exe`，先在该终端执行：

```bat
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
```

或从「x64 Native Tools Command Prompt for VS 2022」启动。许多机器装好后新开 PowerShell 也能直接找到工具；以本机实测为准。

### 2.3 冒烟（已通过）

在 `%TEMP%\automadoka-rust-smoke\hello_rust`：

1. `vcvars64.bat` 初始化 x64  
2. `cargo new` → `cargo build` → `cargo run`  
3. 输出 `Hello, world!` · 标记 **SMOKE_OK**

**未做：** 仓库内正式 `crates/` 工作区（待主人点名阶段 1 再建）。

### 2.4 Python（本项目用）

| 项 | 值 |
|----|-----|
| 系统 Python | 主人自用（如 3.14）→ **勿动** |
| 项目便携 | `ref-legacy-superset/app/win/runtime/python`（embed 3.11，gitignore 超集内） |
| 业务实跑 | 阶段 0.5 用便携/归档树或探路 APK，**不**为本项目装旧版系统 Python |

---

## 3. 建议的日常检查命令

```powershell
# 刷新当前会话 PATH（若刚装完工具）
$env:Path = [System.Environment]::GetEnvironmentVariable('Path','Machine') + ';' +
            [System.Environment]::GetEnvironmentVariable('Path','User')

rustc --version
cargo --version
rustup show

# 可选：确认 link
where.exe link
```

---

## 4. 相关文档

| 文件 | 内容 |
|------|------|
| [NORMS.md](./NORMS.md) | R0 + PowerShell 中文纪律摘要 |
| [POWERSHELL_WIN11.md](./POWERSHELL_WIN11.md) | Win11 PowerShell 中文/编码细则 |
| [HANDOFF.md](./HANDOFF.md) | 交接与下一刀 |
| log：`docs/logs/2026-08-06-rust-dev-env.md` | 本批过程 |
