/**
 * 环境与变量 CRUD。
 */

import { ref } from 'vue';
import { db } from '@/core/db';
import { nowSql } from '../shared';

export interface EnvRow {
  id: number;
  name: string;
}

export interface EnvVarRow {
  id: number;
  environment_id: number;
  key: string;
  value: string;
  enabled: number;
  is_secret: number;
  sort_order: number;
}

function maxSort(items: { sort_order: number }[]): number {
  return items.reduce((max, item) => Math.max(max, item.sort_order), -1) + 1;
}

export function useEnvironments() {
  const environments = ref<EnvRow[]>([]);
  const vars = ref<EnvVarRow[]>([]);

  async function refresh(): Promise<void> {
    const [envs, rows] = await Promise.all([
      db.findAll<EnvRow>('network_tools_environments', { orderBy: 'id' }),
      db.findAll<EnvVarRow>('network_tools_env_vars', { orderBy: 'sort_order' }),
    ]);
    environments.value = envs;
    vars.value = rows;
  }

  function varsOf(environmentId: number | null): EnvVarRow[] {
    if (environmentId === null) return [];
    return vars.value.filter((item) => item.environment_id === environmentId);
  }

  async function createEnvironment(name: string): Promise<number> {
    const id = await db.insert('network_tools_environments', { name });
    await refresh();
    return id ?? 0;
  }

  async function renameEnvironment(id: number, name: string): Promise<void> {
    await db.updateById('network_tools_environments', id, { name, updated_at: nowSql() });
    await refresh();
  }

  async function deleteEnvironment(id: number): Promise<void> {
    await db.deleteWhere('network_tools_env_vars', { environment_id: id });
    await db.deleteById('network_tools_environments', id);
    await refresh();
  }

  async function addVar(environmentId: number, patch: Partial<EnvVarRow> = {}): Promise<void> {
    await db.insert('network_tools_env_vars', {
      environment_id: environmentId,
      key: '',
      value: '',
      enabled: 1,
      is_secret: 0,
      sort_order: maxSort(varsOf(environmentId)),
      ...patch,
    });
    await refresh();
  }

  async function updateVar(id: number, patch: Partial<EnvVarRow>): Promise<void> {
    await db.updateById('network_tools_env_vars', id, patch);
    await refresh();
  }

  async function deleteVar(id: number): Promise<void> {
    await db.deleteById('network_tools_env_vars', id);
    await refresh();
  }

  return {
    environments,
    refresh,
    varsOf,
    createEnvironment,
    renameEnvironment,
    deleteEnvironment,
    addVar,
    updateVar,
    deleteVar,
  };
}
