# RustMadoka crates

| Crate | 角色 |
|-------|------|
| `rustmadoka-core` | 协议（Gree/AES/指纹）、账号存储、日常模块、组队、诊断 |
| `rustmadoka-app` | Windows 优先：CLI、Owner HTTP、静态浏览器网页前端、TaskGate |
| `rustmadoka-mobile` | Android JNI 壳 → 同源 `run_embedded_serve` |

文档：`docs/HANDOFF.md` · `docs/PLAN_RUSTMADOKA_FULL_REWRITE.md` · `docs/tech/`。

## 构建

```bat
powershell -File scripts\build-win-dual.ps1
```

产物：仓库根 **`RustMadoka.exe`**（普通）与 **`RustMadoka_debug.exe`**（`wire_record`）。  
数据：旁路 **`RustMadoka_data/`** · 默认端口 **14103**。

## 范围（诚实）

- 日常约 26 模块、流式进度、洗词条、指纹 rules+内嵌+槽、本机 Web。  
- 无主人点测不写 FIXED。台服 Sonet 登录未实现。
