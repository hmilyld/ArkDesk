<!-- 表清单：搜索过滤 + 行数徽标，点击选中 -->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
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
  <div class="rounded-lg border border-border bg-card">
    <div class="border-b border-border p-2.5">
      <Input v-model="search" placeholder="搜索表名…" class="h-8 bg-background text-xs" />
    </div>

    <p v-if="loading" class="py-10 text-center text-xs text-muted-foreground">加载中…</p>
    <p v-else-if="filtered.length === 0" class="py-10 text-center text-xs text-muted-foreground">
      {{ tables.length === 0 ? '数据库暂无表' : '未匹配到表' }}
    </p>

    <!-- md+ 悬浮固定：列表高度跟随视口，保证 sticky 卡片完整可见 -->
    <div v-else class="max-h-[36rem] overflow-y-auto p-1.5 md:max-h-[calc(100dvh-14rem)]">
      <p class="px-2 py-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
        表（{{ filtered.length }}）
      </p>
      <button
        v-for="table in filtered"
        :key="table.name"
        type="button"
        class="flex w-full items-center justify-between gap-2 rounded-md px-2 py-1.5 text-left transition-colors hover:bg-accent/50"
        :class="table.name === selected ? 'bg-primary/10 text-primary' : 'text-foreground/90'"
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
  </div>
</template>
