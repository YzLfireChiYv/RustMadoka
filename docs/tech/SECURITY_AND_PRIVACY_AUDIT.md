# 隐私与安全审计（软件内 + 软件外）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-06；§0.1 纠正 2026-08-07 07:28 |
| **范围** | Win Rust 主程序 · 本地数据 · 公开仓 RustMadoka · 协作纪律 |
| **性质** | 审计与风险登记；**不是**渗透报告或认证 |
| **MAY CONTAIN ERRORS** | Yes — 以源码与部署为准 |
| **Outbound** | `crates/rustmadoka-core/src/account.rs` · `gree.rs`（token 路径）· `crates/rustmadoka-app` 绑定/会话 · `automadoka_data/` 布局 |
| **Inbound** | [HANDOFF.md](../HANDOFF.md) · [TASKBOARD.md](../TASKBOARD.md) · [NORMS.md](../NORMS.md) P8–P10 · 教训 C12 |

---

## 0. 总评（一页）

| 维度 | 等级 | 说明 |
|------|------|------|
| **威胁模型** | 本机辅助工具 | 协作侧：误传 git、误开公网。产品**不**把本机读盘防护当优化目标（见下） |
| **设计意图** | 不云存用户引继/密码 | 符合产品红线 |
| **当前整体** | **可接受（本机 loopback）** | 及格加密用户组 + 默认 loopback **已够**；**不再**把 S2/S3/S6 当优先加固 |
| **公开仓** | **干净快照已推** | 无 `automadoka_data`/exe；勿再推带上游 OAuth 历史的分支 |

**不要期望：** 对抗本机 root/管理员、或把工具伪装成银行级零信任。

### 0.1 产品安全投入边界（2026-08-07 07:28 主人钉死 · 纠正）

权威条文：[NORMS.md](../NORMS.md) **P8 / P8b / P9 / P9b / P10** · 教训 C12。

| 事实 | 产品结论 |
|------|----------|
| 游戏账号 / token **不值钱**；不考虑黑客攻击、本机数据泄露威胁模型 | **永远允许**本地明文保存 token、引继、密码等 |
| CLI / 脚本 / 跨端测试需要直接读文件 | **能明文就别加密**；加密是自找麻烦 |
| 母项目明文存密码等 | 本 fork **不必**更严的本地防护 |
| 端范围 | **Windows · macOS · Arch Linux · OpenWrt** 同口径、永久 |
| 审计表 S1–S3、S5–S6 等本机加固 | **登记即可**；**默认不排期**；仅点名 |
| **唯一必须的防护** | ① 不把本机账号材料推远程仓；② 不把 data 打进分发 exe/安装包 |
| 另保留（产品防呆） | 防**误耗游戏资源**的默认全关/二次确认；同 data 单 Owner |

本文其余章节保留技术事实与风险登记。S* 本机加固默认不排期。本地明文 token 符合 NORMS **P9**。

---

## 1. 数据资产清单

| 资产 | 位置 | 敏感度 | 保护现状 |
|------|------|--------|----------|
| 游戏引继码 + 游戏密码 | `automadoka_data/users/*.json` | **极高** | 加密组：AES-256-GCM + PBKDF2；**明文组：磁盘明文** |
| 用户组密码 | 仅内存 `session_password` / 登录时输入 | 高 | 不落盘明文（加密组） |
| Gree token + 设备私钥 | `cache/token/*.json` | **极高** | 文件明文落盘；`.gitignore` 挡 `*token*.json` |
| 设备画像 | `cache/device_profile.json` | 中 | 固定设备串，降低多端感；可关联账号 |
| 昵称/等级缓存 | users JSON | 中 | 加密组一并加密 |
| 模块设置/二次确认 | users JSON `config` | 低～中 | 明文；剪贴板导出不含凭证 |
| 任务日志 | `task_logs/` | 中 | 可能含模块输出/游戏侧文案；完整日志定稿后可读 |
| 设置通知历史 | `notifications/` | 低 | 变更后状态 |
| Web 会话 token | 内存 + `localStorage` `token` | 高 | 仅本机浏览器；**非 HttpOnly Cookie** |
| 指纹 JSON | `publish/` · cache | 低 | 公开可下；非用户隐私 |
| 构建 stamp | app_runtime.json | 低 | 本地 |

