# 本地层（fork 专属）

本仓库在 **ArkDesk base** 之上叠加了自用工具。框架文件不做改动；个人内容集中在
「本地层」，以便从上游 base 同步更新（见文末）。base 只保留通用模板，本文件为 ArkDesk 实际内容。

## 个人插件

- `plugins/daily-tools`：文件转换 + 图片 OCR
- `plugins/tender-optimizer`：投标报价测算（蒙特卡洛）
- `plugins/text2video`：图文生成竖屏滚动短视频

（这些目录即全部个人内容，删除目录即彻底移除；后端命令/迁移为构建期自动注册。）

## Rust 依赖（fork-owned）

`src-tauri/Cargo.toml` 末尾的 `local plugin deps` 段与两张 `local plugin target deps`
表：`calamine` / `rust_xlsxwriter` / `rand` / `rand_distr` / `anytomd` / `lopdf` /
`ab_glyph` / `scraper` / `regex` / `opener` / `ocr-rs`。

## 资源（字体 / OCR 模型）

个人工具使用的字体与 OCR 模型放在 `src-tauri/local-resources/`（`*.otf` / `*.mnn` 二进制不入库；
许可 `fonts/OFL.txt` 与 charset `ocr-models/ppocr_keys_v6_small.txt` 文本入库）。

- 下载：`pnpm assets`（= `scripts/local/prepare.mjs`）；也可 `pnpm fonts` / `pnpm ocr-models` 单独下载
- `tauri dev` / `tauri build` 前由 `scripts/prepare.mjs` 自动调用（`CI` 环境默认跳过）
- **发布构建**须显式预取（见 `.github/workflows/release.yml` 的 `pre-build`）：
  `npm run fonts && npm run ocr-models`
- 国内网络：默认经 `https://gh.javaing.com/` 代理；可用 `GITHUB_PROXY`（置空=直连）、
  `FONT_SOURCE_BASE` / `OCR_MODEL_SOURCE_BASE`（镜像）覆盖
- 来源固定：字体 `notofonts/noto-cjk@Sans2.004`；OCR `zibo-chen/rust-paddle-ocr@v2.4.1`
- 校验值在 `scripts/local/fetch-*.mjs`；**升级版本必须同步 SHA-256**

## 编译系统依赖（ocr-rs）

- **macOS**：`cc-rs` 找不到 SDK 中的 C++ 头文件时，在 `~/.zshrc` 设置
  `export CXXFLAGS="-std=c++14 -I$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1"`
  （等价 `pnpm env:cpp`）
- **Windows**：`bindgen` 需要 libclang → `winget install LLVM.LLVM` 后重启终端
  （`LIBCLANG_PATH` 自动设置；或 `set LIBCLANG_PATH=C:\Program Files\LLVM\bin`）
- CI 与发布 workflow 已内置相应步骤（发布时经 `native-cpp: true` 触发），无需改 CI

## 从 base 同步更新

```bash
git remote add upstream <base 仓库地址>   # 首次
git fetch upstream
git merge upstream/main
```

个人内容集中在上述位置；合并一般无冲突。`Cargo.lock`（含个人依赖）如冲突，执行
`cargo build` 重新生成即可。

## 发布（自用）

完整流程见 [RELEASE.md](RELEASE.md)。要点：

- **自动（推荐）**：推送 `vX.Y.Z` tag 或手动运行 `.github/workflows/release.yml`；需配仓库
  Variable `UPDATE_BASE_URL` 与 Secrets `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。
- **手动**：生成密钥 → 填 `plugins.updater.pubkey`（ArkDesk 已开启 `createUpdaterArtifacts`）→
  `pnpm assets && TAURI_SIGNING_PRIVATE_KEY_PATH=~/.tauri/arkdesk.key pnpm tauri build` →
  `pnpm release -- --base-url <更新服务器地址> --changelog src/content/changelog.md`。
