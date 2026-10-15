# Packaging, Release and Online Updates

This document is the single source of truth for **app packaging, release and online updates**; the README only keeps a capability summary.
The base provides a **reusable release workflow**, and a fork only needs a caller of a few lines; fork-specific items
(prefetched resources, C++ build dependencies, keys) are documented in [`local.md`](local.md).

> Related divisions of labour: [start.md](start.md) = starting from zero; [extending.md](extending.md) = architecture and extension;
> [AGENTS.md](../../AGENTS.en.md) = development conventions and pitfalls; [local.md](local.md) = the fork local layer;
> **this document = release and updates**.

---

## 1. Overview

- Update capability is provided by the framework: frontend `src/core/updater`, Rust `src-tauri/src/updater.rs`
  (commands `updater_check` / `updater_install` / `updater_restart`), backed by the official
  `tauri-plugin-updater` + a self-hosted static manifest.
- Release artifacts = installer packages for each platform + the corresponding `.sig` (minisign signature) + the static manifest `latest.json`
  (plus a generated `checksums.txt`).
- Distribution: put the installers and `latest.json` in a directory that is **anonymously accessible over HTTPS** (the update server);
  when the app starts (or manually) it requests `<server address>/latest.json`.
- The single source of truth for the version = `src-tauri/tauri.conf.json > version`; update selection is strict semver,
  the **remote must be > the local**, therefore the version number must be **monotonically increasing**.
- The signature public key is pinned in `tauri.conf.json > plugins.updater.pubkey` (the root of trust, not changeable from the UI);
  the update server address is provided at runtime by the `updateServerUrl` setting.

> **minisign update signing** (this process) and the **operating system code signing** of macOS / Windows are two different things
> and do not replace each other. This process only does the former (required for Tauri update verification); the latter is optional, see §11.

## 2. One-time configuration

### 2.1 Generate the update signing key

```bash
pnpm tauri signer generate -w ~/.tauri/<app>.key   # will prompt for a passphrase (may be left empty)
```

- Private key: `~/.tauri/<app>.key` (**never committed**, be sure to back it up).
- Public key: `~/.tauri/<app>.key.pub`.

### 2.2 Fill in the public key, enable update artifacts

Edit `src-tauri/tauri.conf.json`:

```json
{
  "bundle": { "createUpdaterArtifacts": true },
  "plugins": {
    "updater": {
      "pubkey": "<contents of ~/.tauri/<app>.key.pub>"
    }
  }
}
```

> There is no need to fill in `plugins.updater.endpoints` — the update address is always overridden at runtime by the setting.

### 2.3 Give the app a default update address (optional)

When the setting is empty the app will not check automatically. If you want it to work out of the box, you can hard-code your domain in the setting defaults, for example
in `src/stores/settings.ts`:

```ts
const updateEnabled = ref(true);
const updateServerUrl = ref('https://<your-domain>');
```

### 2.4 Prepare the update server

- A static directory that supports **anonymous GET** (no login, no 403).
- Must be **HTTPS** (the updater rejects non-https).
- Put `latest.json` at the path corresponding to the address you configured (if the address is the root domain, then `<your-domain>/latest.json` is requested).
- Self-check: `curl -s https://<your-domain>/latest.json` should return JSON (not a 403/500/login page).

### 2.5 Fork: configure GitHub

- **Variables** (Settings → Secrets and variables → Actions → Variables):
  `UPDATE_BASE_URL = https://<your-domain>`
