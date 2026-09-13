# text2video 插件 — 开发约定（AGENTS）

> 功能说明见同目录 `README.md`；项目级文档只做引用。

## 后端

- 命令写在 `backend/mod.rs`，一律 `text2video_*` 前缀，返回 `Result<T, AppError>`。
- 子模块：`models.rs`（DTO/设置/表模型）、`pipeline.rs`（生成流水线）、`ffmpeg.rs`（编解码）、
  `ai.rs`（AI 文案）、`sources.rs`（素材来源）、`cleaner.rs`（文本清洗）、
  `render/`（`background` / `fonts` / `scroll` / `text`）、`migrations.rs`。
- 长任务经框架 `crate::tasks` 回传 `task://` 进度并可取消；前端 `core/tasks` 消费。
- 字体等资源来自本地层 `src-tauri/local-resources/`（见根 `LOCAL.md`），勿在插件内硬编码绝对路径。

## 前端

- `views/Generator.vue`（生成）、`views/Drafts.vue`（草稿箱）、`views/History.vue`（记录）。
- `composables/useGeneration.ts` 统一进度/取消；`components/ArticleForm.vue` 文章表单；
  `settings/Settings.vue` 设置面板。
- 类型/默认值在 `shared.ts`，表在 `schema.ts`；错误提示统一用 `shared.ts` 的 `errorMessage()`。

## 校验

```bash
pnpm lint && pnpm build
pnpm fmt:rs && pnpm lint:rs && pnpm test:rs
```
