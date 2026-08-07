# 发布指纹与版本（上传 GitHub）

**发行仓（定稿）：** https://github.com/YzLfireChiYv/RustMadoka  

可清空仓内旧功能树，只保留本目录产物 + Releases 资产。详见 `docs/PLAN_RELEASE_AND_SELF_UPDATE.md`。

## 文件

| 文件 | 用途 |
|------|------|
| **`automadoka.json`** | 指纹（en/jp）；raw 路径 `…/main/publish/automadoka.json` |
| **`RELEASES.json`** | 最新 build_stamp / 下载 URL / sha256（Web 版本检测） |

## 当前内容（本机 XAPK 3.13.0 实提，2026-08-06）

| channel | package_id | version | sign | libcount |
|---------|------------|---------|------|----------|
| en | com.aniplex.magia.exedra.en | 3.13.0 | d929c89a96c474de5772d47848491c00 | 34 |
| **jp** | com.aniplex.magia.exedra.jp | 3.13.0 | **ac5fe842e97b0d5f5c129755c0c63d71** | 34 |

源包：`run-clean/install-packages/{en,jp}/MagiaExedra_*_3.13.0.xapk`

## 主人上传步骤

1. 打开 GitHub 仓库 `rules`（或你配置的指纹仓）  
2. 用本文件 **整体覆盖** 仓库中的 `automadoka.json`  
3. 浏览器打开  
   `https://raw.githubusercontent.com/YzLfireChiYv/rules/main/automadoka.json`  
   确认能看到 `channels.jp`  
4. 客户端：`automadoka.exe` 启动后选日服，或  
   `automadoka fetch-fp --channel jp`

## 以后游戏更新后重提

```bat
cd /d c:\GrokProject\automadoka
call "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
cargo build -p automadoka-app --release
target\release\automadoka.exe build-fp ^
  --xapk en=run-clean\install-packages\en\MagiaExedra_en_x.y.z.xapk ^
  --xapk jp=run-clean\install-packages\jp\MagiaExedra_jp_x.y.z.xapk ^
  --default en --out publish\automadoka.json
```

再上传 `publish\automadoka.json`。