- **Secrets**:
  - `TAURI_SIGNING_PRIVATE_KEY` = the **contents** of `~/.tauri/<app>.key` (base64, single line, no line breaks)
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` = the passphrase used when generating the key (if there is none, leave empty / do not create it)
- Add the caller `.github/workflows/release.yml` (see §8.2).

## 3. Automatic release (recommended)

### 3.1 Prepare a release

```bash
pnpm version:bump 0.2.0     # syncs tauri.conf.json / Cargo.toml / package.json
# edit src/content/changelog.md, add a new section:
# ## [0.2.0] - YYYY-MM-DD
git add -A && git commit -m "chore: release 0.2.0"
```

### 3.2 Trigger

```bash
git push
git tag v0.2.0              # after removing the v it must equal the version in tauri.conf.json
git push origin v0.2.0
```

Or run the `Release` workflow manually on the Actions page (`workflow_dispatch`), filling in `version` and
`base-url`.

### 3.3 What CI does

1. **Resolve the version**: verify that the tag/input matches `tauri.conf.json.version`; `cargo fmt --check`.
2. **Build matrix**: `macos-14` (aarch64-apple-darwin) and `windows-latest`
   (x86_64-pc-windows-msvc); accelerated by `sccache` + `rust-cache`; optional `pre-build`
   (a command run before the build, e.g. prefetching resources) and `native-cpp` (install LLVM on Windows, set CXXFLAGS on macOS);
   built with signing via `tauri build --target <triple> --bundles app|nsis`.
3. **Collect artifacts**: only the installers and `.sig` files under `*/release/bundle/*` are collected and uploaded as Actions artifacts.
4. **Archive the Release**: download all artifacts → generate a **unified** `latest.json` (multi-platform) + `checksums.txt`
   → create/update the GitHub Release and attach all assets.

Artifact naming:

- macOS: `<Product>.app.tar.gz` + `.sig`
- Windows: `<Product>_<version>_x64-setup.exe` + `.sig`
- Manifest: `latest.json`, `checksums.txt`

## 4. After release: upload to the update server

CI only archives the artifacts to the GitHub Release; the update server must be uploaded to yourself:

```bash
mkdir -p ~/release && cd ~/release
gh release download v0.2.0 --repo <owner>/<repo> --dir .
# upload your way:
scp ./* user@host:/path/to/webroot/
# or: rsync -av ./ user@host:/path/to/webroot/
```

Verify:

```bash
curl -s https://<your-domain>/latest.json
```

## 5. Manual release (fallback)

```bash
# 1) fork prefetches resources first (base has no personal resources, may skip)
pnpm assets                 # = fonts + ocr-models, see local.md

# 2) signed build (explicit --target/--bundles recommended; artifacts in target/<triple>/release/bundle)
TAURI_SIGNING_PRIVATE_KEY_PATH=~/.tauri/<app>.key \
TAURI_SIGNING_PRIVATE_KEY_PASSWORD='<passphrase>' \
pnpm tauri build --target aarch64-apple-darwin --bundles app

# 3) generate the manifest (automatically scans the bundle under target and target/<triple>)
pnpm release -- --base-url https://<your-domain> --changelog src/content/changelog.md
```

When merging multiple platforms you can specify `--platform` explicitly:

```bash
pnpm release -- --base-url https://<your-domain> --version 0.2.0 \
  --platform darwin-aarch64=src-tauri/target/aarch64-apple-darwin/release/bundle/macos/<Product>.app.tar.gz \
  --platform windows-x86_64=src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/<Product>_0.2.0_x64-setup.exe
```

`scripts/release.mjs` parameters:

- `--base-url` (required): the prefix of the URLs in the manifest.
- `--version`: if omitted, read from `tauri.conf.json`.
- `--notes <file|text>` or `--changelog <file>`: the release notes; `--changelog` automatically extracts
  the section for the corresponding version.
- `--platform <key>=<path>`: a manual platform mapping (repeatable); by default it automatically scans
  `target/release/bundle` and `target/<triple>/release/bundle`, recognising
  `.app.tar.gz` / `*-setup.exe` / `.nsis.zip` / `.msi.zip` / `.AppImage.tar.gz`
  and pairing them with the `.sig` of the same name.

## 6. `latest.json` specification

```json
{
  "version": "0.2.0",
  "notes": "## [0.2.0] ...",
  "pub_date": "2026-09-11T00:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "<contents of the .sig file>",
      "url": "https://<your-domain>/<Product>.app.tar.gz"
    },
    "windows-x86_64": {
      "signature": "<contents of the .sig file>",
      "url": "https://<your-domain>/<Product>_0.2.0_x64-setup.exe"
    }
  }
}
```

- `signature`: the **full contents** of the corresponding installer's `.sig` file (not a URL).
- `url`: the absolute HTTPS address of the installer; the file name must match the actually uploaded file.
- Platform keys: `darwin-aarch64` / `darwin-x86_64` / `windows-x86_64` / `linux-x86_64`.
- `pub_date`: RFC 3339.
- `version` must = the build version and must be **>** the installed version, otherwise it will never prompt or will prompt on every startup.

## 7. Division of labour between base and fork

- **base**: `scripts/release.mjs`, `.github/workflows/release-reusable.yml` (reusable),
  this document.
- **fork**: `.github/workflows/release.yml` (caller), keys/variables, the prefetched resources
  in [`local.md`](local.md) and build dependencies.
- **Sync**: after a change in base, the fork runs `git merge upstream/main`, so `release.mjs`, the workflow
  and this document stay consistent between the two.

## 8. Reusable workflow reference

### 8.1 Inputs and secrets

`hmilyld/PocketArk/.github/workflows/release-reusable.yml` (`workflow_call`):

| input            | default  | description                                               |
| ---------------- | -------- | --------------------------------------------------------- |
| `version`        | `''`     | version number; if empty, derived from the triggering tag |
| `base-url`       | required | update server address                                     |
| `pre-build`      | `''`     | command run before the build (e.g. prefetching resources) |
| `native-cpp`     | false    | install LLVM / set CXXFLAGS                               |
| `create-release` | true     | whether to create a GitHub Release archive                |

secrets: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

### 8.2 fork caller example

`.github/workflows/release.yml`:

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
      pre-build: 'npm run fonts && npm run ocr-models' # fork-specific, base may leave empty
      native-cpp: true
    secrets: inherit
```

## 9. Performance and caching

- The two platforms cache separately (`Swatinem/rust-cache`, the cache key includes the job name and the profile): each
  "platform × job type" is **cold the first time**, warm afterwards.
- `sccache` (`SCCACHE_GHA_ENABLED=true`) reuses compilation results across jobs/profiles.
- Reference timings: cold is about 12–35 minutes; once warm macOS ≈ 4 minutes, Windows ≈ 14 minutes.
- Cache invalidation: `Cargo.lock` changes, a Rust toolchain upgrade, expiry after 7 days unused, eviction by GitHub's 10GB per-repository limit,
  and frequent consecutive pushes preempting the cache.

## 10. Troubleshooting

- **`packages field missing or empty` (pnpm)**: `pnpm-workspace.yaml` is missing the `packages:`
  field and pnpm 9's `pnpm store path` errors; CI has switched to **npm** (locally pnpm is still used).
- **Update artifact (`.sig`) not found**: confirm `createUpdaterArtifacts: true` and that the build was signed;
  when using `--target` the artifacts are in `target/<triple>/release/bundle` (`release.mjs` already handles that path).
- **Windows update package**: in Tauri v2 it is `*-setup.exe` (+ `.exe.sig`), **not** `.nsis.zip`.
- **The Release mixes in build intermediates** (such as `build-script-build*.exe`): the collection scope must be limited to
  `*/release/bundle/*`.
- **macOS unsigned package blocked by Gatekeeper**: on first run execute
  `xattr -dr com.apple.quarantine /Applications/<Product>.app`.
- **Server 403/500**: ensure the static directory is anonymously readable and `latest.json` returns 200 + JSON.
- **It prompts for an update every time**: `latest.json.version` does not match the build version, or is not monotonically increasing.

## 11. Security and code signing

- The private key and passphrase are **never committed**; they are kept only in CI Secrets and the local `~/.tauri/`.
- If the passphrase is lost you can only regenerate the key and update `pubkey` (already-released installs will no longer receive updates).
- Operating system code signing (optional, not required by this process):
  - Unsigned on macOS means Gatekeeper quarantine and update-stability risks; integrating Apple Developer ID +
    notarization can solve this at the root;
  - Unsigned on Windows means a SmartScreen prompt.
