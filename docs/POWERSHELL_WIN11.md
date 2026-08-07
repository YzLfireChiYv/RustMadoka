# Win11 PowerShell 与中文（开发规范）

> **墙钟：** 2026-08-06  
> **本机实测：** Windows PowerShell **5.1**（`$PSVersionTable.PSEdition = Desktop`）  
> **依据：** [Microsoft about_Character_Encoding](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_character_encoding) + 社区/工程实践  
> **MAY CONTAIN ERRORS：** 以本机点测为准。

---

## 1. 为什么要单独写这份

本项目文档、脚本、日志大量中文。在 **Windows PowerShell 5.1** 下，编码默认与「现代 UTF-8 无 BOM」不一致时，会出现：

- `.ps1` 含中文注释/字符串 → 解析报错或乱码  
- 重定向/日志文件中文变成 `����`  
- AI 或编辑器存成 UTF-8 **无 BOM**，5.1 按系统 ANSI（常为 GBK/CP936）误读  

PowerShell **7+** 默认 `utf8NoBOM`，跨平台更友好；**本机当前开发壳仍是 5.1**，必须以 5.1 兼容为底线。

---

## 2. 必须遵守（脚本与中文）

### 2.1 含非 ASCII 的 `.ps1`：保存为 **UTF-8 with BOM**

| 场景 | 要求 |
|------|------|
| 脚本内有中文（注释、`Write-Host`、路径、消息） | 文件编码 = **UTF-8 带 BOM**（`EF BB BF` 开头） |
| 纯 ASCII 的 `.ps1` | UTF-8 无 BOM 或 ASCII 均可 |
| 给 **仅** PowerShell 7+ / 跨 Unix 的脚本 | 可优先无 BOM；若仍要在 5.1 跑且含中文 → **仍要 BOM** |

**原因（完整句）：** Windows PowerShell 5.1 在没有 BOM 时，常把脚本字节按系统本地代码页解释；UTF-8 中文会被拆错，导致语法错误或错误字符串。BOM 让 5.1 正确识别为 UTF-8。

**Microsoft 原文要点：** 若脚本含非 ASCII，保存为 UTF-8 **with BOM**；无 BOM 时 Windows PowerShell 可能误判。

### 2.2 在编辑器里怎么设

| 工具 | 建议 |
|------|------|
| VS Code / Cursor | 右下角编码 →「通过编码保存」→ **UTF-8 with BOM**；工作区可对 `*.ps1` 设 `"files.encoding": "utf8bom"` |
| 记事本 | 另存为 → 编码选 **UTF-8**（Win10/11 记事本「UTF-8」通常带签名；若有「UTF-8 有 BOM」选项则选它） |
| AI 写文件 | 若工具默认无 BOM：用下面 §3 的命令重写一遍带 BOM 的文件 |

### 2.3 用 PowerShell 写出「UTF-8 有 BOM」文件

**Windows PowerShell 5.1：**

```powershell
# Out-File / Set-Content -Encoding utf8 在 5.1 中通常会写带 BOM 的 UTF-8
'Write-Host "你好"' | Out-File -FilePath .\example.ps1 -Encoding utf8

# 或显式：
$utf8Bom = New-Object System.Text.UTF8Encoding $true
[System.IO.File]::WriteAllText((Resolve-Path .).Path + '\example.ps1', "Write-Host `"你好`"`n", $utf8Bom)
```

**PowerShell 7+：** `-Encoding utf8` 常为**无 BOM**；需要 BOM 时用：

```powershell
$utf8Bom = New-Object System.Text.UTF8Encoding $true
[System.IO.File]::WriteAllText("$PWD\example.ps1", "Write-Host `"你好`"`n", $utf8Bom)
# 或 -Encoding utf8BOM（7+ 支持的命名以本机 Get-Help 为准）
```

### 2.4 读取文件时指定编码

```powershell
# 读 UTF-8（含 BOM 文件）
Get-Content -Path .\log.txt -Encoding UTF8

# 读系统 ANSI/GBK 遗留日志（中文 Windows 常见）
Get-Content -Path .\legacy.log -Encoding Default
```

不要假设「管道里永远是 Unicode」；从外部 exe 抓 stdout 时注意控制台代码页。

---

## 3. 控制台显示中文（倾向）

| 做法 | 说明 |
|------|------|
| 终端字体 | 使用能显示中文的字体（如「微软雅黑」、新版 Windows Terminal 默认字体） |
| Windows Terminal | 优先用 WT 而不是旧版 `conhost` 裸窗，减少乱码 |
| 代码页 | 需要时 `chcp 65001`（UTF-8）；可能影响部分仅 ANSI 的老工具，按会话临时改 |
| 输出编码 | 会话内可设：`[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()`；与调用的子进程约定仍可能不一致 |

**质量目标：** 不要求绝对零乱码；要求**仓库内自维护的 `.ps1` 与 docs 日志**在 5.1 下可解析、可复现。

---

## 4. 其它易踩坑（完整句）

1. **路径与工作目录：** 含中文的路径在引号内传递；外部程序参数优先完整路径。  
2. **`&&`：** Windows PowerShell 5.1 **不支持** bash 式 `&&` 链（部分新版本/实验特性除外）；顺序执行用 `;`，或按 `$LASTEXITCODE` 分支。  
3. **执行策略：** 本机脚本可能被 `ExecutionPolicy` 拦住；开发可用 `RemoteSigned` 或对单文件 `Unblock-File`，**不要**在未说明时全局改成 `Unrestricted` 并写入用户配置。  
4. **换行：** 仓库内 `.ps1` 可用 CRLF（Win 习惯）；Git 的 `core.autocrlf` 以本机已有设置为准，避免无说明整库乱改。  
5. **JSON / 配置：** `ConvertTo-Json` 输出编码仍服从重定向编码；写 UTF-8 文件时明确 `-Encoding` 或 `WriteAllText`。  
6. **不要**用「系统改成 Beta: 使用 Unicode UTF-8 提供全球语言支持」作为本项目默认前提（会改变大量无 BOM 文件的解释方式，牵连其它软件）；项目脚本用 **BOM** 自描述编码即可。

---

## 5. 与本项目其它规范的关系

| 层 | 关系 |
|----|------|
| R0 | 不因编码折腾去装旧 Python；Rust/MSVC 安装不受 Python 禁令限制 |
| 日志 | `docs/logs/` 中文用 UTF-8；Markdown 推荐 UTF-8（BOM 可选；MD 读端多支持无 BOM） |
| 产品脚本 | 将来 `publish.ps1` 等若含中文 → **UTF-8 BOM** + 在 DEV_ENV 记验证命令 |

---

## 6. 一页检查清单（写 `.ps1` 前）

1. 脚本里有没有中文或其它非 ASCII？有 → **UTF-8 with BOM**。  
2. 目标壳是 5.1 还是 7+？两端都要跑且含中文 → BOM。  
3. 是否用了 5.1 没有的语法（`&&`、部分三元、PS7 only cmdlet）？  
4. 写文件是否显式指定了编码？  
5. 在本机 5.1 里 `.\script.ps1` 能否无解析错误跑通？
