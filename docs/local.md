# 本地层（fork 专属）

PocketArk 框架自身**不含任何个人工具、不下载任何资源**。当你从本仓库 fork 出自己的
软件时，个人内容应集中在「本地层」，以便随上游 base 同步更新。base 仓库只保留本文件
作为约定说明；你的 fork 可将其替换为实际内容。

## 放哪里

| 内容                    | 位置                                                                                   |
| ----------------------- | -------------------------------------------------------------------------------------- |
| 个人插件（前后端同处）  | `plugins/<id>/`（构建期自动注册，无需手动登记）                                        |
| 个人 Rust 依赖          | `src-tauri/Cargo.toml` 末尾 `# ── local plugin deps（fork-owned）──` 段                |
| 个人资产（字体/模型/…） | `src-tauri/local-resources/`                                                           |
| 生命周期钩子实现        | `src-tauri/local/lifecycle.rs`（实现 `AppLifecycle`，见 [lifecycle.md](lifecycle.md)） |
| 个人下载/构建脚本       | `scripts/local/`（`scripts/prepare.mjs` 自动调用 `scripts/local/prepare.mjs`）         |
| 浏览器侧额外权限        | `src-tauri/capabilities/local.json`（Tauri 自动发现；base 无此文件）                   |

## 资源与构建

- 资产放 `src-tauri/local-resources/`：`bundle.resources` 已预置 `local-resources/**/*`，
  无需改 `tauri.conf.json`。
- 资源下载写在 `scripts/local/prepare.mjs`：`tauri dev` / `tauri build` 前由
  `scripts/prepare.mjs` 自动执行；CI 环境建议自行跳过下载。
- 如需 C++ 依赖（bindgen / cc）：macOS 自行导出 `CXXFLAGS`（例如
  `export CXXFLAGS="-std=c++14 -I$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1"`）；
  Windows 需安装 LLVM（`LIBCLANG_PATH`）。

## 同步 base

```bash
git remote add upstream <base 地址>
git fetch upstream
git merge upstream/main
```

个人内容集中在上述位置，合并一般无冲突；`Cargo.lock` 冲突时执行 `cargo build` 重建。

## 发布

若启用在线更新：生成自己的密钥（`pnpm tauri signer generate`），将公钥填入
`tauri.conf.json > plugins.updater.pubkey`，开启 `bundle.createUpdaterArtifacts: true`，
带签名构建，并用 `pnpm release` 生成清单（详见 [release.md](release.md)）。