---

## 2. 软件内（实现）

### 2.1 做得好的

| 点 | 说明 |
|----|------|
| 绑定 `127.0.0.1` | 默认不监听 0.0.0.0 |
| 加密用户组 | vault + 随机 salt/nonce + PBKDF2 迭代 |
| 复制/导出配置 | 明确不写引继码/游戏密码 |
| 导入设置 | 过滤 `username`/`password`/`code`/`migration` 键 |
| 同 data 单 Owner | 降低双写 users JSON |
| 引继码级任务互斥 | 降低并发打服竞态 |
| 二次确认默认罩非低风险 | 降低误耗资源 |
| 日常默认全关、商店 0 | 产品层安全默认 |
| 门禁 `ALLOW_TOOL_RUN` | 洗词条等写操作默认关 |

### 2.2 风险与建议（按优先级）

| ID | 风险 | 等级 | 建议（未来任务） |
|----|------|------|------------------|
| **S1** | **明文用户组**磁盘明文存引继+密码 | 高（本机） | UI 强提示「本机明文」；文档劝加密组；可选启动时扫描明文组警告 |
| **S2** | **Web `x-token` 存 localStorage**，CORS `permissive` | 中 | 本机风险可控；可改为 HttpOnly Cookie + 收紧 CORS 仅 127.0.0.1；token 过期/登出吊销 |
| **S3** | **Gree token 文件明文**（含私钥材料） | 高（本机） | 可选用用户组密钥再包一层；文件 ACL 仅当前用户 |
| **S4** | 部分 API 仅依赖「持有 x-token」 | 中 | 会话绑定 group；敏感操作二次鉴权；禁止 token 枚举（UUID 已够长） |
| **S5** | 任务日志可能含业务输出 | 中 | 日志脱敏（引继片段、token）；导出日志默认打码 |
| **S6** | IPC 命名管道无鉴权（同机任意进程可连） | 中～高 | 管道握手：Owner 写随机 secret 到 data 仅限当前用户读；Client 带 secret |
| **S7** | 剪贴板导出设置 | 低 | 已无密码；提醒用户勿贴到公网 |
| **S8** | 客户端内嵌游戏侧签名密钥材料 | 已知 | 与官服客户端同类；无法真正保密；保持不上传用户凭证 |
| **S9** | 改密后会话 | 中 | 产品已定：改密须重登；实现需确保旧 token 失效（若改密 API 未齐则列任务） |
| **S10** | 删除卡片不删 token 文件 | 低～中 | 删卡时可选清理对应 token 缓存 |

### 2.3 网络与更新

| 点 | 现状 | 建议 |
|----|------|------|
| 指纹/版本拉取 | HTTPS raw GitHub | 校验 JSON schema；可选 pinned host |
| 自更新 | 规划中 | **必须** sha256 + HTTPS；禁止静默降级 |
| 13200 | 警告+确认 | 保持 |

---

## 3. 软件外（协作 · 仓库 · 本机）

### 3.1 公开仓 `YzLfireChiYv/RustMadoka`

| 检查 | 结果 |
|------|------|
| 是否公开 | 是 |
| 是否含 users/token/exe/data | **否**（干净快照推送） |
| 上游 collab 全历史 | **未**推到 RustMadoka（因 OAuth 字符串触发 push protection） |
| 指纹 `publish/automadoka.json` | 可公开（非用户隐私） |
| 文档是否含测试引继 | **禁止**；`plan/` gitignore |

### 3.2 本机协作风险

