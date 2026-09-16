<!-- 表清单：搜索过滤 + 行数徽标，点击选中 -->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { Badge } from '@/components/ui/badge';
import { Table2 } from '@lucide/vue';
import Panel from '@/components/tool/Panel.vue';
import EmptyState from '@/components/native/EmptyState.vue';
import LoadingState from '@/components/native/LoadingState.vue';
import SearchField from '@/components/native/SearchField.vue';
import type { TableInfo } from '../shared';

const props = defineProps<{
  tables: TableInfo[];
  selected: string | null;
  loading: boolean;
}>();

const emit = defineEmits<{ select: [name: string] }>();

const search = ref('');

const filtered = computed(() => {
  const keyword = search.value.trim().toLowerCase();
  if (!keyword) return props.tables;
  return props.tables.filter((table) => table.name.toLowerCase().includes(keyword));
});
</script>

<template>
  <Panel title="表" :hint="`${filtered.length} 张`" body-class="p-1.5 space-y-1.5">
    <template #actions>
      <SearchField v-model="search" class="w-44" placeholder="搜索表名" />
    </template>

    <LoadingState v-if="loading" variant="spinner" label="读取表结构…" />
    <EmptyState
      v-else-if="filtered.length === 0"
      :icon="Table2"
      :title="tables.length === 0 ? '数据库暂无表' : '未匹配到表'"
      :description="tables.length === 0 ? '应用首次启动会自动创建所需表' : '换一个关键词试试'"
    />
    <div v-else class="max-h-[36rem] overflow-y-auto md:max-h-[calc(100dvh-14rem)]">
      <button
        v-for="table in filtered"
        :key="table.name"
        type="button"
        class="flex w-full items-center justify-between gap-2 rounded-md px-2 py-1.5 text-left transition-colors hover:bg-accent"
        :class="
          table.name === selected
            ? 'bg-primary/10 font-medium text-foreground'
            : 'text-foreground/90'
        "
        @click="emit('select', table.name)"
      >
        <span class="min-w-0 truncate font-mono text-xs" :title="table.name">
          {{ table.name }}
        </span>
        <Badge
          v-if="table.rowCount > 0"
          variant="outline"
          class="shrink-0 font-mono text-[10px] text-muted-foreground"
        >
          {{ table.rowCount }}
        </Badge>
      </button>
    </div>
  </Panel>
</template>
