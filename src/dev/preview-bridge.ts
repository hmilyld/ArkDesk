/**
 * 浏览器预览桥（仅开发期、仅非 Tauri 环境生效）。
 *
 * 目的：在普通浏览器里渲染真实页面，便于设计走查与截图（无需启动原生窗口）。
 * 做法：实现 `window.__TAURI_INTERNALS__` 内部协议（invoke / 回调 / 元数据），
 * 用 localStorage 模拟 `plugin-store`，其余插件与业务命令返回中性值。
 *
 * URL 参数（方便截图不同形态）：
 *   ?theme=light|dark   ?font=13|14|15   ?platform=win   ?accent=teal
 *
 * 生产构建与本机 Tauri 运行时都不会进入此模块分支（`import.meta.env.DEV` + 无 internals）。
 */
const globalWindow = window as unknown as Record<string, unknown>;

if (import.meta.env.DEV && !globalWindow.__TAURI_INTERNALS__) {
  const params = new URLSearchParams(location.search);
  const theme = params.get('theme');
  const font = params.get('font');
  const accent = params.get('accent');
  const platform = params.get('platform');

  if (theme) localStorage.setItem('arkdesk.theme', theme);
  if (font) localStorage.setItem('arkdesk.fontSize', font);
  if (accent) localStorage.setItem('arkdesk.accent', accent);
  if (platform === 'win') document.documentElement.setAttribute('data-platform', 'win');

  type Callback = (payload: unknown) => void;
  const callbacks = new Map<number, Callback>();
  let nextCallbackId = 1;

  const stores = new Map<number, { path: string; data: Record<string, unknown> }>();
  let nextResourceId = 1;

  function storeKey(path: string): string {
    return `arkdesk.preview.store:${path}`;
  }

  function loadStore(rid: number): { path: string; data: Record<string, unknown> } {
    const store = stores.get(rid);
    if (!store) throw new Error(`[preview] store rid ${rid} 不存在`);
    return store;
  }

  function persist(rid: number): void {
    const store = loadStore(rid);
    localStorage.setItem(storeKey(store.path), JSON.stringify(store.data));
  }

  /** 业务命令的预览夹具：按需扩充，让页面拿到足够数据进入「有内容」状态 */
  const fixtures: Record<string, unknown> = {
    get_version: '0.0.0-preview',
    plugin_app_version: '0.0.0-preview',
    'plugin:app|version': '0.0.0-preview',
    'plugin:app|name': `${document.title || 'Preview'}（预览）`,
    'plugin:app|tauri_version': '2.x',
    take_pending_open: [],
    db_query_values: [],
    db_execute: { rowsAffected: 0, lastInsertId: null },
    file_read_text: '',
    task_cancel: null,
    http_request: { status: 200, headers: [], body: '' },
    http_send: { status: 200, headers: [], body: '' },
  };

  /** 预览用 SQL 夹具：只覆盖数据维护页会发出的几种查询形态 */
  function previewQuery(sql: string): { columns: string[]; rows: unknown[][] } {
    if (/FROM sqlite_master WHERE type = 'table'/.test(sql)) {
      return { columns: ['name'], rows: [['hello_tasks'], ['cap_items'], ['video_drafts']] };
    }
    if (/COUNT\(\*\) AS n FROM/.test(sql)) return { columns: ['n'], rows: [[42]] };
    if (/FROM pragma_table_info/.test(sql)) {
      return {
        columns: ['cid', 'name', 'type', 'not_null', 'dfltValue', 'pk'],
        rows: [
          [0, 'id', 'INTEGER', 0, null, 1],
          [1, 'title', 'TEXT', 1, null, 0],
          [2, 'status', 'TEXT', 1, "'pending'", 0],
          [3, 'created_at', 'TEXT', 1, null, 0],
        ],
      };
    }
    if (/SELECT sql FROM sqlite_master WHERE type = 'table'/.test(sql)) {
      return {
        columns: ['sql'],
        rows: [
          [
            "CREATE TABLE hello_tasks (\n  id INTEGER PRIMARY KEY AUTOINCREMENT,\n  title TEXT NOT NULL,\n  status TEXT NOT NULL DEFAULT 'pending',\n  created_at TEXT NOT NULL\n)",
          ],
        ],
      };
    }
    if (/SELECT rowid FROM/.test(sql)) return { columns: ['rowid'], rows: [[1]] };
    if (/SELECT rowid AS __rid, \* FROM/.test(sql)) {
      return {
        columns: ['__rid', 'id', 'title', 'status', 'created_at'],
        rows: [
          [1, 1, '整理设计规范', 'done', '2026-09-10 09:12'],
          [2, 2, '补齐 P2 组件', 'pending', '2026-09-11 14:03'],
          [3, 3, '插件层迁移', 'pending', '2026-09-12 20:41'],
        ],
      };
    }
    return { columns: ['result'], rows: [] };
  }

  function invoke(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
    // ── 事件：注册/注销返回占位 id（浏览器里不触发真实事件） ──
    if (cmd === 'plugin:event|listen') return Promise.resolve(nextCallbackId++);
    if (cmd === 'plugin:event|unlisten' || cmd === 'plugin:event|emit')
      return Promise.resolve(null);

    // ── 日志插件：静默 ──
    if (cmd === 'plugin:log|log') return Promise.resolve(null);

    // ── 存储：localStorage 兜底（设置页/主题/侧栏折叠都靠它） ──
    if (cmd === 'plugin:store|load') {
      const path = String(args.path ?? 'store.json');
      const rid = nextResourceId++;
      const raw = localStorage.getItem(storeKey(path));
      stores.set(rid, { path, data: raw ? (JSON.parse(raw) as Record<string, unknown>) : {} });
      return Promise.resolve(rid);
    }
    if (cmd === 'plugin:store|get') {
      const { data } = loadStore(Number(args.rid));
      return Promise.resolve([data[String(args.key)], String(args.key) in data]);
    }
    if (cmd === 'plugin:store|set') {
      const store = loadStore(Number(args.rid));
      store.data[String(args.key)] = args.value;
      persist(Number(args.rid));
      return Promise.resolve(null);
    }
    if (cmd === 'plugin:store|delete') {
      const store = loadStore(Number(args.rid));
      delete store.data[String(args.key)];
      persist(Number(args.rid));
      return Promise.resolve(true);
    }
    if (cmd === 'plugin:store|has') {
      const { data } = loadStore(Number(args.rid));
      return Promise.resolve(String(args.key) in data);
    }
    if (cmd === 'plugin:store|keys')
      return Promise.resolve(Object.keys(loadStore(Number(args.rid)).data));
    if (cmd === 'plugin:store|values')
      return Promise.resolve(Object.values(loadStore(Number(args.rid)).data));
    if (cmd === 'plugin:store|entries')
      return Promise.resolve(Object.entries(loadStore(Number(args.rid)).data));
    if (cmd === 'plugin:store|length')
      return Promise.resolve(Object.keys(loadStore(Number(args.rid)).data).length);
    if (cmd === 'plugin:store|save' || cmd === 'plugin:store|reload') {
      persist(Number(args.rid));
      return Promise.resolve(null);
    }
    if (cmd === 'plugin:store|reset') {
      const store = loadStore(Number(args.rid));
      store.data = {};
      persist(Number(args.rid));
      return Promise.resolve(null);
    }
    if (cmd === 'plugin:store|clear') {
      const store = loadStore(Number(args.rid));
      store.data = {};
      persist(Number(args.rid));
      return Promise.resolve(null);
    }

    // ── 窗口：查询类给中性值，操作类静默成功 ──
    if (cmd === 'plugin:window|is_maximized') return Promise.resolve(false);
    if (cmd === 'plugin:window|is_fullscreen') return Promise.resolve(false);
    if (cmd === 'plugin:window|is_minimized') return Promise.resolve(false);
    if (cmd === 'plugin:window|is_visible') return Promise.resolve(true);
    if (cmd === 'plugin:window|scale_factor') return Promise.resolve(1);
    if (cmd === 'plugin:window|theme') return Promise.resolve('dark');
    if (cmd === 'plugin:window|inner_size') return Promise.resolve({ width: 1512, height: 982 });
    if (cmd === 'plugin:window|inner_position') return Promise.resolve({ x: 0, y: 0 });
    if (cmd === 'plugin:window|outer_size') return Promise.resolve({ width: 1512, height: 982 });
    if (cmd.startsWith('plugin:window|on_')) return Promise.resolve(nextCallbackId++);
    if (cmd.startsWith('plugin:window|')) return Promise.resolve(null);

    // ── 其他插件：中性值 ──
    if (cmd === 'plugin:updater|check') return Promise.resolve(null);
    if (cmd.startsWith('plugin:updater|')) return Promise.resolve(null);
    if (cmd === 'plugin:notification|is_permission_granted') return Promise.resolve(false);
    if (cmd === 'plugin:notification|request_permission') return Promise.resolve('granted');
    if (cmd.startsWith('plugin:notification|')) return Promise.resolve(null);
    if (cmd === 'plugin:autostart|is_enabled') return Promise.resolve(false);
    if (cmd.startsWith('plugin:autostart|')) return Promise.resolve(null);
    if (cmd === 'plugin:global-shortcut|is_registered') return Promise.resolve(false);
    if (cmd.startsWith('plugin:global-shortcut|')) return Promise.resolve(null);
    if (cmd === 'plugin:clipboard-manager|read_text') return Promise.resolve('');
    if (cmd.startsWith('plugin:clipboard-manager|')) return Promise.resolve(null);
    if (cmd === 'plugin:dialog|confirm' || cmd === 'plugin:dialog|ask')
      return Promise.resolve(false);
    if (cmd.startsWith('plugin:dialog|')) return Promise.resolve(null);
    if (cmd === 'plugin:window-state|restore_state' || cmd.startsWith('plugin:window-state|'))
      return Promise.resolve(null);
    if (cmd.startsWith('plugin:opener|')) return Promise.resolve(null);
    if (cmd.startsWith('plugin:deep-link|')) return Promise.resolve(null);
    if (cmd.startsWith('plugin:log|')) return Promise.resolve(null);

    // ── text2video：列表类夹具（页面直接消费数组） ──
    if (cmd === 'text2video_draft_list') {
      return Promise.resolve([
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
      ]);
    }
    if (cmd === 'text2video_history') {
      return Promise.resolve([
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
      ]);
    }
    if (cmd === 'text2video_env_check') {
      return Promise.resolve({
        ffmpegOk: true,
        ffmpegPath: '/opt/homebrew/bin/ffmpeg',
        fontsOk: true,
        fontPath: '/System/Library/Fonts/PingFang.ttc',
        outputDir: '/Users/me/Downloads',
      });
    }

    // ── 数据库：按 SQL 形态给夹具，让「数据维护」等页面进入有内容状态 ──
    if (cmd === 'db_query_values') {
      const { sql } = args.args as { sql: string };
      return Promise.resolve(previewQuery(sql));
    }

    if (cmd in fixtures) return Promise.resolve(fixtures[cmd]);

    if (import.meta.env.DEV) console.warn(`[preview] 未模拟的命令：${cmd}`, args);
    return Promise.resolve(null);
  }

  globalWindow.__TAURI_INTERNALS__ = {
    invoke,
    transformCallback: (callback: Callback, _once = false) => {
      const id = nextCallbackId++;
      callbacks.set(id, callback);
      return id;
    },
    unregisterCallback: (id: number) => {
      callbacks.delete(id);
    },
    convertFileSrc: (path: string) => path,
    metadata: {
      currentWindow: { label: 'main' },
      currentWebview: { label: 'main' },
    },
    plugins: {},
  };

  // 提示：方便确认当前是预览环境
  console.info(
    '[preview] 浏览器预览桥已启用（Tauri API 由 localStorage/中性值模拟）',
    theme || font || accent || platform ? `· 参数 ${location.search}` : ''
  );
}
