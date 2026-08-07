# 任务书：发行仓 · 版本号（构建分钟）· Web 控自更新

| 项 | 内容 |
|----|------|
| **状态** | **规划已定 · 版本检测本批落地 · Web 一键更新本批骨架/下批完善** |
| **墙钟** | 2026-08-06 |
| **发行仓** | https://github.com/YzLfireChiYv/RustMadoka （可清空旧内容后只放本协作产物） |
| **后续** | 更复杂功能等 Android 版再说 |

---

## 1. 版本号规则（正式启用）

| 规则 | 说明 |
|------|------|
| **主版本标识** | **构建时间**，精确到**分钟** |
| **格式** | `YYYY.MM.DD.HHMM`（默认按 +08:00 生成；CI 可设 `AUTOMADOKA_BUILD_STAMP`） |
| **写入** | 编译期 `build.rs` → `env!("AUTOMADOKA_BUILD_STAMP")` |
| **落盘** | 启动时写入 `automadoka_data/app_runtime.json`：`build_stamp`、`exe_path`、`started_at` |
| **展示** | Web `/api/version`、关于/设置区、CLI `--version`（可选） |

Cargo `package.version` 可保留为兼容字段，**产品以 build_stamp 为准**。

---

## 2. 从 GitHub 获取「当前版本信息」

| 能力 | 说明 |
|------|------|
| 默认源 | `YzLfireChiYv/RustMadoka` raw：`publish/automadoka.json`（指纹）+ `publish/RELEASES.json`（发行说明） |
| 添加自定义信息源 | `app.json` → `info_sources[]`；Web 可增删 |
| 手动粘贴 | `manual_version_note` 或粘贴整段 JSON 解析指纹 |
| 拉取失败 | 人话错误 + 仍可用本地缓存 |

`RELEASES.json` 草案：

```json
{
  "schema": 1,
  "latest": {
    "build_stamp": "2026.08.06.1530",
    "asset": "automadoka.exe",
    "url": "https://github.com/YzLfireChiYv/RustMadoka/releases/download/…/automadoka.exe",
    "sha256": "…",
    "notes": "修复…"
  }
}
```

---

## 3. Web 控制 .exe 自动更新（规划 → 实现阶段）

| 阶段 | 交付 |
|------|------|
| **本批** | 版本检测 API + UI 展示本地/远端；信息源管理；**不强制**一键替换 exe（可先「打开下载页/提示」） |
| **下批** | Web 点「更新」→ 下载到临时文件 → 校验 sha256 → 写更新脚本 → 退出后替换根目录 exe 并重启（Windows） |
| **安全** | 仅 https；校验哈希；失败可回滚；不静默强更 |

依赖：GitHub Releases 资产命名约定 + `RELEASES.json` 维护。

---

## 4. 发行仓清空与上传（主人验收通过后）

仓库可清空历史功能树，只保留：

```text
README.md
publish/automadoka.json
publish/RELEASES.json
（可选）docs 摘要
Releases: automadoka.exe 按 build_stamp 打 tag
```

本机 collab 完整源码仍在 `C:\GrokProject\automadoka`；上传策略由主人定（整仓推送或只推 publish + release 资产）。

---

## 5. 明确不做（本阶段）

- Android  
- 复杂托管/R4 全量  
- 与「版本/卡片/删号」无关的新业务  

## 6. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-06 | 定稿；build_stamp 本批启用 |
