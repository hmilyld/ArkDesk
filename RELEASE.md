# RELEASE —— 打包、发布与在线更新

本文件是**应用打包、发布与在线更新**的统一事实源；README 只保留能力简介。
base 提供**可复用的发布 workflow**，fork 只需一个几行的 caller；fork 的专属项
（预取资源、C++ 编译依赖、密钥）记录在 [`LOCAL.md`](LOCAL.md)。

> 相关分工：`START.md` = 从零起步；`README.md` = 架构与扩展；`AGENTS.md` = 开发约定与陷阱；
> `LOCAL.md` = fork 本地层；**本文件 = 发布与更新**。

---

## 1. 概览

- 更新能力由框架提供：前端 `src/core/updater`，Rust `src-tauri/src/updater.rs`
  （命令 `updater_check` / `updater_install` / `updater_restart`），底层官方
  `tauri-plugin-updater` + 自建静态清单。
- 发布物 = 各平台安装包 + 对应 `.sig`（minisign 签名）+ 静态清单 `latest.json`
  （另生成 `checksums.txt`）。
- 分发：把安装包与 `latest.json` 放到一个**可匿名 HTTPS 访问**的目录（更新服务器）；
  App 启动时（或手动）请求 `<服务器地址>/latest.json`。
- 版本唯一事实源 = `src-tauri/tauri.conf.json > version`；更新选择为严格 semver，
  **远端必须 > 本地**，因此版本号**单调递增**。
- 签名公钥固化在 `tauri.conf.json > plugins.updater.pubkey`（信任根，界面不可改）；
  更新服务器地址由设置项 `updateServerUrl` 在运行时提供。

> **minisign 更新签名**（本流程）与 macOS / Windows 的**操作系统代码签名**是两回事，
> 互不替代。本流程只做前者（Tauri 更新校验必需）；后者可选，见 §11。

## 2. 一次性配置

### 2.1 生成更新签名密钥

```bash
pnpm tauri signer generate -w ~/.tauri/<app>.key   # 会提示设置口令（可留空）
```

- 私钥：`~/.tauri/<app>.key`（**不入库**，务必备份）。
- 公钥：`~/.tauri/<app>.key.pub`。

### 2.2 填入公钥、开启更新产物

编辑 `src-tauri/tauri.conf.json`：

```json
{
  "bundle": { "createUpdaterArtifacts": true },
  "plugins": {
    "updater": {
      "pubkey": "<~/.tauri/<app>.key.pub 的内容>"
    }
  }
}
```

> 无需填写 `plugins.updater.endpoints`——更新地址始终由设置项在运行时覆盖。

### 2.3 让 App 有默认更新地址（可选）

设置项默认为空时 App 不会自动检查。想开箱即用，可在设置默认值里写死你的域名，例如
`src/stores/settings.ts`：

```ts
const updateEnabled = ref(true);
const updateServerUrl = ref('https://<你的域名>');
```

### 2.4 准备更新服务器

- 静态目录，能**匿名 GET**（无登录、无 403）。
- 必须 **HTTPS**（更新器拒绝非 https）。
- `latest.json` 放在你配置的地址对应的路径（若地址是根域名，则请求 `<域名>/latest.json`）。
- 自检：`curl -s https://<域名>/latest.json` 应返回 JSON（不是 403/500/登录页）。

### 2.5 fork：配置 GitHub

- **变量**（Settings → Secrets and variables → Actions → Variables）：
  `UPDATE_BASE_URL = https://<域名>`
