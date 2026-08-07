# 给其它 AI / 低价 API 模型的跟进手册（Upstream Playbook）

| 项 | 内容 |
|----|------|
| **墙钟** | 2026-08-06 |
| **用途** | 换模型、换会话、便宜 API 时 **不靠聊天记忆** 仍能正确改本仓库 |
| **语言** | 完整句；禁止用对仗句发明立场（见 NORMS / AI_PROJECT_NORMS_PORTABLE） |
| **MAY CONTAIN ERRORS** | Yes — **真机与当前源码优先** |

---

## 0. 你是谁、先读什么（5 分钟）

```text
1. docs/HANDOFF.md          ← 唯一完整现状入口
2. docs/NORMS.md            ← 规则 G/P + 案例 + 索引（短）
3. docs/LESSONS.md          ← 教训索引；改 Gree/AES 打开 L1/L2 正文
4. 本任务相关 PLAN_*.md     ← 只做点名阶段
5. docs/tech/ 对应专题      ← 协议/UI/实例/Android
6. 再改 crates/ 或 static/
```

**禁止：** 未读 HANDOFF 就重写架构；未读 L1/L2 就改 Gree/AES。  
**分层：** NORMS 不堆教训全文；教训字段见 LESSONS.md。
---

## 1. 证据优先级（写结论时）

```text
1 真机 / CLI 可复现输出（带墙钟）
2 当前源码 crates/ · static/
3 archive/pre-rust-2026-08 对照（只读）
4 docs/HANDOFF · PLAN · tech
5 docs/logs
6 聊天记录
```

无测不写 FIXED /「用户已可用」。CLI 冒烟可写「CLI 已验证」。

---

## 2. 仓库地图（改哪里）

| 路径 | 角色 | 双端？ |
|------|------|--------|
| `crates/rustmadoka-core/` | 协议、账号、日常模块、指纹 | **Win+Android 共用** |
| `crates/rustmadoka-app/` | Win exe、axum、IPC、CLI | Win |
| `crates/rustmadoka-app/static/` | Web SPA | **尽量共用** |
| `docs/` | 交接与任务书 | 人类+AI |
| `archive/pre-rust-2026-08/` | Python 对照 | **只读** |
| `publish/` | 指纹与 RELEASES 草案 | 发版 |
| `automadoka_data/` | 运行时数据 | **不进 git** |

**新功能默认落点：** core 或 static；不要只塞进 Win `main.rs` 导致 Android 无法复用（见 PLAN_ANDROID S1）。

---

## 3. 产品硬约束（背这几条即可）

1. 用户 **不** 主路径导 XAPK；指纹三元组上云。  
2. **不** 云存引继码/密码。  
3. 日常默认 **全关**；商店优先级默认 0；门禁 `safety.rs`。  
4. 同 `automadoka_data` **单 Owner**；打服互斥键 **channel+引继码**。  
5. 加密组无用户组密码 = 解不开本地数据 = 不能操作。  
6. 不破坏主人日常包 `com.automadoka.app`。  
7. 版本产品号 = **构建时间到分钟** `build_stamp`。  
8. Android 未齐基础清单前，不大开新玩法（主人可推翻）。

---

## 4. 协议雷区（改登录/请求前必读）

完整版：[LESSONS_RUST_PORT.md](./LESSONS_RUST_PORT.md)

| ID | 一句话 |
|----|--------|
| L1 | Gree initialize = HMAC；之后 RSA Prehashed SHA1 |
| L2 | AES key 派生结果 `/TZh+1VxrtkNiDEH`；msgpack 形状对齐 Python |
| L3 | 洗词条角色来自登录后 mst |
| L4 | 指纹 version/sign/libcount |
| L8 | sm 格式钉死 |

对照源码：`archive/.../sdk/greeclient.py` · `crypto` · `crates/.../gree.rs` · `crypto.rs`。

---

## 5. 任务怎么开工（标准循环）

```text
A. 读 HANDOFF §0 → 确认本阶段 PLAN
B. 读 tech 对应篇 + Outbound 源码路径
C. 实现（小步；注释写文档路径）
D. cargo build -p automadoka-app --release
E. 覆盖 C:\GrokProject\automadoka\automadoka.exe（NORMS 允许时可杀进程）
F. CLI 冒烟：run info -g … -a … --json（门禁允许时）
G. 写 docs/logs/墙钟-主题.md
H. 更新 HANDOFF 至少三行
```

**备忘 ≠ 开工。** 主人未点名不做 R4 托管/Android 实现等。

---

## 6. 文档双向链接

- 新模块文件头：`//! 文档: docs/tech/….md`  
- 改 PLAN 时改 tech Inbound  
- 日志必须含：改了什么、为何、测了啥、没测啥  

---

## 7. Android 跟进时额外步骤

1. 读 [PLAN_ANDROID_AND_DUAL_PLATFORM.md](../PLAN_ANDROID_AND_DUAL_PLATFORM.md) §2 清单 B1–B10  
2. 读 [ANDROID_DUAL_PLATFORM.md](./ANDROID_DUAL_PLATFORM.md)  
3. **先**确认改动是否进 core/static；**禁止**只改 Kotlin 复制协议  
4. 验收写 Win + Android 两行  

---

## 8. 常见错误（直接禁止）

| 错误 | 正确 |
|------|------|
| 把 archive Python 当主工程改 | 只对照；产品在 crates |
| 系统装旧 Python | 便携 embed；见 NORMS |
| 无测写 FIXED | 写「未测」 |
| 双开同 data 两个 serve | Owner 锁 |
| URL 放引继码 | 禁止 |
| 假设聊天里的账号进 git | plan/ 与 data 不提交密钥 |

---

## 9. 便宜模型提示词建议（主人可复制）

```text
你在仓库 automadoka collab。先读 docs/HANDOFF.md 与 docs/NORMS.md。
只做主人点名的 PLAN 文件中的条目。协议坑读 docs/tech/LESSONS_RUST_PORT.md。
新业务优先 crates/rustmadoka-core 与 static。做完写 docs/logs 并更新 HANDOFF。
无真机测不写 FIXED。
```

---

## 10. 修订

| 日期 | 内容 |
|------|------|
| 2026-08-06 | 首版：给跨模型持续更新用 |
