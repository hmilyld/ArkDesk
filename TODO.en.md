# TODO

> [中文](TODO.md) · English

> To-do list. **On completion, delete the corresponding line**; register new items in the format `- [ ] YYYY-MM-DD …` (the date is the date of registration).
> For scope, see [`docs/README.md`](docs/README.md); for change ownership, see [`docs/ownership.json`](docs/ownership.json).

## To-do

- [ ] 2026-09-17 Dual-track package manager: CI uses `npm ci` + `package-lock.json`, while local and docs use pnpm — pick one and unify (belongs upstream)
- [ ] 2026-09-17 Upstream gap: `plugins/<id>/frontend/preview.ts` is not included in the `plugins/README.md` whitelist, currently exempted via `lint-plugins-ignore`; remove the exemption once upstream includes it
- [ ] 2026-09-17 Automation: `pnpm sync:upstream` (exports framework patches for PocketArk to apply, per `docs/ownership.json`) is not yet implemented
- [ ] 2026-09-17 network-tools: start / stop / system proxy / CA results still use toast (system-level operations have no in-place location for now); the sidebar's "cannot move to itself or its descendants" is constrained by the component contract
- [ ] 2026-09-17 tender-optimizer: Monte Carlo calculation has no progress or cancel (requires the backend to add channel/task parameters); the "Scoring parameters" panel has an empty `paramsRow` under the preview fixture
- [ ] 2026-09-17 On-device verification: macOS vibrancy and Windows Mica Alt look and feel; confirm item by item per [`docs/en/design-macos.md`](docs/en/design-macos.md) / [`docs/en/design-windows.md`](docs/en/design-windows.md) §11 and backfill