- **Secrets**：
  - `TAURI_SIGNING_PRIVATE_KEY` = `~/.tauri/<app>.key` 的**内容**（base64 单行，无换行）
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` = 生成密钥时的口令（无口令可留空/不建）
- 新增 caller `.github/workflows/release.yml`（见 §8.2）。

## 3. 自动发布（推荐）

### 3.1 准备一次发布

```bash
pnpm version:bump 0.2.0     # 同步 tauri.conf.json / Cargo.toml / package.json
# 编辑 src/content/changelog.md，新增一段：
# ## [0.2.0] - YYYY-MM-DD
git add -A && git commit -m "chore: release 0.2.0"
```

### 3.2 触发

```bash
git push
git tag v0.2.0              # 去掉 v 后必须等于 tauri.conf.json 的 version
git push origin v0.2.0
```

或在 Actions 页面手动运行 `Release` workflow（`workflow_dispatch`），填 `version` 与
`base-url`。

### 3.3 CI 做了什么

1. **解析版本**：校验 tag/输入 与 `tauri.conf.json.version` 一致；`cargo fmt --check`。
2. **构建矩阵**：`macos-14`（aarch64-apple-darwin）与 `windows-latest`
   （x86_64-pc-windows-msvc）；`sccache` + `rust-cache` 加速；可选 `pre-build`
   （构建前命令，如预取资源）与 `native-cpp`（Windows 装 LLVM、macOS 设 CXXFLAGS）；
   以 `tauri build --target <triple> --bundles app|nsis` 带签名构建。
3. **收集产物**：仅收集 `*/release/bundle/*` 下的安装包与 `.sig`，上传为 Actions artifact。
4. **归档 Release**：下载全部产物 → 生成**统一** `latest.json`（多平台）+ `checksums.txt`
   → 创建/更新 GitHub Release 并附上全部资产。

产物命名：

- macOS：`<Product>.app.tar.gz` + `.sig`
- Windows：`<Product>_<version>_x64-setup.exe` + `.sig`
- 清单：`latest.json`、`checksums.txt`

## 4. 发布后：上传到更新服务器

CI 只把产物归档到 GitHub Release，更新服务器需自行上传：

```bash
mkdir -p ~/release && cd ~/release
gh release download v0.2.0 --repo <owner>/<repo> --dir .
# 按你的方式上传：
scp ./* user@host:/path/to/webroot/
# 或：rsync -av ./ user@host:/path/to/webroot/
```

验证：

```bash
curl -s https://<域名>/latest.json
```

## 5. 手动发布（备用）

```bash
# 1) fork 先预取资源（base 无个人资源可跳过）
pnpm assets                 # = fonts + ocr-models，见 LOCAL.md

# 2) 带签名构建（建议显式 --target/--bundles；产物在 target/<triple>/release/bundle）
TAURI_SIGNING_PRIVATE_KEY_PATH=~/.tauri/<app>.key \
TAURI_SIGNING_PRIVATE_KEY_PASSWORD='<口令>' \
pnpm tauri build --target aarch64-apple-darwin --bundles app

# 3) 生成清单（自动扫描 target 与 target/<triple> 下的 bundle）
pnpm release -- --base-url https://<域名> --changelog src/content/changelog.md
```

多平台合并时可用 `--platform` 显式指定：

```bash
pnpm release -- --base-url https://<域名> --version 0.2.0 \
  --platform darwin-aarch64=src-tauri/target/aarch64-apple-darwin/release/bundle/macos/<Product>.app.tar.gz \
  --platform windows-x86_64=src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/<Product>_0.2.0_x64-setup.exe
```

`scripts/release.mjs` 参数：

- `--base-url`（必填）：清单中 URL 的前缀。
- `--version`：缺省读 `tauri.conf.json`。
- `--notes <文件|文本>` 或 `--changelog <文件>`：更新说明；`--changelog` 自动抽取
  对应版本的段落。
- `--platform <key>=<path>`：手动平台映射（可多次）；缺省自动扫描
  `target/release/bundle` 与 `target/<triple>/release/bundle`，识别
  `.app.tar.gz` / `*-setup.exe` / `.nsis.zip` / `.msi.zip` / `.AppImage.tar.gz`
  并配对同名 `.sig`。

## 6. `latest.json` 规范

```json
{
  "version": "0.2.0",
  "notes": "## [0.2.0] ...",
  "pub_date": "2026-09-11T00:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "<.sig 文件内容>",
      "url": "https://<域名>/<Product>.app.tar.gz"
    },
    "windows-x86_64": {
      "signature": "<.sig 文件内容>",
      "url": "https://<域名>/<Product>_0.2.0_x64-setup.exe"
    }
  }
}
```

- `signature`：对应安装包 `.sig` 文件的**完整内容**（不是 URL）。
- `url`：安装包的绝对 HTTPS 地址，文件名需与实际上传文件一致。
- 平台 key：`darwin-aarch64` / `darwin-x86_64` / `windows-x86_64` / `linux-x86_64`。
- `pub_date`：RFC 3339。
- `version` 必须 = 构建版本，且 **>** 已安装版本，否则不会提示或每次启动都提示。

## 7. base 与 fork 的分工

- **base**：`scripts/release.mjs`、`.github/workflows/release-reusable.yml`（可复用）、
  本文件。
- **fork**：`.github/workflows/release.yml`（caller）、密钥/变量、`LOCAL.md` 中的预取资源
  与编译依赖。
- **同步**：base 改动后 fork 执行 `git merge upstream/main`，因此 `release.mjs`、workflow
  与本文件在两者间保持一致。

## 8. 可复用 workflow 参考

### 8.1 输入与密钥

`hmilyld/PocketArk/.github/workflows/release-reusable.yml`（`workflow_call`）：

| input            | 默认  | 说明                             |
| ---------------- | ----- | -------------------------------- |
| `version`        | `''`  | 版本号，留空则由触发 tag 推导    |
| `base-url`       | 必填  | 更新服务器地址                   |
| `pre-build`      | `''`  | build 前执行的命令（如预取资源） |
| `native-cpp`     | false | 安装 LLVM / 设置 CXXFLAGS        |
| `create-release` | true  | 是否创建 GitHub Release 归档     |

secrets：`TAURI_SIGNING_PRIVATE_KEY`、`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。

### 8.2 fork caller 示例

`.github/workflows/release.yml`：

```yaml
name: Release
on:
  push:
    tags: ['v*']
  workflow_dispatch:
    inputs:
      version:
        required: false
        type: string
      base-url:
        required: false
        type: string
permissions:
  contents: write
jobs:
  release:
    uses: hmilyld/PocketArk/.github/workflows/release-reusable.yml@main
    with:
      version: ${{ inputs.version }}
      base-url: ${{ inputs.base-url || vars.UPDATE_BASE_URL }}
      pre-build: 'npm run fonts && npm run ocr-models' # fork 专属，base 可留空
      native-cpp: true
    secrets: inherit
```

## 9. 性能与缓存

- 两个平台各自缓存（`Swatinem/rust-cache`，缓存键含 job 名与 profile）：每个
  「平台 × 任务类型」**首次 cold**，之后暖。
- `sccache`（`SCCACHE_GHA_ENABLED=true`）跨 job/profile 复用编译结果。
- 参考耗时：cold 约 12–35 分钟；暖后 macOS ≈ 4 分钟、Windows ≈ 14 分钟。
- 缓存失效：`Cargo.lock` 变化、Rust 工具链升级、7 天未用过期、GitHub 每仓库 10GB 上限
  淘汰、频繁连续推送抢占缓存。

## 10. 故障排查

- **`packages field missing or empty`（pnpm）**：`pnpm-workspace.yaml` 缺少 `packages:`
  字段，pnpm 9 的 `pnpm store path` 报错；CI 已改用 **npm**（本地仍用 pnpm）。
- **找不到更新产物（`.sig`）**：确认 `createUpdaterArtifacts: true` 且构建带签名；
  使用 `--target` 时产物在 `target/<triple>/release/bundle`（`release.mjs` 已兼容该路径）。
- **Windows 更新包**：Tauri v2 是 `*-setup.exe`（+ `.exe.sig`），**不是** `.nsis.zip`。
- **Release 混入构建中间产物**（如 `build-script-build*.exe`）：收集范围须限定
  `*/release/bundle/*`。
- **macOS 未签名包被 Gatekeeper 拦截**：首次运行执行
  `xattr -dr com.apple.quarantine /Applications/<Product>.app`。
- **服务器 403/500**：确保静态目录匿名可读，`latest.json` 返回 200 + JSON。
- **每次都提示更新**：`latest.json.version` 与构建版本不一致，或未单调递增。

## 11. 安全与代码签名

- 私钥与口令**绝不入库**，只在 CI Secrets 与本地 `~/.tauri/` 中保存。
- 口令丢失只能重新生成密钥并更新 `pubkey`（已发布的安装将无法再收到更新）。
- 操作系统代码签名（可选，非本流程必需）：
  - macOS 未签名会有 Gatekeeper 隔离与更新稳定性风险，接入 Apple Developer ID +
    公证可根治；
  - Windows 未签名会有 SmartScreen 提示。
