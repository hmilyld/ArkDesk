# text2video 图文视频插件

将文章渲染为竖屏滚动短视频，含草稿箱与处理记录。开发约定见同目录 [`AGENTS.md`](AGENTS.md)。

## 工具

| 工具     | 入口                           | 说明                          |
| -------- | ------------------------------ | ----------------------------- |
| 生成     | `frontend/views/Generator.vue` | 手动/AI 文案 → 视频，支持批量 |
| 草稿箱   | `frontend/views/Drafts.vue`    | 保存、编辑、批量生成          |
| 处理记录 | `frontend/views/History.vue`   | 打开产物、显示位置、删除      |

## 依赖与资源

- 视频编码依赖系统 `ffmpeg`/`ffprobe`；字体等资源位于本地层 `src-tauri/local-resources/`
  （下载方式见 [`docs/local.md`](../../docs/local.md)）。
