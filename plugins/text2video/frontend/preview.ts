/**
 * 浏览器预览夹具（仅 dev + 非 Tauri 环境生效）。
 *
 * 预览桥会自动并入本插件 `frontend/preview.ts` 的 **默认导出**
 * （`Record<命令名, 返回值>`），让页面在无后端时也能进入「有内容」状态，
 * 供 `node scripts/design-shot.mjs` 走查截图（见 DESIGN-appendix.md §3.5）。
 *
 * lint-plugins-ignore：本文件路径由框架预览桥按约定 glob（`plugins/<id>/frontend/preview.ts`），
 * 无法放进 `frontend/` 白名单目录；属框架扩展点，待上游把 `preview.ts` 纳入白名单后可移除。
 */
export default {
  text2video_draft_list: [
    {
      id: 1,
      title: '早起的价值',
      author: '佚名',
      content: '第一段…\n\n第二段…',
      source: 'manual',
      generatedRefId: null,
      createdAt: '2026-09-11 09:20',
      updatedAt: '2026-09-11 09:20',
    },
    {
      id: 2,
      title: '坚持的意义（AI 草稿）',
      author: '佚名',
      content: 'AI 生成内容…',
      source: 'ai',
      generatedRefId: '20260912-001',
      createdAt: '2026-09-12 20:10',
      updatedAt: '2026-09-12 20:30',
    },
  ],
  text2video_history: [
    {
      refId: '20260912-001',
      kind: 'video',
      title: '坚持的意义',
      status: 'done',
      detail: '9 段 · 42s',
      video: '/Users/me/Movies/坚持的意义.mp4',
      author: '佚名',
      source: 'ai',
      content: '正文备份…',
      createdAt: '2026-09-12 20:31',
    },
    {
      refId: '20260912-002',
      kind: 'video',
      title: '早起的价值',
      status: 'failed',
      detail: 'ffmpeg 退出码 1',
      video: '',
      author: '佚名',
      source: 'manual',
      content: '正文备份…',
      createdAt: '2026-09-12 21:02',
    },
  ],
  text2video_env_check: {
    ffmpegOk: true,
    ffmpegPath: '/opt/homebrew/bin/ffmpeg',
    fontsOk: true,
    fontPath: '/System/Library/Fonts/PingFang.ttc',
    outputDir: '/Users/me/Downloads',
  },
} satisfies Record<string, unknown>;
