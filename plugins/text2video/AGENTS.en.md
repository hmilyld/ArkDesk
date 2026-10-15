# text2video Plugin — Development Conventions (AGENTS)

> For feature documentation see `README.en.md` in the same directory; project-level docs are only referenced.

## Backend

- Commands are written in `backend/mod.rs`, all with the `text2video_*` prefix, returning `Result<T, AppError>`.
- Submodules: `models.rs` (DTO/settings/table models), `pipeline.rs` (generation pipeline), `ffmpeg.rs` (encoding/decoding),
  `ai.rs` (AI copy), `sources.rs` (asset sources), `cleaner.rs` (text cleaning),
  `render/` (`background` / `fonts` / `scroll` / `text`), `migrations.rs`.
- Long-running tasks report `task://` progress and can be cancelled through the framework's `crate::tasks`; the frontend consumes them via `core/tasks`.
- Assets such as fonts come from the local layer `src-tauri/local-resources/` (see [`docs/en/local.md`](../../docs/en/local.md)); do not hardcode absolute paths inside the plugin.

## Frontend

- `views/Generator.vue` (generate), `views/Drafts.vue` (drafts), `views/History.vue` (history).
- `composables/useGeneration.ts` unifies progress/cancellation; `components/ArticleForm.vue` is the article form;
  `settings/Settings.vue` is the settings panel.
- Types/defaults are in `shared.ts`, tables in `schema.ts`; error messages uniformly use `errorMessage()` from `shared.ts`.

## Verification

```bash
pnpm lint && pnpm build
pnpm fmt:rs && pnpm lint:rs && pnpm test:rs
```
