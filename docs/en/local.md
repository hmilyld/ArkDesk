# Local Layer (fork-specific)

This repository layers personal tools on top of the **ArkDesk base**. Framework files are left
untouched; personal content is concentrated in the
"local layer" so that it can be synced from the upstream base (see the end of this document). The
base only keeps generic templates; this document covers ArkDesk's actual content.

## Personal Plugins

- `plugins/daily-tools`: file conversion + image OCR + encryption/decryption tools
- `plugins/tender-optimizer`: bid pricing calculation (Monte Carlo)
- `plugins/text2video`: generate portrait scrolling short videos from images and text
- `plugins/network-tools`: API testing + local MITM request interception (hudsucker)

(These directories are all the personal content; deleting a directory removes it completely;
backend commands/migrations are registered automatically at build time.)

## Lifecycle Hooks (local layer implementation location)

The base only defines the contract (`src-tauri/src/lifecycle.rs`) and **contains no implementation**. If you need to intercept
the startup / shutdown phases (such as validating registration before startup or saving data before exit), just implement
`AppLifecycle` in `src-tauri/local/lifecycle.rs` (`pub struct Hooks;`, mounted automatically at build time, a no-op when there is no file). For the contract, the four phases
and the semantics of `Hold`, see [lifecycle.md](lifecycle.md). This file belongs to the fork and is not back-filled from upstream.

## Rust Dependencies (fork-owned)

The `local plugin deps` section at the end of `src-tauri/Cargo.toml` and the two `local plugin target deps`
tables: `calamine` / `rust_xlsxwriter` / `rand` / `rand_distr` / `anytomd` / `lopdf` /
`ab_glyph` / `scraper` / `regex` / `opener` / `ocr-rs`.

Request interception (`plugins/network-tools`) additionally pulls in: `hudsucker` (MITM proxy), `http-body-util`,
`flate2` / `brotli` / `zstd` (decompression of captured bodies). Note: hudsucker's default `tokio-rustls` features
**indirectly pull in `aws-lc-rs`/`aws-lc-sys`** (even though we explicitly use the `ring` provider at runtime); its compilation on
macOS automatically uses a portable build; a Windows build may require CMake/NASM (like `ocr-rs`, this falls under the local layer
toolchain, see below).

The RustCrypto / `gmcrypto-core` dependencies additionally pulled in by the encryption/decryption tools belong to the same section; for the list and the trade-offs see
[`plugins/daily-tools/AGENTS.en.md`](../../plugins/daily-tools/AGENTS.en.md#dependencies-fork-local-layer).

## Resources (fonts / OCR models)

The fonts and OCR models used by the personal tools live in `src-tauri/local-resources/` (the `*.otf` / `*.mnn` binaries are not committed;
the license `fonts/OFL.txt` and the charset `ocr-models/ppocr_keys_v6_small.txt` are committed as text).

- Download: `pnpm assets` (= `scripts/local/prepare.mjs`); you can also download them separately with `pnpm fonts` / `pnpm ocr-models`
- Called automatically by `scripts/prepare.mjs` before `tauri dev` / `tauri build` (skipped by default in a `CI` environment)
- **Release builds** must prefetch explicitly (see `pre-build` in `.github/workflows/release.yml`):
  `npm run fonts && npm run ocr-models`
- Networks in mainland China: goes through the `https://gh.javaing.com/` proxy by default; it can be overridden with `GITHUB_PROXY` (empty = direct connection),
  `FONT_SOURCE_BASE` / `OCR_MODEL_SOURCE_BASE` (mirrors)
- Sources are pinned: fonts `notofonts/noto-cjk@Sans2.004`; OCR `zibo-chen/rust-paddle-ocr@v2.4.1`
- Checksums are in `scripts/local/fetch-*.mjs`; **upgrading a version requires updating the SHA-256 in sync**

## Build System Dependencies (ocr-rs)

- **macOS**: when `cc-rs` cannot find the C++ headers in the SDK, set
  `export CXXFLAGS="-std=c++14 -I$(xcrun --sdk macosx --show-sdk-path)/usr/include/c++/v1"` in `~/.zshrc`
  (equivalent to `pnpm env:cpp`)
- **Windows**: `bindgen` needs libclang → restart the terminal after `winget install LLVM.LLVM`
  (`LIBCLANG_PATH` is set automatically; or `set LIBCLANG_PATH=C:\Program Files\LLVM\bin`)
- The CI and release workflows already include the corresponding steps (triggered by `native-cpp: true` at release time); no CI changes are needed

## Syncing Updates from the base

```bash
git remote add upstream <base repo URL>   # first time
git fetch upstream
git merge upstream/main
```

The personal content is concentrated in the locations above; merges generally have no conflicts. If
`Cargo.lock` (which includes the personal dependencies) conflicts, just run `cargo build` to regenerate it.

## Release (personal use)

For the complete process see [release.md](release.md). Key points:

- **Automatic (recommended)**: push a `vX.Y.Z` tag or run `.github/workflows/release.yml` manually; you need to configure the repository
  Variable `UPDATE_BASE_URL` and the Secrets `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- **Manual**: generate a key → fill in `plugins.updater.pubkey` (ArkDesk already has `createUpdaterArtifacts` enabled) →
  `pnpm assets && TAURI_SIGNING_PRIVATE_KEY_PATH=~/.tauri/arkdesk.key pnpm tauri build` →
  `pnpm release -- --base-url <update server URL> --changelog src/content/changelog.md`.
