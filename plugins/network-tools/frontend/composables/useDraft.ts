/**
 * 单行草稿持久化：重启后恢复上次编辑中的请求。
 */

import { db } from '@/core/db';
import { createRequestSpec, nowSql, type HttpRequestSpec, type RequestMeta } from '../shared';

interface DraftPayload {
  spec: HttpRequestSpec;
  meta: RequestMeta;
}

export function useDraft() {
  async function save(spec: HttpRequestSpec, meta: RequestMeta): Promise<void> {
    const payload = JSON.stringify({ spec, meta });
    await db.execute(
      `INSERT INTO network_tools_drafts (id, payload, updated_at) VALUES (1, $1, $2)
       ON CONFLICT(id) DO UPDATE SET payload = excluded.payload, updated_at = excluded.updated_at`,
      [payload, nowSql()]
    );
  }

  async function load(): Promise<DraftPayload | null> {
    const rows = await db.findAll<{ payload: string }>('network_tools_drafts');
    const row = rows[0];
    if (!row) return null;
    try {
      const parsed = JSON.parse(row.payload) as DraftPayload;
      return {
        spec: parsed.spec ?? createRequestSpec(),
        meta: parsed.meta ?? { id: null, name: '未命名请求', collectionId: null },
      };
    } catch {
      return null;
    }
  }

  return { save, load };
}