| 风险 | 说明 | 建议 |
|------|------|------|
| `origin` 仍指 cc004 | 误 `git push origin` 可能推母项目 | 推送只用 `rustmadoka`；文档写清 |
| `collab` 分支含上游历史 | 勿 force 推到公开仓 | 公开仓仅维护干净 main / orphan |
| `gh auth` 在本机 | token 在 keyring | 勿把 `GH_TOKEN` 写入仓库 |
| 本地 `plan/local-test-accounts.md` | 测试号 | 保持 gitignore |
| 根目录 XAPK | 大文件 | gitignore `*.xapk` |
| 对话/截图 | 可能含昵称 | 分享日志前脱敏 |

### 3.3 合规与游戏 ToS（非技术但产品）

- 本工具模拟客户端协议，存在 **游戏服务条款** 风险（封号等）——产品层需主人自担；文档可简短声明「自用风险」。  
- 不代肝公网、不云存账号——降低被滥用面。

---

## 4. 威胁场景速查

| 场景 | 后果 | 缓解 |
|------|------|------|
| 家人共用 PC 开明文组 | 可读引继 | 用加密组 + Windows 用户隔离 |
| 恶意扩展读 localStorage | 可读 x-token，调本机 API | loopback；仍建议短会话/登出清 token |
| 误把 automadoka_data 打 zip 上传 | **灾难** | gitignore；发版检查清单 |
| 误推 collab 全历史到公开仓 | 触发密钥扫描/泄露客户端 OAuth 串 | 只用干净历史；已发生过拦截属好事 |
| 局域网扫描 13220 | 默认仅 127.0.0.1 **不可达** | 勿改 bind 为 0.0.0.0 |

---

## 5. 发版 / 交接检查清单（每次发布）

```text
[ ] git status 无 automadoka_data / *.exe / plan / token json
[ ] 推送目标 remote = rustmadoka（非 origin/cc004）
[ ] RELEASES.json 与 build_stamp 一致；sha256 已填
[ ] 文档未粘贴真实引继/密码
[ ] 加密组路径冒烟；明文组 UI 有「本机明文」提示
```

---

## 6. 公开仓再审（2026-08-06 · 二次）

| 检查项 | 结果 |
|--------|------|
| `automadoka_data/` / exe / token json / plan / xapk | **未跟踪**；gitignore 有效 |
| 游戏密码 / 用户组密码明文 | **未**在公开跟踪文件中发现 |
| GitHub token / 私钥 PEM | **未**发现 |
| `publish/automadoka.json` | 仅 version/sign/libcount（协议指纹，非用户隐私） |
| Gree `app_id` / `app_secret`（源码） | **有** — 与官服客户端同类材料（S8）；**不是**用户凭证 |
| **用户引继码** | **曾泄露**：`docs/logs/2026-08-06-info-hang-fix.md` 曾写真实引继全文（工作区已 `[REDACTED]`） |
| **角色昵称/等级/资源** | **曾泄露**：多份 logs 含真实角色名与资源数值（工作区已脱敏） |
| `automadoka_data_backup_*` | **曾未 ignore**（脚枪）；已补 `/automadoka_data_backup*/` |
| 未跟踪 `archive/pre-rust-…` 整树 | 勿 `git add archive/` 整包；公开仓仅保留 STATEMENT/MIGRATED |
| **git 历史** | 公开 `main` 基线提交仍含脱敏前 blob；**仅改工作区不够**，须改写历史或接受残留 |

**处置状态：** 工作区已 REDACT；推送与历史清洗 **待主人确认**（force-push 须授权）。

**日志：** [docs/logs/2026-08-06-public-repo-privacy-rescan.md](../logs/2026-08-06-public-repo-privacy-rescan.md)

---

## 7. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-06 | 首版审计 + 风险 ID S1–S10 |
| 2026-08-06 | §6 公开仓再审：引继/昵称泄露 + backup ignore + 历史残留说明 |
| 2026-08-06 | §0.1 产品投入边界：本地安全不加码；与 NORMS §0.1 对齐 |
