# ArkDesk

> [中文](README.md) · English

A personal desktop toolbox (Mac / Windows), built on the **PocketArk framework base**
(Tauri 2 + Vue 3 + TypeScript + Tailwind 4).

## Built-in tools

| Group            | Plugin             | Tools                                                                                |
| ---------------- | ------------------ | ------------------------------------------------------------------------------------ |
| Daily tools      | `daily-tools`      | File conversion (→ Markdown), image OCR (PaddleOCR MNN), **encryption / decryption** |
| Bidding          | `tender-optimizer` | Monte Carlo estimation for second-round bid pricing, estimation history              |
| Content creation | `text2video`       | Article → vertical scrolling short video (with drafts and processing history)        |
| Network tools    | `network-tools`    | API testing (HTTP debugging), request interception (local MITM proxy)                |
| System tools     | `system`           | Data maintenance (browse table schema, edit data)                                    |

> Each plugin's feature notes live in `plugins/<id>/README.md`; its development conventions live in
> `plugins/<id>/AGENTS.md`.

## Tech stack

| Layer             | Choice                                                                                                                 |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Desktop framework | Tauri 2 (tray-icon / macos-private-api feature)                                                                        |
| Frontend          | Vue 3 + TypeScript + Vite 6                                                                                            |
| Visuals           | Tailwind CSS 4 + shadcn-vue (Reka UI) + lucide icons                                                                   |
| State / routing   | Pinia + vue-router (hash; routes driven by the plugin registry)                                                        |
| Data              | Drizzle ORM (object-style queries on the frontend) + a Rust-built sqlx channel (SQLite) + plugin-store (settings JSON) |
| Infrastructure    | tauri-plugin-log / window-state, system tray, unified error pipeline                                                   |

## Quick start

```bash
pnpm install
pnpm tauri dev      # develop (the first Rust build takes a few minutes)
pnpm tauri build    # package
pnpm lint           # eslint + design / plugin-structure / docs checks
pnpm build          # type check + frontend build
pnpm test           # vitest unit tests
```

Requirements: Rust, Node 20+, pnpm. macOS needs Xcode Command Line Tools; Linux needs system libraries
such as webkit2gtk (see [Tauri prerequisites](https://tauri.app/start/prerequisites)).

> On first run it downloads assets on demand (Chinese fonts ~32 MB, OCR models ~15 MB) into
> `src-tauri/local-resources/`; large files aren't committed — only the license and charset text files
> are. `pnpm assets` prefetches them separately; for networks in China, mirrors and OCR build
> dependencies, see [`docs/local.md`](docs/local.md).

## Documentation

| Want to know                                                      | Look at                                                |
| ----------------------------------------------------------------- | ------------------------------------------------------ |
| Full documentation index                                          | [`docs/README.md`](docs/README.md)                     |
| Build your own app from scratch (rename / icons / packaging)      | [`docs/start.md`](docs/start.md)                       |
| Architecture and extension development (commands / DB / settings) | [`docs/extending.md`](docs/extending.md)               |
| UI design spec                                                    | [`docs/design.md`](docs/design.md) + platform chapters |
| Packaging, release and in-app updates                             | [`docs/release.md`](docs/release.md)                   |
| Local layer (personal dependencies / assets)                      | [`docs/local.md`](docs/local.md)                       |
| Development conventions, capability index, pitfalls               | [`AGENTS.md`](AGENTS.md)                               |
| TODO list                                                         | [`TODO.md`](TODO.md)                                   |

> The documentation under `docs/` is currently written in Chinese.

## This repo and upstream

This repo is derived from the upstream framework [PocketArk](https://github.com/hmilyld/PocketArk):
framework code is synced through the `upstream` git remote, while personal content (plugins,
dependencies, assets, keys) lives in the "local layer" (see [`docs/local.md`](docs/local.md)). Change
ownership is determined by [`docs/ownership.json`](docs/ownership.json), and the collaboration flow is
described in [`AGENTS.md`](AGENTS.md).

## License

This project is open-sourced under the [MIT License](LICENSE), copyright hmilyld.
