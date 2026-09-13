/**
 * 请求历史：每次发送自动落库，按上限裁剪，可清空 / 重放 / 删除。
 */

import { ref } from 'vue';
import { db } from '@/core/db';

export interface HistoryRow {
  id: number;
  method: string;
  url: string;
  request: string;
  status: number | null;
  ok: number | null;
  elapsed_ms: number | null;
  size_bytes: number | null;
  response_headers: string;
  response_body: string | null;
  body_truncated: number;
  error: string | null;
  environment_id: number | null;
  created_at: string;
}

export interface HistoryEntry {
  method: string;
  url: string;
  request: string;
  status: number | null;
  ok: boolean | null;
  elapsedMs: number | null;
  sizeBytes: number | null;
  responseHeaders: string;
  responseBody: string | null;
  bodyTruncated: boolean;
  error: string | null;
  environmentId: number | null;
}

export function useHistory() {
  const items = ref<HistoryRow[]>([]);

  async function refresh(limit = 200): Promise<void> {
    items.value = await db.findAll<HistoryRow>('network_tools_history', {
      orderBy: 'id DESC',
      limit,
    });
  }

  async function record(entry: HistoryEntry): Promise<void> {
    await db.insert('network_tools_history', {
      method: entry.method,
      url: entry.url,
      request: entry.request,
      status: entry.status,
      ok: entry.ok === null ? null : entry.ok ? 1 : 0,
      elapsed_ms: entry.elapsedMs,
      size_bytes: entry.sizeBytes,
      response_headers: entry.responseHeaders,
      response_body: entry.responseBody,
      body_truncated: entry.bodyTruncated ? 1 : 0,
      error: entry.error,
      environment_id: entry.environmentId,
    });
  }

  async function prune(limit: number): Promise<void> {
    if (limit <= 0) return;
    await db.execute(
      'DELETE FROM network_tools_history WHERE id NOT IN (SELECT id FROM network_tools_history ORDER BY id DESC LIMIT $1)',
      [limit]
    );
  }

  async function remove(id: number): Promise<void> {
    await db.deleteById('network_tools_history', id);
  }

  async function clear(): Promise<void> {
    await db.execute('DELETE FROM network_tools_history');
  }

  return { items, refresh, record, prune, remove, clear };
}
