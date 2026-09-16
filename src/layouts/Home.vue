<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { RouterLink } from 'vue-router';
import { getName } from '@tauri-apps/api/app';
import { Search, ArrowRight } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { getAllTools, type ToolPlugin } from '@/core/plugins';
import { useSettingsStore } from '@/stores/settings';

const settings = useSettingsStore();

const appName = ref('ArkDesk');
const query = ref('');
const filterInput = ref<InstanceType<typeof Input> | null>(null);
const now = ref(new Date());
let clock: number | undefined;

onMounted(async () => {
  try {
    appName.value = await getName();
  } catch {
    // 读取失败回退默认名
  }
  clock = window.setInterval(() => (now.value = new Date()), 10_000);
  window.addEventListener('keydown', onKeydown);
});

onBeforeUnmount(() => {
  if (clock) window.clearInterval(clock);
  window.removeEventListener('keydown', onKeydown);
});

/** 按下 / 聚焦筛选；Esc 清空并失焦（输入中不劫持） */
function onKeydown(event: KeyboardEvent): void {
  const target = event.target as HTMLElement | null;
  const typing =
    target instanceof HTMLElement &&
    (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable);
  if (event.key === '/' && !typing) {
    event.preventDefault();
    (filterInput.value?.$el as HTMLInputElement | undefined)?.focus();
  } else if (event.key === 'Escape' && document.activeElement === filterInput.value?.$el) {
    query.value = '';
    (filterInput.value?.$el as HTMLInputElement | undefined)?.blur();
  }
}

const greeting = computed(() => {
  const hour = now.value.getHours();
  if (hour < 5) return '夜深了';
  if (hour < 11) return '早上好';
  if (hour < 13) return '中午好';
  if (hour < 18) return '下午好';
  return '晚上好';
});

const dateText = computed(() =>
  now.value.toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    weekday: 'long',
  })
);

const timeText = computed(() =>
  now.value.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', hour12: false })
);

const enabledTools = computed(() =>
  getAllTools().filter((tool) => settings.isToolEnabled(tool.meta.id))
);

interface ToolGroup {
  name: string;
  tools: ToolPlugin[];
}

const groups = computed<ToolGroup[]>(() => {
  const q = query.value.trim().toLowerCase();
  const map = new Map<string, ToolPlugin[]>();
  for (const tool of enabledTools.value) {
    const group = tool.meta.group ?? '其他';
    if (q) {
      const haystack = `${tool.meta.name} ${group} ${tool.meta.description ?? ''}`.toLowerCase();
      if (!haystack.includes(q)) continue;
    }
    const list = map.get(group) ?? [];
    list.push(tool);
    map.set(group, list);
  }
  return [...map.entries()].map(([name, tools]) => ({ name, tools }));
});
</script>

