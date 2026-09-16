<!--
  表清单：搜索过滤 + 行数尾部标记，点击选中。
  模块外框由 Panel 提供；行列表用 ListRow（选中态 / hover / 键盘 Enter·Space 就地可用），
  空 / 加载 / 错误三态齐全，错误可重试。
-->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { Table2 } from '@lucide/vue';
import Panel from '@/components/tool/Panel.vue';
import EmptyState from '@/components/native/EmptyState.vue';
import ErrorState from '@/components/native/ErrorState.vue';
import ListRow from '@/components/native/ListRow.vue';
import LoadingState from '@/components/native/LoadingState.vue';
import SearchField from '@/components/native/SearchField.vue';
import type { TableInfo } from '../shared';

const props = withDefaults(
  defineProps<{
    tables: TableInfo[];
    selected: string | null;
    loading: boolean;
    /** 加载失败信息（非空时显示错误态与重试） */
    error?: string;
  }>(),
  { error: undefined }
);

const emit = defineEmits<{ select: [name: string]; retry: [] }>();

const search = ref('');

const filtered = computed(() => {
  const keyword = search.value.trim().toLowerCase();
  if (!keyword) return props.tables;
  return props.tables.filter((table) => table.name.toLowerCase().includes(keyword));
});
</script>

<template>
  <Panel title="表" :hint="`${filtered.length} 张`" body-class="space-y-2 p-2">
    <SearchField v-model="search" placeholder="搜索表名" />

    <ErrorState v-if="error" :message="error" :on-retry="() => emit('retry')" compact />
    <LoadingState v-if="loading" variant="spinner" label="读取表清单…" />
    <EmptyState
      v-else-if="filtered.length === 0 && !error"
      :icon="Table2"
      :title="tables.length === 0 ? '数据库暂无表' : '未匹配到表'"
      :description="tables.length === 0 ? '应用首次启动会自动创建所需表' : '换一个关键词试试'"
    />
    <div
      v-else-if="filtered.length > 0"
      class="max-h-[36rem] space-y-0.5 overflow-y-auto md:max-h-[calc(100dvh-14rem)]"
      role="listbox"
      aria-label="表清单"
    >
      <ListRow
        v-for="table in filtered"
        :key="table.name"
        interactive
        :selected="table.name === selected"
        :title="table.name"
        class="[&_p]:font-mono"
        @select="emit('select', table.name)"
      >
        <template #trailing>
          <span v-if="table.rowCount > 0" class="text-xs tabular-nums text-muted-foreground">
            {{ table.rowCount }}
          </span>
        </template>
      </ListRow>
    </div>
  </Panel>
</template>
