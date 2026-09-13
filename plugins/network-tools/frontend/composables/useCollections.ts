/**
 * 集合 / 文件夹 / 已保存请求的 CRUD（任意层级树）。
 */

import { ref } from 'vue';
import { db } from '@/core/db';
import {
  nowSql,
  recordToSpec,
  specToRecord,
  type HttpRequestSpec,
  type RequestMeta,
  type RequestRow,
} from '../shared';

export interface CollectionRow {
  id: number;
  parent_id: number | null;
  type: string;
  name: string;
  sort_order: number;
}

export interface RequestBriefRow {
  id: number;
  collection_id: number | null;
  name: string;
  method: string;
  url: string;
  sort_order: number;
}

function maxSort(items: { sort_order: number }[]): number {
  return items.reduce((max, item) => Math.max(max, item.sort_order), -1) + 1;
}

export function useCollections() {
  const collections = ref<CollectionRow[]>([]);
  const requests = ref<RequestBriefRow[]>([]);

  async function refresh(): Promise<void> {
    const [cols, reqs] = await Promise.all([
      db.findAll<CollectionRow>('network_tools_collections', { orderBy: 'sort_order' }),
      db.findAll<RequestBriefRow>('network_tools_requests', { orderBy: 'sort_order' }),
    ]);
    collections.value = cols;
    requests.value = reqs;
  }

  async function createCollection(
    name: string,
    parentId: number | null = null,
    type: 'folder' | 'collection' = 'folder'
  ): Promise<number> {
    const siblings = collections.value.filter((item) => item.parent_id === parentId);
    const id = await db.insert('network_tools_collections', {
      parent_id: parentId,
      type,
      name,
      sort_order: maxSort(siblings),
    });
    await refresh();
    return id ?? 0;
  }

  async function renameCollection(id: number, name: string): Promise<void> {
    await db.updateById('network_tools_collections', id, { name, updated_at: nowSql() });
    await refresh();
  }

  async function moveCollection(id: number, parentId: number | null): Promise<void> {
    const siblings = collections.value.filter(
      (item) => item.parent_id === parentId && item.id !== id
    );
    await db.updateById('network_tools_collections', id, {
      parent_id: parentId,
      sort_order: maxSort(siblings),
      updated_at: nowSql(),
    });
    await refresh();
  }

  function descendants(id: number, acc: number[] = []): number[] {
    acc.push(id);
    for (const child of collections.value.filter((item) => item.parent_id === id)) {
      descendants(child.id, acc);
    }
    return acc;
  }

  async function deleteCollection(id: number): Promise<void> {
    for (const cid of descendants(id)) {
      await db.deleteWhere('network_tools_requests', { collection_id: cid });
      await db.deleteById('network_tools_collections', cid);
    }
    await refresh();
  }

  async function saveRequest(spec: HttpRequestSpec, meta: RequestMeta): Promise<number> {
    const record = specToRecord(spec, meta);
    if (meta.id === null) {
      const siblings = requests.value.filter((item) => item.collection_id === meta.collectionId);
      const id = await db.insert('network_tools_requests', {
        ...record,
        sort_order: maxSort(siblings),
      });
      await refresh();
      return id ?? 0;
    }
    await db.updateById('network_tools_requests', meta.id, record);
    await refresh();
    return meta.id;
  }

  async function loadRequest(id: number): Promise<HttpRequestSpec | null> {
    const row = await db.findById<RequestRow>('network_tools_requests', id);
    return row ? recordToSpec(row) : null;
  }

  async function renameRequest(id: number, name: string): Promise<void> {
    await db.updateById('network_tools_requests', id, { name, updated_at: nowSql() });
    await refresh();
  }

  async function moveRequest(id: number, collectionId: number | null): Promise<void> {
    const siblings = requests.value.filter(
      (item) => item.collection_id === collectionId && item.id !== id
    );
    await db.updateById('network_tools_requests', id, {
      collection_id: collectionId,
      sort_order: maxSort(siblings),
      updated_at: nowSql(),
    });
    await refresh();
  }

  async function deleteRequest(id: number): Promise<void> {
    await db.deleteById('network_tools_requests', id);
    await refresh();
  }

  return {
    collections,
    requests,
    refresh,
    createCollection,
    renameCollection,
    moveCollection,
    deleteCollection,
    saveRequest,
    loadRequest,
    renameRequest,
    moveRequest,
    deleteRequest,
  };
}