<template>
  <div class="mx-auto grid w-full grid-cols-12">
    <div class="col-span-12 space-y-5 p-5 lg:col-start-2 lg:col-span-10">
      <!-- 状态台：卡片基面 + 强调色光晕，随主题/主题色联动（浅色不刺眼、深色有层次） -->
      <section
        class="home-enter relative overflow-hidden rounded-lg border bg-card px-6 py-6 shadow-sm sm:px-8"
      >
        <div
          class="pointer-events-none absolute -right-20 -top-24 size-72 rounded-full bg-primary/10 blur-3xl"
        />
        <div
          class="pointer-events-none absolute -bottom-28 -left-16 size-64 rounded-full bg-primary/5 blur-3xl"
        />
        <div class="relative flex items-start justify-between gap-6">
          <div class="min-w-0">
            <p
              class="flex items-center gap-2 font-mono text-xs uppercase tracking-[0.2em] text-muted-foreground"
            >
              <span class="inline-block size-1.5 rounded-full bg-primary" />
              {{ appName }} · 工作台
            </p>
            <h1 class="mt-3 text-3xl font-semibold tracking-tight text-foreground">
              {{ greeting }}
            </h1>
            <p class="mt-2 text-sm text-muted-foreground">
              <template v-if="enabledTools.length">
                已启用 {{ enabledTools.length }} 个工具。点选即可开始，或按
                <kbd
                  class="mx-0.5 rounded border px-1.5 py-0.5 font-mono text-[0.72rem] text-foreground/70"
                  >/</kbd
                >
                筛选。
              </template>
              <template v-else> 还没有启用的工具，先到设置里启用吧。 </template>
            </p>
          </div>
          <div class="hidden shrink-0 text-right sm:block">
            <p class="font-mono text-3xl tabular-nums tracking-tight text-foreground">
              {{ timeText }}
            </p>
            <p class="mt-1 text-xs text-muted-foreground">{{ dateText }}</p>
          </div>
        </div>
      </section>

      <!-- 筛选 -->
      <div v-if="enabledTools.length" class="relative">
        <Search
          class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
        />
        <Input
          ref="filterInput"
          v-model="query"
          type="text"
          aria-label="筛选工具"
          placeholder="筛选工具"
          class="h-10 pl-9 pr-12"
        />
        <kbd
          class="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 rounded border px-1.5 py-0.5 font-mono text-[0.72rem] text-muted-foreground"
          >/</kbd
        >
      </div>

      <!-- 工具分组 -->
      <template v-if="groups.length">
        <section v-for="group in groups" :key="group.name" class="space-y-2.5">
          <div class="flex items-baseline justify-between border-b pb-2">
            <h2 class="text-sm font-semibold tracking-tight">{{ group.name }}</h2>
            <span class="font-mono text-xs text-muted-foreground">{{ group.tools.length }}</span>
          </div>
          <div class="grid grid-cols-1 gap-2.5 sm:grid-cols-2 xl:grid-cols-3">
            <RouterLink
              v-for="tool in group.tools"
              :key="tool.meta.id"
              :to="`/tool/${tool.meta.id}`"
              class="group flex items-start gap-3 rounded-lg border bg-card p-3.5 transition-colors hover:border-primary/40 hover:bg-accent/40 focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/60"
            >
              <span
                class="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground transition-colors group-hover:text-primary"
              >
                <component :is="tool.meta.icon" class="size-4" />
              </span>
              <span class="min-w-0 flex-1">
                <span class="flex items-center gap-1.5">
                  <span class="truncate text-sm font-medium">{{ tool.meta.name }}</span>
                  <ArrowRight
                    class="size-3.5 shrink-0 -translate-x-1 text-primary opacity-0 transition-all group-hover:translate-x-0 group-hover:opacity-100"
                  />
                </span>
                <span
                  v-if="tool.meta.description"
                  class="mt-0.5 line-clamp-2 block text-xs text-muted-foreground"
                >
                  {{ tool.meta.description }}
                </span>
              </span>
            </RouterLink>
          </div>
        </section>
      </template>

      <!-- 无启用工具 -->
      <div
        v-else-if="!enabledTools.length"
        class="rounded-lg border border-dashed px-6 py-14 text-center"
      >
        <p class="text-sm font-medium">还没有启用的工具</p>
        <p class="mt-1 text-xs text-muted-foreground">
          在「设置 → 工具管理」里启用后，它们会出现在这里。
        </p>
        <Button as-child variant="secondary" size="sm" class="mt-4">
          <RouterLink to="/settings">前往设置</RouterLink>
        </Button>
      </div>

      <!-- 筛选无结果 -->
      <div v-else class="rounded-lg border border-dashed px-6 py-14 text-center">
        <p class="text-sm font-medium">没有匹配「{{ query }}」的工具</p>
        <Button variant="ghost" size="sm" class="mt-3" @click="query = ''">清除筛选</Button>
      </div>
    </div>
  </div>
</template>

<style scoped>
@media (prefers-reduced-motion: no-preference) {
  .home-enter {
    animation: home-enter 220ms ease-out both;
  }
}

@keyframes home-enter {
  from {
    opacity: 0;
    transform: translateY(4px);
  }
  to {
    opacity: 1;
    transform: none;
  }
}
</style>
