# 技术规格：Windows 系统通知（可选 · 默认关）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-08（实现批） |
| **状态** | **CODE**（默认关；CLI 可读写；无主人点测 FIXED） |
| **失真声明** | **AI 落盘，有可能出错和失真。** |
| **Inbound** | 主人：允许 exe 弹 Windows 系统通知；在**数据文件夹**配置；**默认没有**（关） |
| **Outbound** | `crates/rustmadoka-app/src/system_toast.rs` · `RustMadoka_data/notifications/system_toast.json` · CLI `notify system-*` |

---

## 1. 产品条件（完整）

1. 可在**程序运行过程中**向 Windows 弹出**系统级**通知（常见为操作中心 / 右下角系统 toast），不是仅浏览器页面内 toast。  
2. 开关与选项写在**数据文件夹**（旁路 `RustMadoka_data`），**默认关闭**，新装无感。  
3. 须有 **CLI** 可读/写该配置（P23：CLI ⊇ 网页）。  
4. 不把系统通知当任务成功的唯一证据；失败仍以日志与 CLI 退出码为准。

---

## 2. 与现有「通知」的区别

| 现有 | 是什么 | 不是 |
|------|--------|------|
| 浏览器 toast | SPA 右下角提示 | 非系统通知 |
| `notify.rs` + `notifications/*.json` | 设置变更历史落盘 | 非系统 toast |
| 本规格 | **Windows 系统通知** | **CODE** 默认关 |

---

## 3. 实现（2026-08-08 CODE）

| 项 | 内容 |
|----|------|
| 配置文件 | `RustMadoka_data/notifications/system_toast.json`：`enabled`（默认 false）、`on_task_success`、`on_task_error` |
| 触发点 | `task_log::finalize_session` 定稿时（一键/单模块/CLI 共用） |
| 技术 | PowerShell + WinRT ToastNotifier；失败只记日志 |
| CLI | `notify system-get` · `notify system-set --enabled true` · `notify system-test` |
| 默认 | **enabled=false** |

---

## 4. 修订

| 墙钟 | 内容 |
|------|------|
| 2026-08-07 23:53 | 首版：与网页 toast 区分；默认关；未实现 |
